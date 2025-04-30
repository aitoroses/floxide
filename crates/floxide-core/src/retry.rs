//! Retry integration is handled via a wrapper node (`RetryNode`) around any `Node`.
//! This preserves the existing `Transition` API without adding a retry variant.
//!
//! Users can opt in by wrapping nodes in `RetryNode`, which applies the `RetryPolicy`.
use crate::context::{Context, WorkflowCtx};
use crate::error::FloxideError;
use crate::node::Node;
use crate::transition::Transition;
use async_trait::async_trait;
use std::time::Duration;

/// Helper trait: run a delay respecting cancellation and timeouts if available.
#[async_trait]
pub trait RetryDelay {
    /// Wait the given duration, returning an error if cancelled or timed out.
    async fn wait(&self, dur: Duration) -> Result<(), FloxideError>;
}

#[async_trait]
impl<S: Context> RetryDelay for WorkflowCtx<S> {
    async fn wait(&self, dur: Duration) -> Result<(), FloxideError> {
        self.run_future(async {
            tokio::time::sleep(dur).await;
            Ok(())
        })
        .await
    }
}

#[async_trait]
impl<S: Context> RetryDelay for S {
    async fn wait(&self, dur: Duration) -> Result<(), FloxideError> {
        tokio::time::sleep(dur).await;
        Ok(())
    }
}

/// Strategy for computing backoff durations.
#[derive(Clone, Copy, Debug)]
pub enum BackoffStrategy {
    /// Delay = initial_backoff * attempt_count
    Linear,
    /// Delay = initial_backoff * 2^(attempt_count - 1)
    Exponential,
}

/// Which errors should be retried.
#[derive(Clone, Copy, Debug)]
pub enum RetryError {
    /// Retry on any error.
    All,
    /// Retry only on cancellation.
    Cancelled,
    /// Retry only on timeout.
    Timeout,
    /// Retry only on generic errors.
    Generic,
}

/// Policy controlling retry behavior for nodes.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first).
    pub max_attempts: usize,
    /// Initial backoff duration between retries.
    pub initial_backoff: Duration,
    /// Maximum backoff duration allowed.
    pub max_backoff: Duration,
    /// Strategy to compute backoff durations.
    pub strategy: BackoffStrategy,
    /// Optional fixed jitter to add to each backoff.
    pub jitter: Option<Duration>,
    /// Error predicate controlling which errors to retry.
    pub retry_error: RetryError,
}

impl RetryPolicy {
    /// Construct a new RetryPolicy.
    pub fn new(
        max_attempts: usize,
        initial_backoff: Duration,
        max_backoff: Duration,
        strategy: BackoffStrategy,
        retry_error: RetryError,
    ) -> Self {
        RetryPolicy {
            max_attempts,
            initial_backoff,
            max_backoff,
            strategy,
            jitter: None,
            retry_error,
        }
    }

    /// Specify a fixed jitter offset to add to each backoff.
    pub fn with_jitter(mut self, jitter: Duration) -> Self {
        self.jitter = Some(jitter);
        self
    }

    /// Determine whether an error should be retried for the given attempt (1-based).
    pub fn should_retry(&self, error: &FloxideError, attempt: usize) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }
        match self.retry_error {
            RetryError::All => true,
            RetryError::Cancelled => matches!(error, FloxideError::Cancelled),
            RetryError::Timeout => matches!(error, FloxideError::Timeout(_)),
            RetryError::Generic => matches!(error, FloxideError::Generic(_)),
        }
    }

    /// Compute the backoff duration before the next retry given the attempt count (1-based).
    pub fn backoff_duration(&self, attempt: usize) -> Duration {
        let base = match self.strategy {
            BackoffStrategy::Linear => self.initial_backoff.saturating_mul(attempt as u32),
            BackoffStrategy::Exponential => {
                // Compute 2^(attempt-1) with shift, saturating at 32 bits
                let exp = attempt.saturating_sub(1);
                let factor = if exp < 32 { 1_u32 << exp } else { u32::MAX };
                self.initial_backoff.saturating_mul(factor)
            }
        };
        let capped = if base > self.max_backoff {
            self.max_backoff
        } else {
            base
        };
        if let Some(j) = self.jitter {
            capped.saturating_add(j)
        } else {
            capped
        }
    }
}

// Ergonomic retry helpers
/// Wrap an existing node with retry behavior according to the given policy.
///
/// # Example
///
/// ```rust
/// use floxide_core::*;
/// use std::time::Duration;
/// // Define a policy: up to 3 attempts, exponential backoff 100ms→200ms→400ms
/// let policy = RetryPolicy::new(
///     3,
///     Duration::from_millis(100),
///     Duration::from_secs(1),
///     BackoffStrategy::Exponential,
///     RetryError::All,
/// );
/// let my_node = FooNode::new();
/// let retry_node = with_retry(my_node, policy.clone());
/// ```
///
/// In future macro syntax you could write:
///
/// ```ignore
/// #[node(retry = "my_policy")]
/// foo: FooNode;
/// ```
pub fn with_retry<N>(node: N, policy: RetryPolicy) -> RetryNode<N> {
    RetryNode::new(node, policy)
}

/// Wrapper node that applies a `RetryPolicy` on inner node failures.
///
/// Internally it will re-run the inner node up to `policy.max_attempts`,
/// using backoff delays between attempts.
#[derive(Clone, Debug)]
pub struct RetryNode<N> {
    /// Inner node to invoke.
    pub inner: N,
    /// Policy controlling retry attempts and backoff.
    pub policy: RetryPolicy,
}

impl<N> RetryNode<N> {
    /// Create a new retry wrapper around `inner` with the given `policy`.
    pub fn new(inner: N, policy: RetryPolicy) -> Self {
        RetryNode { inner, policy }
    }
}
// RetryNode implements Node by looping on errors according to its policy.

#[async_trait]
impl<C, N> Node<C> for RetryNode<N>
where
    C: Context + RetryDelay,
    N: Node<C> + Clone + Send + Sync + 'static,
    N::Input: Clone + Send + 'static,
    N::Output: Send + 'static,
{
    type Input = N::Input;
    type Output = N::Output;

    async fn process(
        &self,
        ctx: &C,
        input: Self::Input,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let mut attempt = 1;
        loop {
            match self.inner.process(ctx, input.clone()).await {
                Ok(Transition::NextAll(vs)) => return Ok(Transition::NextAll(vs)),
                Ok(Transition::Next(out)) => return Ok(Transition::Next(out)),
                Ok(Transition::Hold) => return Ok(Transition::Hold),
                Ok(Transition::Abort(e)) | Err(e) => {
                    // emit tracing event for retry evaluation
                    tracing::debug!(attempt, error=%e, "RetryNode: caught error, evaluating retry policy");
                    if self.policy.should_retry(&e, attempt) {
                        let backoff = self.policy.backoff_duration(attempt);
                        tracing::debug!(attempt, backoff=?backoff, "RetryNode: retrying after backoff");
                        ctx.wait(backoff).await?;
                        attempt += 1;
                        continue;
                    } else {
                        tracing::warn!(attempt, error=%e, "RetryNode: aborting after reaching retry limit or non-retryable error");
                        return Err(e);
                    }
                }
            }
        }
    }
}

// Unit tests for RetryPolicy backoff and retry logic
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_linear_backoff() {
        let policy = RetryPolicy::new(
            5,
            Duration::from_millis(100),
            Duration::from_millis(1000),
            BackoffStrategy::Linear,
            RetryError::All,
        );
        assert_eq!(policy.backoff_duration(1), Duration::from_millis(100));
        assert_eq!(policy.backoff_duration(3), Duration::from_millis(300));
        // capped at max_backoff
        assert_eq!(policy.backoff_duration(20), Duration::from_millis(1000));
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::new(
            5,
            Duration::from_millis(50),
            Duration::from_millis(400),
            BackoffStrategy::Exponential,
            RetryError::All,
        );
        // 1 -> 50ms, 2 -> 100ms, 3 -> 200ms, 4 -> 400ms, capped thereafter
        assert_eq!(policy.backoff_duration(1), Duration::from_millis(50));
        assert_eq!(policy.backoff_duration(2), Duration::from_millis(100));
        assert_eq!(policy.backoff_duration(3), Duration::from_millis(200));
        assert_eq!(policy.backoff_duration(4), Duration::from_millis(400));
        assert_eq!(policy.backoff_duration(5), Duration::from_millis(400));
    }

    #[test]
    fn test_jitter_addition() {
        let mut policy = RetryPolicy::new(
            3,
            Duration::from_millis(100),
            Duration::from_millis(1000),
            BackoffStrategy::Linear,
            RetryError::All,
        );
        policy = policy.with_jitter(Duration::from_millis(25));
        assert_eq!(
            policy.backoff_duration(2),
            Duration::from_millis(100 * 2 + 25)
        );
    }

    #[test]
    fn test_retry_predicates() {
        let mut policy = RetryPolicy::new(
            3,
            Duration::from_millis(10),
            Duration::from_millis(100),
            BackoffStrategy::Linear,
            RetryError::Generic,
        );
        let gen_err = FloxideError::Generic("oops".into());
        let cancel_err = FloxideError::Cancelled;
        let timeout_err = FloxideError::Timeout(Duration::from_secs(1));
        // Generic only
        assert!(policy.should_retry(&gen_err, 1));
        assert!(!policy.should_retry(&cancel_err, 1));
        assert!(!policy.should_retry(&timeout_err, 1));
        // Exhausted attempts
        assert!(!policy.should_retry(&gen_err, 3));
        // RetryError::All
        policy.retry_error = RetryError::All;
        assert!(policy.should_retry(&cancel_err, 2));
        assert!(policy.should_retry(&timeout_err, 2));
    }
}
