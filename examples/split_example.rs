// examples/split_example.rs
// Demonstrates using a custom SplitNode to fan out inputs to multiple parallel nodes.

use async_trait::async_trait;
use floxide_macros::workflow;
use floxide_core::{SplitNode, Node, Transition, WorkflowCtx, Workflow, FloxideError};

/// A simple node that prints its input.
/// A simple node that prints its input.
#[derive(Clone, Debug)]
pub struct PrintNode;

#[async_trait]
impl Node<()> for PrintNode {
    type Input = i32;
    type Output = ();

    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("PrintNode received: {}", input);
        Ok(Transition::Next(()))
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
    }
    context = ();
    start = split;
    edges {
        split => { [print] };
        print => { [] };
    }
}

use std::collections::VecDeque;

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    // Instantiate split and print nodes in the workflow
    let wf = SplitWorkflow {
        split: SplitNode::new(splitter),
        print: PrintNode,
    };
    let ctx = WorkflowCtx::new(());
    // Seed the split with initial input
    let mut queue: VecDeque<SplitWorkflowWorkItem> = VecDeque::new();
    queue.push_back(SplitWorkflowWorkItem::Split(10));
    // Manually process all items until queue is empty
    while let Some(item) = queue.pop_front() {
        // process_work_item enqueues successors and returns Some(output) on terminal nodes
        let terminal = wf.process_work_item(&ctx, item, &mut queue).await?;
        if let Some(_) = terminal {
            // terminal outputs ignored for PrintNode
        }
    }
    Ok(())
}