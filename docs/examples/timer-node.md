# Timer Node Example

This example demonstrates how to use timer nodes in the Flowrs framework for scheduling and periodic execution.

## Overview

Timer nodes allow you to:
- Execute workflow nodes at specific times
- Run periodic tasks
- Implement delays and timeouts
- Handle scheduled workflows

## Implementation

First, let's create a simple timer-based workflow:

```rust
use flowrs_core::{DefaultAction, FlowrsError, NodeId};
use flowrs_timer::{Schedule, SimpleTimer, TimerNode, TimerWorkflow};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::sync::Arc;

// Define our context
#[derive(Debug, Clone)]
struct BackupContext {
    last_backup: Option<DateTime<Utc>>,
    backup_count: usize,
    status: String,
}

// Create different types of timer nodes
fn create_backup_workflow() -> TimerWorkflow<BackupContext, DefaultAction> {
    // Daily backup at 2 AM
    let daily_backup = Arc::new(SimpleTimer::with_id(
        "daily_backup",
        Schedule::Cron("0 2 * * *".to_string()),
        |ctx: &mut BackupContext| {
            ctx.backup_count += 1;
            ctx.last_backup = Some(Utc::now());
            ctx.status = format!("Daily backup completed ({})", ctx.backup_count);
            Ok(DefaultAction::Next)
        },
    ));

    // Hourly health check
    let health_check = Arc::new(SimpleTimer::with_id(
        "health_check",
        Schedule::Periodic(ChronoDuration::hours(1)),
        |ctx: &mut BackupContext| {
            ctx.status = "Health check passed".to_string();
            Ok(DefaultAction::Next)
        },
    ));

    // One-time initialization
    let init = Arc::new(SimpleTimer::with_id(
        "init",
        Schedule::Once(Utc::now()),
        |ctx: &mut BackupContext| {
            ctx.status = "Backup system initialized".to_string();
            Ok(DefaultAction::Next)
        },
    ));

    // Create the workflow
    let mut workflow = TimerWorkflow::new(
        init,
        DefaultAction::complete(), // Action to terminate the workflow
    );

    // Add nodes and set up routes
    workflow.add_node(daily_backup.clone());
    workflow.add_node(health_check.clone());

    // Set up routing
    workflow.set_route(&init.id(), DefaultAction::Next, &daily_backup.id());
    workflow.set_route(&daily_backup.id(), DefaultAction::Next, &health_check.id());
    workflow.set_route(&health_check.id(), DefaultAction::Next, &daily_backup.id());

    workflow
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    let mut ctx = BackupContext {
        last_backup: None,
        backup_count: 0,
        status: String::new(),
    };

    let workflow = create_backup_workflow();
    
    // Execute the workflow with proper error handling
    match workflow.execute(&mut ctx).await {
        Ok(()) => {
            println!("Workflow completed successfully");
            println!("Final status: {}", ctx.status);
            println!("Total backups: {}", ctx.backup_count);
        }
        Err(e) => {
            eprintln!("Workflow error: {}", e);
            // Implement your error recovery strategy here
        }
    }

    Ok(())
}
```

## Running the Example

To run the timer nodes, execute the `main` function.

## Advanced Usage

### Combining with Other Nodes

Timer nodes can be combined with other node types:

```rust
// Create a workflow that processes data periodically
let mut workflow = TimerWorkflow::new(create_periodic_node())
    .then(create_processor_node())
    .then(create_cleanup_node());
```

### Error Handling

Timer nodes include built-in error handling:

```rust
fn create_robust_timer() -> impl TimerNode<BackupContext, DefaultAction> {
    SimpleTimer::with_id(
        "robust_timer",
        Schedule::Periodic(ChronoDuration::hours(1)),
        |ctx: &mut BackupContext| {
            if let Err(e) = process_data().await {
                ctx.status = format!("Error: {}", e);
                return Ok(DefaultAction::Stop);
            }
            Ok(DefaultAction::Next)
        },
    )
    .with_retry(3) // Retry up to 3 times on failure
    .with_backoff(ChronoDuration::seconds(1)) // Wait 1 second between retries
}
```

### Cancellation

Timer nodes support graceful cancellation:

```rust
fn create_cancellable_timer() -> impl TimerNode<BackupContext, DefaultAction> {
    SimpleTimer::with_id(
        "cancellable_timer",
        Schedule::Periodic(ChronoDuration::hours(1)),
        |ctx: &mut BackupContext| {
            if ctx.backup_count >= 10 {
                return Ok(DefaultAction::Stop);
            }
            Ok(DefaultAction::Next)
        },
    )
    .with_cancellation()
}
```

## Best Practices

1. **Resource Management**
   - Always implement proper cleanup for timer resources
   - Use appropriate timeout values
   - Consider system resource limitations

2. **Error Handling**
   - Implement retry logic for transient failures
   - Log timer execution errors
   - Handle edge cases (e.g., system time changes)

3. **Testing**
   - Use shorter durations in tests
   - Mock time-dependent operations
   - Test cancellation scenarios

## See Also

- [Timer Node Implementation ADR](../adrs/0021-timer-node-implementation.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
- [Event-Driven Architecture](../guides/event_driven_architecture.md)
