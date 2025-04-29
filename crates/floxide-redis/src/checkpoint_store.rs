//! Redis implementation of the CheckpointStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::{
    checkpoint::{Checkpoint, CheckpointError, CheckpointStore},
    context::Context,
    workflow::WorkItem,
};
use redis::AsyncCommands;
use tracing::{error, instrument, trace};

/// Redis implementation of the CheckpointStore trait.
#[derive(Clone)]
pub struct RedisCheckpointStore<C: Context, WI: WorkItem> {
    client: RedisClient,
    _phantom_c: std::marker::PhantomData<C>,
    _phantom_wi: std::marker::PhantomData<WI>,
}

impl<C: Context, WI: WorkItem> RedisCheckpointStore<C, WI> {
    /// Create a new Redis checkpoint store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self {
            client,
            _phantom_c: std::marker::PhantomData,
            _phantom_wi: std::marker::PhantomData,
        }
    }

    /// Get the Redis key for the checkpoint for a specific workflow run.
    fn checkpoint_key(&self, workflow_id: &str) -> String {
        self.client.prefixed_key(&format!("checkpoint:{}", workflow_id))
    }
}

#[async_trait]
impl<C, WI> CheckpointStore<C, WI> for RedisCheckpointStore<C, WI>
where
    C: Context,
    WI: WorkItem,
{
    #[instrument(skip(self, checkpoint), level = "trace")]
    async fn save(
        &self,
        workflow_id: &str,
        checkpoint: &Checkpoint<C, WI>,
    ) -> Result<(), CheckpointError> {
        let key = self.checkpoint_key(workflow_id);

        // Serialize the checkpoint
        let serialized = serde_json::to_string(&checkpoint).map_err(|e| {
            error!("Failed to serialize checkpoint: {}", e);
            CheckpointError::Store(format!("Serialization error: {}", e))
        })?;

        // Store the serialized checkpoint in Redis
        let mut conn = self.client.conn.clone();
        let _result: () = conn.set(&key, serialized)
            .await
            .map_err(|e| {
                error!("Redis error while saving checkpoint: {}", e);
                CheckpointError::Store(format!("Redis error: {}", e))
            })?;

        trace!("Saved checkpoint for workflow {}", workflow_id);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<C, WI>>, CheckpointError> {
        let key = self.checkpoint_key(workflow_id);
        let mut conn = self.client.conn.clone();

        // Get the serialized checkpoint from Redis
        let result: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| {
                error!("Redis error while loading checkpoint: {}", e);
                CheckpointError::Store(format!("Redis error: {}", e))
            })?;

        // If the checkpoint exists, deserialize it
        if let Some(serialized) = result {
            let serializable: Checkpoint<C, WI> = serde_json::from_str(&serialized).map_err(|e| {
                println!("Serialized: {}", serialized);
                error!("Failed to deserialize checkpoint: {}", e);
                CheckpointError::Store(format!("Deserialization error: {}", e))
            })?;

            // Convert from serializable checkpoint
            let checkpoint = Checkpoint::from(serializable);

            trace!("Loaded checkpoint for workflow {}", workflow_id);
            Ok(Some(checkpoint))
        } else {
            trace!("No checkpoint found for workflow {}", workflow_id);
            Ok(None)
        }
    }
}
