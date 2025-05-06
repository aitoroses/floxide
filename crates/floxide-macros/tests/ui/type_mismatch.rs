use async_trait::async_trait;
use floxide_core::*;
use floxide_macros::workflow;

/// Define two nodes with mismatched types: FooNode outputs u32, BarNode expects i32
#[derive(Clone, Debug)]
struct FooNode;

#[async_trait]
impl Node for FooNode {
    type Input = i32;
    type Output = u32;
    async fn process(&self, _ctx: &(), input: i32) -> Result<Transition<u32>, FloxideError> {
        Ok(Transition::Next(input as u32))
    }
}

#[derive(Clone, Debug)]
struct BarNode;

#[async_trait]
impl Node for BarNode {
    type Input = i32; // Intentionally mismatched: expects i32, but FooNode outputs u32
    type Output = i32;
    async fn process(&self, _ctx: &(), input: i32) -> Result<Transition<i32>, FloxideError> {
        Ok(Transition::Next(input))
    }
}

// Declare a workflow connecting FooNode to BarNode
workflow! {
    struct TestWorkflow {
        foo: FooNode,
        bar: BarNode,
    }
    start = foo;
    context = ();
    edges {
        foo => { [ bar ] };
        bar => {};
    }
}
//~ ERROR assert_output_of_foo_matches_input_of_bar
//~ ERROR expected `i32`, found `u32`

fn main() {}