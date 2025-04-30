//! Redis implementation of the ContextStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::{
    context::Context,
    distributed::context_store::{ContextStore, ContextStoreError},
    merge::Merge,
};
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{error, instrument, trace};

/// Redis implementation of the ContextStore trait.
#[derive(Clone)]
pub struct RedisContextStore<C: Context + Merge + Default> {
    client: RedisClient,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Context + Merge + Default> RedisContextStore<C> {
    /// Create a new Redis context store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self {
            client,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the Redis key for the context for a specific run.
    fn context_key(&self, run_id: &str) -> String {
        self.client.prefixed_key(&format!("context:{}", run_id))
    }
}

#[async_trait]
impl<C> ContextStore<C> for RedisContextStore<C>
where
    C: Context + Merge + Default + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    #[instrument(skip(self), level = "trace")]
    async fn get(&self, run_id: &str) -> Result<Option<C>, ContextStoreError> {
        let key = self.context_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the serialized context from Redis
        let result: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| {
                error!("Redis error while getting context: {}", e);
                ContextStoreError::Io(e.to_string())
            })?;

        // If the context exists, deserialize it
        if let Some(serialized) = result {
            let context = serde_json::from_str(&serialized).map_err(|e| {
                error!("Failed to deserialize context: {}", e);
                ContextStoreError::Other(format!("Deserialization error: {}", e))
            })?;
            trace!("Got context for run {}", run_id);
            Ok(Some(context))
        } else {
            trace!("No context found for run {}", run_id);
            Ok(None)
        }
    }

    #[instrument(skip(self, ctx), level = "trace")]
    async fn set(&self, run_id: &str, ctx: C) {
        let key = self.context_key(run_id);
        let mut conn = self.client.conn.clone();

        // Serialize the context
        let serialized = match serde_json::to_string(&ctx) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to serialize context: {}", e);
                return;
            }
        };

        // Store the serialized context in Redis
        if let Err(e) = conn.set(&key, serialized).await as Result<(), _> {
            error!("Redis error while setting context: {}", e);
        } else {
            trace!("Set context for run {}", run_id);
        }
    }

    #[instrument(skip(self, ctx), level = "trace")]
    async fn merge(&self, run_id: &str, ctx: C) {
        let key = self.context_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current context from Redis
        let current: Option<String> = match conn.get(&key).await {
            Ok(val) => val,
            Err(e) => {
                error!("Redis error while getting context for merge: {}", e);
                return;
            }
        };

        let merged = if let Some(serialized) = current {
            match serde_json::from_str::<C>(&serialized) {
                Ok(mut existing) => {
                    existing.merge(ctx);
                    existing
                }
                Err(e) => {
                    error!("Failed to deserialize context for merge: {}", e);
                    ctx
                }
            }
        } else {
            ctx
        };

        // Serialize the merged context
        let serialized = match serde_json::to_string(&merged) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to serialize merged context: {}", e);
                return;
            }
        };

        // Store the merged context in Redis
        if let Err(e) = conn.set(&key, serialized).await as Result<(), _> {
            error!("Redis error while setting merged context: {}", e);
        } else {
            trace!("Merged context for run {}", run_id);
        }
    }
} 