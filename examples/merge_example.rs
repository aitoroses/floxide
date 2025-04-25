// examples/merge_example.rs
// Demonstrates using a custom MergeNode to collect split outputs

use async_trait::async_trait;
use floxide::context::SharedState;
use floxide_macros::workflow;
use floxide_core::{Node, Workflow, Transition, SplitNode, WorkflowCtx, FloxideError};

/// Context for MergeWorkflow: stores collected values and threshold
#[derive(Clone, Debug)]
struct MergeContext {
    values: SharedState<Vec<i32>>,
    expected: usize,
}

/// Node that merges incoming values, holding until all expected are received
#[derive(Clone, Debug)]
pub struct MergeNode;

#[async_trait]
impl Node<MergeContext> for MergeNode {
    type Input = i32;
    type Output = Vec<i32>;

    async fn process(
        &self,
        ctx: &MergeContext,
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let mut vals = ctx.values.get().await;
        vals.push(input);
        if vals.len() < ctx.expected {
            Ok(Transition::Hold)
        } else {
            let merged = vals.clone();
            Ok(Transition::Next(merged))
        }
    }
}

/// Terminal node that prints the merged vector
#[derive(Clone, Debug)]
pub struct TerminalNode;

#[async_trait]
impl Node<MergeContext> for TerminalNode {
    type Input = Vec<i32>;
    type Output = Vec<i32>;

    async fn process(
        &self,
        _ctx: &MergeContext,
        input: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("Merged values: {:?}", input);
        Ok(Transition::Next(input))
    }
}

// Define workflow: split -> merge -> terminal
workflow! {
    pub struct MergeWorkflow {
        split: SplitNode<i32, i32, fn(i32) -> Vec<i32>>,
        merge: MergeNode,
        terminal: TerminalNode,
    }
    context = MergeContext;
    start = split;
    edges {
        split => { [merge] };
        merge => { [terminal] };
        terminal => { [] };
    }
}

async fn run_merge_example() -> Result<Vec<i32>, FloxideError> {
    let ctx = MergeContext {
        values: SharedState::new(Vec::new()),
        expected: 3,
    };
    let wf_ctx = WorkflowCtx::new(ctx);
    let wf = MergeWorkflow {
        split: SplitNode::new(|n| vec![n - 1, n, n + 1]),
        merge: MergeNode,
        terminal: TerminalNode,
    };
    let result = wf.run(&wf_ctx, 10).await?;
    tracing::info!("Result: {:?}", result);
    Ok(result)
}

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_merge_example().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_merge_example() {
        let result = run_merge_example().await.expect("merge example failed");
        assert_eq!(result, vec![9, 10, 11]);
    }
}