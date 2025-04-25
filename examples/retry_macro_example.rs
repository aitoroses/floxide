//! Example: defining retries via the `workflow!` macro annotation
use async_trait::async_trait;
use floxide::context::SharedState;
use floxide_core::*;
use floxide_macros::workflow;
use std::time::Duration;

/// A node that fails a fixed number of times before succeeding
#[derive(Clone, Debug)]
pub struct FlakyNode {
    max_failures: usize,
    state: SharedState<usize>,
}

impl FlakyNode {
    fn new(max_failures: usize) -> Self {
        FlakyNode { max_failures, state: SharedState::new(0) }
    }
}

#[async_trait]
impl Node<()> for FlakyNode {
    type Input = ();
    type Output = String;

    async fn process(
        &self,
        _ctx: &(),
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let mut count = self.state.get().await;
        *count += 1;
        println!("FlakyNode: attempt {}", *count);
        if *count <= self.max_failures {
            Err(FloxideError::Generic(format!("failure #{}", *count)))
        } else {
            println!("FlakyNode: success on attempt {}", *count);
            Ok(Transition::Next("success".to_string()))
        }
    }
}

// Generate a workflow that attaches a retry policy to `flaky` via `#[retry=pol]`
workflow! {
    pub struct MacroRetryWorkflow {
        #[retry = pol]
        flaky: FlakyNode,
        pol: RetryPolicy,
    }
    start = flaky;
    context = ();
    edges {
        flaky => {}; // terminal
    }
}

/// Runs the macro retry workflow and returns the result string
pub async fn run_retry_macro_example() -> Result<String, Box<dyn std::error::Error>> {
    // Create policy: 3 attempts, linear backoff 50ms
    let policy = RetryPolicy::new(
        3,
        Duration::from_millis(50),
        Duration::from_secs(1),
        BackoffStrategy::Linear,
        RetryError::All,
    );
    // Instantiate with FlakyNode(1) so it succeeds on the second call
    let wf = MacroRetryWorkflow { flaky: FlakyNode::new(1), pol: policy };
    let ctx = WorkflowCtx::new(());
    let result = wf.run(&ctx, ()).await?;
    println!("Workflow result: {}", result);
    Ok(result)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();

    run_retry_macro_example().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_retry_macro_example() {
        let result = run_retry_macro_example().await.expect("workflow should run");
        assert_eq!(result, "success");
    }
}