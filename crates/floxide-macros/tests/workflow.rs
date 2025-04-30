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
    type Output = Vec<i32>;

    async fn process(
        &self,
        _ctx: &(),
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("LargeNode: {:?}", inputs);
        Ok(Transition::Next(inputs))
    }
}

/// Node that handles small batches
#[derive(Clone, Debug)]
pub struct SmallNode;

#[async_trait]
impl Node for SmallNode {
    type Input = Vec<i32>;
    type Output = Vec<i32>;

    async fn process(
        &self,
        _ctx: &(),
        inputs: Vec<i32>,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("SmallNode: {:?}", inputs);
        Ok(Transition::Next(inputs))
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

#[tokio::test]
async fn test_batch_workflow() {
    // Initialize the workflow with batch size and threshold
    let wf = BatchWorkflow {
        multiply: BatchNode::new(Multiply2, 2),
        branch: BranchAfterMultiply { threshold: 20 },
        large: LargeNode,
        small: SmallNode,
    };
    let ctx = WorkflowCtx::new(());
    let input = vec![1, 2, 3, 4, 5];
    let output = wf.run(&ctx, input).await.unwrap();
    assert_eq!(output, vec![2, 4, 6, 8, 10]);
}
