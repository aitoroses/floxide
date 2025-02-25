# flowrs-transform API Reference

This document provides a reference for the `flowrs-transform` crate, which extends the core Flowrs framework with transformation and asynchronous capabilities.

## Overview

The `flowrs-transform` crate provides extensions to the core Flowrs framework for building more complex workflows, including:

- Asynchronous node execution
- Transformation operations
- Batch processing capabilities
- Advanced workflow patterns

## Core Modules

### Transform Module

The Transform module provides utilities for transforming data within workflows:

```rust
use flowrs_transform::transform::{TransformNode, Transformer};

// Create a transformer function
let uppercase_transformer = |input: String| -> Result<String, FlowrsError> {
    Ok(input.to_uppercase())
};

// Create a transform node
let transform_node = TransformNode::new(uppercase_transformer);
```

### Async Module

The Async module provides asynchronous execution capabilities:

```rust
use flowrs_transform::async_ext::{AsyncNode, AsyncFlow};
use async_trait::async_trait;

struct MyAsyncNode;

#[async_trait]
impl AsyncNode for MyAsyncNode {
    type Context = MyContext;

    async fn exec_async(&self, ctx: &mut Self::Context) -> Result<(), FlowrsError> {
        // Perform async operations
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }
}

// Create an async flow
let async_flow = AsyncFlow::new(MyAsyncNode);
```

### Batch Module

The Batch module provides utilities for batch processing:

```rust
use flowrs_transform::batch::{BatchNode, BatchFlow, BatchContext};

struct MyBatchNode;

impl Node for MyBatchNode {
    type Context = MyBatchContext;

    fn exec(&self, ctx: &mut Self::Context) -> Result<(), FlowrsError> {
        // Process a single item
        Ok(())
    }
}

impl BatchNode<MyItem> for MyBatchNode {}

// Create a batch flow with concurrency limit of 4
let batch_flow = BatchFlow::new(MyBatchNode, 4);
```

## Key Types

### TransformNode

A node that applies a transformation function to input data:

```rust
pub struct TransformNode<F, I, O>
where
    F: Fn(I) -> Result<O, FlowrsError> + Send + Sync + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    transformer: F,
    _phantom: PhantomData<(I, O)>,
}

impl<F, I, O> TransformNode<F, I, O>
where
    F: Fn(I) -> Result<O, FlowrsError> + Send + Sync + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    pub fn new(transformer: F) -> Self {
        Self {
            transformer,
            _phantom: PhantomData,
        }
    }
}
```

### AsyncFlow

A wrapper for executing nodes asynchronously:

```rust
pub struct AsyncFlow<N>
where
    N: AsyncNode,
{
    node: N,
}

impl<N> AsyncFlow<N>
where
    N: AsyncNode,
{
    pub fn new(node: N) -> Self {
        Self { node }
    }

    pub async fn execute(&self, mut ctx: N::Context) -> Result<N::Context, FlowrsError> {
        self.node.exec_async(&mut ctx).await?;
        Ok(ctx)
    }
}
```

### BatchFlow

A wrapper for executing batch processing nodes:

```rust
pub struct BatchFlow<T, N>
where
    N: BatchNode<T>,
{
    node: N,
    concurrency: usize,
    _phantom: PhantomData<T>,
}

impl<T, N> BatchFlow<T, N>
where
    N: BatchNode<T>,
{
    pub fn new(node: N, concurrency: usize) -> Self {
        Self {
            node,
            concurrency,
            _phantom: PhantomData,
        }
    }

    pub fn execute<C>(&self, mut context: C) -> Result<C, FlowrsError>
    where
        C: BatchContext<T>,
    {
        self.node.process_batch(&mut context, self.concurrency)?;
        Ok(context)
    }
}
```

## Usage Examples

### Basic Transformation

```rust
use flowrs_core::prelude::*;
use flowrs_transform::prelude::*;

// Define a state type
struct MyState {
    value: String,
}

// Create a context
let state = MyState { value: "hello".to_string() };
let mut context = Context::new(state);

// Create a transform node
let transform_node = TransformNode::new(|state: &mut MyState| -> Result<(), FlowrsError> {
    state.value = state.value.to_uppercase();
    Ok(())
});

// Execute the node
transform_node.exec(&mut context)?;

// Check the result
assert_eq!(context.state().value, "HELLO");
```

### Async Execution

```rust
use flowrs_core::prelude::*;
use flowrs_transform::prelude::*;
use async_trait::async_trait;

// Define an async node
struct DelayedTransformNode;

#[async_trait]
impl AsyncNode for DelayedTransformNode {
    type Context = Context<MyState>;

    async fn exec_async(&self, ctx: &mut Self::Context) -> Result<(), FlowrsError> {
        // Simulate a delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Transform the state
        ctx.state_mut().value = ctx.state().value.to_uppercase();

        Ok(())
    }
}

// Create and execute the async flow
#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    let state = MyState { value: "hello".to_string() };
    let context = Context::new(state);

    let async_flow = AsyncFlow::new(DelayedTransformNode);
    let result_context = async_flow.execute(context).await?;

    assert_eq!(result_context.state().value, "HELLO");

    Ok(())
}
```

## Best Practices

When using the `flowrs-transform` crate, consider these best practices:

1. **Use Appropriate Concurrency Limits**: Set batch processing concurrency limits based on your system's capabilities.
2. **Handle Async Errors**: Properly handle errors in async code using the `?` operator or explicit error handling.
3. **Compose Transformations**: Chain multiple transform nodes for complex data transformations.
4. **Leverage Type Safety**: Use Rust's type system to ensure type safety in transformations.
5. **Consider Resource Usage**: Be mindful of resource usage in batch processing operations.

## Related Documentation

- [Async Runtime Selection](../architecture/async-runtime-selection.md)
- [Batch Processing Implementation](../architecture/batch-processing-implementation.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
- [Batch Processing Example](../examples/batch-processing.md)
