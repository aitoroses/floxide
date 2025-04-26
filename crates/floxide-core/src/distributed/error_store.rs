//! Error store for distributed workflow runs.
//!
//! This module defines the ErrorStore trait for recording and retrieving workflow errors,
//! and provides an in-memory implementation for testing and local development.

use crate::distributed::{ErrorStoreError, WorkflowError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Trait for a distributed workflow error store.
///
/// Implementations record and retrieve errors encountered during workflow execution.
#[async_trait]
pub trait ErrorStore {
    /// Record an error for a workflow run.
    async fn record_error(&self, run_id: &str, error: WorkflowError)
        -> Result<(), ErrorStoreError>;
    /// Get all errors for a workflow run.
    async fn get_errors(&self, run_id: &str) -> Result<Vec<WorkflowError>, ErrorStoreError>;
}

/// In-memory implementation of ErrorStore for testing and local development.
#[derive(Clone, Default)]
pub struct InMemoryErrorStore {
    inner: Arc<Mutex<HashMap<String, Vec<WorkflowError>>>>,
}

#[async_trait]
impl ErrorStore for InMemoryErrorStore {
    async fn record_error(
        &self,
        run_id: &str,
        error: WorkflowError,
    ) -> Result<(), ErrorStoreError> {
        let mut map = self.inner.lock().await;
        map.entry(run_id.to_string()).or_default().push(error);
        Ok(())
    }
    async fn get_errors(&self, run_id: &str) -> Result<Vec<WorkflowError>, ErrorStoreError> {
        let map = self.inner.lock().await;
        Ok(map.get(run_id).cloned().unwrap_or_default())
    }
}
