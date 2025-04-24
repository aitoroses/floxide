//! Abstractions for distributed workflow execution.

/// A simple FIFO work‐queue for workflow work‐items.
pub trait WorkQueue<W> {
    /// Enqueue one work‐item under this `workflow_id`.
    /// Returns Err(String) on failure.
    fn enqueue(&self, workflow_id: &str, work: W) -> Result<(), String>;

    /// Dequeue one work‐item from the queue for `workflow_id`.
    /// Returns Ok(Some(item)) if an item was dequeued,
    /// Ok(None) if the queue is empty,
    /// or Err(String) on failure.
    fn dequeue(&self, workflow_id: &str) -> Result<Option<W>, String>;
}