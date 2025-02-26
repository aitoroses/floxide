# floxide-timer API Reference

The `floxide-timer` crate provides time-based workflow capabilities through the `TimerNode` trait and various schedule implementations, as defined in [ADR-0021](../adrs/0021-timer-node-implementation.md).

## Core Types

### Schedule

```rust
pub enum Schedule {
    /// Execute once at a specific time
    Once(DateTime<Utc>),
    /// Execute repeatedly at fixed intervals
    Interval(Duration),
    /// Execute daily at a specified hour and minute
    Daily(u32, u32),
    /// Execute weekly on a specified day and time
    Weekly(Weekday, u32, u32),
    /// Execute monthly on a specified day and time
    Monthly(u32, u32, u32),
    /// Execute according to a cron expression (future)
    Cron(String),
}
```

The `Schedule` enum defines when a timer node should execute, supporting:
- One-time execution at specific times
- Fixed interval execution
- Daily/Weekly/Monthly scheduling
- Future support for cron expressions

### TimerNode

```rust
#[async_trait]
pub trait TimerNode<Context, Action>: Send + Sync {
    /// Get the node's schedule
    fn schedule(&self) -> Schedule;
    
    /// Execute when the schedule triggers
    async fn execute_on_schedule(&self, ctx: &mut Context) -> Result<Action, FloxideError>;
    
    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

The `TimerNode` trait is the core abstraction for timer-based execution.

## Built-in Implementations

### SimpleTimer

```rust
pub struct SimpleTimer<F> {
    schedule: Schedule,
    action: F,
    id: NodeId,
}

impl<F> SimpleTimer<F> {
    pub fn new(schedule: Schedule, action: F) -> Self;
    pub fn with_id(id: impl Into<String>, schedule: Schedule, action: F) -> Self;
}
```

A basic timer implementation that executes a function based on a schedule:
- Configurable execution schedule
- Custom action function
- Optional custom node identity

### TimerWorkflow

```rust
pub struct TimerWorkflow<Context, Action> {
    nodes: HashMap<NodeId, Arc<dyn TimerNode<Context, Action>>>,
    routes: HashMap<(NodeId, Action), NodeId>,
    termination_action: Action,
}
```

Orchestrates execution of multiple timer nodes:
- Node routing based on actions
- Workflow-level termination conditions
- Execution state management

### TimerNodeAdapter

```rust
pub struct TimerNodeAdapter<Context, Action> {
    node: Arc<dyn TimerNode<Context, Action>>,
    execute_immediately: bool,
    id: NodeId,
}
```

Adapts a `TimerNode` to be used as a standard `Node`:
- Optional immediate execution
- Schedule-based processing
- Standard node interface compatibility

### NestedTimerWorkflow

```rust
pub struct NestedTimerWorkflow<Context, Action> {
    workflow: Arc<TimerWorkflow<Context, Action>>,
    complete_action: Action,
    id: NodeId,
}
```

Allows using a timer workflow as a standard node:
- Workflow nesting
- Completion action configuration
- Workflow isolation

## Usage Examples

### Simple Timer

```rust
use floxide_timer::{SimpleTimer, Schedule};
use chrono::{Utc, Duration};

// Create a timer that runs every minute
let timer = SimpleTimer::new(
    Schedule::Interval(Duration::minutes(1)),
    |ctx: &mut Context| async move {
        println!("Timer executed at: {}", Utc::now());
        Ok(DefaultAction::Next)
    }
);

// Use in a workflow
let mut workflow = Workflow::new(timer);
workflow.run(context).await?;
```

### Timer Workflow

```rust
use floxide_timer::TimerWorkflow;

// Create nodes
let daily_report = Arc::new(SimpleTimer::new(
    Schedule::Daily(9, 0), // 9:00 AM
    generate_daily_report
));

let weekly_cleanup = Arc::new(SimpleTimer::new(
    Schedule::Weekly(Weekday::Sun, 0, 0), // Sunday midnight
    perform_weekly_cleanup
));

// Create workflow
let mut workflow = TimerWorkflow::new(
    daily_report.clone(),
    DefaultAction::Stop
);

// Add nodes and routes
workflow.add_node(weekly_cleanup.clone());
workflow.set_route(
    &daily_report.id(),
    DefaultAction::Next,
    &weekly_cleanup.id()
);

// Execute
workflow.execute(&mut context)?;
```

## Best Practices

1. **Schedule Configuration**
   - Choose appropriate intervals for your use case
   - Consider timezone implications
   - Handle daylight savings time
   - Use UTC for consistency

2. **Resource Management**
   - Clean up timers when done
   - Handle cancellation properly
   - Monitor system load
   - Consider resource limits

3. **Error Handling**
   - Handle missed executions
   - Implement retry logic
   - Log timing issues
   - Handle edge cases

4. **Testing**
   - Test schedule calculations
   - Verify timer behavior
   - Check workflow routing
   - Test error scenarios

## See Also

- [ADR-0021: Timer Node Implementation](../adrs/0021-timer-node-implementation.md)
- [Timer Node Example](../examples/timer-node.md)
- [Event-Driven Architecture](../guides/event_driven_architecture.md)
