use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A snapshot of a workflow’s pending work and its context.
#[derive(Serialize, Deserialize)]
pub struct Checkpoint<C, W> {
    /// The user‐provided context for the workflow
    pub context: C,
    /// The queue of pending work items
    pub queue: VecDeque<W>,
}

impl<C, W> Checkpoint<C, W>
where
    C: Serialize + for<'de> Deserialize<'de>,
    W: Serialize + for<'de> Deserialize<'de>,
{
    /// Serialize the checkpoint to a JSON byte vector.
    pub fn to_bytes(&self) -> Result<Vec<u8>, CheckpointError> {
        serde_json::to_vec(self).map_err(CheckpointError::Serde)
    }
    /// Deserialize a checkpoint from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CheckpointError> {
        serde_json::from_slice(bytes).map_err(CheckpointError::Serde)
    }
}

/// Errors occurring during checkpoint persistence or serialization.
#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Store error: {0}")]
    Store(String),
}

/// A trait for persisting and loading workflow checkpoints.
pub trait CheckpointStore {
    /// Persist the given byte blob under `workflow_id`.
    fn save(&self, workflow_id: &str, data: &[u8]) -> Result<(), CheckpointError>;
    /// Load the last‐saved blob for `workflow_id`, if any.
    fn load(&self, workflow_id: &str) -> Result<Option<Vec<u8>>, CheckpointError>;
}