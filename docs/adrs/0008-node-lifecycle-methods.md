# ADR-0008: Node Lifecycle Methods

## Status

Accepted

## Date

2025-02-27

## Context

The Flow Framework uses a three-phase lifecycle for nodes:

1. `prep`: Preparation phase for setup and validation
2. `execCore`: Core execution with potential retry mechanisms
3. `post`: Post-processing phase that determines routing

This pattern provides clear separation of concerns and allows for specialized behaviors in each phase. We need to implement this pattern in Rust while following Rust idioms.

## Decision

We will introduce a `LifecycleNode` trait that explicitly models the three-phase lifecycle, while maintaining compatibility with the existing `Node` trait through adapter patterns.

### LifecycleNode Trait

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

    /// Post-execution phase - determine the next action and update context
    async fn post(&self, prep_result: Self::PrepOutput,
                 exec_result: Self::ExecOutput,
                 ctx: &mut Context) -> Result<Action, FloxideError>;
}
```

### Adapter Pattern

To maintain compatibility with the existing `Node` trait, we'll implement an adapter that converts LifecycleNodes to Nodes:

```rust
pub struct LifecycleNodeAdapter<LN, Context, Action>
where
    LN: LifecycleNode<Context, Action>,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    inner: LN,
    _phantom: PhantomData<(Context, Action)>,
}

#[async_trait]
impl<LN, Context, Action> Node<Context, Action> for LifecycleNodeAdapter<LN, Context, Action>
where
    LN: LifecycleNode<Context, Action> + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    LN::ExecOutput: Send + Sync + 'static,
{
    type Output = LN::ExecOutput;

    fn id(&self) -> NodeId {
        self.inner.id()
    }

    async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, Action>, FloxideError> {
        // Run the three-phase lifecycle
        debug!(node_id = %self.id(), "Starting prep phase");
        let prep_result = self.inner.prep(ctx).await?;

        debug!(node_id = %self.id(), "Starting exec phase");
        let exec_result = self.inner.exec(prep_result.clone()).await?;

        debug!(node_id = %self.id(), "Starting post phase");
        let next_action = self.inner.post(prep_result, exec_result.clone(), ctx).await?;

        // Return the appropriate outcome based on the action
        Ok(NodeOutcome::RouteToAction(next_action))
    }
}
```

### Builder Function

For convenience, we'll provide a closure-based API that makes it easy to create lifecycle nodes:

```rust
pub fn lifecycle_node<PrepFn, ExecFn, PostFn, Context, Action, PrepOut, ExecOut, PrepFut, ExecFut, PostFut>(
    id: Option<String>,
    prep_fn: PrepFn,
    exec_fn: ExecFn,
    post_fn: PostFn,
) -> impl Node<Context, Action, Output = ExecOut>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    PrepOut: Send + Sync + Clone + 'static,
    ExecOut: Send + Sync + Clone + 'static,
    PrepFn: Fn(&mut Context) -> PrepFut + Send + Sync + 'static,
    ExecFn: Fn(PrepOut) -> ExecFut + Send + Sync + 'static,
    PostFn: Fn(PrepOut, ExecOut, &mut Context) -> PostFut + Send + Sync + 'static,
    PrepFut: Future<Output = Result<PrepOut, FloxideError>> + Send + 'static,
    ExecFut: Future<Output = Result<ExecOut, FloxideError>> + Send + 'static,
    PostFut: Future<Output = Result<Action, FloxideError>> + Send + 'static,
{
    // Implementation details...
}
```

## Consequences

### Advantages

1. **Clear Separation**: Each phase has a distinct purpose and signature
2. **Compatibility**: Works with existing Node interface through the adapter
3. **Type Safety**: Phase outputs are properly typed
4. **Flexibility**: Different nodes can define their own prep/exec types
5. **Consistency**: Maintains a clear and structured lifecycle approach for workflow nodes

### Disadvantages

1. **Complexity**: More complex than a single process method
2. **Clone Requirements**: Requires Clone trait on phase outputs
3. **Type Complexity**: More generic parameters than the simpler Node trait

### Migration Path

Existing nodes using the Node trait can continue to work without changes. New nodes can use the LifecycleNode trait with the adapter, or use the convenient lifecycle_node builder function.

## Alternatives Considered

### Single Method with Internal Phases

We considered having a single `process` method that internally calls prep/exec/post methods, but this would make the phase outputs harder to type correctly and require more dynamic typing.

### Complete Replacement

We considered completely replacing the Node trait with LifecycleNode, but this would break compatibility with existing code.

### Dynamic Function Parameters

We evaluated using dynamic function parameters to allow more flexibility in the lifecycle, but this would have required more complex trait bounds and potentially runtime checks.

## Implementation Notes

- The LifecycleNode trait requires prep/exec outputs to implement Clone for simplicity
- The adapter automatically converts to NodeOutcome::RouteToAction
- Unit tests verify the full lifecycle and error propagation between phases
