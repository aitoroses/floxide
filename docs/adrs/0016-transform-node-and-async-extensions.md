# ADR-0016: TransformNode Renaming and Async Extension Patterns

## Status

Proposed

## Date

2025-02-27

## Context

The Floxide framework currently has an `AsyncNode` abstraction that doesn't actually provide unique asynchronous capabilities beyond what's already available in other node types (`Node` and `LifecycleNode`). All node methods in the framework are already `async`, so the name "AsyncNode" is potentially misleading.

What our current `AsyncNode` actually provides is:

1. A simplified input/output model (vs. context modification)
2. Direct error types specific to the node
3. A more functional programming style

At the same time, the framework lacks node types that truly leverage asynchronous programming patterns like event-driven programming, time-based triggering, and reactive patterns. These patterns would enable workflows to:

1. Wait for external events without blocking or polling
2. Execute nodes based on time schedules or conditions
3. Handle long-running processes with checkpoints
4. React to changes in external systems

## Decision

We will implement two key changes:

### 1. Rename AsyncNode to TransformNode

We will rename the current `AsyncNode` trait to `TransformNode` to better reflect its actual purpose - providing a functional transformation interface rather than special async capabilities:

```rust
#[async_trait]
pub trait TransformNode<Input, Output, Error>: Send + Sync
where
    Input: Send + 'static,
    Output: Send + 'static,
    Error: std::error::Error + Send + Sync + 'static,
{
    /// Preparation phase
    async fn prep(&self, input: Input) -> Result<Input, Error>;

    /// Execution phase
    async fn exec(&self, input: Input) -> Result<Output, Error>;

    /// Post-execution phase
    async fn post(&self, output: Output) -> Result<Output, Error>;
}
```

All related types and functions will also be renamed:

- `AsyncNodeAdapter` → `TransformNodeAdapter`
- `AsyncContext` → `TransformContext`
- `async_node` → `transform_node`
- `to_lifecycle_node` → Remains the same but will work with `TransformNode`

### 2. Introduce True Async Extension Patterns

We will introduce new node traits that enable truly async-specific patterns:

#### A. EventDrivenNode

A node that waits for external events:

```rust
#[async_trait]
pub trait EventDrivenNode<Event, Context, Action>: Send + Sync
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    /// Wait for an external event to occur
    async fn wait_for_event(&self) -> Result<Event, FloxideError>;

    /// Process the received event and update context
    async fn process_event(
        &self,
        event: Event,
        ctx: &mut Context
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

#### B. TimerNode

A node that executes based on time schedules:

```rust
/// Represents a time schedule for execution
pub enum Schedule {
    Once(DateTime<Utc>),
    Interval(Duration),
    Daily(u32, u32), // Hour, minute
    Weekly(Weekday, u32, u32), // Day of week, hour, minute
    Monthly(u32, u32, u32), // Day of month, hour, minute
    Cron(String), // Cron expression
}

#[async_trait]
pub trait TimerNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
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

#### C. LongRunningNode

A node that handles long-running processes with checkpoints:

```rust
pub enum LongRunningOutcome<T, S> {
    /// Processing is complete with result
    Complete(T),
    /// Processing needs to be suspended with saved state
    Suspend(S),
}

#[async_trait]
pub trait LongRunningNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Self::State: Serialize + Deserialize + Send + Sync + 'static,
    Self::Output: Send + 'static,
{
    /// Type representing the node's processing state
    type State;

    /// Type representing the final output
    type Output;

    /// Process the next step, potentially suspending execution
    async fn process(
        &self,
        state: Option<Self::State>,
        ctx: &mut Context
    ) -> Result<LongRunningOutcome<Self::Output, Self::State>, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

#### D. ReactiveNode

A node that reacts to changes in external data sources:

```rust
#[async_trait]
pub trait ReactiveNode<Change, Context, Action>: Send + Sync
where
    Change: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    /// Set up a stream of changes to watch
    async fn watch(&self) -> impl Stream<Item = Change> + Send;

    /// React to a detected change
    async fn react_to_change(
        &self,
        change: Change,
        ctx: &mut Context
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

## Consequences

### Advantages

1. **Clearer Naming**: `TransformNode` better reflects the actual purpose of the abstraction
2. **New Capabilities**: The new async node traits enable workflow patterns not possible before
3. **Better Resource Usage**: Event-driven and reactive nodes prevent wasting resources on polling
4. **Extended Use Cases**: Enable integration with external systems, time-based execution, and long-running workflows
5. **Richer Model**: The framework can represent more real-world workflow patterns

### Disadvantages

1. **Increased Complexity**: Adding more node types increases the conceptual overhead
2. **Implementation Effort**: Building the infrastructure for these patterns requires significant work
3. **Integration Challenges**: Integrating these patterns with the existing workflow engine will require careful design

### Migration Path

1. **For TransformNode**: Migration will be straightforward via a deprecation period

   - Mark `AsyncNode` as deprecated with a note to use `TransformNode` instead
   - Keep both traits for a transitional period
   - Eventually remove `AsyncNode` in a future breaking release

2. **For Async Extensions**: These are new capabilities, so no migration is needed

## Alternatives Considered

### 1. Keep AsyncNode As Is

We could keep the current `AsyncNode` as is and just improve documentation to clarify its purpose. However, this would perpetuate the confused naming and miss the opportunity to add truly valuable async capabilities.

### 2. Add Async Capabilities to Existing Abstractions

We could add async capabilities (events, timers, etc.) to the existing node abstractions instead of creating new traits. This would reduce the number of abstractions but would make the existing ones more complex and harder to implement correctly.

### 3. Create a Single AsyncExtension Trait

We could create a single trait that covers all async extension patterns. This would be simpler conceptually but would force nodes to implement capabilities they don't need.

## Implementation Notes

- The async extensions will be implemented in a new `floxide-async-ext` crate
- Each async pattern will include adapter types to integrate with the core workflow engine
- Examples will be provided for each pattern to demonstrate proper usage
- Time-based execution will require a scheduler component
- Event-driven and reactive nodes will require integration with relevant async primitives (channels, streams, etc.)

## Related ADRs

- [ADR-0003: Core Framework Abstractions](0003-core-framework-abstractions.md)
- [ADR-0004: Async Runtime Selection](0004-async-runtime-selection.md)
- [ADR-0015: Node Abstraction Hierarchy](0015-node-abstraction-hierarchy.md)

## References

- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Documentation](https://tokio.rs/tokio/tutorial)
- [Reactive Programming Patterns](https://www.reactivemanifesto.org/)
- [Event-Driven Architecture](https://en.wikipedia.org/wiki/Event-driven_architecture)
