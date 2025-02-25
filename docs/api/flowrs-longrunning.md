# flowrs-longrunning API Reference

The `flowrs-longrunning` crate provides support for long-running operations in the Flowrs framework.

## Overview

This crate implements patterns and utilities for handling long-running operations within workflows. It provides:

- Long-running node types
- Progress tracking
- Cancellation support
- Resource cleanup

## Key Types

### LongRunningNode

```rust
pub trait LongRunningNode<C, A>: Send + Sync {
    async fn start(&self, context: &mut C) -> Result<(), FlowrsError>;
    async fn check_status(&self, context: &mut C) -> Result<LongRunningStatus, FlowrsError>;
    async fn cleanup(&self, context: &mut C) -> Result<A, FlowrsError>;
}
```

The `LongRunningNode` trait defines the interface for nodes that perform long-running operations.

### LongRunningStatus

```rust
pub enum LongRunningStatus {
    Running(Progress),
    Complete,
    Failed(FlowrsError),
}
```

`LongRunningStatus` represents the current state of a long-running operation.

### Progress

```rust
pub struct Progress {
    pub percent: f64,
    pub message: String,
}
```

`Progress` provides information about the progress of a long-running operation.

## Usage Example

```rust
use flowrs_longrunning::{LongRunningNode, LongRunningStatus, Progress};

struct DataProcessingNode;

impl LongRunningNode<ProcessingContext, ProcessingAction> for DataProcessingNode {
    async fn start(&self, context: &mut ProcessingContext) -> Result<(), FlowrsError> {
        // Initialize the long-running operation
        context.start_processing();
        Ok(())
    }

    async fn check_status(&self, context: &mut ProcessingContext) -> Result<LongRunningStatus, FlowrsError> {
        let progress = context.get_progress();
        if progress.percent < 100.0 {
            Ok(LongRunningStatus::Running(progress))
        } else {
            Ok(LongRunningStatus::Complete)
        }
    }

    async fn cleanup(&self, context: &mut ProcessingContext) -> Result<ProcessingAction, FlowrsError> {
        // Clean up resources and return final action
        context.cleanup();
        Ok(ProcessingAction::Complete)
    }
}
```

## Error Handling

The crate uses the standard `FlowrsError` type for error handling. All operations that can fail return a `Result<T, FlowrsError>`.

## Best Practices

1. Implement proper resource cleanup
2. Provide meaningful progress updates
3. Handle cancellation gracefully
4. Consider timeout mechanisms
5. Implement proper error recovery

## See Also

- [Long-Running Node Implementation ADR](../adrs/0022-longrunning-node-implementation.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
