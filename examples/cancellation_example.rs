// examples/cancellation_example.rs
// Demonstrates workflow cancellation: a long-running node will be aborted when its cancellation token is triggered.

use floxide_macros::workflow;
use floxide_core::*;
use async_trait::async_trait;
use tokio::time::sleep;
use std::time::Duration;

/// A node that sleeps for a specified duration before completing.
#[derive(Clone, Debug)]
pub struct SlowNode {
    dur: Duration,
}

#[async_trait]
impl Node for SlowNode {
    type Input = ();
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &C,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        C: Send + Sync + 'static,
    {
        println!("SlowNode: sleeping for {:?}", self.dur);
        sleep(self.dur).await;
        println!("SlowNode: woke up");
        Ok(Transition::Finish)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wf = CancelWorkflow {
        slow: SlowNode { dur: Duration::from_secs(2) },
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
        Ok(_) => println!("Workflow completed successfully"),
        Err(e) => println!("Workflow failed: {}", e),
    }
    Ok(())
}