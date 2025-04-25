use chrono::{DateTime, Utc};
use std::fmt::Debug;
use thiserror::Error;
use crate::error::FloxideError;

/// Status of a workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Information about a workflow run.
#[derive(Debug, Clone)]
pub struct RunInfo {
    pub run_id: String,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Error information for a workflow work item.
#[derive(Debug, Clone)]
pub struct WorkflowError {
    pub work_item: String, // Could be improved to use WorkItem type
    pub error: String,
    pub attempt: usize,
    pub timestamp: DateTime<Utc>,
}

/// Metrics for a workflow run.
#[derive(Debug, Clone, Default)]
pub struct RunMetrics {
    pub total_work_items: usize,
    pub completed: usize,
    pub failed: usize,
    pub retries: usize,
}

#[derive(Debug, Error)]
pub enum ErrorStoreError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum LivenessStoreError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LivenessStatus {
    Alive,
    Stale,
    Dead,
}

#[derive(Debug, Clone)]
pub struct StepError<W: std::fmt::Debug + Clone> {
    pub error: FloxideError,
    pub run_id: Option<String>,
    pub work_item: Option<W>,
} 