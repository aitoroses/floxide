//! Redis implementation of the RunInfoStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use floxide_core::distributed::{RunInfo, RunInfoError, RunInfoStore, RunStatus};
use redis::AsyncCommands;
use tracing::{error, instrument, trace};

/// Redis implementation of the RunInfoStore trait.
#[derive(Clone)]
pub struct RedisRunInfoStore {
    client: RedisClient,
}

impl RedisRunInfoStore {
    /// Create a new Redis run info store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self { client }
    }

    /// Get the Redis key for a specific run info.
    fn run_info_key(&self, run_id: &str) -> String {
        self.client.prefixed_key(&format!("run_info:{}", run_id))
    }

    /// Get the Redis key for the set of all run IDs.
    fn all_runs_key(&self) -> String {
        self.client.prefixed_key("all_runs")
    }

    /// Get the Redis key for the set of run IDs with a specific status.
    fn status_runs_key(&self, status: &RunStatus) -> String {
        self.client.prefixed_key(&format!("status:{:?}", status))
    }
}

#[async_trait]
impl RunInfoStore for RedisRunInfoStore {
    #[instrument(skip(self, info), level = "trace")]
    async fn insert_run(&self, info: RunInfo) -> Result<(), RunInfoError> {
        let run_key = self.run_info_key(&info.run_id);
        let all_runs_key = self.all_runs_key();
        let status_key = self.status_runs_key(&info.status);

        // Convert to serializable run info

        // Serialize the run info
        let serialized = serde_json::to_string(&info).map_err(|e| {
            error!("Failed to serialize run info: {}", e);
            RunInfoError::Other(format!("Serialization error: {}", e))
        })?;

        // Use a Redis pipeline to atomically:
        // 1. Store the serialized run info
        // 2. Add the run ID to the set of all runs
        // 3. Add the run ID to the set of runs with this status
        let mut conn = self.client.conn.clone();
        let _result: () = redis::pipe()
            .set(&run_key, serialized)
            .sadd(&all_runs_key, &info.run_id)
            .sadd(&status_key, &info.run_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while inserting run info: {}", e);
                RunInfoError::Io(e.to_string())
            })?;

        trace!("Inserted run info for run {}", info.run_id);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn update_status(&self, run_id: &str, status: RunStatus) -> Result<(), RunInfoError> {
        let run_key = self.run_info_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current run info
        let result: Option<String> = conn.get(&run_key).await.map_err(|e| {
            error!("Redis error while getting run info: {}", e);
            RunInfoError::Io(e.to_string())
        })?;

        let mut info = match result {
            Some(serialized) => serde_json::from_str::<RunInfo>(&serialized).map_err(|e| {
                error!("Failed to deserialize run info: {}", e);
                RunInfoError::Other(format!("Deserialization error: {}", e))
            })?,
            None => return Err(RunInfoError::NotFound),
        };

        // Get the old status key
        let old_status_key = self.status_runs_key(&info.status);

        // Update the status
        info.status = status.clone();

        // Get the new status key
        let new_status_key = self.status_runs_key(&info.status);

        // Serialize the updated run info
        let serialized = serde_json::to_string(&info).map_err(|e| {
            error!("Failed to serialize run info: {}", e);
            RunInfoError::Other(format!("Serialization error: {}", e))
        })?;

        // Use a Redis pipeline to atomically:
        // 1. Store the updated run info
        // 2. Remove the run ID from the old status set
        // 3. Add the run ID to the new status set
        let _result: () = redis::pipe()
            .set(&run_key, serialized)
            .srem(&old_status_key, run_id)
            .sadd(&new_status_key, run_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while updating run status: {}", e);
                RunInfoError::Io(e.to_string())
            })?;

        trace!("Updated status for run {} to {:?}", run_id, status);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn update_finished_at(
        &self,
        run_id: &str,
        finished_at: DateTime<Utc>,
    ) -> Result<(), RunInfoError> {
        let run_key = self.run_info_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current run info
        let result: Option<String> = conn.get(&run_key).await.map_err(|e| {
            error!("Redis error while getting run info: {}", e);
            RunInfoError::Io(e.to_string())
        })?;

        let mut info = match result {
            Some(serialized) => serde_json::from_str::<RunInfo>(&serialized).map_err(|e| {
                error!("Failed to deserialize run info: {}", e);
                RunInfoError::Other(format!("Deserialization error: {}", e))
            })?,
            None => return Err(RunInfoError::NotFound),
        };

        // Update the finished_at timestamp
        info.finished_at = Some(finished_at);

        // Serialize the updated run info
        let serialized = serde_json::to_string(&info).map_err(|e| {
            error!("Failed to serialize run info: {}", e);
            RunInfoError::Other(format!("Serialization error: {}", e))
        })?;

        // Store the updated run info
        let _result: () = conn.set(&run_key, serialized).await.map_err(|e| {
            error!("Redis error while updating finished_at: {}", e);
            RunInfoError::Io(e.to_string())
        })?;

        trace!("Updated finished_at for run {} to {}", run_id, finished_at);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn get_run(&self, run_id: &str) -> Result<Option<RunInfo>, RunInfoError> {
        let run_key = self.run_info_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the run info
        let result: Option<String> = conn.get(&run_key).await.map_err(|e| {
            error!("Redis error while getting run info: {}", e);
            RunInfoError::Io(e.to_string())
        })?;

        // If the run info exists, deserialize it
        if let Some(serialized) = result {
            let info = serde_json::from_str::<RunInfo>(&serialized).map_err(|e| {
                error!("Failed to deserialize run info: {}", e);
                RunInfoError::Other(format!("Deserialization error: {}", e))
            })?;

            trace!("Got run info for run {}", run_id);
            Ok(Some(info))
        } else {
            trace!("No run info found for run {}", run_id);
            Ok(None)
        }
    }

    #[instrument(skip(self), level = "trace")]
    async fn list_runs(&self, filter: Option<RunStatus>) -> Result<Vec<RunInfo>, RunInfoError> {
        let mut conn = self.client.conn.clone();

        // Get the set of run IDs to list
        let run_ids: Vec<String> = match filter {
            Some(status) => {
                let status_key = self.status_runs_key(&status);
                conn.smembers(&status_key).await.map_err(|e| {
                    error!("Redis error while getting run IDs by status: {}", e);
                    RunInfoError::Io(e.to_string())
                })?
            }
            None => {
                let all_runs_key = self.all_runs_key();
                conn.smembers(&all_runs_key).await.map_err(|e| {
                    error!("Redis error while getting all run IDs: {}", e);
                    RunInfoError::Io(e.to_string())
                })?
            }
        };

        // Get the run info for each run ID
        let mut runs = Vec::with_capacity(run_ids.len());
        for run_id in run_ids {
            let run_key = self.run_info_key(&run_id);
            let result: Option<String> = conn.get(&run_key).await.map_err(|e| {
                error!("Redis error while getting run info: {}", e);
                RunInfoError::Io(e.to_string())
            })?;

            if let Some(serialized) = result {
                let info = serde_json::from_str::<RunInfo>(&serialized).map_err(|e| {
                    error!("Failed to deserialize run info: {}", e);
                    RunInfoError::Other(format!("Deserialization error: {}", e))
                })?;
                runs.push(info);
            }
        }

        trace!("Listed {} runs", runs.len());
        Ok(runs)
    }

    async fn update_output(
        &self,
        run_id: &str,
        output: serde_json::Value,
    ) -> Result<(), RunInfoError> {
        let run_key = self.run_info_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current run info
        let result: Option<String> = conn.get(&run_key).await.map_err(|e| {
            error!("Redis error while getting run info: {}", e);
            RunInfoError::Io(e.to_string())
        })?;

        let mut info = match result {
            Some(serialized) => serde_json::from_str::<RunInfo>(&serialized).map_err(|e| {
                error!("Failed to deserialize run info: {}", e);
                RunInfoError::Other(format!("Deserialization error: {}", e))
            })?,
            None => return Err(RunInfoError::NotFound),
        };

        // Update the output field
        info.output = Some(output);

        // Serialize the updated run info
        let serialized = serde_json::to_string(&info).map_err(|e| {
            error!("Failed to serialize run info: {}", e);
            RunInfoError::Other(format!("Serialization error: {}", e))
        })?;

        // Store the updated run info
        let _result: () = conn.set(&run_key, serialized).await.map_err(|e| {
            error!("Redis error while updating output: {}", e);
            RunInfoError::Io(e.to_string())
        })?;

        trace!("Updated output for run {}", run_id);
        Ok(())
    }
}
