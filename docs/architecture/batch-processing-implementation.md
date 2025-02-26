# Batch Processing Implementation

This document describes the batch processing implementation in the Floxide framework.

## Overview

Batch processing in the Floxide framework enables efficient parallel execution of workflows on collections of items. This capability is essential for handling large datasets and maximizing throughput in workflow applications.

## Core Concepts

The batch processing implementation is built around several key concepts:

1. **BatchContext**: A specialized context that manages collections of items
2. **BatchNode**: A node that can process items in parallel
3. **BatchFlow**: A workflow orchestrator for batch processing

## BatchContext

The `BatchContext` trait defines how contexts that support batch operations should behave:

```rust
/// Trait for contexts that support batch processing
pub trait BatchContext<T> {
    /// Get the items to process in batch
    fn get_batch_items(&self) -> Result<Vec<T>, FloxideError>;

    /// Create a context for a single item
    fn create_item_context(&self, item: T) -> Result<Self, FloxideError> where Self: Sized;

    /// Update the main context with results from item processing
    fn update_with_results(&mut self, results: Vec<Result<T, FloxideError>>) -> Result<(), FloxideError>;
}
```

A concrete implementation of `BatchContext` typically wraps a standard context and adds batch-specific functionality:

```rust
pub struct SimpleBatchContext<S, T> {
    inner_context: Context<S>,
    items: Vec<T>,
    results: Vec<Result<T, FloxideError>>,
}

impl<S, T> BatchContext<T> for SimpleBatchContext<S, T>
where
    S: Clone + Send + 'static,
    T: Clone + Send + 'static,
{
    fn get_batch_items(&self) -> Result<Vec<T>, FloxideError> {
        Ok(self.items.clone())
    }

    fn create_item_context(&self, item: T) -> Result<Self, FloxideError> {
        Ok(SimpleBatchContext {
            inner_context: Context::new(self.inner_context.state().clone()),
            items: vec![item],
            results: Vec::new(),
        })
    }

    fn update_with_results(&mut self, results: Vec<Result<T, FloxideError>>) -> Result<(), FloxideError> {
        self.results = results;
        Ok(())
    }
}
```

## BatchNode

The `BatchNode` trait extends the standard `Node` trait with batch processing capabilities:

```rust
pub trait BatchNode<T>: Node {
    /// Process a batch of items concurrently
    fn process_batch(&self, ctx: &mut Self::Context, concurrency: usize) -> Result<(), FloxideError>
    where
        Self::Context: BatchContext<T>,
    {
        let items = ctx.get_batch_items()?;
        let processor = BatchProcessor::new(concurrency);

        let results = processor.process_items(items, |item| {
            let mut item_ctx = ctx.create_item_context(item)?;
            self.exec(&mut item_ctx)?;
            Ok(item)
        })?;

        ctx.update_with_results(results)?;
        Ok(())
    }
}
```

## BatchProcessor

The `BatchProcessor` handles the actual parallel execution of items:

```rust
pub struct BatchProcessor {
    concurrency_limit: usize,
}

impl BatchProcessor {
    pub fn new(concurrency_limit: usize) -> Self {
        Self { concurrency_limit }
    }

    pub fn process_items<T, F>(&self, items: Vec<T>, processor: F) -> Result<Vec<Result<T, FloxideError>>, FloxideError>
    where
        F: Fn(T) -> Result<T, FloxideError> + Send + Sync + 'static,
        T: Send + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        let processor = Arc::new(processor);

        let handles: Vec<_> = items
            .into_iter()
            .map(|item| {
                let semaphore = Arc::clone(&semaphore);
                let processor = Arc::clone(&processor);

                thread::spawn(move || {
                    let _permit = semaphore.acquire();
                    processor(item)
                })
            })
            .collect();

        let results = handles
            .into_iter()
            .map(|handle| match handle.join() {
                Ok(result) => result,
                Err(_) => Err(FloxideError::ThreadPanicked),
            })
            .collect();

        Ok(results)
    }
}
```

## BatchFlow

The `BatchFlow` provides a simplified API for batch processing workflows:

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

    pub fn execute<C>(&self, mut context: C) -> Result<C, FloxideError>
    where
        C: BatchContext<T>,
    {
        self.node.process_batch(&mut context, self.concurrency)?;
        Ok(context)
    }
}
```

## Usage Example

Here's an example of using the batch processing capabilities:

```rust
// Define a state type
struct MyState {
    processed_count: usize,
}

// Define an item type
struct MyItem {
    id: usize,
    value: String,
}

// Create a batch context
let state = MyState { processed_count: 0 };
let items = vec![
    MyItem { id: 1, value: "item1".to_string() },
    MyItem { id: 2, value: "item2".to_string() },
    MyItem { id: 3, value: "item3".to_string() },
];
let context = SimpleBatchContext::new(state, items);

// Create a batch node
struct MyBatchNode;

impl Node for MyBatchNode {
    type Context = SimpleBatchContext<MyState, MyItem>;

    fn exec(&self, ctx: &mut Self::Context) -> NodeResult {
        // Process a single item
        let item = &ctx.get_batch_items()?[0];
        println!("Processing item {}: {}", item.id, item.value);
        ctx.inner_context.state_mut().processed_count += 1;
        Ok(())
    }
}

impl BatchNode<MyItem> for MyBatchNode {}

// Create and execute a batch flow
let batch_flow = BatchFlow::new(MyBatchNode, 4);
let result = batch_flow.execute(context);
```

## Conclusion

The batch processing implementation in the Floxide framework provides a powerful and flexible approach to parallel execution of workflows. By leveraging Rust's concurrency features and the framework's core abstractions, it enables efficient processing of large datasets while maintaining type safety and proper error handling.

For more detailed information on the batch processing implementation, refer to the [Batch Processing Implementation ADR](../adrs/0007-batch-processing-implementation.md).
