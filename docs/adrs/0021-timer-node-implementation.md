# ADR-0021: Timer Node Implementation

## Status

Accepted

## Date

2024-02-25

## Context

In ADR-0016, we outlined the need for an asynchronous extension pattern for timer-based execution of nodes. This extension would allow workflows to execute nodes based on time schedules, providing capabilities for periodic tasks, delayed execution, and time-based triggering without constant polling.

The workflow system currently has event-driven nodes implemented, which allow nodes to wait for external events. Timer nodes extend this pattern by introducing time-based scheduling as another form of event trigger.

Key requirements for timer nodes include:

1. Support for various scheduling patterns (one-time, intervals, daily, weekly, monthly)
2. Integration with the existing workflow engine
3. Ability to use timer nodes as standard nodes in workflows
4. Support for nested timer workflows
5. Proper error handling and timeout mechanisms

## Decision

We will implement a new crate `floxide-timer` that provides the `TimerNode` trait and related implementations as described in ADR-0016. The implementation will follow these design decisions:

### 1. Core `Schedule` Enum

We will implement a `Schedule` enum that represents different scheduling patterns:

```rust
pub enum Schedule {
    Once(DateTime<Utc>),
    Interval(Duration),
    Daily(u32, u32), // Hour, minute
    Weekly(Weekday, u32, u32), // Day of week, hour, minute
    Monthly(u32, u32, u32), // Day of month, hour, minute
    Cron(String), // Cron expression (placeholder for future implementation)
}
```

The `Schedule` type will provide methods to calculate the next execution time and the duration until that time.

### 2. `TimerNode` Trait

We will implement the `TimerNode` trait as outlined in ADR-0016:

```rust
#[async_trait]
pub trait TimerNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
{
    /// Define the execution schedule
    fn schedule(&self) -> Schedule;

    /// Execute the node on schedule
    async fn execute_on_schedule(
        &self,
        ctx: &mut Context
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

### 3. Basic Implementations

We will provide these concrete implementations:

- `SimpleTimer`: A timer node that executes a function on a schedule
- `TimerWorkflow`: A workflow that orchestrates execution of timer nodes
- `TimerNodeAdapter`: An adapter to use a timer node as a standard node
- `NestedTimerWorkflow`: A nested timer workflow that can be used as a standard node

### 4. Integration with Core Workflow Engine

The `TimerNodeAdapter` will implement the `Node` trait, allowing timer nodes to be used in standard workflows. This adapter will handle the wait period before executing the node.

### 5. Utility Extension Traits

We will provide a `TimerActionExt` trait that extends `ActionType` with timer-specific actions:

```rust
pub trait TimerActionExt: ActionType {
    /// Create a complete action for timer nodes
    fn complete() -> Self;

    /// Create a retry action for timer nodes
    fn retry() -> Self;
}
```

## Consequences

### Advantages

1. **Time-Based Execution**: The framework can now execute nodes based on time schedules.
2. **Resource Efficiency**: Timer nodes eliminate the need for polling-based implementations.
3. **Flexibility**: Different scheduling patterns support a wide range of use cases.
4. **Integration**: Timer nodes can be used alongside existing node types in workflows.
5. **Composability**: Timer workflows can be nested within standard workflows.

### Disadvantages

1. **Complexity**: Adds another node type to the framework, increasing complexity.
2. **Maintenance**: Additional code to maintain and test.
3. **Scheduling Edge Cases**: Time-based scheduling has many edge cases (time zones, DST changes, etc.).
4. **Resource Consumption**: Long-running timer workflows may consume resources while waiting.

### Implementation Notes

1. The implementation uses Tokio's `sleep` function for time-based waiting.
2. The `Cron` schedule type is a placeholder for future implementation.
3. Proper error handling is implemented for invalid schedules.
4. Unit tests are provided to validate schedule calculations and timer node execution.

## Alternatives Considered

### 1. Use External Scheduling Libraries

We considered using external scheduling libraries like `cron` or `job_scheduler`, but decided to implement our own scheduling to maintain control over the implementation and to ensure seamless integration with our workflow engine.

### 2. Implement as Part of Event-Driven Nodes

We considered implementing timers as a special case of event-driven nodes, but decided that a separate abstraction would be clearer and more maintainable, especially given the specialized scheduling logic required.

### 3. Operating System Level Scheduling

We considered integrating with OS-level scheduling (cron jobs, Windows Task Scheduler), but this would limit portability and would not integrate well with in-process workflows.

## Related ADRs

- [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md)
- [ADR-0017: Event-Driven Node Pattern](0017-event-driven-node-pattern.md)

## References

- [Tokio time](https://docs.rs/tokio/latest/tokio/time/index.html)
- [Chrono crate](https://docs.rs/chrono/latest/chrono/)
