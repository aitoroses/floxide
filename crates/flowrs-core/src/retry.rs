use async_trait::async_trait;
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::time::Duration;

use crate::action::ActionType;
use crate::error::FlowrsError;
use crate::node::{Node, NodeId, NodeOutcome};

/// Backoff strategy for retries
#[derive(Clone)]
pub enum BackoffStrategy {
    /// Constant time between retries
    Constant(Duration),
    /// Linear increase in time between retries
    Linear { base: Duration, increment: Duration },
    /// Exponential increase in time between retries (base * 2^attempt)
    Exponential { base: Duration, max: Duration },
    /// Custom backoff strategy implemented as a function
    Custom(CustomBackoff),
}

/// A wrapper for custom backoff functions that can be cloned and debugged
pub struct CustomBackoff {
    func: Box<dyn Fn(usize) -> Duration + Send + Sync>,
}

impl Clone for CustomBackoff {
    fn clone(&self) -> Self {
        // We can't actually clone the function, so this is a hack
        // In practice, RetryNode should be created just once and not cloned
        Self {
            func: Box::new(|attempt| Duration::from_millis(100 * attempt as u64)),
        }
    }
}

impl Debug for CustomBackoff {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomBackoff")
            .field("func", &"<function>")
            .finish()
    }
}

impl BackoffStrategy {
    /// Calculate the delay for a given attempt
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        match self {
            Self::Constant(duration) => *duration,
            Self::Linear { base, increment } => *base + (*increment * attempt as u32),
            Self::Exponential { base, max } => {
                let calculated = *base * u32::pow(2, attempt as u32);
                std::cmp::min(calculated, *max)
            }
            Self::Custom(custom) => (custom.func)(attempt),
        }
    }
}

impl Debug for BackoffStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(duration) => f.debug_tuple("Constant").field(duration).finish(),
            Self::Linear { base, increment } => f
                .debug_struct("Linear")
                .field("base", base)
                .field("increment", increment)
                .finish(),
            Self::Exponential { base, max } => f
                .debug_struct("Exponential")
                .field("base", base)
                .field("max", max)
                .finish(),
            Self::Custom(custom) => f.debug_tuple("Custom").field(custom).finish(),
        }
    }
}

/// A node that retries another node if it fails
pub struct RetryNode<N, Context, A = crate::action::DefaultAction>
where
    N: Node<Context, A>,
    Context: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    /// The node to retry
    inner_node: N,
    /// Maximum number of retry attempts
    max_retries: usize,
    /// Backoff strategy
    backoff_strategy: BackoffStrategy,
    /// Type markers
    _context: PhantomData<Context>,
    _action: PhantomData<A>,
}

impl<N, Context, A> RetryNode<N, Context, A>
where
    N: Node<Context, A>,
    Context: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    /// Create a new retry node with a constant backoff
    pub fn with_constant_backoff(inner_node: N, max_retries: usize, delay: Duration) -> Self {
        Self {
            inner_node,
            max_retries,
            backoff_strategy: BackoffStrategy::Constant(delay),
            _context: PhantomData,
            _action: PhantomData,
        }
    }

    /// Create a new retry node with linear backoff
    pub fn with_linear_backoff(
        inner_node: N,
        max_retries: usize,
        base: Duration,
        increment: Duration,
    ) -> Self {
        Self {
            inner_node,
            max_retries,
            backoff_strategy: BackoffStrategy::Linear { base, increment },
            _context: PhantomData,
            _action: PhantomData,
        }
    }

    /// Create a new retry node with exponential backoff
    pub fn with_exponential_backoff(
        inner_node: N,
        max_retries: usize,
        base: Duration,
        max: Duration,
    ) -> Self {
        Self {
            inner_node,
            max_retries,
            backoff_strategy: BackoffStrategy::Exponential { base, max },
            _context: PhantomData,
            _action: PhantomData,
        }
    }

    /// Create a new retry node with a custom backoff strategy
    pub fn with_custom_backoff<F>(inner_node: N, max_retries: usize, f: F) -> Self
    where
        F: Fn(usize) -> Duration + Send + Sync + 'static,
    {
        Self {
            inner_node,
            max_retries,
            backoff_strategy: BackoffStrategy::Custom(CustomBackoff { func: Box::new(f) }),
            _context: PhantomData,
            _action: PhantomData,
        }
    }
}

#[async_trait]
impl<N, Context, A> Node<Context, A> for RetryNode<N, Context, A>
where
    N: Node<Context, A> + std::fmt::Debug + Send + Sync,
    Context: std::fmt::Debug + Send + Sync + 'static,
    A: crate::action::ActionType + Default + std::fmt::Debug + Send + Sync + 'static,
    N::Output: Clone + Send + Sync + 'static,
{
    type Output = N::Output;

    fn id(&self) -> NodeId {
        self.inner_node.id()
    }

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, A>, FlowrsError> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.inner_node.process(ctx).await {
                Ok(outcome) => {
                    tracing::debug!(
                        attempt = attempt,
                        node_id = %self.id(),
                        "Node completed successfully after {} attempts",
                        attempt
                    );
                    return Ok(outcome);
                }
                Err(err) => {
                    if attempt >= self.max_retries {
                        tracing::error!(
                            attempt = attempt,
                            max_retries = self.max_retries,
                            node_id = %self.id(),
                            error = %err,
                            "Maximum retry attempts reached, failing"
                        );
                        return Err(err);
                    }

                    let delay = self.backoff_strategy.calculate_delay(attempt);
                    tracing::warn!(
                        attempt = attempt,
                        node_id = %self.id(),
                        error = %err,
                        delay_ms = delay.as_millis(),
                        "Node execution failed, retrying after {:?}",
                        delay
                    );

                    #[cfg(feature = "async")]
                    {
                        tokio::time::sleep(delay).await;
                    }

                    #[cfg(not(feature = "async"))]
                    {
                        // For compatibility with sync execution, we just delay using std
                        std::thread::sleep(delay);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::DefaultAction;

    #[derive(Debug, Clone)]
    struct TestContext {
        counter: usize,
        should_fail_until: usize,
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        // Create a custom node implementation for testing retry logic
        #[derive(Debug)]
        struct TestNodeImpl {
            id: NodeId,
        }

        #[async_trait]
        impl Node<TestContext, DefaultAction> for TestNodeImpl {
            type Output = String;

            fn id(&self) -> NodeId {
                self.id.clone()
            }

            async fn process(
                &self,
                ctx: &mut TestContext,
            ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FlowrsError> {
                ctx.counter += 1;
                if ctx.counter <= ctx.should_fail_until {
                    Err(FlowrsError::node_execution("test", "Simulated failure"))
                } else {
                    Ok(NodeOutcome::<String, DefaultAction>::Success(
                        "success".to_string(),
                    ))
                }
            }
        }

        let test_node = TestNodeImpl {
            id: "test-node".to_string(),
        };

        let retry_node = RetryNode::with_constant_backoff(
            test_node,
            5,
            Duration::from_millis(10), // short delay for tests
        );

        let mut ctx = TestContext {
            counter: 0,
            should_fail_until: 2, // fail first 2 attempts
        };

        let result = retry_node.process(&mut ctx).await;
        assert!(result.is_ok());
        assert_eq!(ctx.counter, 3); // should have run 3 times
    }

    #[tokio::test]
    async fn test_retry_exhausts_attempts() {
        // Create a custom node implementation that always fails
        #[derive(Debug)]
        struct AlwaysFailNode {
            id: NodeId,
        }

        #[async_trait]
        impl Node<TestContext, DefaultAction> for AlwaysFailNode {
            type Output = String;

            fn id(&self) -> NodeId {
                self.id.clone()
            }

            async fn process(
                &self,
                _ctx: &mut TestContext,
            ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FlowrsError> {
                Err(FlowrsError::node_execution("test", "Always failing"))
            }
        }

        let test_node = AlwaysFailNode {
            id: "always-fail".to_string(),
        };

        let retry_node = RetryNode::with_constant_backoff(test_node, 3, Duration::from_millis(10));

        let mut ctx = TestContext {
            counter: 0,
            should_fail_until: 999, // always fail
        };

        let result = retry_node.process(&mut ctx).await;
        assert!(result.is_err());
        // Should have attempted 3 times (max_retries)
    }

    #[tokio::test]
    async fn test_backoff_strategies() {
        // Test constant backoff
        let constant = BackoffStrategy::Constant(Duration::from_millis(100));
        assert_eq!(constant.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(constant.calculate_delay(2), Duration::from_millis(100));

        // Test linear backoff
        let linear = BackoffStrategy::Linear {
            base: Duration::from_millis(100),
            increment: Duration::from_millis(50),
        };
        assert_eq!(linear.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(linear.calculate_delay(1), Duration::from_millis(150));
        assert_eq!(linear.calculate_delay(2), Duration::from_millis(200));

        // Test exponential backoff
        let exponential = BackoffStrategy::Exponential {
            base: Duration::from_millis(100),
            max: Duration::from_millis(1000),
        };
        assert_eq!(exponential.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(exponential.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(exponential.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(exponential.calculate_delay(3), Duration::from_millis(800));
        assert_eq!(exponential.calculate_delay(4), Duration::from_millis(1000)); // Max reached

        // Test custom backoff
        let custom = BackoffStrategy::Custom(CustomBackoff {
            func: Box::new(|attempt| Duration::from_millis(attempt as u64 * 25)),
        });
        assert_eq!(custom.calculate_delay(1), Duration::from_millis(25));
        assert_eq!(custom.calculate_delay(2), Duration::from_millis(50));
        assert_eq!(custom.calculate_delay(10), Duration::from_millis(250));
    }
}
