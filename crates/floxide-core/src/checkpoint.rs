use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    marker::PhantomData,
    sync::{Arc, RwLock},
};
use thiserror::Error;

use crate::{context::Context, workflow::WorkItem};

/// A snapshot of a workflow's pending work and its context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    bound = "C: Serialize + for<'de2> Deserialize<'de2>, WI: Serialize + for<'de2> Deserialize<'de2>"
)]
pub struct Checkpoint<C: Context, WI: WorkItem> {
    /// The user-provided context for the workflow
    pub context: C,
    /// The queue of pending work items
    pub queue: VecDeque<WI>,
}

// Note: Serialization of Checkpoint<C, W> is the responsibility of the user-provided store.
// The Checkpoint type itself is Serialize + Deserialize, and can be cloned.
impl<C, WI> Checkpoint<C, WI>
where
    C: Context,
    WI: WorkItem,
{
    /// Create a new checkpoint from context and initial queue
    pub fn new(context: C, queue: VecDeque<WI>) -> Self {
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
#[async_trait]
pub trait CheckpointStore<C: Context, WI: WorkItem> {
    /// Persist the given checkpoint under `workflow_id`.
    async fn save(
        &self,
        workflow_id: &str,
        checkpoint: &Checkpoint<C, WI>,
    ) -> Result<(), CheckpointError>;
    /// Load the last-saved checkpoint for `workflow_id`, if any.
    async fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<C, WI>>, CheckpointError>;
}

/// A simple in-memory checkpoint store using JSON serialization
#[derive(Clone)]
pub struct InMemoryCheckpointStore<C: Context, WI: WorkItem> {
    inner: Arc<RwLock<HashMap<String, Checkpoint<C, WI>>>>,
    _phantom: PhantomData<WI>,
}

impl<C: Context, WI: WorkItem> InMemoryCheckpointStore<C, WI> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            _phantom: PhantomData,
        }
    }
}

impl<C: Context, WI: WorkItem> Default for InMemoryCheckpointStore<C, WI> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<C: Context, WI: WorkItem> CheckpointStore<C, WI> for InMemoryCheckpointStore<C, WI> {
    async fn save(
        &self,
        workflow_id: &str,
        checkpoint: &Checkpoint<C, WI>,
    ) -> Result<(), CheckpointError> {
        // serialize checkpoint to JSON bytes
        let mut map = self.inner.write().unwrap();
        map.insert(workflow_id.to_string(), checkpoint.clone());
        Ok(())
    }
    async fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<C, WI>>, CheckpointError> {
        let map = self.inner.read().unwrap();
        let maybe_ck = map.get(workflow_id).cloned();
        match maybe_ck {
            Some(ck) => Ok(Some(ck)),
            None => Ok(None),
        }
    }
}
