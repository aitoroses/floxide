# Contexts

Contexts are a fundamental concept in the Floxide framework that provide a way to share state between nodes in a workflow. They serve as the primary mechanism for passing data through the workflow execution pipeline.

## What is a Context?

A context in Floxide is a container for state that is passed between nodes during workflow execution. It encapsulates all the data needed for a node to perform its operations and allows nodes to communicate with each other by modifying and passing along this state.

Contexts are strongly typed, which means that the type of data they can contain is defined at compile time. This provides type safety and ensures that nodes can only access and modify data in ways that are compatible with the defined types.

## Context Lifecycle

Contexts follow a lifecycle that aligns with the node execution lifecycle:

1. **Creation**: A context is created when a workflow begins execution.
2. **Preparation**: During the `prep` phase, nodes can read from and write to the context to prepare for execution.
3. **Execution**: During the `exec` phase, nodes can read from and write to the context to perform their main operations.
4. **Post-processing**: During the `post` phase, nodes can read from and write to the context to perform cleanup or finalization operations.
5. **Completion**: When the workflow completes, the final state of the context represents the result of the workflow execution.

## Context Types

Floxide supports several types of contexts to accommodate different workflow patterns:

### Basic Context

The basic context is used for simple workflows where a single state object is passed between nodes. It is defined using a generic type parameter that specifies the type of state it can contain.

```rust
use floxide_core::context::Context;

// Define a state type
struct MyState {
    value: i32,
}

// Create a context with the state
let context = Context::new(MyState { value: 42 });
```

### Batch Context

The batch context is used for batch processing workflows where multiple items need to be processed in parallel. It extends the basic context with functionality for managing a collection of items.

```rust
use floxide_batch::context::BatchContext;

// Define a state type
struct MyState {
    values: Vec<i32>,
}

// Create a batch context with the state
let context = BatchContext::new(MyState { values: vec![1, 2, 3, 4, 5] });
```

### Event Context

The event context is used for event-driven workflows where nodes can emit and respond to events. It extends the basic context with functionality for event handling.

```rust
use floxide_event::context::EventContext;

// Define a state type
struct MyState {
    value: i32,
}

// Create an event context with the state
let context = EventContext::new(MyState { value: 42 });
```

## Accessing Context State

Nodes can access and modify the state contained in a context using the context's methods. The most common methods are:

- `state()`: Returns a reference to the state.
- `state_mut()`: Returns a mutable reference to the state.

```rust
use floxide_core::context::Context;
use floxide_core::node::{Node, NodeResult};

struct MyNode;

impl Node for MyNode {
    type Context = Context<MyState>;

    fn exec(&self, ctx: &mut Self::Context) -> NodeResult {
        // Access the state
        let value = ctx.state().value;

        // Modify the state
        ctx.state_mut().value += 1;

        Ok(())
    }
}
```

## Context Extensions

Contexts can be extended with additional functionality through the use of traits. This allows for specialized behavior while maintaining a consistent interface.

For example, the `BatchContext` extends the basic `Context` with methods for batch processing, and the `EventContext` extends it with methods for event handling.

## Best Practices

When working with contexts in Floxide, consider the following best practices:

1. **Keep state minimal**: Include only the data that is necessary for the workflow execution.
2. **Use appropriate context types**: Choose the context type that best matches your workflow pattern.
3. **Respect immutability**: Only modify state when necessary, and prefer immutable access when possible.
4. **Handle errors gracefully**: Ensure that context state remains valid even when errors occur.
5. **Document state requirements**: Clearly document what state a node expects and how it modifies that state.

## Related Concepts

- [Nodes](nodes.md): The processing units that operate on contexts.
- [Workflows](workflows.md): The orchestrators that manage the flow of contexts through nodes.
- [Actions](actions.md): The operations that nodes perform on contexts.

## Conclusion

Contexts are a powerful abstraction in the Floxide framework that enable type-safe state management and communication between nodes in a workflow. By understanding how to use contexts effectively, you can build robust and maintainable workflow applications.
