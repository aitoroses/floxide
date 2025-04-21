// examples/minimal_example.rs

use floxide_core::*;
use floxide_macros::workflow;

// 1) Define two simple nodes using the Node trait directly:

pub struct AddOne;
#[async_trait]
impl Node for AddOne {
    type Input  = i32;
    type Output = i32;

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i32
    ) -> Result<Transition<Self>, FloxideError>
    where Self: Sized + Node, C: Send + Sync + 'static {
        let result = input + 1;
        Ok(Transition::Next(Self, result))
    }
}

pub struct PrintResult;
#[async_trait]
impl Node for PrintResult {
    type Input  = i32;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i32
    ) -> Result<Transition<Self>, FloxideError>
    where Self: Sized + Node, C: Send + Sync + 'static {
        println!("Result = {}", input);
        Ok(Transition::Finish)
    }
}

pub struct PrintResult2;
#[async_trait]
impl Node for PrintResult2 {
    type Input  = i64;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i64
    ) -> Result<Transition<Self>, FloxideError>
    where Self: Sized + Node, C: Send + Sync + 'static {
        println!("Result2 = {}", input);
        Ok(Transition::Finish)
    }
}

// 2) Wire them together in a minimal workflow! macro invocation:

workflow! {
    name = Minimal;
    start = [AddOne];
    edges {
      AddOne     => [PrintResult, PrintResult2];
      PrintResult => [];
    }
}

// 3) Use the WorkflowEngine to run it:

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an empty context (no custom store)
    let mut ctx = WorkflowCtx::new(());

    // Run the 'MinimalWorkflow' with initial input 5
    MinimalWorkflow.run(&mut ctx, 5).await?;

    Ok(())
}
