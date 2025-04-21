use std::time::Duration;
use tokio_util::sync::CancellationToken;

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
}
