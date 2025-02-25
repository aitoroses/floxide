# Batch Processing Example

This document provides a complete example of using batch processing capabilities in the Flowrs framework.

## Overview

Batch processing allows you to efficiently process collections of items in parallel. This example demonstrates how to create and use batch nodes to process multiple items concurrently.

## Prerequisites

Before running this example, ensure you have the following dependencies in your `Cargo.toml`:

```toml
[dependencies]
flowrs-core = "0.1.0"
flowrs-transform = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Example Implementation

### Step 1: Define State and Item Types

First, define the state and item types for your batch processing workflow:

```rust
use flowrs_core::prelude::*;
use flowrs_transform::prelude::*;
use std::sync::Arc;

// Define a state type to track processing
#[derive(Clone)]
struct ProcessingState {
    total_processed: usize,
    total_errors: usize,
}

// Define an item type to process
#[derive(Clone, Debug)]
struct DataItem {
    id: usize,
    value: String,
}
```

### Step 2: Create a Batch Context

Next, create a batch context that will manage the collection of items:

```rust
// Create a batch context implementation
struct DataBatchContext {
    state: ProcessingState,
    items: Vec<DataItem>,
    results: Vec<Result<DataItem, FlowrsError>>,
}

impl DataBatchContext {
    fn new(items: Vec<DataItem>) -> Self {
        Self {
            state: ProcessingState {
                total_processed: 0,
                total_errors: 0,
            },
            items,
            results: Vec::new(),
        }
    }

    fn get_results(&self) -> &Vec<Result<DataItem, FlowrsError>> {
        &self.results
    }
}

impl BatchContext<DataItem> for DataBatchContext {
    fn get_batch_items(&self) -> Result<Vec<DataItem>, FlowrsError> {
        Ok(self.items.clone())
    }

    fn create_item_context(&self, item: DataItem) -> Result<Self, FlowrsError> {
        Ok(Self {
            state: self.state.clone(),
            items: vec![item],
            results: Vec::new(),
        })
    }

    fn update_with_results(&mut self, results: Vec<Result<DataItem, FlowrsError>>) -> Result<(), FlowrsError> {
        self.results = results;

        // Update state based on results
        for result in &self.results {
            match result {
                Ok(_) => self.state.total_processed += 1,
                Err(_) => self.state.total_errors += 1,
            }
        }

        Ok(())
    }
}
```

### Step 3: Implement a Batch Node

Now, implement a batch node that will process each item:

```rust
// Create a batch processing node
struct DataProcessorNode;

impl Node for DataProcessorNode {
    type Context = DataBatchContext;
    type Action = NextAction;

    async fn run(&self, ctx: &mut Self::Context) -> Result<Self::Action, FlowrsError> {
        // Process a single item (this will be called for each item in the batch)
        if let Some(item) = ctx.get_batch_items()?.first() {
            println!("Processing item {}: {}", item.id, item.value);

            // Simulate processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Transform the item (uppercase the value)
            let mut processed_item = item.clone();
            processed_item.value = processed_item.value.to_uppercase();

            // Replace the item in the context
            ctx.items = vec![processed_item];
        }

        Ok(NextAction::Continue)
    }
}

impl BatchNode<DataItem> for DataProcessorNode {}
```

### Step 4: Create and Execute a Batch Flow

Finally, create and execute a batch flow:

```rust
#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    // Create sample data
    let items = vec![
        DataItem { id: 1, value: "item1".to_string() },
        DataItem { id: 2, value: "item2".to_string() },
        DataItem { id: 3, value: "item3".to_string() },
        DataItem { id: 4, value: "item4".to_string() },
        DataItem { id: 5, value: "item5".to_string() },
    ];

    // Create batch context
    let mut context = DataBatchContext::new(items);

    // Create batch processor node
    let processor_node = DataProcessorNode;

    // Create batch flow with concurrency limit of 3
    let batch_flow = BatchFlow::new(processor_node, 3);

    // Execute the batch flow
    let result_context = batch_flow.execute(context).await?;

    // Print results
    println!("Batch processing complete!");
    println!("Total processed: {}", result_context.state.total_processed);
    println!("Total errors: {}", result_context.state.total_errors);

    for result in result_context.get_results() {
        match result {
            Ok(item) => println!("Processed item {}: {}", item.id, item.value),
            Err(e) => println!("Error processing item: {}", e),
        }
    }

    Ok(())
}
```

## Running the Example

To run this example:

1. Create a new Rust project with the dependencies listed above
2. Copy the code into your `src/main.rs` file
3. Run the example with `cargo run`

You should see output similar to:

```
Processing item 1: item1
Processing item 2: item2
Processing item 3: item3
Processing item 4: item4
Processing item 5: item5
Batch processing complete!
Total processed: 5
Total errors: 0
Processed item 1: ITEM1
Processed item 2: ITEM2
Processed item 3: ITEM3
Processed item 4: ITEM4
Processed item 5: ITEM5
```

## Advanced Techniques

### Error Handling

You can add error handling to your batch processing node:

```rust
async fn run(&self, ctx: &mut Self::Context) -> Result<Self::Action, FlowrsError> {
    if let Some(item) = ctx.get_batch_items()?.first() {
        // Simulate an error for items with even IDs
        if item.id % 2 == 0 {
            return Err(FlowrsError::ProcessingFailed(format!("Failed to process item {}", item.id)));
        }

        // Process normally for odd IDs
        println!("Processing item {}: {}", item.id, item.value);

        // Transform the item
        let mut processed_item = item.clone();
        processed_item.value = processed_item.value.to_uppercase();
        ctx.items = vec![processed_item];
    }

    Ok(NextAction::Continue)
}
```

### Custom Batch Processor

You can create a custom batch processor with specialized behavior:

```rust
struct CustomBatchProcessor {
    concurrency_limit: usize,
    retry_count: usize,
}

impl CustomBatchProcessor {
    fn new(concurrency_limit: usize, retry_count: usize) -> Self {
        Self { concurrency_limit, retry_count }
    }

    async fn process_with_retry<T, F, Fut>(&self, items: Vec<T>, processor: F) -> Vec<Result<T, FlowrsError>>
    where
        F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<T, FlowrsError>> + Send,
        T: Clone + Send + 'static,
    {
        // Implementation with retry logic
        // ...
    }
}
```

## Conclusion

This example demonstrates how to use batch processing in the Flowrs framework to efficiently process collections of items in parallel. By leveraging the `BatchContext`, `BatchNode`, and `BatchFlow` abstractions, you can create powerful and flexible batch processing workflows.

For more information on batch processing, refer to the [Batch Processing Implementation](../architecture/batch-processing-implementation.md) documentation.
