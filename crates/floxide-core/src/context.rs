//! The context for a workflow execution.

use crate::error::FloxideError;
use crate::Merge;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::time::Duration;
use std::{fmt::Debug, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub trait Context: Default + DeserializeOwned + Serialize + Debug + Clone + Send + Sync {}
impl<T: Default + DeserializeOwned + Serialize + Debug + Clone + Send + Sync> Context for T {}

/// The context for a workflow execution.
#[derive(Clone, Debug)]
///
/// The context contains the store, cancellation token, and optional timeout.
pub struct WorkflowCtx<S: Context> {
    /// The store for the workflow.
    pub store: S,
    /// The cancellation token for the workflow.
    cancel: CancellationToken,
    /// The optional timeout for the workflow.
    timeout: Option<Duration>,
}

impl<S: Context> WorkflowCtx<S> {
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
#[derive(Clone, Default)]
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
        let value = self
            .0
            .try_lock()
            .expect("Failed to lock mutex on SharedState while serializing");
        // Directly serialize the inner value T
        T::serialize(&*value, serializer)
    }
}

impl<'de, T: Deserialize<'de> + Clone> Deserialize<'de> for SharedState<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Directly deserialize into T
        let value = T::deserialize(deserializer)?;
        Ok(SharedState(Arc::new(Mutex::new(value))))
    }
}

impl<T: Debug> Debug for SharedState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.try_lock() {
            Ok(value) => write!(f, "{:?}", value),
            Err(_) => write!(f, "SharedState(Locked)"),
        }
    }
}

impl<T: Merge> Merge for SharedState<T> {
    fn merge(&mut self, other: Self) {
        let self_ptr = Arc::as_ptr(&self.0) as usize;
        let other_ptr = Arc::as_ptr(&other.0) as usize;
        if self_ptr == other_ptr {
            // Prevent self-deadlock: merging with itself is a no-op
            return;
        }
        // Lock in address order to prevent lock order inversion deadlocks
        let (first, second) = if self_ptr < other_ptr {
            (&self.0, &other.0)
        } else {
            (&other.0, &self.0)
        };
        let mut first_guard = first.blocking_lock();
        let mut second_guard = second.blocking_lock();
        // Always merge into self
        if self_ptr < other_ptr {
            let other_val = std::mem::take(&mut *second_guard);
            first_guard.merge(other_val);
        } else {
            let mut temp = std::mem::take(&mut *first_guard);
            temp.merge(std::mem::take(&mut *second_guard));
            *first_guard = temp;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[tokio::test]
    async fn test_shared_state_serde_direct() {
        let initial_data = vec![10, 20, 30];
        let shared_state = SharedState::new(initial_data.clone());

        // Serialize
        let serialized = serde_json::to_string(&shared_state).expect("Serialization failed");
        // Should serialize directly as the inner Vec<i32>
        assert_eq!(serialized, "[10,20,30]");

        // Deserialize
        let deserialized: SharedState<Vec<i32>> =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        // Verify data
        let final_data = deserialized.get().await;
        assert_eq!(*final_data, initial_data);
    }

    #[tokio::test]
    async fn test_shared_state_serde_within_struct() {
        // Removed PartialEq derive
        #[derive(Serialize, Deserialize, Debug)]
        struct Container {
            id: u32,
            state: SharedState<String>,
        }

        let initial_string = "hello".to_string();
        let container = Container {
            id: 1,
            state: SharedState::new(initial_string.clone()),
        };

        // Serialize
        let serialized = serde_json::to_string(&container).expect("Serialization failed");
        // state should be serialized directly as the string
        assert_eq!(serialized, r#"{"id":1,"state":"hello"}"#);

        // Deserialize
        let deserialized: Container =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        // Verify data manually
        assert_eq!(deserialized.id, container.id);
        let final_string = deserialized.state.get().await;
        assert_eq!(*final_string, initial_string);
    }
}
