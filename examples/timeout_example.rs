// examples/timeout_example.rs
// Demonstrates workflow timeout: a long-running node will be aborted if it exceeds the context timeout.

use floxide_macros::workflow;
use floxide_core::*;
use async_trait::async_trait;
use tokio::time::sleep;
use std::time::Duration;

/// A node that sleeps for a specified duration before completing.
pub struct SlowNode {
    dur: Duration,
}

#[async_trait]
impl Node for SlowNode {
    type Input = ();
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
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
    pub struct TimeoutWorkflow {
        slow: SlowNode,
    }
    start = slow;
    edges {
        slow => {};
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TimeoutWorkflow {
        slow: SlowNode { dur: Duration::from_secs(2) },
    };
    let mut ctx = WorkflowCtx::new(());
    // Set a timeout shorter than the node's sleep
    ctx.set_timeout(Duration::from_millis(500));
    match wf.run(&mut ctx, ()).await {
        Ok(_) => println!("Workflow completed successfully"),
        Err(e) => println!("Workflow failed: {}", e),
    }
    Ok(())
}