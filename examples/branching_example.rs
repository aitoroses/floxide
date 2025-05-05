// examples/minimal_example_macro.rs
use async_trait::async_trait;
use floxide::*;
use serde::{Deserialize, Serialize};
/// Context type for the workflow
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MyCtx {
    pub value: u64,
}

/// Action enum for FooNode: branch carries different payloads
#[derive(Clone, Debug)]
pub enum FooAction {
    Above(u64),    // above threshold: carries numeric value
    Below(String), // below threshold: carries descriptive message
}

/// Node that compares input against a threshold
#[derive(Clone, Debug)]
pub struct FooNode {
    threshold: u64,
}

#[async_trait]
impl Node<MyCtx> for FooNode {
    type Input = u64;
    type Output = FooAction;

    async fn process(
        &self,
        _ctx: &MyCtx,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        if input > self.threshold {
            let out = input * 2;
            println!("FooNode: {} > {} => Above({})", input, self.threshold, out);
            Ok(Transition::Next(FooAction::Above(out)))
        } else {
            let msg = format!("{} <= {}", input, self.threshold);
            println!(
                "FooNode: {} <= {} => Below(\"{}\")",
                input, self.threshold, msg
            );
            Ok(Transition::Next(FooAction::Below(msg)))
        }
    }
}

/// Node that handles values above threshold
#[derive(Clone, Debug)]
pub struct BigNode;

#[async_trait]
impl Node<MyCtx> for BigNode {
    type Input = u64;
    type Output = String;

    async fn process(
        &self,
        _ctx: &MyCtx,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("BigNode: handling value {}", input);
        Ok(Transition::Next(format!("BigNode: {}", input)))
    }
}

/// Node that handles values below threshold
#[derive(Clone, Debug)]
pub struct SmallNode;

#[async_trait]
impl Node<MyCtx> for SmallNode {
    type Input = String;
    type Output = String;

    async fn process(
        &self,
        _ctx: &MyCtx,
        input: String,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("SmallNode: handling message \"{}\"", input);
        Ok(Transition::Next(format!("SmallNode: {}", input)))
    }
}

// Generate the ThresholdWorkflow using our `workflow!` procedural macro
workflow! {
    pub struct ThresholdWorkflow {
        foo: FooNode,
        big: BigNode,
        small: SmallNode,
    }
    start = foo;
    context = MyCtx;
    edges {
        foo => {
            FooAction::Above(_) => [ big ];
            FooAction::Below => [ small ];
        };
        big => {};
        small => {};
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_threshold_workflow_example().await?;
    Ok(())
}

/// Runs the threshold workflow with two inputs and returns Ok(()) if both succeed
pub async fn run_threshold_workflow_example() -> Result<(), Box<dyn std::error::Error>> {
    let wf = ThresholdWorkflow {
        foo: FooNode { threshold: 10 },
        big: BigNode,
        small: SmallNode,
    };
    let ctx = WorkflowCtx::new(MyCtx { value: 0 });
    println!("Running with input 5:");
    let res = wf.run(&ctx, 5).await?;
    assert_eq!(res, "SmallNode: 5 <= 10");
    println!("Running with input 42:");
    let res = wf.run(&ctx, 42).await?;
    assert_eq!(res, "BigNode: 84");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_threshold_workflow_example() {
        run_threshold_workflow_example()
            .await
            .expect("workflow should run for both inputs");
    }
}
