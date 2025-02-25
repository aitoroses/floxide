# ADR-0022: LongRunning Node Implementation

## Status

Accepted

## Date

2024-02-25

## Context

In ADR-0016, we outlined the need for a node type that can handle long-running processes with checkpoints. These long-running nodes enable workflows to:

1. Process work incrementally over multiple sessions
2. Save state between executions
3. Resume from the last checkpoint
4. Handle complex multi-step processes that may be paused and resumed

Long-running nodes are particularly useful for:

- Processing that spans multiple sessions
- Workflows that need to wait for external events or human interaction
- Resource-intensive tasks that should be broken into manageable chunks
- Processes that may be interrupted but need to resume from a checkpoint

## Decision

We will implement a new crate `flowrs-longrunning` that provides the `LongRunningNode` trait and related implementations as described in ADR-0016. The implementation will follow these design decisions:

### 1. Core `LongRunningOutcome` Enum

We will implement a `LongRunningOutcome` enum that represents the two possible outcomes of a long-running process:

```rust
pub enum LongRunningOutcome<T, S> {
    /// Processing is complete with result
    Complete(T),
    /// Processing needs to be suspended with saved state
    Suspend(S),
}
```

### 2. `LongRunningNode` Trait

We will implement the `LongRunningNode` trait as outlined in ADR-0016:

```rust
#[async_trait]
pub trait LongRunningNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Self::State: Serialize + Deserialize<'static> + Send + Sync + 'static,
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
        ctx: &mut Context,
    ) -> Result<LongRunningOutcome<Self::Output, Self::State>, FlowrsError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

### 3. `LongRunningActionExt` Extension Trait

We will provide a `LongRunningActionExt` trait to extend `ActionType` with long-running specific actions:

```rust
pub trait LongRunningActionExt: ActionType {
    /// Create a suspend action for long-running nodes
    fn suspend() -> Self;

    /// Create a resume action for long-running nodes
    fn resume() -> Self;

    /// Create a complete action for long-running nodes
    fn complete() -> Self;

    /// Check if this is a suspend action
    fn is_suspend(&self) -> bool;

    /// Check if this is a resume action
    fn is_resume(&self) -> bool;

    /// Check if this is a complete action
    fn is_complete(&self) -> bool;
}
```

### 4. Concrete Implementations

We will provide these concrete implementations:

- `SimpleLongRunningNode`: A long-running node that uses a closure for processing
- `LongRunningNodeAdapter`: An adapter to use a long-running node as a standard node
- `StateStore`: A trait for storing and retrieving node states
- `InMemoryStateStore`: A simple in-memory implementation of `StateStore` for testing

### 5. Workflow Integration

The `LongRunningNodeAdapter` will implement the `Node` trait, allowing long-running nodes to be used in standard workflows. This adapter will handle state management and action conversion.

## Consequences

### Advantages

1. **State Persistence**: Enables workflows to save and resume state across multiple executions.
2. **Incremental Processing**: Allows breaking down large tasks into manageable chunks.
3. **Checkpoint Recovery**: Provides a mechanism for resuming from the last successful checkpoint.
4. **Workflow Suspension**: Supports pausing workflows for external events or human interaction.
5. **Resource Efficiency**: Prevents long-running tasks from blocking workflow execution.

### Disadvantages

1. **Complexity**: Adds another node type, increasing the conceptual overhead.
2. **State Management**: Requires proper state serialization and storage infrastructure.
3. **Debugging Challenges**: Stateful workflows can be more difficult to debug and reason about.
4. **Implementation Overhead**: Users need to implement proper state management.

### Implementation Notes

1. States must be serializable and deserializable to be properly stored between executions.
2. The actual storage mechanism (database, file system, etc.) is left to the implementation.
3. The `LongRunningNodeAdapter` allows using long-running nodes seamlessly in standard workflows.
4. Integration tests demonstrate proper state management and resumption.

## Alternatives Considered

### 1. Using Event-Driven Nodes for Long-Running Processes

We considered implementing long-running process support as a special case of event-driven nodes. However, the state management requirements are sufficiently different to warrant a separate abstraction.

### 2. Implicit State Management in the Workflow Engine

We considered building state management directly into the workflow engine rather than in the nodes. While this would simplify the node implementation, it would make the workflow engine more complex and limit flexibility.

### 3. Framework-Provided Storage Backend

We considered implementing a standard storage backend for states. However, we decided that providing a trait-based interface allows users to implement storage that best fits their needs.

## Related ADRs

- [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md)
- [ADR-0021: Timer Node Implementation](0021-timer-node-implementation.md)

## References

- [Serde for Rust](https://serde.rs/)
- [Tokio Documentation](https://tokio.rs/tokio/tutorial)
