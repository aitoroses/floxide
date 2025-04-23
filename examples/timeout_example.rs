// examples/timeout_example.rs
// Demonstrates workflow timeout: a long-running node will be aborted if it exceeds the context timeout.

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

    async fn process(
        &self,
        _ctx: &(),
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError>
    {
        println!("SlowNode: sleeping for {:?}", self.dur);
        sleep(self.dur).await;
        println!("SlowNode: woke up");
        Ok(Transition::Next(()))
    }
}

// Generate a simple workflow with a single SlowNode
workflow! {
    pub struct TimeoutWorkflow {
        slow: SlowNode,
    }
    context = ();
    start = slow;
    edges {
        slow => {};
    }
}

/// Runs the timeout workflow and returns whether it timed out (true if timeout error, false if completed)
pub async fn run_timeout_workflow() -> Result<bool, Box<dyn std::error::Error>> {
    let wf = TimeoutWorkflow {
        slow: SlowNode { dur: Duration::from_secs(2) },
    };
    let mut ctx = WorkflowCtx::new(());
    // Set a timeout shorter than the node's sleep
    ctx.set_timeout(Duration::from_millis(500));
    match wf.run(&ctx, ()).await {
        Ok(_) => Ok(false), // did not timeout
        Err(_e) => Ok(true), // timed out
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timed_out = run_timeout_workflow().await?;
    if timed_out {
        println!("Workflow failed due to timeout");
    } else {
        println!("Workflow completed successfully");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_timeout_workflow() {
        let timed_out = run_timeout_workflow().await.expect("should run workflow");
        assert!(timed_out, "Workflow should time out");
    }
}