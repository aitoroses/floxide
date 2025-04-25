//! The context for a workflow execution.

use std::fmt;
use std::{fmt::Debug, sync::Arc};
use std::time::Duration;
use std::future::Future;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use crate::error::FloxideError;

/// The context for a workflow execution.
#[derive(Clone, Debug)]
///
/// The context contains the store, cancellation token, and optional timeout.
pub struct WorkflowCtx<S> {
    /// The store for the workflow.
    pub store: S,
    /// The cancellation token for the workflow.
    cancel: CancellationToken,
    /// The optional timeout for the workflow.
    timeout: Option<Duration>,
}

impl<S> WorkflowCtx<S>
where
    S: Send + Sync + 'static,
{
    /// Creates a new workflow context with the given store.
    pub fn new(store: S) -> Self {
        Self {
            store,
            cancel: CancellationToken::new(),
            timeout: None,
        }
    }

    /// Runs the provided function with a reference to the store.
    pub fn with_store<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&S) -> R,
    {
        f(&self.store)
    }

    /// Returns a reference to the cancellation token.
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    /// Sets a timeout for the workflow.
    pub fn set_timeout(&mut self, d: Duration) {
        self.timeout = Some(d);
    }

    /// Cancel the workflow execution.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Returns true if the workflow has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Asynchronously wait until the workflow is cancelled.
    pub async fn cancelled(&self) {
        self.cancel.cancelled().await;
    }

    /// Runs the provided future, respecting cancellation and optional timeout.
    pub async fn run_future<R, F>(&self, fut: F) -> Result<R, FloxideError>
    where
        F: Future<Output = Result<R, FloxideError>>,
    {
        if let Some(duration) = self.timeout {
            tokio::select! {
                _ = self.cancel.cancelled() => Err(FloxideError::Cancelled),
                _ = tokio::time::sleep(duration) => Err(FloxideError::Timeout(duration)),
                res = fut => res,
            }
        } else {
            tokio::select! {
                _ = self.cancel.cancelled() => Err(FloxideError::Cancelled),
                res = fut => res,
            }
        }
    }
}

/// Arc<Mutex<T>> wrapper with custom (de)serialization and debug support
#[derive(Clone)]
pub struct SharedState<T>(Arc<Mutex<T>>);

impl<T> SharedState<T> {
    pub fn new(value: T) -> Self {
        SharedState(Arc::new(Mutex::new(value)))
    }

    pub async fn get(&self) -> tokio::sync::MutexGuard<'_, T> {
        self.0.lock().await
    }

    pub async fn set(&self, value: T) {
        *self.0.lock().await = value;
    }
}

impl<T: Serialize + Clone> Serialize for SharedState<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Arc<Mutex<T>>", 1)?;
        let value = self.0.try_lock().unwrap();
        state.serialize_field("value", &value.clone())?;
        state.end()
    }
}

impl<'de, T: Deserialize<'de> + Clone> Deserialize<'de> for SharedState<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(SharedState(Arc::new(Mutex::new(value))))
    }
}

impl<T: Debug> Debug for SharedState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0.try_lock().unwrap();
        write!(f, "{:?}", value)
    }
}