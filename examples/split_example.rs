// examples/split_example.rs
// Demonstrates using a custom SplitNode to fan out inputs to multiple parallel nodes.

use async_trait::async_trait;
use floxide_macros::workflow;
use floxide_core::{SplitNode, Node, Transition, WorkflowCtx, Workflow, FloxideError};

/// A simple node that prints its input.
#[derive(Clone, Debug)]
pub struct PrintNode;

#[async_trait]
impl Node<()> for PrintNode {
    type Input = i32;
    type Output = i32;

    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("PrintNode received: {}", input);
        Ok(Transition::Next(input))
    }
}

/// A Terminal node that returns a value.
#[derive(Clone, Debug)]
pub struct TerminalNode;

#[async_trait]
impl Node<()> for TerminalNode {
    type Input = i32;
    type Output = i32;

    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        Ok(Transition::Next(input))
    }
}

/// Split function: given n, emit n-1, n, n+1
fn splitter(n: i32) -> Vec<i32> {
    vec![n - 1, n, n + 1]
}

// Workflow that splits an integer and prints each result
workflow! {
    pub struct SplitWorkflow {
        split: SplitNode<i32, i32, fn(i32) -> Vec<i32>>,
        print: PrintNode,
        terminal: TerminalNode,
    }
    context = ();
    start = split;
    edges {
        split => { [print] };
        print => { [terminal] };
        terminal => { [] };
    }
}


async fn run_split_example() -> Result<(), FloxideError> {
    // Instantiate split and print nodes in the workflow
    let wf = SplitWorkflow {
        split: SplitNode::new(splitter),
        print: PrintNode,
        terminal: TerminalNode,
    };
    let ctx = WorkflowCtx::new(());
    // Run the workflow from start (split) to terminal (terminal).
    // This will fan-out and print all values.
    wf.run(&ctx, 10).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_split_example().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_split_example() {
        run_split_example().await.expect("split example failed");
    }
}
