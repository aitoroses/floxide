//! Redis implementation of the MetricsStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::distributed::{MetricsError, MetricsStore, RunMetrics};
use redis::AsyncCommands;
use tracing::{error, instrument, trace};

/// Redis implementation of the MetricsStore trait.
#[derive(Clone)]
pub struct RedisMetricsStore {
    client: RedisClient,
}

impl RedisMetricsStore {
    /// Create a new Redis metrics store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self { client }
    }

    /// Get the Redis key for metrics for a specific run.
    fn metrics_key(&self, run_id: &str) -> String {
        self.client.prefixed_key(&format!("metrics:{}", run_id))
    }
}

#[async_trait]
impl MetricsStore for RedisMetricsStore {
    #[instrument(skip(self, metrics), level = "trace")]
    async fn update_metrics(&self, run_id: &str, metrics: RunMetrics) -> Result<(), MetricsError> {
        let key = self.metrics_key(run_id);

        // Serialize the metrics
        let serialized = serde_json::to_string(&metrics).map_err(|e| {
            error!("Failed to serialize metrics: {}", e);
            MetricsError::Other(format!("Serialization error: {}", e))
        })?;

        // Store the serialized metrics in Redis
        let mut conn = self.client.conn.clone();
        let _result: () = conn.set(&key, serialized).await.map_err(|e| {
            error!("Redis error while updating metrics: {}", e);
            MetricsError::Other(format!("Redis error: {}", e))
        })?;

        trace!("Updated metrics for run {}", run_id);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn get_metrics(&self, run_id: &str) -> Result<Option<RunMetrics>, MetricsError> {
        let key = self.metrics_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the serialized metrics from Redis
        let result: Option<String> = conn.get(&key).await.map_err(|e| {
            error!("Redis error while getting metrics: {}", e);
            MetricsError::Other(format!("Redis error: {}", e))
        })?;

        // If the metrics exist, deserialize them
        if let Some(serialized) = result {
            let metrics = serde_json::from_str(&serialized).map_err(|e| {
                error!("Failed to deserialize metrics: {}", e);
                MetricsError::Other(format!("Deserialization error: {}", e))
            })?;

            trace!("Got metrics for run {}", run_id);
            Ok(Some(metrics))
        } else {
            trace!("No metrics found for run {}", run_id);
            Ok(None)
        }
    }
}
