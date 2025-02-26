# Node Lifecycle Methods

This document describes the node lifecycle methods in the Floxide framework.

## Overview

The Floxide framework implements a three-phase lifecycle for nodes, providing a clear separation of concerns and enabling specialized behaviors at each stage of node execution. This approach enhances maintainability, testability, and flexibility in workflow design.

## Lifecycle Phases

Each node in the Floxide framework goes through three distinct phases during execution:

1. **Preparation (`prep`)**: Setup and validation phase
2. **Execution (`exec`)**: Core execution with potential retry mechanisms
3. **Post-processing (`post`)**: Determines routing and handles results

## LifecycleNode Trait

The `LifecycleNode` trait explicitly models this three-phase lifecycle:

```rust
#[async_trait]
pub trait LifecycleNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Self::PrepOutput: Clone + Send + Sync + 'static,
    Self::ExecOutput: Clone + Send + Sync + 'static,
{
    /// Output type from the preparation phase
    type PrepOutput;

    /// Output type from the execution phase
    type ExecOutput;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;

    /// Preparation phase - perform setup and validation
    async fn prep(&self, ctx: &mut Context) -> Result<Self::PrepOutput, FloxideError>;

    /// Execution phase - perform the main work
    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError>;

    /// Post-processing phase - determine routing
    async fn post(&self, exec_result: Self::ExecOutput) -> Result<Action, FloxideError>;
}
```

## Phase Responsibilities

### Preparation Phase

The preparation phase is responsible for:

- Validating input data
- Setting up resources
- Performing preliminary checks
- Preparing data for execution

Example implementation:

```rust
async fn prep(&self, ctx: &mut MyContext) -> Result<PrepData, FloxideError> {
    // Validate input
    if ctx.input.is_empty() {
        return Err(FloxideError::ValidationFailed("Input cannot be empty".to_string()));
    }

    // Prepare data for execution
    let prep_data = PrepData {
        input: ctx.input.clone(),
        timestamp: Utc::now(),
    };

    Ok(prep_data)
}
```

### Execution Phase

The execution phase is responsible for:

- Performing the main work of the node
- Handling retries for transient failures
- Processing data
- Producing execution results

Example implementation:

```rust
async fn exec(&self, prep_result: PrepData) -> Result<ExecData, FloxideError> {
    // Perform main work
    let result = process_data(&prep_result.input)?;

    // Create execution result
    let exec_data = ExecData {
        result,
        processing_time: Utc::now() - prep_result.timestamp,
    };

    Ok(exec_data)
}
```

### Post-processing Phase

The post-processing phase is responsible for:

- Determining the next action (routing)
- Cleaning up resources
- Logging results
- Preparing data for the next node

Example implementation:

```rust
async fn post(&self, exec_result: ExecData) -> Result<Action, FloxideError> {
    // Determine routing based on execution result
    let action = if exec_result.result > 100 {
        Action::Route("high_value_path")
    } else {
        Action::Route("standard_path")
    };

    // Log processing time
    log::info!("Node {} completed in {:?}", self.id(), exec_result.processing_time);

    Ok(action)
}
```

## Adapter Pattern

To maintain compatibility with the existing `Node` trait, the framework uses adapter patterns:

```rust
impl<T, C, A> Node for T
where
    T: LifecycleNode<C, A>,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    type Context = C;
    type Action = A;

    async fn run(&self, ctx: &mut Self::Context) -> Result<Self::Action, FloxideError> {
        let prep_result = self.prep(ctx).await?;
        let exec_result = self.exec(prep_result).await?;
        let action = self.post(exec_result).await?;
        Ok(action)
    }
}
```

## Benefits

The three-phase lifecycle approach offers several benefits:

1. **Separation of Concerns**: Each phase has a clear, distinct responsibility
2. **Testability**: Each phase can be tested independently
3. **Flexibility**: Specialized behaviors can be implemented for each phase
4. **Error Handling**: Different error handling strategies can be applied to each phase
5. **Observability**: Metrics and logging can be added at phase boundaries

## Best Practices

When implementing nodes with the lifecycle pattern:

1. **Keep Phases Focused**: Each phase should have a single responsibility
2. **Minimize State**: Pass necessary data between phases through return values
3. **Handle Errors Appropriately**: Use different error handling strategies for each phase
4. **Consider Retries**: Implement retries in the execution phase for transient failures
5. **Add Observability**: Log important events at phase boundaries

## Conclusion

The node lifecycle methods in the Floxide framework provide a powerful pattern for implementing workflow nodes with clear separation of concerns. By following this pattern, developers can create maintainable, testable, and flexible workflow components.

For more detailed information on the node lifecycle methods, refer to the [Node Lifecycle Methods ADR](../adrs/0008-node-lifecycle-methods.md).
