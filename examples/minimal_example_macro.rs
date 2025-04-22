// examples/minimal_example_macro.rs
use floxide_macros::workflow;
use floxide_core::*;
use async_trait::async_trait;

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
impl Node for FooNode {
    type Input = u64;
    type Output = FooAction;

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        if input > self.threshold {
            let out = input * 2;
            println!("FooNode: {} > {} => Above({})", input, self.threshold, out);
            Ok(Transition::Next(
                FooAction::Above(out)
            ))
        } else {
            let msg = format!("{} <= {}", input, self.threshold);
            println!("FooNode: {} <= {} => Below(\"{}\")", input, self.threshold, msg);
            Ok(Transition::Next(
                FooAction::Below(msg),
            ))
        }
    }
}

/// Node that handles values above threshold
#[derive(Clone, Debug)]
pub struct BigNode;

#[async_trait]
impl Node for BigNode {
    type Input = u64;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        println!("BigNode: handling value {}", input);
        Ok(Transition::Finish)
    }
}

/// Node that handles values below threshold
#[derive(Clone, Debug)]
pub struct SmallNode;

#[async_trait]
impl Node for SmallNode {
    type Input = String;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: String,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        println!("SmallNode: handling message \"{}\"", input);
        Ok(Transition::Finish)
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
    edges {
        foo => {
            FooAction::Above(v) => [ big ];
            FooAction::Below(s) => [ small ];
        };
        big => {};
        small => {};
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = ThresholdWorkflow {
        foo: FooNode { threshold: 10 },
        big: BigNode,
        small: SmallNode,
    };
    let mut ctx = WorkflowCtx::new(());
    println!("Running with input 5:");
    wf.run(&mut ctx, 5).await?;
    println!("Running with input 42:");
    wf.run(&mut ctx, 42).await?;
    Ok(())
}
