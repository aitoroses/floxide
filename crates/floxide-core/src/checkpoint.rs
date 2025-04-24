use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A snapshot of a workflow’s pending work and its context.
#[derive(Serialize, Deserialize, Clone)]
pub struct Checkpoint<C, W> {
    /// The user‐provided context for the workflow
    pub context: C,
    /// The queue of pending work items
    pub queue: VecDeque<W>,
}

// Note: Serialization of Checkpoint<C, W> is the responsibility of the user-provided store.
// The Checkpoint type itself is Serialize + Deserialize, and can be cloned.
impl<C, W> Checkpoint<C, W>
where
    C: Clone,
    W: Clone,
{
    /// Create a new checkpoint from context and initial queue
    pub fn new(context: C, queue: VecDeque<W>) -> Self {
        Checkpoint { context, queue }
    }
}

/// Errors occurring during checkpoint persistence.
#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Store error: {0}")]
    Store(String),
}

/// A trait for persisting and loading workflow checkpoints.
pub trait CheckpointStore<C, W> {
    /// Persist the given checkpoint under `workflow_id`.
    fn save(&self, workflow_id: &str, checkpoint: &Checkpoint<C, W>) -> Result<(), CheckpointError>;
    /// Load the last‐saved checkpoint for `workflow_id`, if any.
    fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<C, W>>, CheckpointError>;
}