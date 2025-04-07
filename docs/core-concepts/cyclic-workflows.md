# Cyclic Workflows

Floxide supports cyclic workflows as an optional feature. This document explains how to use cyclic workflows, when they are appropriate, and how to avoid common pitfalls.

## Overview

By default, Floxide prevents cycles in workflows to avoid infinite loops. When a node is visited more than once during workflow execution, a `WorkflowCycleDetected` error is returned.

However, there are legitimate use cases for cyclic workflows, such as:

- Retry loops
- Polling mechanisms
- State machines with loops
- Iterative processing

## Enabling Cyclic Workflows

To enable cycles in your workflow, use the `allow_cycles` method:

```rust
let mut workflow = Workflow::new(start_node)
    .add_node(process_node)
    .add_node(check_node)
    .set_default_route(&start_id, &process_id)
    .connect(&process_id, DefaultAction::Next, &check_id)
    .connect(&check_id, DefaultAction::Next, &process_id) // Create a cycle
    .allow_cycles(true); // Enable cycles
```

## Preventing Infinite Loops

When cycles are allowed, you should implement one of these safeguards to prevent infinite loops:

### 1. Set a Cycle Limit

You can set a maximum number of times a node can be visited:

```rust
workflow.set_cycle_limit(10); // Limit each node to 10 visits
```

If a node is visited more than the specified limit, a `WorkflowCycleDetected` error will be returned.

### 2. Implement a Conditional Exit

A better approach is to implement a conditional exit in your node logic:

```rust
let loop_node = closure::node(|mut ctx: MyContext| async move {
    ctx.iteration_count += 1;

    // Process data
    // ...

    // Check exit condition
    if ctx.is_complete() || ctx.iteration_count >= 10 {
        // Exit the loop
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Complete)))
    } else {
        // Continue the loop
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Next)))
    }
});
```

## Example: Retry Loop

Here's a complete example of a workflow with a retry loop:

```rust
use floxide_core::{closure::node, Workflow, DefaultAction, NodeOutcome};

// Define context
#[derive(Clone)]
struct RetryContext {
    data: String,
    attempts: usize,
    max_attempts: usize,
    success: bool,
}

// Create nodes
let start_node = node(|mut ctx: RetryContext| async move {
    ctx.attempts = 0;
    Ok((ctx, NodeOutcome::Success(())))
});

let process_node = node(|mut ctx: RetryContext| async move {
    ctx.attempts += 1;

    // Simulate processing that might fail
    if ctx.attempts < 3 {
        // Simulate failure
        ctx.success = false;
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Error)))
    } else {
        // Simulate success
        ctx.success = true;
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Next)))
    }
});

let check_node = node(|mut ctx: RetryContext| async move {
    if ctx.success {
        // Processing succeeded, continue to end
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Next)))
    } else if ctx.attempts < ctx.max_attempts {
        // Retry
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Error)))
    } else {
        // Too many attempts, give up
        Ok((ctx, NodeOutcome::RouteToAction(DefaultAction::Next)))
    }
});

let end_node = node(|ctx: RetryContext| async move {
    Ok((ctx, NodeOutcome::Success(())))
});

// Build workflow
let mut workflow = Workflow::new(start_node);
let start_id = workflow.start_node.clone();
let process_id = process_node.id();
let check_id = check_node.id();
let end_id = end_node.id();

workflow
    .add_node(process_node)
    .add_node(check_node)
    .add_node(end_node)
    .set_default_route(&start_id, &process_id)
    .set_default_route(&process_id, &check_id)
    .set_default_route(&check_id, &end_id)
    .connect(&check_id, DefaultAction::Error, &process_id) // Create a cycle for retry
    .allow_cycles(true); // Enable cycles
```

## Best Practices

1. **Always have an exit condition**: Every cyclic workflow should have a clear exit condition to prevent infinite loops.

2. **Use cycle limits as a safety net**: Set a reasonable cycle limit as a fallback safety mechanism.

3. **Track iteration counts**: Include an iteration counter in your context to help with debugging and to implement exit conditions.

4. **Consider performance implications**: Be mindful of the performance impact of loops, especially for long-running workflows.

5. **Log cycle iterations**: Add logging to help debug cyclic workflows.

## Troubleshooting

If you encounter a `WorkflowCycleDetected` error:

1. Check if cycles are enabled with `allow_cycles(true)`
2. Verify that your cycle limit isn't too low
3. Ensure your exit conditions are working correctly
4. Add logging to track the state during each iteration
