use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use thiserror::Error;
use async_trait::async_trait;

use crate::workflow::WorkItem;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkItemStatus {
    #[default]
    Pending,
    InProgress,
    Failed,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkItemState<W: WorkItem> {
    status: WorkItemStatus,
    attempts: u32,
    work_item: W,
}

impl<W: WorkItem> WorkItemState<W> {
    fn new(work_item: W) -> Self {
        Self { work_item, status: WorkItemStatus::Pending, attempts: 0 }
    }
}

#[async_trait]
pub trait WorkItemStateStore<W: WorkItem>: Send + Sync {
    async fn get_status(&self, run_id: &str, item: &W) -> Result<WorkItemStatus, WorkItemStateStoreError>;
    async fn set_status(&self, run_id: &str, item: &W, status: WorkItemStatus) -> Result<(), WorkItemStateStoreError>;
    async fn get_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError>;
    async fn increment_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError>;
    async fn reset_attempts(&self, run_id: &str, item: &W) -> Result<(), WorkItemStateStoreError>;
    async fn purge_run(&self, run_id: &str) -> Result<(), WorkItemStateStoreError>;
}

#[derive(Debug, Clone)]
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
    async fn get_status(&self, run_id: &str, item: &W) -> Result<WorkItemStatus, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store.entry(format!("{:?}", item)).or_insert(WorkItemState::new(item.clone()));
        Ok(item_status.status.clone())
    }

    async fn set_status(&self, run_id: &str, item: &W, status: WorkItemStatus) -> Result<(), WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        // TODO: run_id is not enough, we need work_item_id, else reentering into the same node will reuse
        let item_status = run_store.entry(format!("{:?}", item)).or_insert(WorkItemState::new(item.clone()));
        item_status.status = status;
        Ok(())
    }

    async fn get_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store.entry(format!("{:?}", item)).or_insert(WorkItemState::new(item.clone()));
        Ok(item_status.attempts)
    }

    async fn increment_attempts(&self, run_id: &str, item: &W) -> Result<u32, WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store.entry(format!("{:?}", item)).or_insert(WorkItemState::new(item.clone()));
        item_status.attempts += 1;
        Ok(item_status.attempts)
    }

    async fn reset_attempts(&self, run_id: &str, item: &W) -> Result<(), WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        let run_store = store.entry(run_id.to_string()).or_insert_with(HashMap::new);
        let item_status = run_store.entry(format!("{:?}", item)).or_insert(WorkItemState::new(item.clone()));
        item_status.attempts = 0;
        Ok(())
    }

    async fn purge_run(&self, run_id: &str) -> Result<(), WorkItemStateStoreError> {
        let mut store = self.store.lock().await;
        store.remove(run_id);
        Ok(())
    }
}