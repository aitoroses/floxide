# ADR-0015: Node Abstraction Hierarchy

## Status

Proposed

## Date

2024-02-25

## Context

The Flowrs framework provides multiple node abstractions to support different programming models and use cases:

1. The base `Node` trait with a single `process` method
2. The `LifecycleNode` trait with the prep/exec/post lifecycle
3. A planned `AsyncNode` trait for async-specific workflows

This creates confusion about which abstraction to use when implementing workflow nodes. The README currently shows an example using an `AsyncNode` trait that doesn't match the actual implementation, while the examples use the base `Node` trait directly.

We need to clarify the relationship between these abstractions, their intended use cases, and provide clear guidance on when to use each approach.

## Decision

We will establish a clear hierarchy of node abstractions with well-defined relationships:

### 1. Base Node Trait

The `Node` trait will remain the core abstraction that all workflow nodes must implement:

```rust
#[async_trait]
pub trait Node<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Self::Output: Send + Sync + 'static,
{
    /// The output type produced by this node
    type Output;

    /// Get the unique identifier for this node
    fn id(&self) -> NodeId;

    /// Process the node asynchronously
    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FlowrsError>;
}
```

This trait is the foundation of the workflow system and is used by the `Workflow` struct to execute nodes.

### 2. LifecycleNode Trait

The `LifecycleNode` trait provides a more structured approach with three distinct phases:

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
    async fn prep(&self, ctx: &mut Context) -> Result<Self::PrepOutput, FlowrsError>;

    /// Execution phase - perform the main work
    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FlowrsError>;

    /// Post-execution phase - determine the next action and update context
    async fn post(
        &self,
        prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        ctx: &mut Context,
    ) -> Result<Action, FlowrsError>;
}
```

The `LifecycleNode` trait is adapted to the base `Node` trait using the `LifecycleNodeAdapter` struct, which implements the `process` method by calling the three lifecycle methods in sequence.

### 3. TransformNode Trait (Formerly AsyncNode)

As detailed in [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md), we will rename the `AsyncNode` trait to `TransformNode` to better reflect its actual purpose - providing a functional transformation interface:

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

This trait will be adapted to the `LifecycleNode` trait, which in turn adapts to the base `Node` trait.

### Adapter Pattern

We will use the adapter pattern to convert between these abstractions:

1. `LifecycleNodeAdapter`: Converts a `LifecycleNode` to a `Node`
2. `TransformNodeAdapter` (formerly `AsyncNodeAdapter`): Converts a `TransformNode` to a `LifecycleNode`

This approach allows users to choose the abstraction that best fits their use case while maintaining compatibility with the core workflow system.

### Usage Guidelines

We will provide clear guidelines on when to use each abstraction:

1. **Base Node**: Use when you need complete control over the node execution process or when implementing custom node types that don't fit the lifecycle pattern.

2. **LifecycleNode**: Use for most workflow nodes that benefit from the clear separation of concerns provided by the prep/exec/post lifecycle.

3. **TransformNode**: Use for simple transformations where the input and output types are known and consistent, and you prefer a functional programming style.

## Consequences

### Advantages

1. **Clear Hierarchy**: Establishes a clear relationship between the different node abstractions
2. **Flexibility**: Allows users to choose the abstraction that best fits their use case
3. **Compatibility**: Maintains compatibility with existing code through the adapter pattern
4. **Separation of Concerns**: The lifecycle pattern provides clear separation of concerns for node implementation

### Disadvantages

1. **Complexity**: Multiple abstractions increase the learning curve for new users
2. **Adapter Overhead**: The adapter pattern introduces some runtime overhead
3. **Documentation Burden**: Requires clear documentation to explain the different abstractions

### Migration Path

Existing code using the base `Node` trait can continue to work without changes. For new code, we recommend:

1. Use the `LifecycleNode` trait for most workflow nodes
2. Use the base `Node` trait for custom node types that don't fit the lifecycle pattern
3. Use the `TransformNode` trait for simple transformations (previously called `AsyncNode`)

## Alternatives Considered

### 1. Single Node Trait

We considered having a single `Node` trait with optional lifecycle methods, but this would make the API less clear and harder to implement correctly.

### 2. Complete Replacement

We considered completely replacing the base `Node` trait with the `LifecycleNode` trait, but this would break compatibility with existing code.

### 3. Macro-Based Approach

We evaluated using macros to generate the appropriate trait implementations, but this would make the code harder to understand and debug.

## Implementation Notes

- The `LifecycleNode` trait requires prep/exec outputs to implement Clone for simplicity
- The adapter automatically converts to NodeOutcome::RouteToAction
- Unit tests verify the full lifecycle and error propagation between phases

## Related ADRs

- [ADR-0003: Core Framework Abstractions](0003-core-framework-abstractions.md)
- [ADR-0004: Async Runtime Selection](0004-async-runtime-selection.md)
- [ADR-0008: Node Lifecycle Methods](0008-node-lifecycle-methods.md)
- [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md)

## References

- [Adapter Pattern](https://refactoring.guru/design-patterns/adapter)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
