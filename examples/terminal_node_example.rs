// examples/terminal_node_example.rs
// Demonstrates a workflow with a single terminal node returning a value

use floxide_macros::workflow;
use floxide_core::*;
use async_trait::async_trait;

/// A node that multiplies its input by 3
#[derive(Clone, Debug)]
pub struct TripleNode;

#[async_trait]
impl Node for TripleNode {
    type Input = i32;
    type Output = i32;

    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError>
    {
        let out = input * 3;
        println!("TripleNode: {} * 3 = {}", input, out);
        Ok(Transition::Next(out))
    }
}

// Generate a workflow where the single node is both the start and terminal
workflow! {
    pub struct TerminalWorkflow {
        triple: TripleNode,
    }
    // context = ();
    start = triple;
    edges {
        triple => {};
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wf = TerminalWorkflow {
        triple: TripleNode,
    };
    let ctx = WorkflowCtx::new(());
    let input = 7;
    println!("Running workflow with input {}:", input);
    // wf.run now returns the node's Output type (i32 here)
    let output: i32 = wf.run(&ctx, input).await?;
    println!("Workflow output: {}", output);
    Ok(())
}