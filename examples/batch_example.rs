// examples/batch_example.rs
// Demonstrates batch processing with routing and the workflow macro

use floxide_macros::workflow;
use floxide_core::*;
use async_trait::async_trait;

/// A simple node that multiplies its input by 2
#[derive(Clone, Debug)]
pub struct Multiply2;

#[async_trait]
impl Node for Multiply2 {
    type Input = i32;
    type Output = i32;

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i32,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Clone + Send + Sync + 'static,
    {
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

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Clone + Send + Sync + 'static,
    {
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

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Clone + Send + Sync + 'static,
    {
        println!("LargeNode: {:?}", inputs);
        Ok(Transition::Finish)
    }
}

/// Node that handles small batches
#[derive(Clone, Debug)]
pub struct SmallNode;

#[async_trait]
impl Node for SmallNode {
    type Input = Vec<i32>;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Clone + Send + Sync + 'static,
    {
        println!("SmallNode: {:?}", inputs);
        Ok(Transition::Finish)
    }
}

// Define a workflow that ties together batch processing and routing
workflow! {
    pub struct BatchWorkflow {
        multiply: BatchNode<Multiply2>,
        branch: BranchAfterMultiply,
        large: LargeNode,
        small: SmallNode,
    }
    start = multiply;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the workflow with batch size and threshold
    let mut wf = BatchWorkflow {
        multiply: BatchNode::new(Multiply2, 2),
        branch: BranchAfterMultiply { threshold: 20 },
        large: LargeNode,
        small: SmallNode,
    };
    let mut ctx = WorkflowCtx::new(());
    let inputs = vec![1, 2, 3, 4, 5];
    println!("Running batch workflow on {:?}", inputs);
    wf.run(&mut ctx, inputs).await?;
    Ok(())
}