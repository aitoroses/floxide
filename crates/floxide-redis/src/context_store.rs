//! Redis implementation of the ContextStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::{
    context::Context,
    distributed::context_store::{ContextStore, ContextStoreError},
    merge::Merge,
};
use redis::{AsyncCommands, Value};
use serde::{de::DeserializeOwned, Serialize};
use tokio::time::{sleep, Duration};
use tracing::{error, instrument, trace, warn};
use rand::Rng;

const LOCK_TIMEOUT_MS: usize = 5000; // 5 seconds lock validity
const MAX_LOCK_RETRIES: usize = 10; // Max attempts to acquire lock
const BASE_RETRY_DELAY_MS: u64 = 50; // Min delay between retries
const MAX_RETRY_DELAY_MS: u64 = 500; // Max delay between retries

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

    /// Get the Redis key for the context lock for a specific run.
    fn lock_key(&self, run_id: &str) -> String {
        self.client.prefixed_key(&format!("lock:context:{}", run_id))
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
    async fn set(&self, run_id: &str, ctx: C) -> Result<(), ContextStoreError> {
        let key = self.context_key(run_id);
        let mut conn = self.client.conn.clone();

        // Serialize the context
        let serialized = match serde_json::to_string(&ctx) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to serialize context: {}", e);
                return Err(ContextStoreError::Other(format!("Serialization error: {}", e)));
            }
        };

        // Store the serialized context in Redis
        if let Err(e) = conn.set(&key, serialized).await as Result<(), _> {
            error!("Redis error while setting context: {}", e);
            return Err(ContextStoreError::Other(format!("Redis error while setting context: {}", e)));
        } else {
            trace!("Set context for run {}", run_id);
            Ok(())
        }
    }

    #[instrument(skip(self, ctx), level = "trace")]
    async fn merge(&self, run_id: &str, ctx: C) -> Result<(), ContextStoreError> {
        let key = self.context_key(run_id);
        let lock_key = self.lock_key(run_id);
        let lock_value = format!("worker_{}", rand::thread_rng().gen::<u32>()); // Unique value for this attempt
        let mut conn = self.client.conn.clone();
        let mut acquired_lock = false;

        // --- Attempt to acquire lock ---
        for attempt in 0..MAX_LOCK_RETRIES {
            trace!(run_id, attempt, "Attempting to acquire context lock");
            // Use redis::cmd for SET NX PX
            let result: Result<Value, redis::RedisError> = redis::cmd("SET")
                .arg(&lock_key)
                .arg(&lock_value)
                .arg("NX")
                .arg("PX")
                .arg(LOCK_TIMEOUT_MS)
                .query_async(&mut conn) // Pass mutable connection
                .await;

            match result {
                Ok(Value::Okay) => {
                    trace!(run_id, "Successfully acquired context lock");
                    acquired_lock = true;
                    break; // Lock acquired, exit loop
                }
                Ok(Value::Nil) => {
                    trace!(run_id, "Context lock already held, retrying...");
                }
                Ok(other) => {
                    warn!(run_id, ?other, "Unexpected response from Redis SET NX PX while acquiring lock");
                }
                Err(e) => {
                    error!(run_id, error = %e, "Redis error while acquiring context lock");
                    // Depending on error, might want to break early
                }
            }

            // Wait with random backoff before next attempt
            let delay = rand::thread_rng().gen_range(BASE_RETRY_DELAY_MS..=MAX_RETRY_DELAY_MS);
            trace!(run_id, attempt, delay_ms = delay, "Waiting before lock retry");
            sleep(Duration::from_millis(delay)).await;
        }

        if !acquired_lock {
            error!(run_id, "Failed to acquire context lock after {} retries, aborting merge", MAX_LOCK_RETRIES);
            return Err(ContextStoreError::Other(format!("Failed to acquire context lock after {} retries, aborting merge", MAX_LOCK_RETRIES)));
        }

        // --- Lock Acquired: Perform Read-Modify-Write ---
        // Use a block to manage the RMW logic and ensure lock release
        let rmw_result = async {
            // Get the current context from Redis
            let current: Option<String> = match conn.get(&key).await {
                Ok(val) => val,
                Err(e) => {
                    error!(run_id, error = %e, "Redis error while getting context for merge");
                    return Err(()); // Indicate error within RMW block
                }
            };

        let merged = if let Some(serialized) = current {
            match serde_json::from_str::<C>(&serialized) {
                Ok(mut existing) => {
                    trace!(run_id, ?existing, ?ctx, "Context before merge");
                    existing.merge(ctx);
                    trace!(run_id, ?existing, "Context after merge");
                    existing
                }
                Err(e) => {
                    error!(run_id, error = %e, "Failed to deserialize context for merge");
                    // Return error instead of using potentially incomplete new context
                    return Err(()); // Indicate error within RMW block
                }
            }
        } else {
            trace!(run_id, ?ctx, "No existing context found, using new context");
            ctx
        };

        // Serialize the merged context
        let serialized = match serde_json::to_string(&merged) {
            Ok(s) => s,
            Err(e) => {
                error!(run_id, error = %e, "Failed to serialize merged context");
                return Err(()); // Indicate error within RMW block
            }
        };

        // Store the merged context in Redis
        trace!(run_id, context_to_write=?merged, "Attempting to write merged context to Redis");
        if let Err(e) = conn.set(&key, serialized).await as Result<(), _> {
            error!(run_id, error = %e, "Redis error while setting merged context");
            Err(()) // Indicate error within RMW block
        } else {
            trace!(run_id, "Successfully wrote merged context for run {}", run_id);
            Ok(()) // Indicate success within RMW block
        }
        }.await; // End of RMW async block

        // --- Release Lock ---
        // Release the lock regardless of RMW outcome.
        // A more robust implementation might check if the lock value still matches `lock_value` before deleting.
        trace!(run_id, "Releasing context lock");
        if let Err(e) = conn.del(&lock_key).await as Result<(), _> {
            error!(run_id, error = %e, "Failed to release context lock");
        } else {
            trace!(run_id, "Successfully released context lock");
        }

        if rmw_result.is_err() {
            error!(run_id, "Merge operation failed during read-modify-write phase");
            // Potentially signal error further up? For now, just log.
        }
        Ok(())
    }
}