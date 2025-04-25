//! Work queue for distributed workflow execution.
//!
//! This module defines the WorkQueue trait for enqueuing and dequeuing workflow work items,
//! and provides an in-memory implementation for testing and local development.

use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;

use crate::context::Context;
use crate::workflow::WorkItem;

/// Errors that can occur in a WorkQueue implementation.
#[derive(Debug, Error)]
pub enum WorkQueueError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Queue is empty")]
    Empty,
    #[error("Other error: {0}")]
    Other(String),
}

/// Trait for a distributed workflow work queue.
///
/// Implementations provide FIFO queueing of work items for distributed workers.
#[async_trait]
pub trait WorkQueue<C: Context, WI: WorkItem>: Clone + Send + Sync + 'static {
    /// Enqueue one work-item under this `workflow_id`.
    /// Returns Err(WorkQueueError) on failure.
    async fn enqueue(&self, workflow_id: &str, work: WI) -> Result<(), WorkQueueError>;

    /// Dequeue the next available work-item from any workflow.
    /// Returns Ok(Some((workflow_id, item))) if an item was dequeued,
    /// Ok(None) if the queue is empty,
    /// or Err(WorkQueueError) on failure.
    async fn dequeue(&self) -> Result<Option<(String, WI)>, WorkQueueError>;

    /// Peek at the next available work-item from any workflow.
    /// Returns Ok(Some((workflow_id, item))) if an item was peeked,
    /// Ok(None) if the queue is empty,
    /// or Err(WorkQueueError) on failure.
    async fn peek(&self) -> Result<Option<(String, WI)>, WorkQueueError>;

    /// Purge all work items for a given workflow run.
    /// Removes all queued work for the specified run_id.
    async fn purge_run(&self, run_id: &str) -> Result<(), WorkQueueError>;

    /// Get pending work for a run.
    async fn pending_work(&self, run_id: &str) -> Result<Vec<WI>, WorkQueueError>;
}

/// In-memory implementation of WorkQueue for testing and local development.
#[derive(Clone)]
pub struct InMemoryWorkQueue<WI: WorkItem>(Arc<Mutex<HashMap<String, VecDeque<WI>>>>);

impl<WI: WorkItem> InMemoryWorkQueue<WI> {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl<WI: WorkItem> Default for InMemoryWorkQueue<WI> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<C: Context, WI: WorkItem + 'static> WorkQueue<C, WI> for InMemoryWorkQueue<WI> {
    async fn enqueue(&self, workflow_id: &str, work: WI) -> Result<(), WorkQueueError> {
        let mut map = self.0.lock().await;
        map.entry(workflow_id.to_string())
            .or_default()
            .push_back(work);
        Ok(())
    }
    async fn dequeue(&self) -> Result<Option<(String, WI)>, WorkQueueError> {
        let mut map = self.0.lock().await;
        for (run_id, q) in map.iter_mut() {
            if let Some(item) = q.pop_front() {
                return Ok(Some((run_id.clone(), item)));
            }
        }
        Ok(None)
    }
    async fn peek(&self) -> Result<Option<(String, WI)>, WorkQueueError> {
        let map = self.0.lock().await;
        for (run_id, q) in map.iter() {
            if let Some(item) = q.front() {
                return Ok(Some((run_id.clone(), item.clone())));
            }
        }
        Ok(None)
    }
    async fn purge_run(&self, run_id: &str) -> Result<(), WorkQueueError> {
        let mut map = self.0.lock().await;
        map.remove(run_id);
        Ok(())
    }
    async fn pending_work(&self, run_id: &str) -> Result<Vec<WI>, WorkQueueError> {
        let map = self.0.lock().await;
        let q = map.get(run_id).ok_or(WorkQueueError::Empty)?;
        Ok(q.iter().cloned().collect())
    }
}

