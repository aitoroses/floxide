//! Redis implementation of the WorkItemStateStore trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::distributed::{
    WorkItemState, WorkItemStateStore, WorkItemStateStoreError, WorkItemStatus,
};
use floxide_core::workflow::WorkItem;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{error, instrument, trace};

/// Redis implementation of the WorkItemStateStore trait.
#[derive(Clone)]
pub struct RedisWorkItemStateStore<W: WorkItem> {
    client: RedisClient,
    _phantom: std::marker::PhantomData<W>,
}

impl<W: WorkItem> RedisWorkItemStateStore<W> {
    /// Create a new Redis work item state store with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self {
            client,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the Redis key for work item states for a specific run.
    fn work_item_states_key(&self, run_id: &str) -> String {
        self.client
            .prefixed_key(&format!("work_item_states:{}", run_id))
    }

    /// Get the Redis key for a specific work item state.
    fn work_item_state_key(&self, run_id: &str, item_id: &str) -> String {
        self.client
            .prefixed_key(&format!("work_item_state:{}:{}", run_id, item_id))
    }
}

#[async_trait]
impl<W: WorkItem + Serialize + DeserializeOwned> WorkItemStateStore<W>
    for RedisWorkItemStateStore<W>
{
    #[instrument(skip(self, item), level = "trace")]
    async fn get_status(
        &self,
        run_id: &str,
        item: &W,
    ) -> Result<WorkItemStatus, WorkItemStateStoreError> {
        let item_id = item.instance_id();
        let key = self.work_item_state_key(run_id, &item_id);
        let mut conn = self.client.conn.clone();

        // Get the serialized work item state from Redis
        let result: Option<String> = conn.get(&key).await.map_err(|e| {
            error!("Redis error while getting work item state: {}", e);
            WorkItemStateStoreError::Io(e.to_string())
        })?;

        // If the work item state exists, deserialize it and return the status
        if let Some(serialized) = result {
            let state = serde_json::from_str::<WorkItemState<W>>(&serialized).map_err(|e| {
                error!("Failed to deserialize work item state: {}", e);
                WorkItemStateStoreError::Other(format!("Deserialization error: {}", e))
            })?;

            trace!(
                "Got status for work item {} in run {}: {:?}",
                item_id,
                run_id,
                state.status
            );
            Ok(state.status)
        } else {
            // If the work item state doesn't exist, return the default status
            trace!(
                "No status found for work item {} in run {}, returning default",
                item_id,
                run_id
            );
            Ok(WorkItemStatus::default())
        }
    }

    #[instrument(skip(self, item, status), level = "trace")]
    async fn set_status(
        &self,
        run_id: &str,
        item: &W,
        status: WorkItemStatus,
    ) -> Result<(), WorkItemStateStoreError> {
        let item_id = item.instance_id();
        let key = self.work_item_state_key(run_id, &item_id);
        let states_key = self.work_item_states_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current work item state or create a new one
        let state = match conn.get::<_, Option<String>>(&key).await {
            Ok(Some(serialized)) => {
                let mut state =
                    serde_json::from_str::<WorkItemState<W>>(&serialized).map_err(|e| {
                        error!("Failed to deserialize work item state: {}", e);
                        WorkItemStateStoreError::Other(format!("Deserialization error: {}", e))
                    })?;
                state.status = status;
                state
            }
            _ => WorkItemState {
                status,
                attempts: 0,
                work_item: item.clone(),
            },
        };
        // Clone status for debug log before it is moved
        let status_for_log = state.status.clone();
        // Serialize the updated work item state
        let serialized = serde_json::to_string(&state).map_err(|e| {
            error!("Failed to serialize work item state: {}", e);
            WorkItemStateStoreError::Other(format!("Serialization error: {}", e))
        })?;
        // Use a Redis pipeline to atomically:
        // 1. Store the updated work item state
        // 2. Add the work item ID to the set of work items for this run
        let _result: () = redis::pipe()
            .set(&key, serialized)
            .sadd(&states_key, &item_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while updating work item status: {}", e);
                WorkItemStateStoreError::Io(e.to_string())
            })?;
        trace!(
            "Updated status for work item {} in run {} to {:?}",
            item_id,
            run_id,
            status_for_log
        );
        Ok(())
    }

    #[instrument(skip(self, item), level = "trace")]
    async fn increment_attempts(
        &self,
        run_id: &str,
        item: &W,
    ) -> Result<u32, WorkItemStateStoreError> {
        let item_id = item.instance_id();
        let key = self.work_item_state_key(run_id, &item_id);
        let states_key = self.work_item_states_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current work item state or create a new one
        let mut state = match conn.get::<_, Option<String>>(&key).await {
            Ok(Some(serialized)) => {
                serde_json::from_str::<WorkItemState<W>>(&serialized).map_err(|e| {
                    error!("Failed to deserialize work item state: {}", e);
                    WorkItemStateStoreError::Other(format!("Deserialization error: {}", e))
                })?
            }
            _ => WorkItemState {
                status: WorkItemStatus::default(),
                attempts: 0,
                work_item: item.clone(),
            },
        };

        // Increment the attempts counter
        state.attempts += 1;

        // Serialize the updated work item state
        let serialized = serde_json::to_string(&state).map_err(|e| {
            error!("Failed to serialize work item state: {}", e);
            WorkItemStateStoreError::Other(format!("Serialization error: {}", e))
        })?;

        // Use a Redis pipeline to atomically:
        // 1. Store the updated work item state
        // 2. Add the work item ID to the set of work items for this run
        let _result: () = redis::pipe()
            .set(&key, serialized)
            .sadd(&states_key, &item_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while incrementing attempts: {}", e);
                WorkItemStateStoreError::Io(e.to_string())
            })?;

        trace!(
            "Incremented attempts for work item {} in run {} to {}",
            item_id,
            run_id,
            state.attempts
        );
        Ok(state.attempts)
    }

    #[instrument(skip(self, item), level = "trace")]
    async fn get_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError> {
        let item_id = item.instance_id();
        let key = self.work_item_state_key(run_id, &item_id);
        let mut conn = self.client.conn.clone();

        // Get the serialized work item state from Redis
        let result: Option<String> = conn.get(&key).await.map_err(|e| {
            error!("Redis error while getting work item state: {}", e);
            WorkItemStateStoreError::Io(e.to_string())
        })?;

        // If the work item state exists, deserialize it and return the attempts
        if let Some(serialized) = result {
            let state = serde_json::from_str::<WorkItemState<W>>(&serialized).map_err(|e| {
                error!("Failed to deserialize work item state: {}", e);
                WorkItemStateStoreError::Other(format!("Deserialization error: {}", e))
            })?;

            trace!(
                "Got attempts for work item {} in run {}: {}",
                item_id,
                run_id,
                state.attempts
            );
            Ok(state.attempts)
        } else {
            // If the work item state doesn't exist, return 0 attempts
            trace!(
                "No attempts found for work item {} in run {}, returning 0",
                item_id,
                run_id
            );
            Ok(0)
        }
    }

    #[instrument(skip(self, item), level = "trace")]
    async fn reset_attempts(&self, run_id: &str, item: &W) -> Result<(), WorkItemStateStoreError> {
        let item_id = item.instance_id();
        let key = self.work_item_state_key(run_id, &item_id);
        let states_key = self.work_item_states_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get the current work item state or create a new one
        let mut state = match conn.get::<_, Option<String>>(&key).await {
            Ok(Some(serialized)) => {
                serde_json::from_str::<WorkItemState<W>>(&serialized).map_err(|e| {
                    error!("Failed to deserialize work item state: {}", e);
                    WorkItemStateStoreError::Other(format!("Deserialization error: {}", e))
                })?
            }
            _ => WorkItemState {
                status: WorkItemStatus::default(),
                attempts: 0,
                work_item: item.clone(),
            },
        };

        // Reset the attempts counter
        state.attempts = 0;

        // Serialize the updated work item state
        let serialized = serde_json::to_string(&state).map_err(|e| {
            error!("Failed to serialize work item state: {}", e);
            WorkItemStateStoreError::Other(format!("Serialization error: {}", e))
        })?;

        // Use a Redis pipeline to atomically:
        // 1. Store the updated work item state
        // 2. Add the work item ID to the set of work items for this run
        let _result: () = redis::pipe()
            .set(&key, serialized)
            .sadd(&states_key, &item_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while resetting attempts: {}", e);
                WorkItemStateStoreError::Io(e.to_string())
            })?;

        trace!(
            "Reset attempts for work item {} in run {} to 0",
            item_id,
            run_id
        );
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn get_all(
        &self,
        run_id: &str,
    ) -> Result<Vec<WorkItemState<W>>, WorkItemStateStoreError> {
        let states_key = self.work_item_states_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get all work item IDs for this run
        let item_ids: Vec<String> = conn.smembers(&states_key).await.map_err(|e| {
            error!("Redis error while getting all work items: {}", e);
            WorkItemStateStoreError::Io(e.to_string())
        })?;

        // Get the work item state for each ID
        let mut items = Vec::with_capacity(item_ids.len());
        for item_id in item_ids {
            let key = self.work_item_state_key(run_id, &item_id);
            let result: Option<String> = conn.get(&key).await.map_err(|e| {
                error!("Redis error while getting work item state: {}", e);
                WorkItemStateStoreError::Io(e.to_string())
            })?;

            if let Some(serialized) = result {
                let state = serde_json::from_str::<WorkItemState<W>>(&serialized).map_err(|e| {
                    error!("Failed to deserialize work item state: {}", e);
                    WorkItemStateStoreError::Other(format!("Deserialization error: {}", e))
                })?;

                items.push(state);
            }
        }

        trace!("Got {} work item states for run {}", items.len(), run_id);
        Ok(items)
    }
}
