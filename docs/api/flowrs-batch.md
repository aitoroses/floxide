# flowrs-batch API Reference

The `flowrs-batch` crate provides batch processing capabilities for the Flowrs framework.

## Overview

This crate implements batch processing patterns for handling collections of items in parallel. It provides:

- Batch nodes for processing collections
- Configurable concurrency limits
- Progress tracking
- Error handling for batch operations

## Key Types

### BatchNode

```rust
pub trait BatchNode<I, O>: Send + Sync {
    async fn process_batch(&self, items: Vec<I>) -> Result<Vec<O>, FlowrsError>;
    fn concurrency_limit(&self) -> usize;
}
```

The `BatchNode` trait defines the core interface for batch processing nodes.

### BatchContext

```rust
pub struct BatchContext<T> {
    items: Vec<T>,
    results: Vec<T>,
    errors: Vec<FlowrsError>,
    progress: Progress,
}
```

`BatchContext` holds the state of a batch processing operation.

### BatchOptions

```rust
pub struct BatchOptions {
    pub concurrency_limit: usize,
    pub chunk_size: usize,
    pub retry_count: usize,
    pub backoff_duration: Duration,
}
```

`BatchOptions` configures the behavior of batch processing.

## Usage Example

```rust
use flowrs_batch::{batch_node, BatchNode, BatchContext};

// Create a batch processing node
fn create_batch_processor() -> impl BatchNode<String, String> {
    batch_node(
        10, // Concurrency limit
        |item: String| async move {
            Ok(item.to_uppercase())
        }
    )
}

// Use the node in a workflow
let node = create_batch_processor();
let mut context = BatchContext::new(vec![
    "hello".to_string(),
    "world".to_string(),
]);

let result = node.process_batch(context.items).await?;
println!("Processed items: {:?}", result);
```

## Advanced Features

### Chunked Processing

```rust
let node = batch_node(5, process_item)
    .with_chunk_size(100) // Process in chunks of 100 items
    .with_progress_callback(|progress| {
        println!("Progress: {}%", progress.percent);
    });
```

### Error Handling

```rust
let node = batch_node(5, process_item)
    .with_retry(3) // Retry failed items up to 3 times
    .with_backoff(Duration::from_secs(1)) // Wait between retries
    .with_error_handler(|e| {
        eprintln!("Processing error: {}", e);
        None // Skip failed items
    });
```

### Custom Batch Processing

```rust
struct CustomBatchProcessor;

impl BatchNode<String, String> for CustomBatchProcessor {
    async fn process_batch(&self, items: Vec<String>) -> Result<Vec<String>, FlowrsError> {
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(process_item(item).await?);
        }
        Ok(results)
    }

    fn concurrency_limit(&self) -> usize {
        10
    }
}
```

## Error Handling

The crate uses the standard `FlowrsError` type for error handling. All operations that can fail return a `Result<T, FlowrsError>`.

## Best Practices

1. Choose appropriate concurrency limits based on:
   - System resources
   - External service limits
   - Data characteristics

2. Implement proper error handling:
   - Retry transient failures
   - Log permanent failures
   - Clean up resources

3. Monitor batch processing:
   - Track progress
   - Log performance metrics
   - Handle resource constraints

4. Consider chunking for large datasets:
   - Balance memory usage
   - Maintain responsiveness
   - Handle partial failures

## See Also

- [Batch Processing Implementation ADR](../adrs/0007-batch-processing-implementation.md)
- [Batch Processing Example](../examples/batch-processing.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
