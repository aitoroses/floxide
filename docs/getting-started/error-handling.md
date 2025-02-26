# Error Handling in Floxide

This guide explains how to handle errors effectively in the Floxide framework.

## Overview

Floxide provides a comprehensive error handling system that allows you to:
- Define custom error types for your nodes
- Handle errors at different lifecycle phases
- Implement recovery strategies
- Maintain type safety

## Error Types

### FloxideError

The core error type in Floxide is `FloxideError`:

```rust
pub enum FloxideError {
    NodeError(String),
    WorkflowError(String),
    StateError(String),
    Other(String),
}
```

### Custom Error Types

You can define custom error types for your nodes:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyNodeError {
    #[error("Validation failed: {0}")]
    ValidationError(String),
    #[error("Processing failed: {0}")]
    ProcessingError(String),
}

impl From<MyNodeError> for FloxideError {
    fn from(err: MyNodeError) -> Self {
        FloxideError::NodeError(err.to_string())
    }
}
```

## Error Handling in Nodes

### Lifecycle Node Error Handling

```rust
#[async_trait]
impl LifecycleNode<MyContext, DefaultAction> for MyNode {
    async fn prep(&self, ctx: &mut MyContext) -> Result<Self::PrepOutput, FloxideError> {
        match validate_input(ctx) {
            Ok(input) => Ok(input),
            Err(e) => Err(MyNodeError::ValidationError(e.to_string()).into()),
        }
    }

    async fn exec(&self, input: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        process_data(input).map_err(|e| MyNodeError::ProcessingError(e.to_string()).into())
    }

    async fn post(&self, output: Self::ExecOutput) -> Result<DefaultAction, FloxideError> {
        Ok(DefaultAction::Next)
    }
}
```

### Transform Node Error Handling

```rust
#[async_trait]
impl TransformNode<Input, Output, MyNodeError> for MyTransformNode {
    async fn prep(&self, input: Input) -> Result<Input, MyNodeError> {
        if !is_valid(&input) {
            return Err(MyNodeError::ValidationError("Invalid input".into()));
        }
        Ok(input)
    }

    async fn exec(&self, input: Input) -> Result<Output, MyNodeError> {
        transform_data(input)
            .map_err(|e| MyNodeError::ProcessingError(e.to_string()))
    }

    async fn post(&self, output: Output) -> Result<Output, MyNodeError> {
        Ok(output)
    }
}
```

## Error Recovery Strategies

### Retry Logic

```rust
impl MyNode {
    async fn with_retry<T, F>(&self, f: F) -> Result<T, FloxideError>
    where
        F: Fn() -> Future<Output = Result<T, FloxideError>>,
    {
        let mut attempts = 0;
        while attempts < 3 {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts == 3 {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        unreachable!()
    }
}
```

### Fallback Values

```rust
impl MyNode {
    async fn with_fallback<T>(&self, f: impl Fn() -> T) -> Result<T, FloxideError> {
        match self.process_data().await {
            Ok(result) => Ok(result),
            Err(_) => Ok(f()),
        }
    }
}
```

## Error Propagation

### In Workflows

```rust
let workflow = Workflow::new(node1)
    .then(node2)
    .on_error(|e| {
        eprintln!("Workflow error: {}", e);
        DefaultAction::Stop
    });
```

### With Context

```rust
impl MyContext {
    fn record_error(&mut self, error: &FloxideError) {
        self.errors.push(ErrorRecord {
            timestamp: Utc::now(),
            message: error.to_string(),
        });
    }
}
```

## Best Practices

1. **Custom Error Types**
   - Define specific error types for your nodes
   - Use `thiserror` for error definitions
   - Implement `From` for `FloxideError`

2. **Error Context**
   - Include relevant context in errors
   - Use structured error types
   - Maintain error chains

3. **Recovery Strategies**
   - Implement appropriate retry logic
   - Use fallback values when suitable
   - Clean up resources on error

4. **Testing**
   - Test error conditions
   - Verify error recovery
   - Check error propagation

## Related Topics

- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
- [Transform Node Implementation](../architecture/transform-node-implementation.md)
- [ADR-0005: State Serialization/Deserialization](../adrs/0005-state-serialization-deserialization.md)
