use std::time::Duration;
use std::future::Future;
use tokio_util::sync::CancellationToken;
use crate::error::FloxideError;

#[derive(Clone, Debug)]
pub struct WorkflowCtx<S> {
    pub store: S,
    cancel: CancellationToken,
    timeout: Option<Duration>,
}

impl<S> WorkflowCtx<S>
where
    S: Send + Sync + 'static,
{
    pub fn new(store: S) -> Self {
        Self {
            store,
            cancel: CancellationToken::new(),
            timeout: None,
        }
    }

    pub fn with_store<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&S) -> R,
    {
        f(&self.store)
    }

    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

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
