# Quick Start Guide

This guide will help you create your first Flowrs workflow. We'll build a simple workflow that processes a message and returns a result.

## Prerequisites

Before starting, make sure you have:

- Installed Rust and Cargo (see [Installation](installation.md))
- Created a new Rust project or added Flowrs to an existing project

## Step 1: Set Up Your Project

First, let's set up a new Rust project and add the necessary dependencies:

```bash
cargo new flowrs_quickstart
cd flowrs_quickstart
```

Edit your `Cargo.toml` file to include the required dependencies:

```toml
[dependencies]
flowrs-core = "0.1.0"
tokio = { version = "1.28", features = ["full"] }
async-trait = "0.1.68"
```

## Step 2: Create Your First Workflow

Now, let's create a simple workflow that processes a message. Replace the contents of `src/main.rs` with the following code:

```rust
use flowrs_core::{lifecycle_node, LifecycleNode, Workflow, DefaultAction};
use async_trait::async_trait;
use std::sync::Arc;

// Define your context type
#[derive(Debug, Clone)]
struct MessageContext {
    input: String,
    result: Option<String>,
}

// Create a node using the convenience function
fn create_processor_node() -> impl LifecycleNode<MessageContext, DefaultAction> {
    lifecycle_node(
        Some("processor"), // Node ID
        |ctx: &mut MessageContext| async move {
            // Preparation phase
            let input = ctx.input.clone();
            Ok(input)
        },
        |input: String| async move {
            // Execution phase
            let processed = format!("Processed: {}", input.to_uppercase());
            Ok(processed)
        },
        |ctx: &mut MessageContext, result: String| async move {
            // Post-processing phase
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}

#[tokio::main]
async fn main() {
    // Create the initial context
    let context = MessageContext {
        input: "hello world".to_string(),
        result: None,
    };

    // Create and run the workflow
    let node = create_processor_node();
    let mut workflow = Workflow::new(node);
    
    let result = workflow.run(context).await.unwrap();
    println!("Result: {:?}", result.result);
}
```

## Step 3: Add Error Handling

Let's enhance our workflow with error handling:

```rust
use std::error::Error;

#[derive(Debug)]
struct ProcessingError(String);

impl std::fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Processing error: {}", self.0)
    }
}

impl Error for ProcessingError {}

fn create_error_handling_node() -> impl LifecycleNode<MessageContext, DefaultAction> {
    lifecycle_node(
        Some("error_handler"),
        |ctx: &MessageContext| async move {
            let input = ctx.input.clone();
            if input.is_empty() {
                return Err(Box::new(ProcessingError("Empty input".to_string())));
            }
            Ok(input)
        },
        |input: String| async move {
            Ok(input.to_uppercase())
        },
        |ctx: &mut MessageContext, result: String| async move {
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}
```

## Step 4: Create a Multi-Node Workflow

Now let's create a more complex workflow with multiple nodes:

```rust
#[derive(Debug, Clone)]
enum CustomAction {
    Success,
    Error,
}

// Validator node
fn create_validator_node() -> impl LifecycleNode<MessageContext, CustomAction> {
    lifecycle_node(
        Some("validator"),
        |ctx: &MessageContext| async move {
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            if input.len() < 3 {
                Ok(CustomAction::Error)
            } else {
                Ok(CustomAction::Success)
            }
        },
        |ctx: &mut MessageContext, action: CustomAction| async move {
            Ok(action)
        },
    )
}

// Success handler node
fn create_success_node() -> impl LifecycleNode<MessageContext, DefaultAction> {
    lifecycle_node(
        Some("success_handler"),
        |ctx: &MessageContext| async move {
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            Ok(format!("SUCCESS: {}", input.to_uppercase()))
        },
        |ctx: &mut MessageContext, result: String| async move {
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}

// Error handler node
fn create_error_node() -> impl LifecycleNode<MessageContext, DefaultAction> {
    lifecycle_node(
        Some("error_handler"),
        |ctx: &MessageContext| async move {
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            Ok(format!("ERROR: Input '{}' is too short", input))
        },
        |ctx: &mut MessageContext, result: String| async move {
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}

#[tokio::main]
async fn main() {
    // Create nodes
    let validator = create_validator_node();
    let success_handler = create_success_node();
    let error_handler = create_error_node();

    // Create workflow with conditional branching
    let mut workflow = Workflow::new(validator)
        .on(CustomAction::Success, success_handler)
        .on(CustomAction::Error, error_handler);

    // Test with valid input
    let context = MessageContext {
        input: "hello world".to_string(),
        result: None,
    };
    let result = workflow.run(context).await.unwrap();
    println!("Valid input result: {:?}", result.result);

    // Test with invalid input
    let context = MessageContext {
        input: "hi".to_string(),
        result: None,
    };
    let result = workflow.run(context).await.unwrap();
    println!("Invalid input result: {:?}", result.result);
}
```

## Next Steps

Now that you've created your first workflow, you can:

1. Learn about [Core Concepts](../core-concepts/overview.md) in depth
2. Explore [Event-Driven Architecture](../guides/event_driven_architecture.md)
3. Check out more [Examples](../examples/basic-workflow.md)
4. Read about [Actions](../core-concepts/actions.md) and flow control

## Common Patterns

Here are some common patterns you might want to try:

1. **Parallel Processing**:
   - Use multiple nodes that process data concurrently
   - Aggregate results in a final node

2. **Error Recovery**:
   - Implement retry logic
   - Use fallback nodes for error cases

3. **State Management**:
   - Share state between nodes
   - Persist workflow state

4. **Monitoring**:
   - Add logging nodes
   - Track workflow metrics

For more detailed examples and patterns, check out the [Examples](../examples/basic-workflow.md) section.
