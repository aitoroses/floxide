//! Example demonstrating in-memory checkpointing and resume
use floxide::checkpoint::InMemoryCheckpointStore;
use floxide_core::{FloxideError, Node, Transition, Workflow, WorkflowCtx};
use floxide_macros::{node, workflow};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::sync::{Arc, LazyLock, Mutex};

// Global mutable failed flag
static FAILED: LazyLock<Arc<Mutex<bool>>> = LazyLock::new(|| Arc::new(Mutex::new(false)));

#[derive(Clone, Debug, Default)]
struct Ctx {
    failed: Arc<Mutex<bool>>,
}

impl Serialize for Ctx {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // get the failed value
        let failed = self.failed.lock().unwrap();
        // serialize the failed value
        serializer.serialize_bool(*failed)
    }
}

impl<'de> Deserialize<'de> for Ctx {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // deserialize the failed value
        let failed = bool::deserialize(deserializer)?;
        // create a new Ctx
        Ok(Ctx {
            failed: Arc::new(Mutex::new(failed)),
        })
    }
}

impl Display for Ctx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ctx {{ failed: {} }}", self.failed.lock().unwrap())
    }
}

// Define the action enum for counter progress
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CounterAction {
    Continue(i32),
    Done(i32),
}

// Node that increments until reaching max, then signals Done
node! {
    pub struct CounterNode { max: i32 };
    context = Ctx;
    input = i32;
    output = CounterAction;
    |ctx, x| {
        println!("{}: counter at {} (max={})", ctx, x, self.max);
        let next = x + 1;
        let mut global_failed = FAILED.lock().unwrap();
        let mut failed = ctx.failed.lock().unwrap();
        if x == 2 && !*global_failed {
            *failed = true;
            *global_failed = true;
            return Err(FloxideError::Generic("test error".to_string()))
        }
        if next < self.max {
            Ok(Transition::Next(CounterAction::Continue(next)))
        } else {
            Ok(Transition::Next(CounterAction::Done(next)))
        }
    }
}

// Terminal node that outputs the final value
node! {
    pub struct TerminalNode {};
    context = Ctx;
    input = i32;
    output = i32;
    |ctx, x| {
        println!("{}: terminating at {}", ctx, x);
        Ok(Transition::Next(x))
    }
}

// Composite workflow chaining the above nodes with checkpoint support
workflow! {
    pub struct CounterWorkflow {
        counter: CounterNode,
        terminal: TerminalNode,
    }
    context = Ctx;
    start = counter;
    edges {
        counter => {
            CounterAction::Continue(n) => [ counter ];
            CounterAction::Done(n) => [ terminal ];
        };
        terminal => {};
    }
}

/// Runs the checkpoint workflow example and returns Ok(()) if all scenarios succeed
pub async fn run_checkpoint_example() -> Result<(), Box<dyn std::error::Error>> {
    // Each job has its own context (e.g., job name)
    let ctx = WorkflowCtx::new(Ctx {
        failed: Arc::new(Mutex::new(false)),
    });
    // Build the workflow with its nodes
    let wf = CounterWorkflow {
        counter: CounterNode { max: 5 },
        terminal: TerminalNode {},
    };
    let store = InMemoryCheckpointStore::default();

    // Run with checkpointing enabled
    let result = wf.run_with_checkpoint(&ctx, 0, &store, "job1").await;

    // If the first run fails, resume from the last checkpoint
    let result = if let Err(e) = result {
        println!("First run failed: {}", e);
        println!("Resuming run");
        wf.resume(&store, "job1").await?
    } else {
        unreachable!("First run should not fail")
    };
    println!("First run result = {}", result);

    // Simulate process restart: start over by clearing the checkpoint store
    println!("Restarting workflow from scratch");
    let fresh_store = InMemoryCheckpointStore::default();
    // This run_with_checkpoint sees no prior checkpoint and begins anew
    let restarted = wf
        .run_with_checkpoint(&ctx, 0, &fresh_store, "job1")
        .await?;
    println!("Restarted run result = {}", restarted);

    // Simulate a full restart: reset both checkpoint store and in-memory context
    println!("Restarting workflow from scratch (fresh context)");
    // New context to clear the failed flag
    let fresh_ctx = WorkflowCtx::new(Ctx {
        failed: Arc::new(Mutex::new(false)),
    });
    let fresh_store = InMemoryCheckpointStore::new();
    // This run_with_checkpoint sees no prior checkpoint and begins anew
    let restarted = wf
        .run_with_checkpoint(&fresh_ctx, 0, &fresh_store, "job1")
        .await?;
    println!("Restarted run result (fresh context) = {}", restarted);

    // Simulate already completed workflow: use resume to detect no pending work, then run again
    println!("Simulating running a completed workflow");
    let already_completed = wf
        .run_with_checkpoint(&fresh_ctx, 0, &fresh_store, "job1")
        .await;

    assert!(already_completed.is_ok());
    println!("Already completed run result = {:?}", already_completed);

    // Resume again to detect already completed
    let resumed = wf.resume(&fresh_store, "job1").await;
    
    assert!(matches!(resumed, Err(FloxideError::AlreadyCompleted)), "Expected AlreadyCompleted error, got: {:?}", resumed);
    println!("Resumed run result = {:?}", resumed);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_checkpoint_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_checkpoint_example() {
        run_checkpoint_example()
            .await
            .expect("checkpoint workflow should run");
    }
}
