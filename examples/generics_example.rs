use std::fmt::{Debug, Display};

use async_trait::async_trait;
// Demonstrates workflow! macro with generic node types
use floxide::workflow;
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
        Ok(Transition::Finish)
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

#[tokio::main]
async fn main() {
    let wf = GenericsWorkflow {
        foo: FooNode { value: 42 },
        bar: BarNode { value: "hello".to_string() },
    };
    let ctx = WorkflowCtx::new(());
    let result = wf.run(&ctx, 123u32).await;
    println!("Workflow result: {:?}", result);
} 