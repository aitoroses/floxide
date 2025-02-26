# Actions

Actions in Floxide determine the flow of execution in a workflow. After a node completes its execution, it returns an action that indicates what should happen next. This page explains the different types of actions and how to use them to control workflow execution.

## The Action Concept

Actions are the mechanism by which nodes communicate their execution results and control the flow of a workflow. When a node completes its execution, it returns an action that the workflow uses to determine the next step.

In Floxide, actions are typically represented as enum variants, allowing for a clear and type-safe way to express different execution paths.

## Default Actions

Floxide provides a `DefaultAction` enum that covers the most common workflow control patterns:

```rust
pub enum DefaultAction {
    Next,
    Stop,
}
```

- `Next`: Continue to the next node in the workflow
- `Stop`: Stop the workflow execution

Here's how to use the default actions:

```rust
use floxide_core::{lifecycle_node, LifecycleNode, Workflow, DefaultAction};

fn create_processor_node() -> impl LifecycleNode<MessageContext, DefaultAction> {
    lifecycle_node(
        Some("processor"),
        |ctx: &mut MessageContext| async move {
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            Ok(format!("Processed: {}", input))
        },
        |_prep, exec_result, ctx: &mut MessageContext| async move {
            ctx.result = Some(exec_result);

            // Continue to the next node
            Ok(DefaultAction::Next)

            // Or stop the workflow
            // Ok(DefaultAction::Stop)
        },
    )
}
```

## Custom Actions

For more complex workflows, you can define custom actions using enums:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderAction {
    Approve,
    Reject,
    Review,
    Error,
}

fn create_validation_node() -> impl LifecycleNode<OrderContext, OrderAction> {
    lifecycle_node(
        Some("validator"),
        |ctx: &mut OrderContext| async move {
            Ok(ctx.order.clone())
        },
        |order: Order| async move {
            // Validate the order
            let validation_result = validate_order(&order);
            Ok(validation_result)
        },
        |_prep, validation_result, ctx: &mut OrderContext| async move {
            match validation_result {
                ValidationResult::Valid => Ok(OrderAction::Approve),
                ValidationResult::Invalid => Ok(OrderAction::Reject),
                ValidationResult::NeedsReview => Ok(OrderAction::Review),
                ValidationResult::Error => Ok(OrderAction::Error),
            }
        },
    )
}
```

## Using Actions in Workflows

Actions are used to control the flow of execution in a workflow. You can use the `on` method to specify which node to execute for each action:

```rust
let validation_node = Arc::new(create_validation_node());
let approval_node = Arc::new(create_approval_node());
let rejection_node = Arc::new(create_rejection_node());
let review_node = Arc::new(create_review_node());
let error_node = Arc::new(create_error_node());

let mut workflow = Workflow::new(validation_node)
    .on(OrderAction::Approve, approval_node)
    .on(OrderAction::Reject, rejection_node)
    .on(OrderAction::Review, review_node)
    .on(OrderAction::Error, error_node);
```

This creates a workflow that executes different nodes based on the action returned by the validation node.

## Action Patterns

### Linear Flow

The simplest action pattern is a linear flow, where each node returns `DefaultAction::Next` to continue to the next node:

```rust
let node1 = Arc::new(create_node1());
let node2 = Arc::new(create_node2());
let node3 = Arc::new(create_node3());

let mut workflow = Workflow::new(node1)
    .then(node2)
    .then(node3);
```

### Conditional Branching

Actions enable conditional branching, where the execution path depends on the result of a node:

```rust
enum PaymentAction {
    Success,
    Failure,
    Pending,
}

let payment_node = Arc::new(create_payment_node());
let success_node = Arc::new(create_success_node());
let failure_node = Arc::new(create_failure_node());
let pending_node = Arc::new(create_pending_node());

let mut workflow = Workflow::new(payment_node)
    .on(PaymentAction::Success, success_node)
    .on(PaymentAction::Failure, failure_node)
    .on(PaymentAction::Pending, pending_node);
```

### Error Handling

Actions can be used to implement error handling patterns:

```rust
enum ProcessAction {
    Success,
    Error,
    Retry,
}

let process_node = Arc::new(create_process_node());
let success_node = Arc::new(create_success_node());
let error_node = Arc::new(create_error_node());
let retry_node = Arc::new(create_retry_node());

let mut workflow = Workflow::new(process_node)
    .on(ProcessAction::Success, success_node)
    .on(ProcessAction::Error, error_node)
    .on(ProcessAction::Retry, retry_node);
```

### State Machine

Actions can be used to implement a state machine, where each node represents a state and actions represent transitions:

```rust
enum OrderState {
    Created,
    Validated,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

let created_node = Arc::new(create_created_node());
let validated_node = Arc::new(create_validated_node());
let paid_node = Arc::new(create_paid_node());
let shipped_node = Arc::new(create_shipped_node());
let delivered_node = Arc::new(create_delivered_node());
let cancelled_node = Arc::new(create_cancelled_node());

let mut workflow = Workflow::new(created_node)
    .on(OrderState::Validated, validated_node)
    .on(OrderState::Paid, paid_node)
    .on(OrderState::Shipped, shipped_node)
    .on(OrderState::Delivered, delivered_node)
    .on(OrderState::Cancelled, cancelled_node);
```

## Best Practices

When using actions, consider the following best practices:

1. **Keep actions focused**: Each action should have a clear meaning and purpose.
2. **Use descriptive names**: Choose action names that clearly communicate their intent.
3. **Consider all possible paths**: Ensure that all possible actions are handled in your workflow.
4. **Leverage type safety**: Use Rust's type system to ensure that actions are used correctly.
5. **Document actions**: Clearly document what each action means and when it should be used.

## Next Steps

Now that you understand actions, you can learn about:

- [Contexts](contexts.md): How to define and use contexts in your workflows
- [Nodes](nodes.md): Learn more about the different types of nodes
- [Workflows](workflows.md): How to compose nodes into workflows
