# Transform Node Example

This example demonstrates how to use transform nodes in the Floxide framework for functional data transformations with explicit input and output types.

## Overview

Transform nodes enable:
- Functional programming style with explicit input/output types
- Direct error types specific to the node
- Three-phase transformation lifecycle (prep, exec, post)
- Easy composition of transformations

## Implementation

Let's create a data processing pipeline that validates, transforms, and enriches JSON data:

```rust
use async_trait::async_trait;
use floxide_core::{DefaultAction, FloxideError};
use floxide_transform::{TransformNode, TransformContext, to_lifecycle_node};
use serde_json::{Value as JsonValue, json};
use thiserror::Error;

// Custom error type
#[derive(Debug, Error)]
enum DataTransformError {
    #[error("Validation failed: {0}")]
    ValidationError(String),
    #[error("Transform failed: {0}")]
    TransformError(String),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

// A transform node that validates and enriches user data
struct UserDataTransformer;

#[async_trait]
impl TransformNode<JsonValue, JsonValue, DataTransformError> for UserDataTransformer {
    async fn prep(&self, input: JsonValue) -> Result<JsonValue, DataTransformError> {
        // Validate required fields
        if !input.is_object() {
            return Err(DataTransformError::ValidationError("Input must be an object".into()));
        }

        let obj = input.as_object().unwrap();
        if !obj.contains_key("name") || !obj.contains_key("email") {
            return Err(DataTransformError::ValidationError("Missing required fields".into()));
        }

        Ok(input)
    }

    async fn exec(&self, input: JsonValue) -> Result<JsonValue, DataTransformError> {
        let mut obj = input.as_object().unwrap().clone();
        
        // Transform name to uppercase
        if let Some(name) = obj.get("name") {
            let uppercase_name = name.as_str()
                .ok_or_else(|| DataTransformError::TransformError("Invalid name format".into()))?
                .to_uppercase();
            obj.insert("name".into(), json!(uppercase_name));
        }

        // Add metadata
        obj.insert("processed_at".into(), json!(chrono::Utc::now().to_rfc3339()));
        obj.insert("version".into(), json!("1.0.0"));

        Ok(JsonValue::Object(obj))
    }

    async fn post(&self, output: JsonValue) -> Result<JsonValue, DataTransformError> {
        // Add a summary field
        let mut obj = output.as_object().unwrap().clone();
        let summary = format!(
            "Processed user data for: {}",
            obj.get("name").and_then(|n| n.as_str()).unwrap_or("unknown")
        );
        obj.insert("summary".into(), json!(summary));

        Ok(JsonValue::Object(obj))
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    // Create input data
    let input_data = json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
    });

    // Create transform node and convert to lifecycle node
    let transformer = UserDataTransformer;
    let lifecycle_node = to_lifecycle_node(transformer);

    // Create context with input data
    let mut ctx = TransformContext::new(input_data);

    // Process the data through all phases
    let prep_result = lifecycle_node.prep(&mut ctx)?;
    let exec_result = lifecycle_node.exec(prep_result)?;
    let _action = lifecycle_node.post(prep_result, exec_result, &mut ctx)?;

    // Access the transformed data from the context
    println!("Transformed data: {}", serde_json::to_string_pretty(&ctx.input)?);

    Ok(())
}
```

## Advanced Usage

### Composing Transform Nodes

Transform nodes can be easily composed into pipelines:

```rust
// Create a pipeline of transform nodes
let pipeline = to_lifecycle_node(
    ValidatorNode
        .and_then(TransformerNode)
        .and_then(EnricherNode)
);
```

### Using Helper Functions

The framework provides helper functions for simpler transformations:

```rust
use floxide_transform::transform_node;

// Create a simple transform node using closures
let simple_transformer = transform_node(
    // Prep function
    |input: String| async move {
        if input.is_empty() {
            Err(DataTransformError::ValidationError("Empty input".into()))
        } else {
            Ok(input)
        }
    },
    // Exec function
    |input: String| async move {
        Ok(input.to_uppercase())
    },
    // Post function
    |output: String| async move {
        Ok(format!("Processed: {}", output))
    },
);
```

### Error Handling

Transform nodes support custom error types for better error handling:

```rust
// Create a transform node with robust error handling
let robust_transformer = transform_node(
    |input: JsonValue| async move {
        match validate_input(&input) {
            Ok(valid_input) => Ok(valid_input),
            Err(e) => Err(DataTransformError::ValidationError(e.to_string())),
        }
    },
    |input: JsonValue| async move {
        transform_data(&input)
            .map_err(|e| DataTransformError::TransformError(e.to_string()))
    },
    |output: JsonValue| async move {
        enrich_data(&output)
            .map_err(|e| DataTransformError::TransformError(e.to_string()))
    },
);
```

## Best Practices

1. **Use Custom Error Types**: Define specific error types for your transformations to provide clear error handling.
2. **Validate Early**: Use the `prep` phase to validate input data before processing.
3. **Keep Transformations Pure**: Avoid side effects in transform nodes when possible.
4. **Compose Nodes**: Break complex transformations into smaller, composable nodes.
5. **Add Context**: Use the post phase to add metadata about the transformation process.
