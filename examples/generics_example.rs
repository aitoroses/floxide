use std::fmt::{Debug, Display};

use async_trait::async_trait;
// Demonstrates workflow! macro with generic node types
use floxide::{workflow, Workflow};
use floxide_core::node::Node;
use floxide_core::transition::Transition;
use floxide_core::error::FloxideError;
use floxide_core::WorkflowCtx;

// A generic FooNode<T>
#[derive(Debug, Clone)]
pub struct FooNode<T> {
    pub value: T,
}

#[async_trait]
impl<T: Display + Debug + Clone + Sync + Send + 'static> Node<()> for FooNode<T> {
    type Input = T;
    type Output = String;
    async fn process(
        &self,
        _ctx: &(),
        input: T,
    ) -> Result<Transition<String>, FloxideError> {
        println!("FooNode processing: {:?}", input);
        Ok(Transition::Next(format!("foo-{}", self.value.clone())))
    }
}

// A generic BarNode<U>
#[derive(Debug, Clone)]
pub struct BarNode<U> {
    pub value: U,
}

#[async_trait]
impl<U: Display + Debug + Clone + Sync + Send + 'static> Node<()> for BarNode<U> {
    type Input = String;
    type Output = usize;
    async fn process(
        &self,
        _ctx: &(),
        input: String,
    ) -> Result<Transition<usize>, FloxideError> {
        println!("BarNode processing: {}", input);
        Ok(Transition::Next(input.len()))
    }
}

// Define the workflow using generic node types
workflow! {
    pub struct GenericsWorkflow {
        foo: FooNode<u32>,
        bar: BarNode<String>,
    }
    start = foo;
    // context is omitted, defaults to ()
    edges {
        foo => { [bar] };
        bar => { [] };
    }
}

/// Runs the GenericsWorkflow and returns the result
pub async fn run_generics_example() -> Result<usize, Box<dyn std::error::Error>> {
    let wf = GenericsWorkflow {
        foo: FooNode { value: 42 },
        bar: BarNode { value: "hello".to_string() },
    };
    let ctx = WorkflowCtx::new(());
    let result = wf.run(&ctx, 123u32).await?;
    println!("Workflow result: {:?}", result);
    Ok(result)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_generics_example().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_generics_example() {
        let result = run_generics_example().await.expect("workflow should run");
        // The BarNode returns the length of the string passed from FooNode
        assert_eq!(result, "foo-42".len());
    }
} 