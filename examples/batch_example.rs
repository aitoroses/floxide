// examples/batch_example.rs
// Demonstrates batch processing with routing and the workflow macro

use async_trait::async_trait;
use floxide_core::*;
use floxide_macros::workflow;

/// A simple node that multiplies its input by 2
#[derive(Clone, Debug)]
pub struct Multiply2;

#[async_trait]
impl Node for Multiply2 {
    type Input = i32;
    type Output = i32;

    async fn process(
        &self,
        _ctx: &(),
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let out = input * 2;
        println!("Multiply2: {} * 2 = {}", input, out);
        Ok(Transition::Next(out))
    }
}

/// Action enum for branching after batch multiply
#[derive(Clone, Debug)]
pub enum BatchAction {
    Large(Vec<i32>),
    Small(Vec<i32>),
}

/// Node that sums a batch and branches based on sum
#[derive(Clone, Debug)]
pub struct BranchAfterMultiply {
    threshold: i32,
}

#[async_trait]
impl Node for BranchAfterMultiply {
    type Input = Vec<i32>;
    type Output = BatchAction;

    async fn process(
        &self,
        _ctx: &(),
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        // Sum and branch
        let sum: i32 = inputs.iter().sum();
        if sum > self.threshold {
            println!("Branch: sum {} > {} => Large", sum, self.threshold);
            Ok(Transition::Next(BatchAction::Large(inputs)))
        } else {
            println!("Branch: sum {} <= {} => Small", sum, self.threshold);
            Ok(Transition::Next(BatchAction::Small(inputs)))
        }
    }
}

/// Node that handles large batches
#[derive(Clone, Debug)]
pub struct LargeNode;

#[async_trait]
impl Node for LargeNode {
    type Input = Vec<i32>;
    type Output = ();

    async fn process(
        &self,
        _ctx: &(),
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("LargeNode: {:?}", inputs);
        Ok(Transition::Next(()))
    }
}

/// Node that handles small batches
#[derive(Clone, Debug)]
pub struct SmallNode;

#[async_trait]
impl Node for SmallNode {
    type Input = Vec<i32>;
    type Output = ();

    async fn process(
        &self,
        _ctx: &(),
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("SmallNode: {:?}", inputs);
        Ok(Transition::Next(()))
    }
}

// Define a workflow that ties together batch processing and routing
workflow! {
    pub struct BatchWorkflow {
        multiply: BatchNode<(), Multiply2>,
        branch: BranchAfterMultiply,
        large: LargeNode,
        small: SmallNode,
    }
    start = multiply;
    context = ();
    edges {
        // direct edge: multiply outputs feed into branch
        multiply => { [ branch ] };
        branch => {
            BatchAction::Large(v) => [ large ];
            BatchAction::Small(v) => [ small ];
        };
        large => {};
        small => {};
    }
}

/// Runs the batch workflow with a sample input and returns Ok(()) if it succeeds
pub async fn run_batch_example() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the workflow with batch size and threshold
    let wf = BatchWorkflow {
        multiply: BatchNode::new(Multiply2, 2),
        branch: BranchAfterMultiply { threshold: 20 },
        large: LargeNode,
        small: SmallNode,
    };
    let ctx = WorkflowCtx::new(());
    let inputs = vec![1, 2, 3, 4, 5];
    println!("Running batch workflow on {:?}", inputs);
    wf.run(&ctx, inputs).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_batch_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_batch_example() {
        run_batch_example()
            .await
            .expect("batch workflow should run");
    }
}
