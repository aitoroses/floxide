//! Example: defining retries via the `workflow!` macro annotation
use async_trait::async_trait;
use floxide::core::*;
use floxide_macros::workflow;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A node that fails a set number of times before succeeding
/// A node that fails a fixed number of times before succeeding
#[derive(Clone, Debug)]
pub struct FlakyNode {
    max_failures: usize,
    state: Arc<Mutex<usize>>,
}

impl FlakyNode {
    fn new(max_failures: usize) -> Self {
        FlakyNode { max_failures, state: Arc::new(Mutex::new(0)) }
    }
}

#[async_trait]
impl Node<()> for FlakyNode {
    type Input = ();
    type Output = &'static str;

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
            Ok(Transition::Next("success"))
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}