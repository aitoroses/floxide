//! Redis implementation of the ErrorStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::distributed::{ErrorStore, ErrorStoreError, WorkflowError};
use redis::AsyncCommands;
use tracing::{error, instrument, trace};

/// Redis implementation of the ErrorStore trait.
#[derive(Clone)]
pub struct RedisErrorStore {
    client: RedisClient,
}

impl RedisErrorStore {
    /// Create a new Redis error store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self { client }
    }

    /// Get the Redis key for errors for a specific run.
    fn errors_key(&self, run_id: &str) -> String {
        self.client.prefixed_key(&format!("errors:{}", run_id))
    }
}

#[async_trait]
impl ErrorStore for RedisErrorStore {
    #[instrument(skip(self, error_info), level = "trace")]
    async fn record_error(
        &self,
        run_id: &str,
        error_info: WorkflowError,
    ) -> Result<(), ErrorStoreError> {
        let key = self.errors_key(run_id);
        
        // Serialize the error
        let serialized = serde_json::to_string(&error_info).map_err(|e| {
            error!("Failed to serialize error: {}", e);
            ErrorStoreError::Other(format!("Serialization error: {}", e))
        })?;
        
        // Add the serialized error to the list in Redis
        let mut conn = self.client.conn.clone();
        let _result: () = conn.rpush(&key, serialized)
            .await
            .map_err(|e| {
                error!("Redis error while recording error: {}", e);
                ErrorStoreError::Other(format!("Redis error: {}", e))
            })?;
        
        trace!("Recorded error for run {}", run_id);
        Ok(())
    }

    async fn get_errors(&self, run_id: &str) -> Result<Vec<WorkflowError>, ErrorStoreError> {
        let key = self.errors_key(run_id);
        let mut conn = self.client.conn.clone();
        
        // Get all errors from the list in Redis
        let results: Vec<String> = conn
            .lrange(&key, 0, -1)
            .await
            .map_err(|e| {
                error!("Redis error while getting errors: {}", e);
                ErrorStoreError::Other(format!("Redis error: {}", e))
            })?;
        
        // Deserialize each error
        let mut errors = Vec::with_capacity(results.len());
        for serialized in results {
            let error_info = serde_json::from_str(&serialized).map_err(|e| {
                error!("Failed to deserialize error: {}", e);
                ErrorStoreError::Other(format!("Deserialization error: {}", e))
            })?;
            errors.push(error_info);
        }
        
        trace!("Got {} errors for run {}", errors.len(), run_id);
        Ok(errors)
    }
}
