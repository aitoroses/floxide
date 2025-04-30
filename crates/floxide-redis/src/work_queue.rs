//! Redis implementation of the WorkQueue trait.

use crate::client::RedisClient;
use async_trait::async_trait;
use floxide_core::{
    context::Context,
    distributed::{WorkQueue, WorkQueueError},
    workflow::WorkItem,
};
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{error, instrument, trace};

/// Redis implementation of the WorkQueue trait.
#[derive(Clone)]
pub struct RedisWorkQueue<WI: WorkItem> {
    client: RedisClient,
    _phantom: std::marker::PhantomData<WI>,
}

impl<WI: WorkItem> RedisWorkQueue<WI> {
    /// Create a new Redis work queue with the given client.
    pub fn new(client: RedisClient) -> Self {
        Self {
            client,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the Redis key for the work queue for a specific workflow run.
    fn queue_key(&self, workflow_id: &str) -> String {
        self.client.prefixed_key(&format!("queue:{}", workflow_id))
    }

    /// Get the Redis key for the global work queue.
    fn global_queue_key(&self) -> String {
        self.client.prefixed_key("global_queue")
    }
}

#[async_trait]
impl<C: Context, WI: WorkItem + 'static> WorkQueue<C, WI> for RedisWorkQueue<WI>
where
    WI: Serialize + DeserializeOwned + Send + Sync,
{
    #[instrument(skip(self, work), level = "trace")]
    async fn enqueue(&self, workflow_id: &str, work: WI) -> Result<(), WorkQueueError> {
        let queue_key = self.queue_key(workflow_id);
        let global_queue_key = self.global_queue_key();

        // Serialize the work item
        let serialized = serde_json::to_string(&work).map_err(|e| {
            error!("Failed to serialize work item: {}", e);
            WorkQueueError::Other(format!("Serialization error: {}", e))
        })?;

        // Use a Redis pipeline to atomically:
        // 1. Push the work item to the workflow-specific queue
        // 2. Add the workflow ID to the global queue if not already present
        let mut conn = self.client.conn.clone();
        let _result: () = redis::pipe()
            .rpush(&queue_key, serialized)
            .sadd(&global_queue_key, workflow_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while enqueueing work: {}", e);
                WorkQueueError::Io(e.to_string())
            })?;

        trace!("Enqueued work item for workflow {}", workflow_id);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn dequeue(&self) -> Result<Option<(String, WI)>, WorkQueueError> {
        let global_queue_key = self.global_queue_key();
        let mut conn = self.client.conn.clone();

        // Get all workflow IDs from the global queue
        let workflow_ids: Vec<String> = conn.smembers(&global_queue_key).await.map_err(|e| {
            error!("Redis error while getting workflow IDs: {}", e);
            WorkQueueError::Io(e.to_string())
        })?;

        // Try to dequeue from each workflow queue in turn
        for workflow_id in workflow_ids {
            let queue_key = self.queue_key(&workflow_id);

            // Use LPOP to get the next item from the queue
            let result: Option<String> = conn.lpop(&queue_key, None).await.map_err(|e| {
                error!("Redis error while dequeueing work: {}", e);
                WorkQueueError::Io(e.to_string())
            })?;

            if let Some(serialized) = result {
                // Deserialize the work item
                let work_item = serde_json::from_str(&serialized).map_err(|e| {
                    error!("Failed to deserialize work item: {}", e);
                    WorkQueueError::Other(format!("Deserialization error: {}", e))
                })?;

                // Check if the queue is now empty, and if so, remove it from the global queue
                let queue_len: usize = conn.llen(&queue_key).await.map_err(|e| {
                    error!("Redis error while checking queue length: {}", e);
                    WorkQueueError::Io(e.to_string())
                })?;

                if queue_len == 0 {
                    let _result: () =
                        conn.srem(&global_queue_key, &workflow_id)
                            .await
                            .map_err(|e| {
                                error!(
                                    "Redis error while removing workflow from global queue: {}",
                                    e
                                );
                                WorkQueueError::Io(e.to_string())
                            })?;
                }

                trace!("Dequeued work item for workflow {}", workflow_id);
                return Ok(Some((workflow_id, work_item)));
            }
        }

        // No work items found
        trace!("No work items available");
        Ok(None)
    }

    #[instrument(skip(self), level = "trace")]
    async fn purge_run(&self, run_id: &str) -> Result<(), WorkQueueError> {
        let queue_key = self.queue_key(run_id);
        let global_queue_key = self.global_queue_key();
        let mut conn = self.client.conn.clone();

        // Use a Redis pipeline to atomically:
        // 1. Delete the workflow-specific queue
        // 2. Remove the workflow ID from the global queue
        let _result: () = redis::pipe()
            .del(&queue_key)
            .srem(&global_queue_key, run_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis error while purging run: {}", e);
                WorkQueueError::Io(e.to_string())
            })?;

        trace!("Purged work items for workflow {}", run_id);
        Ok(())
    }

    #[instrument(skip(self), level = "trace")]
    async fn pending_work(&self, run_id: &str) -> Result<Vec<WI>, WorkQueueError> {
        let queue_key = self.queue_key(run_id);
        let mut conn = self.client.conn.clone();

        // Get all items from the queue
        let items: Vec<String> = conn.lrange(&queue_key, 0, -1).await.map_err(|e| {
            error!("Redis error while getting pending work: {}", e);
            WorkQueueError::Io(e.to_string())
        })?;

        // Deserialize each item
        let mut result = Vec::with_capacity(items.len());
        for item in items {
            let work_item = serde_json::from_str(&item).map_err(|e| {
                error!("Failed to deserialize work item: {}", e);
                WorkQueueError::Other(format!("Deserialization error: {}", e))
            })?;
            result.push(work_item);
        }

        trace!(
            "Found {} pending work items for workflow {}",
            result.len(),
            run_id
        );
        Ok(result)
    }
}
