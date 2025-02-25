# ADR-0010: Workflow Cloning Strategy

## Status

Accepted

## Date

2024-02-25

## Context

Our codebase has multiple areas where we need to clone or share workflows:

1. In the `Workflow::from_arc` method, which attempts to clone a workflow from an `Arc<Workflow>`
2. In the `BatchNode` implementation, which stores workflows in an `Arc` and clones the reference for each worker task

However, we're encountering issues because:

1. `Box<dyn Node<...>>` does not implement `Clone`, preventing direct cloning of the nodes HashMap
2. We need to preserve the ability to share workflows between tasks efficiently
3. There are logical ownership constraints in async code that prevent simply using references with lifetimes

## Decision

We will take a multi-pronged approach to solve workflow cloning issues:

### 1. Use Arc for Node Storage

Instead of storing nodes directly in `Box<dyn Node<...>>`, we'll store them in `Arc<dyn Node<...>>`:

```rust
pub(crate) nodes: HashMap<NodeId, Arc<dyn Node<Context, A, Output = Output>>>,
```

This allows easy cloning of the entire node collection without duplicating the actual node implementations.

### 2. Implement Clone for Workflow

We'll implement a proper `Clone` implementation for `Workflow` that clones the structure but shares the node implementations:

```rust
impl<Context, A, Output> Clone for Workflow<Context, A, Output>
where
    Context: Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
    Output: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            start_node: self.start_node.clone(),
            nodes: self.nodes.clone(), // Now possible because we're using Arc
            edges: self.edges.clone(),
            default_routes: self.default_routes.clone(),
        }
    }
}
```

### 3. Remove from_arc Method

The `Workflow::from_arc` method will be removed since it's no longer necessary - Arc<Workflow> can now be dereferenced and cloned directly.

### 4. Refactor BatchNode to Leverage Cloning

The `BatchNode` will be updated to use this clone capability rather than wrapping the workflow in an Arc:

```rust
pub struct BatchNode<Context, ItemType, A = crate::action::DefaultAction>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
{
    id: NodeId,
    item_workflow: Workflow<Context, A>,  // No longer an Arc
    parallelism: usize,
    _phantom: PhantomData<(Context, ItemType, A)>,
}

// In the process method:
let workflow = self.item_workflow.clone();  // Now directly cloneable
```

## Consequences

### Advantages

1. **Cleaner API**: No need for Arc-specific methods
2. **Memory Efficiency**: Node implementations are shared, not duplicated
3. **Thread Safety**: Arc provides thread-safe reference counting
4. **Type Safety**: Cloning is now properly supported at the type level

### Disadvantages

1. **Indirection Cost**: Extra indirection through Arc when accessing nodes
2. **Memory Overhead**: Arc has a small overhead per reference
3. **API Changes**: Will require changes to code that expects Box<dyn Node>

### Migration Plan

1. First, update the `Workflow` struct to use `Arc<dyn Node>` instead of `Box<dyn Node>`
2. Implement `Clone` for `Workflow`
3. Update the `BatchNode` implementation to leverage this new capability
4. Remove the now-redundant `from_arc` method
5. Update tests to verify correct cloning behavior

## Alternatives Considered

### Use Clone Trait Objects

We considered making `Node` require `Clone`, but this would be problematic because:

1. Trait objects cannot use clone to return a new trait object
2. It would require all node implementations to implement Clone

### Keep Arc<Workflow> as the Primary Interface

We considered embracing Arc<Workflow> as the primary way to share workflows, but this would make the API more cumbersome and push complexity to the caller.

### Use Cow (Clone-on-Write)

We explored using Cow<Workflow> to defer cloning until mutation, but this added complexity without significant benefits given our usage patterns.

## Implementation Notes

- The change to Arc will be backward compatible for most code that consumes nodes
- We'll need to update node creation code to wrap nodes in Arc instead of Box
- This change reinforces the immutability of nodes once created
