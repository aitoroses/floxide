//! Abstractions for distributed workflow execution.

use async_trait::async_trait;

/// A simple FIFO work‐queue for workflow work‐items.
#[async_trait]
pub trait WorkQueue<W> {
    /// Enqueue one work‐item under this `workflow_id`.
    /// Returns Err(String) on failure.
    async fn enqueue(&self, workflow_id: &str, work: W) -> Result<(), String>;

    /// Dequeue the next available work‐item from any workflow.
    /// Returns Ok(Some((workflow_id, item))) if an item was dequeued,
    /// Ok(None) if the queue is empty,
    /// or Err(String) on failure.
    async fn dequeue(&self) -> Result<Option<(String, W)>, String>;
}