# Timer Node Implementation

This document describes the implementation details of timer nodes in the Flowrs framework.

## Overview

Timer nodes in Flowrs provide scheduling capabilities with support for various schedule types and proper error handling.

## Core Components

### TimerNode Trait

The `TimerNode` trait defines the core interface for scheduled execution:

```rust
#[async_trait]
pub trait TimerNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    fn schedule(&self) -> Schedule;
    async fn execute_on_schedule(&self, ctx: &mut Context) -> Result<Action, FlowrsError>;
    fn id(&self) -> NodeId;
}
```

### Schedule Types

The `Schedule` enum supports different scheduling patterns:

```rust
pub enum Schedule {
    Once(DateTime<Utc>),
    Periodic(ChronoDuration),
    Cron(String),
}
```

## Implementation Details

### Schedule Management

1. **Schedule Types**
   - One-time execution
   - Periodic execution
   - Cron-based scheduling

2. **Time Handling**
   - UTC time management
   - Timezone considerations
   - Leap second handling

3. **Execution Control**
   - Start/stop capabilities
   - Pause/resume support
   - Graceful shutdown

### Error Handling

1. **Execution Errors**
   - Proper error propagation
   - Retry mechanisms
   - Error reporting

2. **Schedule Errors**
   - Invalid schedule detection
   - Schedule parsing errors
   - Recovery strategies

### Resource Management

1. **Timer Resources**
   - Efficient timer allocation
   - Resource cleanup
   - Memory management

2. **Thread Safety**
   - Thread-safe execution
   - Safe schedule updates
   - Proper synchronization

## Usage Patterns

### Basic Usage

```rust
let node = SimpleTimer::new(
    Schedule::Periodic(ChronoDuration::minutes(5)),
    |ctx| { /* execution implementation */ },
);
```

### With Cron Schedule

```rust
let node = SimpleTimer::new(
    Schedule::Cron("0 2 * * *".to_string()), // Run at 2 AM daily
    |ctx| {
        match perform_daily_task(ctx) {
            Ok(_) => Ok(DefaultAction::Next),
            Err(e) => Err(e.into()),
        }
    },
);
```

### With Error Handling

```rust
let node = SimpleTimer::new(
    Schedule::Once(Utc::now() + ChronoDuration::hours(1)),
    |ctx| {
        if let Err(e) = check_preconditions() {
            return Err(e.into());
        }
        Ok(DefaultAction::complete())
    },
);
```

## Testing

The implementation includes comprehensive tests:

1. **Unit Tests**
   - Schedule parsing
   - Execution timing
   - Error handling
   - Resource cleanup

2. **Integration Tests**
   - Complex schedules
   - Long-running scenarios
   - Edge cases

## Performance Considerations

1. **Timer Efficiency**
   - Minimal allocations
   - Efficient scheduling
   - Low overhead

2. **Resource Usage**
   - Optimized timer pools
   - Efficient thread usage
   - Minimal locking

## Future Improvements

1. **Enhanced Features**
   - More schedule types
   - Advanced retry strategies
   - Schedule composition

2. **Performance Optimizations**
   - Improved timer allocation
   - Better resource sharing
   - Enhanced concurrency

## Related ADRs

- [ADR-0008: Node Lifecycle Methods](../adrs/0008-node-lifecycle-methods.md)
- [ADR-0013: Workflow Patterns](../adrs/0013-workflow-patterns.md)
