//! Liveness and health tracking for distributed workflow workers.
//!
//! This module defines the LivenessStore trait for tracking worker heartbeats and health status,
//! and provides an in-memory implementation for testing and local development.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::distributed::LivenessStoreError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub enum WorkerStatus {
    #[default]
    Idle,
    InProgress,
    Retrying(usize, usize), // (attempt, max_attempts)
}

/// Health and status information for a workflow worker.
#[derive(Debug, Clone, Default)]
pub struct WorkerHealth {
    /// Unique worker ID.
    pub worker_id: usize,
    /// Timestamp of the last heartbeat.
    pub last_heartbeat: DateTime<Utc>,
    /// Number of errors encountered by this worker.
    pub error_count: usize,
    /// Optional custom status string (e.g., "in progress", "permanently failed").
    pub status: WorkerStatus,
    /// Worker's current work item.
    pub current_work_item: Option<String>,
    /// Worker's current work item's run ID.
    pub current_work_item_run_id: Option<String>,
}

/// Trait for a distributed workflow liveness/health store.
///
/// Implementations track worker heartbeats and health for monitoring and fault detection.
#[async_trait]
pub trait LivenessStore {
    /// Update the heartbeat timestamp for a worker.
    async fn update_heartbeat(&self, worker_id: usize, timestamp: DateTime<Utc>) -> Result<(), LivenessStoreError>;
    /// Get the last heartbeat timestamp for a worker.
    async fn get_heartbeat(&self, worker_id: usize) -> Result<Option<DateTime<Utc>>, LivenessStoreError>;
    /// List all known worker IDs.
    async fn list_workers(&self) -> Result<Vec<usize>, LivenessStoreError>;
    /// Update the health status for a worker.
    async fn update_health(&self, worker_id: usize, health: WorkerHealth) -> Result<(), LivenessStoreError>;
    /// Get the health status for a worker.
    async fn get_health(&self, worker_id: usize) -> Result<Option<WorkerHealth>, LivenessStoreError>;
    /// List health status for all workers.
    async fn list_health(&self) -> Result<Vec<WorkerHealth>, LivenessStoreError>;
}

/// In-memory implementation of LivenessStore for testing and local development.
#[derive(Clone, Default)]
pub struct InMemoryLivenessStore {
    inner: Arc<Mutex<HashMap<usize, DateTime<Utc>>>>,
    health: Arc<Mutex<HashMap<usize, WorkerHealth>>>,
}

#[async_trait]
impl LivenessStore for InMemoryLivenessStore {
    async fn update_heartbeat(&self, worker_id: usize, timestamp: DateTime<Utc>) -> Result<(), LivenessStoreError> {
        let mut map = self.inner.lock().await;
        map.insert(worker_id, timestamp);
        Ok(())
    }
    async fn get_heartbeat(&self, worker_id: usize) -> Result<Option<DateTime<Utc>>, LivenessStoreError> {
        let map = self.inner.lock().await;
        Ok(map.get(&worker_id).cloned())
    }
    async fn list_workers(&self) -> Result<Vec<usize>, LivenessStoreError> {
        let map = self.inner.lock().await;
        Ok(map.keys().cloned().collect())
    }
    async fn update_health(&self, worker_id: usize, health: WorkerHealth) -> Result<(), LivenessStoreError> {
        let mut map = self.health.lock().await;
        map.insert(worker_id, health);
        Ok(())
    }
    async fn get_health(&self, worker_id: usize) -> Result<Option<WorkerHealth>, LivenessStoreError> {
        let map = self.health.lock().await;
        Ok(map.get(&worker_id).cloned())
    }
    async fn list_health(&self) -> Result<Vec<WorkerHealth>, LivenessStoreError> {
        let map = self.health.lock().await;
        Ok(map.values().cloned().collect())
    }
}

