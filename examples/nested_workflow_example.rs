// examples/nested_workflow_example.rs
// Demonstrates nesting a workflow as a Node via a local CompositeNode in an example

use async_trait::async_trait;
use floxide::{workflow, CompositeNode, FloxideError, Node, Transition, Workflow, WorkflowCtx};

/// A simple node that doubles its input
#[derive(Clone, Debug)]
pub struct DoubleNode;
#[async_trait]
impl Node for DoubleNode {
    type Input = i32;
    type Output = i32;
    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("DoubleNode: {} -> {}", input, input * 2);
        Ok(Transition::Next(input * 2))
    }
}

/// A simple node that adds 10
#[derive(Clone, Debug)]
pub struct AddTenNode;
#[async_trait]
impl Node for AddTenNode {
    type Input = i32;
    type Output = i32;
    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("AddTenNode: {} -> {}", input, input + 10);
        Ok(Transition::Next(input + 10))
    }
}

// Define a sub-workflow: double then add ten
workflow! {
    pub struct InnerWorkflow {
        double: DoubleNode,
        addten: AddTenNode,
    }
    start = double;
    context = ();
    edges {
        double => { [ addten ] };
        addten => {};
    }
}

/// A final node that prints the result and finishes
#[derive(Clone, Debug)]
pub struct PrintNode;
#[async_trait]
impl Node for PrintNode {
    type Input = i32;
    type Output = ();
    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("PrintNode: final output = {}", input);
        Ok(Transition::Next(()))
    }
}

// Outer workflow uses the CompositeNode to embed InnerWorkflow
workflow! {
    pub struct OuterWorkflow {
        sub: CompositeNode<(), InnerWorkflow>,
        print: PrintNode,
    }
    start = sub;
    context = ();
    edges {
        sub => { [ print ] };
        print => {};
    }
}

/// Runs the nested workflow with input 5 and returns Ok(()) if it succeeds
pub async fn run_nested_workflow_example() -> Result<(), Box<dyn std::error::Error>> {
    // Build the inner workflow and wrap it
    let inner = InnerWorkflow {
        double: DoubleNode,
        addten: AddTenNode,
    };
    let ctx = WorkflowCtx::new(());
    let wf = OuterWorkflow {
        sub: CompositeNode::new(inner, &ctx),
        print: PrintNode,
    };
    println!("Running nested workflow starting at 5:");
    wf.run(&ctx, 5).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_nested_workflow_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_nested_workflow_example() {
        run_nested_workflow_example()
            .await
            .expect("nested workflow should run");
    }
}
