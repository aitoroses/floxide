// examples/retry_example.rs
// Demonstrates a flaky node retried until success using RetryNode
use async_trait::async_trait;
use floxide_core::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A node that fails a fixed number of times before succeeding
#[derive(Clone)]
struct FlakyNode {
    max_failures: usize,
    state: Arc<Mutex<usize>>,
}

impl FlakyNode {
    fn new(max_failures: usize) -> Self {
        FlakyNode {
            max_failures,
            state: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl Node<()> for FlakyNode {
    type Input = ();
    type Output = ();

    async fn process(
        &self,
        _ctx: &(),
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let mut count = self.state.lock().unwrap();
        *count += 1;
        println!("FlakyNode: attempt {}", *count);
        if *count <= self.max_failures {
            Err(FloxideError::Generic(format!("failure #{}", *count)))
        } else {
            println!("FlakyNode: success on attempt {}", *count);
            Ok(Transition::Next(()))
        }
    }
}

/// Runs the flaky node with retry and returns Ok(()) if it succeeds
pub async fn run_retry_example() -> Result<(), Box<dyn std::error::Error>> {
    // Simple context that supports no cancellation or timeouts
    let ctx = WorkflowCtx::new(());
    // Create a flaky node that fails twice before succeeding
    let flaky = FlakyNode::new(2);
    // Retry policy: up to 5 attempts, exponential backoff starting at 100ms
    let policy = RetryPolicy::new(
        5,
        Duration::from_millis(100),
        Duration::from_secs(2),
        BackoffStrategy::Exponential,
        RetryError::All,
    );
    // Wrap with retry logic
    let retry_node = with_retry(flaky, policy);

    println!("Running flaky node with retry:");
    retry_node.process(&ctx.store, ()).await?;
    println!("Completed with retry policy.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_retry_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_retry_example() {
        run_retry_example()
            .await
            .expect("flaky node should eventually succeed");
    }
}
