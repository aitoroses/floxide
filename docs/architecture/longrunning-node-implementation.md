# Long-Running Node Implementation

This document describes the implementation details of long-running nodes in the Flowrs framework.

## Overview

Long-running nodes in Flowrs provide support for tasks that may take significant time to complete, with proper progress tracking and cancellation support.

## Core Components

### LongRunningNode Trait

The `LongRunningNode` trait defines the core interface for long-running tasks:

```rust
#[async_trait]
pub trait LongRunningNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    async fn start(&self, ctx: &mut Context) -> Result<(), FlowrsError>;
    async fn check_progress(&self, ctx: &mut Context) -> Result<Progress, FlowrsError>;
    async fn cancel(&self, ctx: &mut Context) -> Result<(), FlowrsError>;
    fn id(&self) -> NodeId;
}
```

### Progress Tracking

The `Progress` enum represents the current state of a long-running task:

```rust
pub enum Progress {
    Running(f32), // 0.0 to 1.0
    Complete(Action),
    Failed(FlowrsError),
}
```

## Implementation Details

### Task Management

1. **Execution Control**
   - Task initialization
   - Progress monitoring
   - Graceful cancellation

2. **State Management**
   - Progress tracking
   - State persistence
   - Recovery mechanisms

3. **Resource Control**
   - Resource allocation
   - Cleanup procedures
   - Memory management

### Error Handling

1. **Execution Errors**
   - Error propagation
   - Recovery strategies
   - Error reporting

2. **Cancellation Handling**
   - Safe cancellation
   - Resource cleanup
   - State consistency

### Resource Management

1. **Memory Usage**
   - Efficient state tracking
   - Resource cleanup
   - Memory leak prevention

2. **Thread Safety**
   - Thread-safe execution
   - Safe progress updates
   - Proper synchronization

## Usage Patterns

### Basic Usage

```rust
let node = LongRunningTask::new(
    |ctx| { /* start implementation */ },
    |ctx| { /* progress check implementation */ },
    |ctx| { /* cancellation implementation */ },
);
```

### With Progress Tracking

```rust
let node = LongRunningTask::new(
    |ctx| async {
        for i in 0..100 {
            process_chunk(i)?;
            update_progress(i as f32 / 100.0);
        }
        Ok(())
    },
    |ctx| async {
        let progress = get_current_progress();
        if progress >= 1.0 {
            Ok(Progress::Complete(DefaultAction::Next))
        } else {
            Ok(Progress::Running(progress))
        }
    },
    |ctx| async { /* cancellation implementation */ },
);
```

### With Error Handling

```rust
let node = LongRunningTask::new(
    |ctx| async {
        if let Err(e) = check_preconditions() {
            return Err(e.into());
        }
        start_processing()
    },
    |ctx| async {
        match check_task_progress() {
            Ok(progress) => Ok(Progress::Running(progress)),
            Err(e) => Ok(Progress::Failed(e.into())),
        }
    },
    |ctx| async {
        cleanup_resources()?;
        Ok(())
    },
);
```

## Testing

The implementation includes comprehensive tests:

1. **Unit Tests**
   - Task execution
   - Progress tracking
   - Cancellation handling
   - Resource cleanup

2. **Integration Tests**
   - Complex workflows
   - Error scenarios
   - Performance tests

## Performance Considerations

1. **Execution Efficiency**
   - Minimal overhead
   - Efficient progress tracking
   - Resource optimization

2. **Memory Usage**
   - Optimized state storage
   - Efficient progress updates
   - Minimal allocations

## Future Improvements

1. **Enhanced Features**
   - More progress metrics
   - Advanced cancellation strategies
   - Extended recovery options

2. **Performance Optimizations**
   - Improved state tracking
   - Better resource utilization
   - Enhanced concurrency

## Related ADRs

- [ADR-0008: Node Lifecycle Methods](../adrs/0008-node-lifecycle-methods.md)
- [ADR-0013: Workflow Patterns](../adrs/0013-workflow-patterns.md)
