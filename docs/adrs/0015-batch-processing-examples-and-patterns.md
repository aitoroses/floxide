# ADR-0015: Batch Processing Examples and Best Practices

## Status

Accepted

## Date

2024-02-25

## Context

ADR-0007 established the fundamental architecture for batch processing in the Flow Framework, but there are still practical considerations for implementing batch processing in real-world applications. This ADR builds on ADR-0007 to document concrete patterns for batch processing implementations, common pitfalls, and best practices.

Specifically, we need guidance on:

1. How to properly implement the `BatchContext` trait
2. Working with the `BatchNode` and `BatchFlow` abstractions
3. Managing parallelism and resource usage
4. Handling type parameters in Rust's generic system
5. Tracking state across parallel executions

## Decision

We've implemented and documented several batch processing patterns:

### 1. The BatchContext Implementation Pattern

A proper implementation of `BatchContext` should:

```rust
impl BatchContext<Image> for ImageBatchContext {
    // Return the complete batch of items
    fn get_batch_items(&self) -> Result<Vec<Image>, FlowrsError> {
        Ok(self.images.clone())
    }

    // Create a context for a single item (called for each batch item)
    fn create_item_context(&self, item: Image) -> Result<Self, FlowrsError> {
        let mut ctx = self.clone();
        ctx.images = Vec::new();
        ctx.current_image = Some(item);
        Ok(ctx)
    }

    // Update main context with results after processing
    fn update_with_results(
        &mut self,
        results: &[Result<Image, FlowrsError>],
    ) -> Result<(), FlowrsError> {
        // Update statistics
        self.processed_count = results.iter().filter(|r| r.is_ok()).count();
        self.failed_count = results.iter().filter(|r| r.is_err()).count();

        // Update additional statistics if needed
        for result in results {
            match result {
                Ok(_) => self.add_stat("success"),
                Err(_) => self.add_stat("failure"),
            }
        }

        Ok(())
    }
}
```

### 2. Handling Type Parameters in BatchFlow

Working with generic parameters requires explicit type annotation to ensure proper type inference:

```rust
// Helper function to create a BatchFlow with the correct generic parameters
fn create_batch_flow(parallelism: usize) -> BatchFlow<ImageBatchContext, Image, DefaultAction> {
    let processor = SimpleImageProcessor::new("image_processor");

    // Create a workflow for processing a single item
    let workflow = Workflow::new(processor);

    // Create a batch flow
    BatchFlow::new(workflow, parallelism)
}
```

### 3. Direct Parallel Processing (Alternative Pattern)

For simpler cases, we can use Tokio's tasks directly without the full BatchFlow machinery:

```rust
// Process images in parallel with a given parallelism limit
async fn process_batch(
    images: Vec<Image>,
    parallelism: usize
) -> Vec<Result<Image, FlowrsError>> {
    use tokio::sync::Semaphore;
    use futures::stream::{self, StreamExt};

    let semaphore = std::sync::Arc::new(Semaphore::new(parallelism));

    let tasks = stream::iter(images)
        .map(|image| {
            let semaphore = semaphore.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = process_image(image).await;
                drop(_permit);
                result
            }
        })
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await;

    tasks
}
```

## Consequences

### Advantages

1. **Clear Patterns**: Developers have documented patterns to follow for batch processing
2. **Type Safety**: Strong type checking at compile time prevents runtime errors
3. **Flexibility**: Both high-level (BatchFlow) and low-level (direct parallelism) approaches are available
4. **Resource Control**: Explicit concurrency controls with semaphores prevent resource exhaustion

### Disadvantages

1. **Generic Complexity**: Rust's generic type system can be challenging when specifying nested generic types
2. **Memory Usage**: Cloning contexts for each item can lead to increased memory usage
3. **Learning Curve**: Proper implementation requires understanding both the Node trait and BatchContext traits

## Alternatives Considered

### 1. Simplified Trait with No Generics

We considered simplifying the `BatchContext` trait to avoid generic type parameters, but this would have decreased type safety and required more runtime type checking.

### 2. Iterator-Based API

We explored using Rust's iterator traits more extensively but found that the async nature of our operations made the streams-based approach more suitable.

### 3. Shared Mutable State

We considered using shared mutable state with synchronization primitives (Mutex, RwLock) instead of cloning contexts, but this introduced more complexity and potential for deadlocks.

## Implementation Notes

- The `BatchContext` trait requires implementing `Clone` for contexts
- Careful management of generic type parameters is crucial for type inference
- Use the helper function pattern to encapsulate type complexity when creating BatchFlow instances
- Consider direct parallel processing for simpler use cases
- Utilize context statistics to track processing results

## Related ADRs

- [ADR-0007: Batch Processing Implementation](0007-batch-processing-implementation.md)
