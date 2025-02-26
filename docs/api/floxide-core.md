# floxide-core API Reference

The `floxide-core` crate provides the foundational types and traits for building workflows in the Floxide framework. For a detailed overview of the core abstractions, see [Core Framework Abstractions](../architecture/core-framework-abstractions.md).

## Core Types

### Node

```rust
#[async_trait]
pub trait Node<Context, Action>: Send + Sync + 'static
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    /// Get the node's unique identifier
    fn id(&self) -> NodeId;

    /// Execute the node's logic
    async fn execute(&self, ctx: &mut Context) -> Result<Action, FloxideError>;
}
```

The `Node` trait is the fundamental building block of workflows. For detailed information about the node lifecycle, see [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md).

### Workflow

```rust
pub struct Workflow<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    nodes: HashMap<NodeId, Box<dyn Node<Context, Action>>>,
    routes: HashMap<(NodeId, Action), NodeId>,
}
```

The `Workflow` struct orchestrates the execution of nodes. For more information about workflow patterns, see [Event-Driven Workflow Pattern](../architecture/event-driven-workflow-pattern.md).

### ActionType

```rust
pub trait ActionType: Clone + Debug + Send + Sync + 'static {
    /// Convert the action to a string representation
    fn as_str(&self) -> &str;
}
```

The `ActionType` trait defines how nodes signal their completion and routing decisions.

### FloxideError

```rust
#[derive(Debug, Error)]
pub enum FloxideError {
    #[error("Node error: {0}")]
    NodeError(String),
    #[error("Workflow error: {0}")]
    WorkflowError(String),
    #[error("Routing error: {0}")]
    RoutingError(String),
    #[error("Context error: {0}")]
    ContextError(String),
    #[error("Other error: {0}")]
    Other(String),
}
```

The `FloxideError` type provides structured error handling throughout the framework.

## Usage Examples

### Basic Node

```rust
use floxide_core::{Node, NodeId, FloxideError, DefaultAction};

struct MyNode {
    id: NodeId,
}

#[async_trait]
impl Node<MyContext, DefaultAction> for MyNode {
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn execute(&self, ctx: &mut MyContext) -> Result<DefaultAction, FloxideError> {
        // Node implementation
        Ok(DefaultAction::Next)
    }
}
```

### Workflow Construction

```rust
use floxide_core::{Workflow, DefaultAction};

let mut workflow = Workflow::new(start_node)
    .then(process_node)
    .then(end_node);

workflow.add_conditional_route(
    process_node.id(),
    CustomAction::Retry,
    start_node.id(),
);
```

## Best Practices

1. **Node Design**
   - Keep nodes focused on a single responsibility
   - Use appropriate lifecycle methods
   - Handle errors gracefully
   - Clean up resources properly

2. **Workflow Design**
   - Model workflows as directed graphs
   - Use clear routing logic
   - Handle all possible actions
   - Consider error recovery paths

3. **Error Handling**
   - Use specific error types
   - Provide clear error messages
   - Implement proper recovery
   - Log relevant details

4. **Testing**
   - Test individual nodes
   - Test workflow routing
   - Test error scenarios
   - Use mock contexts

## See Also

- [Core Framework Abstractions](../architecture/core-framework-abstractions.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
- [Event-Driven Workflow Pattern](../architecture/event-driven-workflow-pattern.md)
- [Batch Processing Implementation](../architecture/batch-processing-implementation.md)
- [Async Runtime Selection](../architecture/async-runtime-selection.md)
