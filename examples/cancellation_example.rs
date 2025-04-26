// examples/cancellation_example.rs
// Demonstrates workflow cancellation: a long-running node will be aborted when its cancellation token is triggered.

use async_trait::async_trait;
use floxide_core::*;
use floxide_macros::workflow;
use std::time::Duration;
use tokio::time::sleep;

/// A node that sleeps for a specified duration before completing.
#[derive(Clone, Debug)]
pub struct SlowNode {
    dur: Duration,
}

#[async_trait]
impl Node for SlowNode {
    type Input = ();
    type Output = ();

    async fn process(
        &self,
        _ctx: &(),
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("SlowNode: sleeping for {:?}", self.dur);
        sleep(self.dur).await;
        println!("SlowNode: woke up");
        Ok(Transition::Next(()))
    }
}

// Generate a simple workflow with a single SlowNode
workflow! {
    pub struct CancelWorkflow {
        slow: SlowNode,
    }
    start = slow;
    edges {
        slow => {};
    }
}

/// Runs the cancel workflow and returns whether it was cancelled (true if cancelled, false if completed)
pub async fn run_cancellation_example() -> Result<bool, Box<dyn std::error::Error>> {
    let wf = CancelWorkflow {
        slow: SlowNode {
            dur: Duration::from_secs(2),
        },
    };
    let ctx = WorkflowCtx::new(());
    // Clone the cancellation token and trigger it after a delay
    let canceller = ctx.cancel_token().clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(500)).await;
        println!("Cancelling workflow");
        canceller.cancel();
    });
    match wf.run(&ctx, ()).await {
        Ok(_) => Ok(false),  // did not cancel
        Err(_e) => Ok(true), // cancelled
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let cancelled = run_cancellation_example().await?;
    if cancelled {
        println!("Workflow was cancelled");
    } else {
        println!("Workflow completed successfully");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_cancellation_example() {
        let cancelled = run_cancellation_example()
            .await
            .expect("should run workflow");
        assert!(cancelled, "Workflow should be cancelled");
    }
}
