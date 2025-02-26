# Long-Running Node Example

This example demonstrates how to use long-running nodes in the Floxide framework for handling operations that take significant time to complete.

## Overview

Long-running nodes are useful for:
- Batch processing large datasets
- External API calls with unknown duration
- Complex computations
- Resource-intensive operations

## Implementation

Let's create a long-running node that processes a large dataset:

```rust
use floxide_core::{lifecycle_node, LifecycleNode, DefaultAction};
use floxide_longrunning::{LongRunningNode, Progress, LongRunningStatus};

// Define our context
#[derive(Debug, Clone)]
struct ProcessingContext {
    input_size: usize,
    processed_items: usize,
    results: Vec<String>,
}

// Create a data processing node
struct DataProcessor {
    chunk_size: usize,
}

impl LongRunningNode<ProcessingContext, DefaultAction> for DataProcessor {
    async fn start(&self, context: &mut ProcessingContext) -> Result<(), FloxideError> {
        // Initialize processing
        context.processed_items = 0;
        context.results.clear();
        Ok(())
    }

    async fn check_status(&self, context: &mut ProcessingContext) -> Result<LongRunningStatus, FloxideError> {
        if context.processed_items >= context.input_size {
            Ok(LongRunningStatus::Complete)
        } else {
            // Process next chunk
            let start = context.processed_items;
            let end = (start + self.chunk_size).min(context.input_size);
            
            for i in start..end {
                context.results.push(format!("Processed item {}", i));
                context.processed_items += 1;
            }

            let progress = Progress {
                percent: (context.processed_items as f64 / context.input_size as f64) * 100.0,
                message: format!("Processed {}/{} items", context.processed_items, context.input_size),
            };

            Ok(LongRunningStatus::Running(progress))
        }
    }

    async fn cleanup(&self, context: &mut ProcessingContext) -> Result<DefaultAction, FloxideError> {
        println!("Processing complete: {} items processed", context.processed_items);
        Ok(DefaultAction::Next)
    }
}
```

## Running the Example

Here's how to use the long-running node:

```rust
#[tokio::main]
async fn main() {
    // Create initial context
    let context = ProcessingContext {
        input_size: 1000,
        processed_items: 0,
        results: Vec::new(),
    };

    // Create the processor node
    let processor = DataProcessor {
        chunk_size: 100,
    };

    // Create and run the workflow
    let mut workflow = Workflow::new(processor);
    
    match workflow.run(context).await {
        Ok(final_context) => {
            println!("Processing complete!");
            println!("Results: {} items", final_context.results.len());
        }
        Err(e) => eprintln!("Processing failed: {}", e),
    }
}
```

## Advanced Usage

### Progress Monitoring

```rust
// Create a progress monitoring wrapper
struct ProgressMonitor<N> {
    inner: N,
}

impl<N: LongRunningNode<C, A>, C, A> LongRunningNode<C, A> for ProgressMonitor<N> {
    async fn check_status(&self, context: &mut C) -> Result<LongRunningStatus, FloxideError> {
        let status = self.inner.check_status(context).await?;
        if let LongRunningStatus::Running(progress) = &status {
            println!("Progress: {:.1}% - {}", progress.percent, progress.message);
        }
        Ok(status)
    }

    // ... delegate other methods to inner
}
```

### Cancellation Support

```rust
// Add cancellation support to a long-running node
impl LongRunningNode<ProcessingContext, DefaultAction> for DataProcessor {
    async fn check_status(&self, context: &mut ProcessingContext) -> Result<LongRunningStatus, FloxideError> {
        if context.should_cancel {
            return Ok(LongRunningStatus::Failed(
                FloxideError::new("Processing cancelled")
            ));
        }
        // ... normal processing ...
    }
}
```

### Resource Management

```rust
// Implement proper resource cleanup
impl LongRunningNode<ProcessingContext, DefaultAction> for DataProcessor {
    async fn cleanup(&self, context: &mut ProcessingContext) -> Result<DefaultAction, FloxideError> {
        // Clean up any temporary files
        if let Err(e) = cleanup_temp_files().await {
            eprintln!("Warning: cleanup failed: {}", e);
        }

        // Release any held resources
        context.results.shrink_to_fit();
        
        Ok(DefaultAction::Next)
    }
}
```

## Best Practices

1. **Resource Management**
   - Clean up resources in the cleanup phase
   - Handle cancellation gracefully
   - Monitor memory usage

2. **Progress Reporting**
   - Provide meaningful progress updates
   - Include both percentage and descriptive messages
   - Update progress at reasonable intervals

3. **Error Handling**
   - Handle transient failures
   - Provide clear error messages
   - Clean up resources on failure

4. **Testing**
   - Test cancellation scenarios
   - Verify resource cleanup
   - Test progress reporting
   - Simulate various failure modes

## See Also

- [Long-Running Node Implementation ADR](../adrs/0022-longrunning-node-implementation.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
- [Batch Processing](batch-processing.md)
