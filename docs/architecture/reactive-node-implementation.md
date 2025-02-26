# Reactive Node Implementation

This document describes the implementation details of reactive nodes in the Floxide framework.

## Overview

Reactive nodes in Floxide provide stream-based processing capabilities with proper backpressure handling and error management.

## Core Components

### ReactiveNode Trait

The `ReactiveNode` trait defines the core interface for reactive processing:

```rust
#[async_trait]
pub trait ReactiveNode<Change, Context, Action>: Send + Sync
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    async fn watch(&self) -> Result<Box<dyn Stream<Item = Change> + Send + Unpin>, FloxideError>;
    async fn react_to_change(&self, change: Change, ctx: &mut Context) -> Result<Action, FloxideError>;
    fn id(&self) -> NodeId;
}
```

### CustomReactiveNode

The `CustomReactiveNode` provides a flexible implementation that allows users to define their own watch and react functions:

```rust
pub struct CustomReactiveNode<Change, Context, Action, WatchFn, ReactFn> {
    watch_fn: WatchFn,
    react_fn: ReactFn,
    id: NodeId,
    _phantom: PhantomData<(Change, Context, Action)>,
}
```

## Implementation Details

### Stream Management

1. **Watch Function**
   - Creates and manages the underlying stream
   - Handles backpressure through Tokio's async streams
   - Provides proper cleanup on drop

2. **Change Detection**
   - Processes changes as they arrive
   - Maintains order of changes
   - Handles errors gracefully

3. **Context Updates**
   - Updates context based on changes
   - Maintains thread safety
   - Provides atomic updates when needed

### Error Handling

1. **Stream Errors**
   - Proper error propagation
   - Recovery mechanisms
   - Error context preservation

2. **Processing Errors**
   - Custom error types
   - Error recovery strategies
   - Error reporting

### Resource Management

1. **Memory Usage**
   - Efficient stream buffering
   - Proper cleanup of resources
   - Memory leak prevention

2. **Thread Safety**
   - Thread-safe context access
   - Safe concurrent processing
   - Proper synchronization

## Usage Patterns

### Basic Usage

```rust
let node = CustomReactiveNode::new(
    || { /* watch implementation */ },
    |change, ctx| { /* react implementation */ },
);
```

### With Error Handling

```rust
let node = CustomReactiveNode::new(
    || {
        if let Err(e) = check_preconditions() {
            return Err(e.into());
        }
        Ok(create_stream())
    },
    |change, ctx| {
        match process_change(change) {
            Ok(result) => Ok(DefaultAction::change_detected()),
            Err(e) => Err(e.into()),
        }
    },
);
```

### With Backpressure

```rust
let node = CustomReactiveNode::new(
    || {
        Ok(Box::new(
            stream::iter(0..100)
                .throttle(Duration::from_millis(100))
        ))
    },
    |change, ctx| { /* react implementation */ },
);
```

## Testing

The implementation includes comprehensive tests:

1. **Unit Tests**
   - Stream creation
   - Change processing
   - Error handling
   - Resource cleanup

2. **Integration Tests**
   - End-to-end workflows
   - Complex scenarios
   - Performance tests

## Performance Considerations

1. **Stream Efficiency**
   - Minimal allocations
   - Efficient buffering
   - Proper backpressure

2. **Processing Overhead**
   - Optimized change detection
   - Efficient context updates
   - Minimal locking

## Future Improvements

1. **Enhanced Features**
   - More stream combinators
   - Additional error recovery strategies
   - Extended composition patterns

2. **Performance Optimizations**
   - Improved buffering strategies
   - Better resource utilization
   - Enhanced concurrency

## Related ADRs

- [ADR-0008: Event-Driven Node Extensions](../adrs/0008-event-driven-node-extensions.md)
- [ADR-0013: Workflow Patterns](../adrs/0013-workflow-patterns.md)
