// examples/minimal_example.rs

use floxide_core::*;
use floxide_macros::workflow;
use async_trait::async_trait;

/// Example workflow: multiply by 2, add 3, then print result
pub struct Multiply2;
pub struct Add3;
pub struct PrintResult;

#[async_trait]
impl Node for Multiply2 {
    type Input = i32;
    type Output = i32;

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i32,
    ) -> Result<Transition<Self>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        let out = input * 2;
        println!("Multiply2: {} * 2 = {}", input, out);
        Ok(Transition::Next(Multiply2, out))
    }
}

#[async_trait]
impl Node for Add3 {
    type Input = i32;
    type Output = i32;

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i32,
    ) -> Result<Transition<Self>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        let out = input + 3;
        println!("Add3: {} + 3 = {}", input, out);
        Ok(Transition::Next(Add3, out))
    }
}

#[async_trait]
impl Node for PrintResult {
    type Input = i32;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &mut WorkflowCtx<C>,
        input: i32,
    ) -> Result<Transition<Self>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        println!("PrintResult: final result = {}", input);
        Ok(Transition::Finish)
    }
}

// Define a simple linear workflow: Multiply2 -> Add3 -> PrintResult
workflow! {
    name = SimpleMath;
    start = Multiply2;
    edges {
        Multiply2  => [Add3];
        Add3       => [PrintResult];
        PrintResult => [];
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = WorkflowCtx::new(());
    // Run the SimpleMath workflow starting from 5
    SimpleMathWorkflow.run(&mut ctx, 5).await?;
    Ok(())
}
