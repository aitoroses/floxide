# Basic Workflow Example

This example demonstrates how to create a simple workflow using the Floxide framework, showcasing proper node lifecycle management and error handling.

## Overview

In this example, we'll create a workflow that processes text data through multiple stages:

1. Input validation and preparation
2. Text transformation (uppercase)
3. Analysis (character count)
4. Summary generation

The example demonstrates:
- Three-phase node lifecycle (prep, exec, post)
- Proper error handling and recovery
- Type-safe context management
- Node composition patterns

## Prerequisites

Before running this example, make sure you have the Floxide framework installed. See the [Installation Guide](../getting-started/installation.md) for details.

## Implementation

First, let's define our context and error types:

```rust
use floxide_core::{lifecycle_node, LifecycleNode, DefaultAction, FloxideError};
use thiserror::Error;

#[derive(Debug, Clone)]
struct TextContext {
    input: String,
    uppercase: Option<String>,
    char_count: Option<usize>,
    summary: Option<String>,
}

impl TextContext {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            uppercase: None,
            char_count: None,
            summary: None,
        }
    }
}

#[derive(Debug, Error)]
enum TextProcessingError {
    #[error("Input text is empty")]
    EmptyInput,
    #[error("Input exceeds maximum length of {0} characters")]
    InputTooLong(usize),
    #[error("Processing failed: {0}")]
    ProcessingError(String),
}

impl From<TextProcessingError> for FloxideError {
    fn from(e: TextProcessingError) -> Self {
        FloxideError::Other(e.to_string())
    }
}
```

Now, let's create our nodes using the three-phase lifecycle:

```rust
// Node that validates and prepares text
struct ValidatorNode {
    max_length: usize,
}

#[async_trait]
impl LifecycleNode<TextContext, DefaultAction> for ValidatorNode {
    type PrepOutput = String;
    type ExecOutput = String;

    fn id(&self) -> NodeId {
        NodeId::new("validator")
    }

    async fn prep(&self, ctx: &mut TextContext) -> Result<Self::PrepOutput, FloxideError> {
        if ctx.input.is_empty() {
            return Err(TextProcessingError::EmptyInput.into());
        }
        Ok(ctx.input.clone())
    }

    async fn exec(&self, input: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        if input.len() > self.max_length {
            return Err(TextProcessingError::InputTooLong(self.max_length).into());
        }
        Ok(input)
    }

    async fn post(&self, input: Self::ExecOutput) -> Result<DefaultAction, FloxideError> {
        Ok(DefaultAction::Next)
    }
}

// Node that converts text to uppercase with retry logic
struct UppercaseNode {
    retry_count: usize,
}

#[async_trait]
impl LifecycleNode<TextContext, DefaultAction> for UppercaseNode {
    type PrepOutput = String;
    type ExecOutput = String;

    fn id(&self) -> NodeId {
        NodeId::new("uppercase")
    }

    async fn prep(&self, ctx: &mut TextContext) -> Result<Self::PrepOutput, FloxideError> {
        Ok(ctx.input.clone())
    }

    async fn exec(&self, input: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        let mut attempts = 0;
        loop {
            match try_uppercase(&input) {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) if attempts < self.retry_count => {
                    attempts += 1;
                    tracing::warn!("Uppercase conversion failed, attempt {}/{}: {}", 
                        attempts, self.retry_count, e);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(e) => {
                    return Err(TextProcessingError::ProcessingError(
                        format!("Failed after {} retries: {}", self.retry_count, e)
                    ).into());
                }
            }
        }
    }

    async fn post(&self, result: Self::ExecOutput) -> Result<DefaultAction, FloxideError> {
        Ok(DefaultAction::Next)
    }
}

// Node that counts characters with validation
struct CounterNode;

#[async_trait]
impl LifecycleNode<TextContext, DefaultAction> for CounterNode {
    type PrepOutput = String;
    type ExecOutput = usize;

    fn id(&self) -> NodeId {
        NodeId::new("counter")
    }

    async fn prep(&self, ctx: &mut TextContext) -> Result<Self::PrepOutput, FloxideError> {
        ctx.uppercase.clone().ok_or_else(|| 
            TextProcessingError::ProcessingError("No uppercase text available".to_string()).into()
        )
    }

    async fn exec(&self, input: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        Ok(input.chars().count())
    }

    async fn post(&self, count: Self::ExecOutput) -> Result<DefaultAction, FloxideError> {
        Ok(DefaultAction::Next)
    }
}

// Node that generates a summary
struct SummaryNode;

#[async_trait]
impl LifecycleNode<TextContext, DefaultAction> for SummaryNode {
    type PrepOutput = (String, usize);
    type ExecOutput = String;

    fn id(&self) -> NodeId {
        NodeId::new("summary")
    }

    async fn prep(&self, ctx: &mut TextContext) -> Result<Self::PrepOutput, FloxideError> {
        let text = ctx.uppercase.clone().ok_or_else(|| 
            TextProcessingError::ProcessingError("No uppercase text available".to_string())
        )?;
        let count = ctx.char_count.ok_or_else(|| 
            TextProcessingError::ProcessingError("No character count available".to_string())
        )?;
        Ok((text, count))
    }

    async fn exec(&self, (text, count): Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        Ok(format!("Processed text with {} characters: {}", count, text))
    }

    async fn post(&self, summary: Self::ExecOutput) -> Result<DefaultAction, FloxideError> {
        Ok(DefaultAction::Complete)
    }
}

// Helper function that could fail
fn try_uppercase(input: &str) -> Result<String, TextProcessingError> {
    // Simulate potential failures
    if rand::random::<f32>() < 0.2 {
        return Err(TextProcessingError::ProcessingError(
            "Random processing failure".to_string()
        ));
    }
    Ok(input.to_uppercase())
}
```

Now let's create and run the workflow:

```rust
use floxide_core::Workflow;

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create context
    let mut context = TextContext::new("Hello, Floxide!");

    // Create nodes
    let validator = ValidatorNode { max_length: 100 };
    let uppercase = UppercaseNode { retry_count: 3 };
    let counter = CounterNode;
    let summary = SummaryNode;

    // Create workflow
    let mut workflow = Workflow::new(validator)
        .then(uppercase)
        .then(counter)
        .then(summary);

    // Run workflow
    match workflow.run(&mut context).await {
        Ok(_) => {
            println!("Workflow completed successfully!");
            println!("Summary: {}", context.summary.unwrap_or_default());
            Ok(())
        }
        Err(e) => {
            eprintln!("Workflow failed: {}", e);
            Err(e)
        }
    }
}
```

## Key Concepts Demonstrated

1. **Node Lifecycle**
   - Preparation phase for validation and setup
   - Execution phase with retry logic
   - Post-processing phase for routing decisions

2. **Error Handling**
   - Custom error types
   - Proper error propagation
   - Retry mechanisms for transient failures

3. **Type Safety**
   - Strongly typed context
   - Type-safe node outputs
   - Safe error handling

4. **Best Practices**
   - Clear separation of concerns
   - Proper logging and observability
   - Resource cleanup
   - Error recovery strategies

## Next Steps

1. Explore more complex workflow patterns in the [Workflow Patterns](../core-concepts/workflows.md) guide
2. Learn about error handling in the [Error Handling Guide](../getting-started/error-handling.md)
3. Check out the [Event-Driven Workflow](event-driven-workflow.md) example for handling events
4. See the [Batch Processing](batch-processing.md) example for handling collections of items
