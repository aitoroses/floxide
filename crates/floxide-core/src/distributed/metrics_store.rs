//! Metrics store for distributed workflow runs.
//!
//! This module defines the MetricsStore trait for tracking workflow run metrics (e.g., completed, failed, retries),
//! and provides an in-memory implementation for testing and local development.

use crate::distributed::RunMetrics;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Not found")]
    NotFound,
    #[error("Other error: {0}")]
    Other(String),
}

/// Trait for a distributed workflow metrics store.
///
/// Implementations track per-run metrics such as completed, failed, and retried work items.
#[async_trait]
pub trait MetricsStore {
    /// Update the metrics for a workflow run.
    async fn update_metrics(&self, run_id: &str, metrics: RunMetrics) -> Result<(), MetricsError>;
    /// Get the metrics for a workflow run.
    async fn get_metrics(&self, run_id: &str) -> Result<Option<RunMetrics>, MetricsError>;
}

/// In-memory implementation of MetricsStore for testing and local development.
#[derive(Clone, Default)]
pub struct InMemoryMetricsStore {
    inner: Arc<Mutex<HashMap<String, RunMetrics>>>,
}

#[async_trait]
impl MetricsStore for InMemoryMetricsStore {
    async fn update_metrics(&self, run_id: &str, metrics: RunMetrics) -> Result<(), MetricsError> {
        let mut map = self.inner.lock().await;
        map.insert(run_id.to_string(), metrics);
        Ok(())
    }
    async fn get_metrics(&self, run_id: &str) -> Result<Option<RunMetrics>, MetricsError> {
        let map = self.inner.lock().await;
        Ok(map.get(run_id).cloned())
    }
}
