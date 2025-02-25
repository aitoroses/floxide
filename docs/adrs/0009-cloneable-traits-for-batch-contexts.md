# ADR-0009: Cloneable Types for Batch Processing

## Status

Accepted

## Date

2024-02-25

## Context

In implementing batch processing for the flowrs framework, we've encountered an ownership challenge: individual items that are processed in parallel tasks need to be accessed in multiple places:

1. When creating an item-specific context
2. When returning the original item as part of the result
3. When updating the batch context with results

The Rust borrow checker enforces strict ownership rules, and we need a solution that allows:

- Processing items in parallel
- Passing ownership of items into tasks
- Returning processed items from tasks
- Avoiding unnecessary copies of potentially large data

## Decision

We will require that item types used in `BatchContext<T>` must implement the `Clone` trait. This requirement will be documented and enforced through trait bounds.

### Updated BatchContext Trait

```rust
/// Trait for contexts that support batch processing
pub trait BatchContext<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Get the items to process in batch
    fn get_batch_items(&self) -> Result<Vec<T>, FlowrsError>;

    /// Create a context for a single item
    fn create_item_context(&self, item: T) -> Result<Self, FlowrsError>
    where
        Self: Sized;

    /// Update the main context with results from item processing
    fn update_with_results(
        &mut self,
        results: &Vec<Result<T, FlowrsError>>,
    ) -> Result<(), FlowrsError>;
}
```

### BatchNode Implementation

The `BatchNode` implementation will be updated to properly handle cloning:

```rust
// Create tasks for each item
for item in items {
    let semaphore = semaphore.clone();
    let workflow = self.item_workflow.clone();
    let ctx_clone = ctx.clone();

    // Clone the item for use in the task
    let item_clone = item.clone();

    // Spawn a task for each item
    let handle = tokio::spawn(async move {
        // Acquire a permit from the semaphore to limit concurrency
        let _permit = semaphore.acquire().await.unwrap();

        match ctx_clone.create_item_context(item_clone) {
            Ok(mut item_ctx) => match workflow.execute(&mut item_ctx).await {
                Ok(_) => Ok(item),
                Err(e) => Err(FlowrsError::batch_processing(
                    "Failed to process item",
                    Box::new(e),
                )),
            },
            Err(e) => Err(e),
        }
    });

    handles.push(handle);
}
```

## Consequences

### Advantages

1. **Clear Requirements**: Users know exactly what constraints apply to item types
2. **Type Safety**: The compiler enforces the Clone constraint
3. **Efficient Processing**: Items can be processed in parallel without unsafe code
4. **Safe Implementation**: No risk of use-after-move errors

### Disadvantages

1. **Constraint on Types**: Requires all batch item types to implement Clone
2. **Potential Memory Overhead**: May result in more copies than strictly necessary
3. **Potential Performance Impact**: Cloning large items could impact performance

### Alternatives Considered

#### Require Copy Instead of Clone

We considered requiring `Copy` instead of `Clone`, which would eliminate the need for explicit cloning. However, this would be too restrictive, as many useful types (like String, Vec, etc.) don't implement Copy.

#### Use References with Lifetime Parameters

Another approach would be to use references with explicit lifetimes throughout the batch processing system. While this would avoid cloning, it would significantly complicate the API and make it harder to use, especially with async code and closures.

#### Use Arc for Shared Ownership

We could require item types to be wrapped in Arc for shared ownership. This would avoid cloning the actual data but would require users to wrap and unwrap their data, complicating the API.

## Implementation Notes

- The `Clone` constraint will be added to all relevant trait bounds
- Documentation will clearly state that batch item types must be cloneable
- Examples will demonstrate best practices for minimizing cloning overhead
- Unit tests will verify correct behavior with various item types
