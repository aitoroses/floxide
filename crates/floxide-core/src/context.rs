//! The context for a workflow execution.

use std::time::Duration;
use std::future::Future;
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
