# Nodes

Nodes are the fundamental building blocks of workflows in Floxide. Each node represents a discrete unit of work that can be executed as part of a workflow. This page explains the different types of nodes and how to create and use them.

## The Node Trait

At the core of Floxide is the `Node` trait, which defines the interface for executing a node:

```rust
#[async_trait]
pub trait Node<C, A> {
    async fn execute(&self, context: &mut C) -> Result<A, FloxideError>;
}
```

Where:

- `C` is the context type that the node operates on
- `A` is the action type that the node returns

This simple interface allows for a wide variety of node implementations, from simple function-based nodes to complex stateful nodes.

## LifecycleNode

The most common type of node in Floxide is the `LifecycleNode`, which follows a three-phase lifecycle:

1. **Preparation (Prep)**: Extract data from the context
2. **Execution (Exec)**: Process the data
3. **Post-processing (Post)**: Update the context with the result and determine the next action

The `LifecycleNode` trait is defined as:

```rust
#[async_trait]
pub trait LifecycleNode<C, A>: Send + Sync {
    type PrepOutput: Send;
    type ExecOutput: Send;

    async fn prep(&self, context: &mut C) -> Result<Self::PrepOutput, FloxideError>;
    async fn exec(&self, prep_output: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError>;
    async fn post(
        &self,
        prep_output: Self::PrepOutput,
        exec_output: Self::ExecOutput,
        context: &mut C,
    ) -> Result<A, FloxideError>;
}
```

This trait allows for a clear separation of concerns between the different phases of node execution.

## Creating Nodes

### Using the `lifecycle_node` Function

The easiest way to create a `LifecycleNode` is using the `lifecycle_node` function:

```rust
use floxide_core::{lifecycle_node, LifecycleNode, DefaultAction};

// Define your context type
#[derive(Debug, Clone)]
struct MyContext {
    input: String,
    result: Option<String>,
}

// Create a node
fn create_processor_node() -> impl LifecycleNode<MyContext, DefaultAction> {
    lifecycle_node(
        Some("processor"), // Node ID
        |ctx: &mut MyContext| async move {
            // Preparation phase: extract data
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            // Execution phase: process data
            Ok(input.to_uppercase())
        },
        |ctx: &mut MyContext, result: String| async move {
            // Post-processing phase: update context
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}
```

### Implementing the Trait Manually

For more complex nodes, you can implement the `LifecycleNode` trait directly:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct CounterNode {
    id: String,
    counter: AtomicUsize,
}

#[async_trait]
impl LifecycleNode<MyContext, DefaultAction> for CounterNode {
    type PrepOutput = usize;
    type ExecOutput = usize;

    async fn prep(&self, _context: &mut MyContext) -> Result<Self::PrepOutput, FloxideError> {
        Ok(self.counter.load(Ordering::Relaxed))
    }

    async fn exec(&self, current: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        let new_value = current + 1;
        self.counter.store(new_value, Ordering::Relaxed);
        Ok(new_value)
    }

    async fn post(
        &self,
        _prep: Self::PrepOutput,
        exec: Self::ExecOutput,
        context: &mut MyContext,
    ) -> Result<DefaultAction, FloxideError> {
        context.result = Some(format!("Count: {}", exec));
        Ok(DefaultAction::Next)
    }
}
```

## Node Types

### Transform Node

Transform nodes are specialized for data transformation operations:

```rust
use floxide_transform::{transform_node, TransformNode};

fn create_transform_node() -> impl TransformNode<String, String> {
    transform_node(|input: String| async move {
        Ok(input.to_uppercase())
    })
}
```

### Batch Node

Batch nodes process collections of items concurrently:

```rust
use floxide_batch::{batch_node, BatchNode};

fn create_batch_node() -> impl BatchNode<Vec<String>, Vec<String>> {
    batch_node(
        10, // Concurrency limit
        |item: String| async move {
            Ok(item.to_uppercase())
        }
    )
}
```

### Event Node

Event nodes handle asynchronous events:

```rust
use floxide_event::{event_node, EventNode};

fn create_event_node() -> impl EventNode<MyContext, DefaultAction> {
    event_node(
        |event: Event| async move {
            match event {
                Event::Message(msg) => Ok(msg.process()),
                Event::Timeout => Ok(DefaultAction::Stop),
                _ => Ok(DefaultAction::Next),
            }
        }
    )
}
```

## Best Practices

### 1. Error Handling

Always implement proper error handling in your nodes:

```rust
fn create_robust_node() -> impl LifecycleNode<MyContext, DefaultAction> {
    lifecycle_node(
        Some("robust_processor"),
        |ctx: &mut MyContext| async move {
            ctx.input.parse::<i32>()
                .map_err(|e| FloxideError::new(format!("Invalid input: {}", e)))
        },
        |num: i32| async move {
            if num < 0 {
                Err(FloxideError::new("Negative numbers not allowed"))
            } else {
                Ok(num * 2)
            }
        },
        |ctx: &mut MyContext, result: i32| async move {
            ctx.result = Some(result.to_string());
            Ok(DefaultAction::Next)
        },
    )
}
```

### 2. Resource Management

For nodes that manage resources, implement proper cleanup:

```rust
struct ResourceNode {
    connection: Arc<Mutex<Connection>>,
}

impl ResourceNode {
    fn new() -> Self {
        Self {
            connection: Arc::new(Mutex::new(Connection::new())),
        }
    }
}

#[async_trait]
impl LifecycleNode<MyContext, DefaultAction> for ResourceNode {
    // ... implementation ...

    async fn post(
        &self,
        _prep: Self::PrepOutput,
        exec: Self::ExecOutput,
        context: &mut MyContext,
    ) -> Result<DefaultAction, FloxideError> {
        // Clean up resources
        self.connection.lock().await.cleanup();
        Ok(DefaultAction::Next)
    }
}
```

### 3. Context Management

Keep context modifications focused and explicit:

```rust
fn create_focused_node() -> impl LifecycleNode<MyContext, DefaultAction> {
    lifecycle_node(
        Some("focused_processor"),
        |ctx: &mut MyContext| async move {
            // Only access what you need
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            Ok(input.to_uppercase())
        },
        |ctx: &mut MyContext, result: String| async move {
            // Only modify what you need
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}
```

### 4. Testing

Make your nodes testable:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_processor_node() {
        let node = create_processor_node();
        let mut context = MyContext {
            input: "test".to_string(),
            result: None,
        };

        let result = node.execute(&mut context).await;
        assert!(result.is_ok());
        assert_eq!(context.result, Some("TEST".to_string()));
    }
}
```

## Common Patterns

### 1. Conditional Execution

```rust
fn create_conditional_node() -> impl LifecycleNode<MyContext, CustomAction> {
    lifecycle_node(
        Some("conditional"),
        |ctx: &mut MyContext| async move {
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            if input.is_empty() {
                Ok(CustomAction::Skip)
            } else {
                Ok(CustomAction::Process(input))
            }
        },
        |ctx: &mut MyContext, action: CustomAction| async move {
            match action {
                CustomAction::Process(result) => {
                    ctx.result = Some(result);
                    Ok(CustomAction::Next)
                }
                CustomAction::Skip => Ok(CustomAction::Skip),
            }
        },
    )
}
```

### 2. State Management

```rust
struct StatefulNode {
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl StatefulNode {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl LifecycleNode<MyContext, DefaultAction> for StatefulNode {
    // ... implementation with state management ...
}
```

### 3. Retry Logic

```rust
fn create_retry_node() -> impl LifecycleNode<MyContext, DefaultAction> {
    lifecycle_node(
        Some("retry_processor"),
        |ctx: &mut MyContext| async move {
            Ok((ctx.input.clone(), 0)) // Include retry count
        },
        |input: (String, i32)| async move {
            match process_with_retry(input.0, input.1).await {
                Ok(result) => Ok(result),
                Err(e) if input.1 < 3 => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Err(FloxideError::new("Retry"))
                }
                Err(e) => Err(e),
            }
        },
        |ctx: &mut MyContext, result: String| async move {
            ctx.result = Some(result);
            Ok(DefaultAction::Next)
        },
    )
}
```

## Next Steps

Now that you understand nodes, you can:

1. Learn about [Workflows](workflows.md) to see how nodes work together
2. Explore [Actions](actions.md) for flow control
3. Study [Contexts](contexts.md) for state management
4. Check out the [Examples](../examples/basic-workflow.md) section for more patterns

## Specialized Node Types

Floxide provides several specialized node types for common workflow patterns:

### TransformNode

A `TransformNode` is a simplified node that transforms an input to an output without the need for the full lifecycle. It's useful for data transformation steps in a workflow.

```rust
use floxide_transform::{transform_node, TransformNode};

fn create_transform_node() -> impl TransformNode<String, String> {
    transform_node(|input: String| async move {
        Ok(input.to_uppercase())
    })
}
```

### BatchNode

A `BatchNode` processes a collection of items in parallel, with configurable concurrency limits.

```rust
use floxide_batch::{batch_node, BatchNode};

fn create_batch_node() -> impl BatchNode<Vec<String>, Vec<String>> {
    batch_node(
        10, // Concurrency limit
        |item: String| async move {
            Ok(item.to_uppercase())
        }
    )
}
```

### EventNode

An `EventNode` responds to external events, allowing for event-driven workflows.

```rust
use floxide_event::{event_node, EventNode};

fn create_event_node() -> impl EventNode<String, String> {
    event_node(
        |event: String| async move {
            Ok(format!("Processed event: {}", event))
        }
    )
}
```

### TimerNode

A `TimerNode` executes based on time schedules, supporting one-time, interval, and calendar-based scheduling.

```rust
use floxide_timer::{timer_node, TimerNode, TimerContext};
use std::time::Duration;

fn create_timer_node() -> impl TimerNode<(), String> {
    timer_node(
        Duration::from_secs(60), // Execute every 60 seconds
        |_: ()| async move {
            Ok("Timer executed".to_string())
        }
    )
}
```

### ReactiveNode

A `ReactiveNode` reacts to changes in external data sources, such as files, databases, or streams.

```rust
use floxide_reactive::{reactive_node, ReactiveNode, ReactiveContext};

fn create_reactive_node() -> impl ReactiveNode<String, String> {
    reactive_node(
        |change: String| async move {
            Ok(format!("Reacted to change: {}", change))
        }
    )
}
```

### LongRunningNode

A `LongRunningNode` is designed for processes that can be suspended and resumed, with state persistence between executions.

```rust
use floxide_longrunning::{longrunning_node, LongRunningNode, LongRunningContext};

fn create_longrunning_node() -> impl LongRunningNode<String, String> {
    longrunning_node(
        |state: Option<String>, input: String| async move {
            let current_state = state.unwrap_or_default();
            let new_state = format!("{} + {}", current_state, input);
            Ok((new_state.clone(), new_state))
        }
    )
}
```

## Node Composition

Nodes can be composed to create more complex workflows. The most common way to compose nodes is to use the `Workflow` struct, which we'll cover in the [Workflows](workflows.md) section.

## Best Practices

When creating nodes, consider the following best practices:

1. **Keep nodes focused**: Each node should have a single responsibility.
2. **Use appropriate node types**: Choose the right node type for your use case.
3. **Handle errors gracefully**: Properly handle errors in each phase of the node lifecycle.
4. **Consider performance**: Be mindful of performance implications, especially for long-running or resource-intensive operations.
5. **Leverage type safety**: Use Rust's type system to ensure type safety between nodes.

## Next Steps

Now that you understand nodes, you can learn about:

- [Workflows](workflows.md): How to compose nodes into workflows
- [Actions](actions.md): How to control the flow of execution
- [Contexts](contexts.md): How to define and use contexts in your workflows
