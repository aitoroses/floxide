use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use thiserror::Error;

use crate::workflow::WorkItem;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkItemStatus {
    #[default]
    Pending,
    InProgress,
    Failed,
    WaitingRetry,
    PermanentlyFailed,
    Completed,
}

#[derive(Debug, Error)]
pub enum WorkItemStateStoreError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Not found")]
    NotFound,
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(bound = "W: Serialize + for<'de2> Deserialize<'de2>")]
pub struct WorkItemState<W: WorkItem> {
    pub status: WorkItemStatus,
    pub attempts: u32,
    pub work_item: W,
}

impl<W: WorkItem> WorkItemState<W> {
    fn new(work_item: W) -> Self {
        Self {
            work_item,
            status: WorkItemStatus::Pending,
            attempts: 0,
        }
    }
}

#[async_trait]
pub trait WorkItemStateStore<W: WorkItem>: Send + Sync {
    /// Get the status of a work item.
    async fn get_status(
        &self,
        run_id: &str,
        item: &W,
    ) -> Result<WorkItemStatus, WorkItemStateStoreError>;
    /// Set the status of a work item.
    async fn set_status(
        &self,
        run_id: &str,
        item: &W,
        status: WorkItemStatus,
    ) -> Result<(), WorkItemStateStoreError>;
    /// Get the number of attempts for a work item.
    async fn get_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError>;
    /// Increment the number of attempts for a work item.
    async fn increment_attempts(
        &self,
        run_id: &str,
        item: &W,
    ) -> Result<u32, WorkItemStateStoreError>;
    /// Reset the number of attempts for a work item.
    async fn reset_attempts(&self, run_id: &str, item: &W) -> Result<(), WorkItemStateStoreError>;
    /// Get all work items for a run.
    async fn get_all(&self, run_id: &str)
        -> Result<Vec<WorkItemState<W>>, WorkItemStateStoreError>;
}

#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
pub struct InMemoryWorkItemStateStore<W: WorkItem> {
    store: Arc<Mutex<HashMap<String, HashMap<String, WorkItemState<W>>>>>,
}

impl<W: WorkItem> Default for InMemoryWorkItemStateStore<W> {
    fn default() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl<W: WorkItem> WorkItemStateStore<W> for InMemoryWorkItemStateStore<W> {
    async fn get_status(
        &self,
        run_id: &str,
        item: &W,
    ) -> Result<WorkItemStatus, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store
            .entry(item.instance_id())
            .or_insert(WorkItemState::new(item.clone()));
        Ok(item_status.status.clone())
    }

    async fn set_status(
        &self,
        run_id: &str,
        item: &W,
        status: WorkItemStatus,
    ) -> Result<(), WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        // TODO: run_id is not enough, we need work_item_id, else reentering into the same node will reuse
        let item_status = run_store
            .entry(item.instance_id())
            .or_insert(WorkItemState::new(item.clone()));
        item_status.status = status;
        Ok(())
    }

    async fn get_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store
            .entry(item.instance_id())
            .or_insert(WorkItemState::new(item.clone()));
        Ok(item_status.attempts)
    }

    async fn increment_attempts(
        &self,
        run_id: &str,
        item: &W,
    ) -> Result<u32, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store
            .entry(item.instance_id())
            .or_insert(WorkItemState::new(item.clone()));
        item_status.attempts += 1;
        Ok(item_status.attempts)
    }

    async fn reset_attempts(&self, run_id: &str, item: &W) -> Result<(), WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store
            .entry(item.instance_id())
            .or_insert(WorkItemState::new(item.clone()));
        item_status.attempts = 0;
        Ok(())
    }

    async fn get_all(
        &self,
        run_id: &str,
    ) -> Result<Vec<WorkItemState<W>>, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        Ok(run_store.values().cloned().collect())
    }
}
