# Workflows

Workflows in Floxide orchestrate the execution of nodes. A workflow is a directed graph of nodes, where the edges represent transitions between nodes. This page explains how to create, configure, and execute workflows.

## The Workflow Struct

The core of workflow orchestration in Floxide is the `Workflow` struct. This struct manages the execution flow between nodes, handles errors and retries, and provides observability and monitoring.

```rust
pub struct Workflow<C, A> {
    // Internal implementation details
}
```

Where:

- `C` is the context type that the workflow operates on
- `A` is the action type that the nodes in the workflow return

## Creating Workflows

### Basic Workflow

The simplest way to create a workflow is to use the `new` method, which takes a single node:

```rust
use floxide_core::{lifecycle_node, LifecycleNode, Workflow, DefaultAction};
use std::sync::Arc;

// Create a node
let node = Arc::new(create_processor_node());

// Create a workflow with the node
let mut workflow = Workflow::new(node);
```

### Linear Workflow

You can create a linear workflow by chaining nodes using the `then` method:

```rust
let node1 = Arc::new(create_first_node());
let node2 = Arc::new(create_second_node());
let node3 = Arc::new(create_third_node());

let mut workflow = Workflow::new(node1)
    .then(node2)
    .then(node3);
```

This creates a workflow where `node1` is executed first, followed by `node2`, and then `node3`.

### Conditional Branching

You can create conditional branches in your workflow using the `on` method, which takes an action and a node:

```rust
enum CustomAction {
    Success,
    Error,
    Retry,
}

let decision_node = Arc::new(create_decision_node());
let success_node = Arc::new(create_success_node());
let error_node = Arc::new(create_error_node());
let retry_node = Arc::new(create_retry_node());

let mut workflow = Workflow::new(decision_node)
    .on(CustomAction::Success, success_node)
    .on(CustomAction::Error, error_node)
    .on(CustomAction::Retry, retry_node);
```

This creates a workflow where the execution path depends on the action returned by `decision_node`.

## Executing Workflows

To execute a workflow, use the `execute` method, which takes a mutable reference to a context:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a context
    let mut context = MessageContext {
        input: "Hello, Floxide!".to_string(),
        result: None,
    };

    // Create a workflow
    let node = Arc::new(create_processor_node());
    let mut workflow = Workflow::new(node);

    // Execute the workflow
    workflow.execute(&mut context).await?;

    // Print the result
    println!("Result: {:?}", context.result);

    Ok(())
}
```

## Advanced Workflow Patterns

### Error Handling

Floxide provides several ways to handle errors in workflows:

#### Try-Catch Pattern

You can implement a try-catch pattern using conditional branching:

```rust
enum CustomAction {
    Success,
    Error,
}

let try_node = Arc::new(create_try_node());
let success_node = Arc::new(create_success_node());
let catch_node = Arc::new(create_catch_node());

let mut workflow = Workflow::new(try_node)
    .on(CustomAction::Success, success_node)
    .on(CustomAction::Error, catch_node);
```

#### Retry Pattern

You can implement a retry pattern by returning to a previous node:

```rust
enum CustomAction {
    Success,
    Retry,
    Error,
}

let operation_node = Arc::new(create_operation_node());
let success_node = Arc::new(create_success_node());
let retry_node = Arc::new(create_retry_node());
let error_node = Arc::new(create_error_node());

let mut workflow = Workflow::new(operation_node)
    .on(CustomAction::Success, success_node)
    .on(CustomAction::Retry, retry_node)
    .on(CustomAction::Error, error_node);

// Configure retry_node to return to operation_node
```

### Parallel Execution

For parallel execution, you can use the `BatchNode` from the `floxide-batch` crate:

```rust
use floxide_batch::{batch_node, BatchNode, BatchContext};

// Create a batch node that processes items in parallel
let batch_node = Arc::new(batch_node(
    10, // Concurrency limit
    |item: String| async move {
        Ok(item.to_uppercase())
    }
));

// Create a workflow with the batch node
let mut workflow = Workflow::new(batch_node);
```

### Event-Driven Workflows

For event-driven workflows, you can use the `EventNode` from the `floxide-event` crate:

```rust
use floxide_event::{event_node, EventNode, EventContext};

// Create an event node that responds to events
let event_node = Arc::new(event_node(
    |event: String| async move {
        Ok(format!("Processed event: {}", event))
    }
));

// Create a workflow with the event node
let mut workflow = Workflow::new(event_node);
```

## Workflow Composition

You can compose workflows by treating a workflow as a node in a larger workflow:

```rust
// Create a sub-workflow
let sub_workflow_node = Arc::new(create_sub_workflow_node());
let sub_workflow = Workflow::new(sub_workflow_node);

// Create a main workflow that includes the sub-workflow
let main_workflow_node = Arc::new(create_main_workflow_node());
let mut main_workflow = Workflow::new(main_workflow_node)
    .then(Arc::new(sub_workflow));
```

## Workflow Persistence

Floxide supports workflow persistence through serialization and deserialization:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistableContext {
    input: String,
    result: Option<String>,
    state: WorkflowState,
}

// Serialize the context to save the workflow state
let serialized = serde_json::to_string(&context)?;

// Later, deserialize the context to resume the workflow
let mut context: PersistableContext = serde_json::from_str(&serialized)?;
workflow.execute(&mut context).await?;
```

## Best Practices

When creating workflows, consider the following best practices:

1. **Keep workflows focused**: Each workflow should have a clear purpose.
2. **Use appropriate branching**: Choose the right branching pattern for your use case.
3. **Handle errors gracefully**: Implement proper error handling and recovery mechanisms.
4. **Consider performance**: Be mindful of performance implications, especially for parallel execution.
5. **Leverage type safety**: Use Rust's type system to ensure type safety between nodes.

## Next Steps

Now that you understand workflows, you can learn about:

- [Actions](actions.md): How to control the flow of execution
- [Contexts](contexts.md): How to define and use contexts in your workflows
- [Nodes](nodes.md): Learn more about the different types of nodes
