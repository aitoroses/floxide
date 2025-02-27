# ADR-0007: Batch Processing Implementation

## Status

Accepted

## Date

2025-02-27

## Context

The floxide framework needs batch processing capabilities to efficiently handle parallel execution of workflows on collections of items. We need to design a batch processing system that leverages Rust's ownership model and concurrency features.

We need to design a batch processing system that:

1. Efficiently processes collections of items in parallel
2. Respects configurable concurrency limits
3. Provides proper error handling for individual item failures
4. Integrates well with the existing workflow system
5. Follows Rust idioms and best practices

## Decision

We'll implement batch processing with a two-tier approach:

1. A `BatchContext` trait to define contexts that support batch operations
2. A `BatchNode` implementation that can process items concurrently
3. A `BatchFlow` wrapper that provides a simplified API for batch execution

### BatchContext Trait

The `BatchContext` trait will define how batch-supporting contexts should behave:

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

### BatchNode Implementation

The `BatchNode` will implement the `Node` trait and use Tokio tasks to process items in parallel, with a semaphore to control concurrency:

```rust
pub struct BatchNode<Context, ItemType, A = crate::action::DefaultAction>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    id: NodeId,
    item_workflow: Arc<Workflow<Context, A>>,
    parallelism: usize,
    _phantom: PhantomData<(Context, ItemType, A)>,
}
```

### BatchFlow Implementation

The `BatchFlow` will provide a simpler way to execute batch operations without directly dealing with nodes:

```rust
pub struct BatchFlow<Context, ItemType, A = crate::action::DefaultAction>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    id: NodeId,
    batch_node: BatchNode<Context, ItemType, A>,
}
```

## Consequences

### Advantages

1. **Better Parallelism**: Rust's async runtime with Tokio provides efficient parallel processing
2. **Type Safety**: The approach is fully type-safe with no runtime type checking needed
3. **Resource Control**: Explicit concurrency controls prevent overwhelming system resources
4. **Integration**: Seamlessly integrates with the existing workflow system
5. **Error Isolation**: Individual item failures don't stop the entire batch

### Disadvantages

1. **Complexity**: Requires implementing the BatchContext trait for contexts that support batch operations
2. **Resource Overhead**: Each parallel task incurs some overhead for spawning and synchronization
3. **Context Cloning**: Requires contexts to be clonable, which might be inefficient for large contexts

### Testing and Verification

We've added comprehensive tests for both `BatchNode` and `BatchFlow` to verify:

1. Parallel processing capabilities
2. Proper error handling
3. Context updates after processing
4. Integration with the workflow system

## Alternatives Considered

### Stream-Based Processing

We initially considered a purely stream-based approach using the futures crate StreamExt traits:

```rust
stream::iter(items)
    .map(|item| async { /* process item */ })
    .buffer_unordered(self.parallelism)
    .collect::<Vec<_>>()
    .await
```

However, this approach was more limited in handling context updates and didn't provide as much control over error handling.

### Single-Threaded Processing

We considered a simpler, single-threaded approach that processes items sequentially. While simpler, this would not take advantage of multi-core systems for CPU-bound tasks.

## Implementation Notes

- We're using Tokio's Semaphore for concurrency control
- Each item gets its own task, allowing true parallelism for CPU-bound operations
- Results are collected back in the original context after all items are processed
- Error handling preserves information about individual item failures
