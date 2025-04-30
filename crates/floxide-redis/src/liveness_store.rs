//! Redis implementation of the LivenessStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use floxide_core::distributed::{LivenessStore, LivenessStoreError, WorkerHealth};
use redis::AsyncCommands;
use tracing::{error, instrument, trace};

/// Redis implementation of the LivenessStore trait.
#[derive(Clone)]
pub struct RedisLivenessStore {
    client: RedisClient,
}

impl RedisLivenessStore {
    /// Create a new Redis liveness store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self { client }
    }

    /// Get the Redis key for heartbeats.
    fn heartbeats_key(&self) -> String {
        self.client.prefixed_key("heartbeats")
    }

    /// Get the Redis key for worker health.
    fn health_key(&self) -> String {
        self.client.prefixed_key("worker_health")
    }

    /// Get the Redis key for the set of all worker IDs.
    fn workers_key(&self) -> String {
        self.client.prefixed_key("workers")
    }
}

#[async_trait]
impl LivenessStore for RedisLivenessStore {
    #[instrument(skip(self), level = "trace")]
    async fn update_heartbeat(
        &self,
        worker_id: usize,
        timestamp: DateTime<Utc>,
    ) -> Result<(), LivenessStoreError> {
        let heartbeats_key = self.heartbeats_key();
        let workers_key = self.workers_key();
        let mut conn = self.client.conn.clone();

        // Use a Redis pipeline to atomically:
        // 1. Store the heartbeat timestamp
        // 2. Add the worker ID to the set of all workers
        let _result: () = redis::pipe()
            .hset(
                &heartbeats_key,
                worker_id.to_string(),
                timestamp.to_rfc3339(),
            )
            .sadd(&workers_key, worker_id.to_string())
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while updating heartbeat: {}", e);
                LivenessStoreError::Io(e.to_string())
            })?;

        trace!(
            "Updated heartbeat for worker {} to {}",
            worker_id,
            timestamp
        );
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn get_heartbeat(
        &self,
        worker_id: usize,
    ) -> Result<Option<DateTime<Utc>>, LivenessStoreError> {
        let heartbeats_key = self.heartbeats_key();
        let mut conn = self.client.conn.clone();

        // Get the heartbeat timestamp from Redis
        let result: Option<String> = conn
            .hget(&heartbeats_key, worker_id.to_string())
            .await
            .map_err(|e| {
                error!("Redis error while getting heartbeat: {}", e);
                LivenessStoreError::Io(e.to_string())
            })?;

        // If the heartbeat exists, parse it
        if let Some(timestamp_str) = result {
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| {
                    error!("Failed to parse timestamp: {}", e);
                    LivenessStoreError::Other(format!("Timestamp parsing error: {}", e))
                })?
                .with_timezone(&Utc);

            trace!("Got heartbeat for worker {}: {}", worker_id, timestamp);
            Ok(Some(timestamp))
        } else {
            trace!("No heartbeat found for worker {}", worker_id);
            Ok(None)
        }
    }

    #[instrument(skip(self), level = "trace")]
    async fn list_workers(&self) -> Result<Vec<usize>, LivenessStoreError> {
        let workers_key = self.workers_key();
        let mut conn = self.client.conn.clone();

        // Get all worker IDs from Redis
        let worker_ids: Vec<String> = conn.smembers(&workers_key).await.map_err(|e| {
            error!("Redis error while listing workers: {}", e);
            LivenessStoreError::Io(e.to_string())
        })?;

        // Parse each worker ID
        let mut workers = Vec::with_capacity(worker_ids.len());
        for id_str in worker_ids {
            let id = id_str.parse::<usize>().map_err(|e| {
                error!("Failed to parse worker ID: {}", e);
                LivenessStoreError::Other(format!("Worker ID parsing error: {}", e))
            })?;
            workers.push(id);
        }

        trace!("Listed {} workers", workers.len());
        Ok(workers)
    }

    #[instrument(skip(self), level = "trace")]
    async fn update_health(
        &self,
        worker_id: usize,
        health: WorkerHealth,
    ) -> Result<(), LivenessStoreError> {
        let health_key = self.health_key();
        let mut conn = self.client.conn.clone();

        // Serialize the health status
        let serialized = serde_json::to_string(&health).map_err(|e| {
            error!("Failed to serialize health status: {}", e);
            LivenessStoreError::Other(format!("Serialization error: {}", e))
        })?;

        // Store the health status in Redis
        let _result: () = conn
            .hset(&health_key, worker_id.to_string(), serialized)
            .await
            .map_err(|e| {
                error!("Redis error while updating health: {}", e);
                LivenessStoreError::Io(e.to_string())
            })?;

        trace!("Updated health for worker {} to {:?}", worker_id, health);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn get_health(
        &self,
        worker_id: usize,
    ) -> Result<Option<WorkerHealth>, LivenessStoreError> {
        let health_key = self.health_key();
        let mut conn = self.client.conn.clone();

        // Get the health status from Redis
        let result: Option<String> = conn
            .hget(&health_key, worker_id.to_string())
            .await
            .map_err(|e| {
                error!("Redis error while getting health: {}", e);
                LivenessStoreError::Io(e.to_string())
            })?;

        // If the health status exists, deserialize it
        if let Some(serialized) = result {
            let health = serde_json::from_str(&serialized).map_err(|e| {
                error!("Failed to deserialize health status: {}", e);
                LivenessStoreError::Other(format!("Deserialization error: {}", e))
            })?;

            trace!("Got health for worker {}: {:?}", worker_id, health);
            Ok(Some(health))
        } else {
            trace!("No health status found for worker {}", worker_id);
            Ok(None)
        }
    }

    #[instrument(skip(self), level = "trace")]
    async fn list_health(&self) -> Result<Vec<WorkerHealth>, LivenessStoreError> {
        let health_key = self.health_key();
        let workers_key = self.workers_key();
        let mut conn = self.client.conn.clone();

        // Get all worker IDs
        let worker_ids: Vec<String> = conn.smembers(&workers_key).await.map_err(|e| {
            error!("Redis error while listing workers: {}", e);
            LivenessStoreError::Io(e.to_string())
        })?;

        // Get the health status for each worker
        let mut health_statuses = Vec::with_capacity(worker_ids.len());
        for id_str in worker_ids {
            let result: Option<String> = conn.hget(&health_key, &id_str).await.map_err(|e| {
                error!("Redis error while getting health: {}", e);
                LivenessStoreError::Io(e.to_string())
            })?;

            if let Some(serialized) = result {
                let health = serde_json::from_str(&serialized).map_err(|e| {
                    error!("Failed to deserialize health status: {}", e);
                    LivenessStoreError::Other(format!("Deserialization error: {}", e))
                })?;
                health_statuses.push(health);
            }
        }

        trace!("Listed {} health statuses", health_statuses.len());
        Ok(health_statuses)
    }
}
