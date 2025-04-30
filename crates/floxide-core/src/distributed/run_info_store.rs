//! Run info store for distributed workflow runs.
//!
//! This module defines the RunInfoStore trait for tracking workflow run metadata and status,
//! and provides an in-memory implementation for testing and local development.

use crate::distributed::{RunInfo, RunStatus};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json;

/// Errors that can occur in a RunInfoStore implementation.
#[derive(Debug, thiserror::Error)]
pub enum RunInfoError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Not found")]
    NotFound,
    #[error("Other error: {0}")]
    Other(String),
}

/// Trait for a distributed workflow run info store.
///
/// Implementations track metadata and status for each workflow run (e.g., running, completed, failed).
#[async_trait]
pub trait RunInfoStore {
    /// Insert a new workflow run record.
    async fn insert_run(&self, info: RunInfo) -> Result<(), RunInfoError>;
    /// Update the status for a workflow run.
    async fn update_status(&self, run_id: &str, status: RunStatus) -> Result<(), RunInfoError>;
    /// Update the finished_at timestamp for a workflow run.
    async fn update_finished_at(
        &self,
        run_id: &str,
        finished_at: DateTime<Utc>,
    ) -> Result<(), RunInfoError>;
    /// Get the run info for a workflow run.
    async fn get_run(&self, run_id: &str) -> Result<Option<RunInfo>, RunInfoError>;
    /// List all workflow runs, optionally filtered by status.
    async fn list_runs(&self, filter: Option<RunStatus>) -> Result<Vec<RunInfo>, RunInfoError>;
    /// Update the output for a workflow run.
    async fn update_output(
        &self,
        run_id: &str,
        output: serde_json::Value,
    ) -> Result<(), RunInfoError>;
}

/// In-memory implementation of RunInfoStore for testing and local development.
#[derive(Clone, Default)]
pub struct InMemoryRunInfoStore {
    inner: Arc<Mutex<HashMap<String, RunInfo>>>,
}

#[async_trait]
impl RunInfoStore for InMemoryRunInfoStore {
    async fn insert_run(&self, info: RunInfo) -> Result<(), RunInfoError> {
        let mut map = self.inner.lock().await;
        map.insert(info.run_id.clone(), info);
        Ok(())
    }
    async fn update_status(&self, run_id: &str, status: RunStatus) -> Result<(), RunInfoError> {
        let mut map = self.inner.lock().await;
        if let Some(info) = map.get_mut(run_id) {
            info.status = status;
            Ok(())
        } else {
            Err(RunInfoError::NotFound)
        }
    }
    async fn update_finished_at(
        &self,
        run_id: &str,
        finished_at: DateTime<Utc>,
    ) -> Result<(), RunInfoError> {
        let mut map = self.inner.lock().await;
        if let Some(info) = map.get_mut(run_id) {
            info.finished_at = Some(finished_at);
            Ok(())
        } else {
            Err(RunInfoError::NotFound)
        }
    }
    async fn get_run(&self, run_id: &str) -> Result<Option<RunInfo>, RunInfoError> {
        let map = self.inner.lock().await;
        Ok(map.get(run_id).cloned())
    }
    async fn list_runs(&self, filter: Option<RunStatus>) -> Result<Vec<RunInfo>, RunInfoError> {
        let map = self.inner.lock().await;
        let runs = map
            .values()
            .filter(|info| filter.as_ref().is_none_or(|f| *f == info.status))
            .cloned()
            .collect();
        Ok(runs)
    }
    async fn update_output(
        &self,
        run_id: &str,
        output: serde_json::Value,
    ) -> Result<(), RunInfoError> {
        let mut map = self.inner.lock().await;
        if let Some(info) = map.get_mut(run_id) {
            info.output = Some(output);
            Ok(())
        } else {
            Err(RunInfoError::NotFound)
        }
    }
}
