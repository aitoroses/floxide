#![doc = r#"
# Distributed Workflow Example (`distributed_example.rs`)

This example demonstrates how to run a distributed workflow using the `floxide` framework, simulating distributed execution with in-memory queues and stores. The workflow is parallelized, with two worker tasks collaborating to process different branches of the workflow.

---

## Overview

- **Parallel workflow**: Two branches are processed in parallel by two worker tasks.
- **In-memory simulation**: Uses in-memory queue and checkpoint store to simulate distributed execution.
- **Shared context**: All nodes update a shared context (`Ctx`) containing a counter and logs.
- **Checkpointing**: State is checkpointed after each node for recovery and inspection.

---

## Components

### 1. Context (`Ctx`)
- Holds:
  - `local_counter`: a shared integer counter.
  - `logs`: a shared vector of log messages.
- Both are wrapped in a custom `ArcMutex<T>` for safe concurrent access.

### 2. In-Memory Work Queue (`InMemQueue`)
- Implements the `WorkQueue` trait.
- Stores work items in a `HashMap<String, VecDeque<W>>`, keyed by workflow run ID.
- Supports enqueueing and dequeueing work items.

### 3. In-Memory Checkpoint Store (`InMemStore`)
- Implements the `CheckpointStore` trait.
- Stores checkpoints in a `HashMap<String, Checkpoint<Ctx, W>>`, keyed by workflow run ID.

### 4. Workflow Nodes
- **InitialNode**: Starts the workflow, increments the counter by 1, logs the start.
- **SplitNode**: Splits into two parallel branches, increments the counter by 1, logs the split.
- **BranchA**: Increments the counter by 10, logs execution, returns `"branch_a_success"`.
- **BranchB**: Increments the counter by 15, logs execution, returns `"branch_b_success"` (or aborts if a failure flag is set).

### 5. Workflow Definition (`ParallelWorkflow`)
- Nodes: `initial`, `split`, `a` (BranchA), `b` (BranchB).
- Edges:
  - `initial` → `split`
  - `split` → `a`, `b` (parallel branches)
  - `a` and `b` are terminal nodes.

---

## Execution Flow

1. **Setup**
   - The workflow and context are created.
   - The workflow is seeded with the initial node using `start_distributed`.

2. **Worker Tasks**
   - Two worker tasks are spawned (simulating distributed workers).
   - Each worker repeatedly:
     - Dequeues a work item.
     - Loads the latest checkpoint.
     - Processes the node.
     - Saves the checkpoint.
     - Exits when it processes its terminal branch.

3. **Node Processing (Observed Output)**
   - **Worker 0** dequeues and processes `InitialNode`:
     - Counter: 0 → 1
     - Logs: `["InitialNode: starting workflow"]`
   - **Worker 1** dequeues and processes `SplitNode`:
     - Counter: 1 → 2
     - Logs: `["InitialNode: starting workflow", "SplitNode: spawning two branches"]`
   - **Worker 0** processes `BranchA`:
     - Counter: 2 → 12
     - Logs: `["InitialNode: starting workflow", "SplitNode: spawning two branches", "BranchA executed"]`
     - Returns `"branch_a_success"`
   - **Worker 1** processes `BranchB`:
     - Counter: 12 → 27
     - Logs: `["InitialNode: starting workflow", "SplitNode: spawning two branches", "BranchA executed", "BranchB executed"]`
     - Returns `"branch_b_success"`

4. **Completion**
   - Both workers print their results.
   - The final context is printed:
     - `local_counter: 27`
     - `logs: ["InitialNode: starting workflow", "SplitNode: spawning two branches", "BranchA executed", "BranchB executed"]`

---

## Output Summary

```text
Worker 0 processed branch of run run1 with result: "branch_a_success"
Worker 1 processed branch of run run1 with result: "branch_b_success"
Run run1 completed; final context: Ctx { local_counter: 27, logs: ["InitialNode: starting workflow", "SplitNode: spawning two branches", "BranchA executed", "BranchB executed"] }
```

---

## Key Points

- **Parallelism**: The split node enables two branches to be processed in parallel by different workers.
- **State Sharing**: The context is shared and updated by all nodes, with changes visible to subsequent nodes.
- **Checkpointing**: After each node, the state is checkpointed, allowing for recovery or inspection.
- **Logging**: Each node logs its activity, making the workflow traceable.
- **Extensibility**: The example can be extended to more complex workflows or real distributed systems by replacing the in-memory queue/store.

---

## Test

A test is included to verify the final state:

```rust
assert_eq!(*ctx.local_counter.0.lock().await, 27);
assert_eq!(
    *ctx.logs.0.lock().await,
    vec![
        "InitialNode: starting workflow",
        "SplitNode: spawning two branches",
        "BranchA executed",
        "BranchB executed"
    ]
);
```

---

Let us know if you want a diagram, code comments, or further breakdown of any part!
"#]
// examples/distributed_example.rs
// Demonstrates running a floxide workflow in-memory with distributed execution.
//
// This example shows how to:
//   - Define a parallel workflow with multiple branches
//   - Use in-memory queue and checkpoint store to simulate distributed execution
//   - Spawn multiple workers to process workflow steps concurrently
//   - Share and update context between nodes
//   - Log and checkpoint state after each node

use async_trait::async_trait;
use floxide::context::SharedState;
use floxide_core::*;
use floxide_macros::workflow;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, LazyLock},
};
use tokio::time::Duration;
use tracing::Instrument;
use floxide_core::distributed::InMemoryWorkQueue;

// Global flag to simulate failure in BranchB for demonstration purposes
static SHOULD_FAIL: LazyLock<Arc<tokio::sync::Mutex<bool>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(false)));

/// Shared workflow context, accessible and mutable by all nodes.
/// Contains a counter and a log of node activity.
#[derive(Clone, Debug)]
struct Ctx {
    local_counter: SharedState<i32>, // Shared counter, updated by each node
    logs: SharedState<Vec<String>>,  // Shared log, records node activity
}



/// In-memory checkpoint store for workflow state.
/// Each workflow run ID maps to its latest checkpoint.
#[derive(Clone)]
struct InMemStore<W: Clone + Send + 'static>(
    Arc<tokio::sync::Mutex<HashMap<String, Checkpoint<Ctx, W>>>>,
);

impl<W: Clone + Send + 'static> InMemStore<W> {
    /// Create a new, empty in-memory checkpoint store
    fn new() -> Self {
        InMemStore(Arc::new(tokio::sync::Mutex::new(HashMap::new())))
    }
}

#[async_trait]
impl<W: Clone + Send + Sync> CheckpointStore<Ctx, W> for InMemStore<W> {
    /// Save a checkpoint for a workflow run
    async fn save(
        &self,
        workflow_id: &str,
        checkpoint: &Checkpoint<Ctx, W>,
    ) -> Result<(), CheckpointError> {
        let mut map = self.0.lock().await;
        map.insert(workflow_id.to_string(), checkpoint.clone());
        Ok(())
    }
    /// Load the latest checkpoint for a workflow run
    async fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<Ctx, W>>, CheckpointError> {
        let map = self.0.lock().await;
        Ok(map.get(workflow_id).cloned())
    }
}

// === Parallel workflow example illustrating worker collaboration ===
// Define simple nodes: split into two parallel branches, then terminal nodes and an initial node

/// Initial node: starts the workflow, increments counter, logs start
#[derive(Clone, Debug)]
pub struct InitialNode;
#[async_trait]
impl Node<Ctx> for InitialNode {
    type Input = ();
    type Output = ();
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("InitialNode: starting workflow");
        let mut counter = ctx.local_counter.get().await;
        *counter += 1;
        let mut logs = ctx.logs.get().await;
        logs.push(format!("InitialNode: starting workflow"));
        Ok(Transition::Next(()))
    }
}

/// Split node: spawns two parallel branches, increments counter, logs split
#[derive(Clone, Debug)]
pub struct SplitNode;
#[async_trait]
impl Node<Ctx> for SplitNode {
    type Input = ();
    type Output = ();
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("SplitNode: spawning two branches");
        let mut counter = ctx.local_counter.get().await;
        *counter += 1;
        let mut logs = ctx.logs.get().await;
        logs.push(format!("SplitNode: spawning two branches"));
        Ok(Transition::Next(()))
    }
}

/// BranchA: increments counter by 10, logs execution, returns success
#[derive(Clone, Debug)]
pub struct BranchA;
#[async_trait]
impl Node<Ctx> for BranchA {
    type Input = ();
    type Output = &'static str;
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("BranchA executed");
        let mut counter = ctx.local_counter.get().await;
        *counter += 10;
        let mut logs = ctx.logs.get().await;
        logs.push(format!("BranchA executed"));
        Ok(Transition::Next("branch_a_success"))
    }
}

/// BranchB: increments counter by 15, logs execution, can simulate failure
#[derive(Clone, Debug)]
pub struct BranchB;
#[async_trait]
impl Node<Ctx> for BranchB {
    type Input = ();
    type Output = &'static str;
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("BranchB executed");
        let mut counter = ctx.local_counter.get().await;
        *counter += 15;
        let mut logs = ctx.logs.get().await;
        let should_fail = *SHOULD_FAIL.lock().await;
        if should_fail {
            logs.push(format!("BranchB failed"));
            return Ok(Transition::Abort(FloxideError::Generic("branch_b_failed".to_string())))
        }
        logs.push(format!("BranchB executed"));
        Ok(Transition::Next("branch_b_success"))
    }
}

// Define the parallel workflow using the workflow! macro
workflow! {
    pub struct ParallelWorkflow {
        initial: InitialNode,
        split: SplitNode,
        a: BranchA,
        b: BranchB,
    }
    start = initial;
    context = Ctx;
    edges {
        initial => { [ split ] };
        split => {[ a, b ]}; // Parallel branches
        a => {}; // Terminal
        b => {}; // Terminal
    }
}

/// Runs the distributed example: seeds the workflow, spawns workers, prints final context
async fn run_distributed_example() -> Result<Ctx, Box<dyn std::error::Error>> {
    // Create in-memory runtime (queue and checkpoint store)
    let store = InMemStore::<ParallelWorkflowWorkItem>::new();
    let queue = InMemoryWorkQueue::<ParallelWorkflowWorkItem>::new();

    // Build workflow and context
    let wf = ParallelWorkflow {
        initial: InitialNode,
        split: SplitNode,
        a: BranchA,
        b: BranchB,
    };
    let ctx = WorkflowCtx::new(Ctx {
        local_counter: SharedState::new(0),
        logs: SharedState::new(Vec::new()),
    });
    let run_id = "run1";
    // Seed the single run, enqueuing the split node
    wf.start_distributed(&ctx, (), &store, &queue, run_id)
        .await?;
    // Spawn two workers to process distributed steps and collect their JoinHandles
    let mut handles = Vec::new();
    for i in 0..2 {
        let wf = wf.clone();
        let store = store.clone();
        let queue = queue.clone();
        let worker_span = tracing::span!(tracing::Level::DEBUG, "worker_task", worker = i);
        let handle = tokio::spawn(
            async move {
                // Each worker processes steps until it sees its terminal branch event
                loop {
                    let step_result = wf.step_distributed(&store, &queue, i).await;
                    let mut should_fail = SHOULD_FAIL.lock().await;

                    match (*should_fail, step_result) {
                        (true, Err(e)) => {
                            *should_fail = false; // Reset the should_fail flag after simulating failure
                            println!("Worker {} failed to process branch of run {} with error: {:?}", i, run_id, e);
                        }
                        (false, Err(_)) => {
                            unreachable!();
                        }
                        (_, Ok(Some((run_id, res)))) => {
                            println!("Worker {} processed branch of run {} with result: {:?}", i, run_id, res);
                            break; // Worker is done with its branch
                        }
                        (_, Ok(None)) => {
                            // No work available, keep polling
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            .instrument(worker_span),
        );
        handles.push(handle);
    }

    // Wait for all workers to finish processing
    for handle in handles {
        handle.await.expect("worker task panicked");
    }

    // All work is done; print final context
    if let Some(cp) = store.load(run_id).await? {
        println!("Run {} completed; final context: {:?}", run_id, cp.context);
        Ok(cp.context)
    } else {
        Err(FloxideError::NotStarted.into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_distributed_example().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_distributed_example() {
        let ctx = run_distributed_example().await.unwrap();
        assert_eq!(*ctx.local_counter.get().await, 27);
        assert_eq!(
            *ctx.logs.get().await,
            vec![
                "InitialNode: starting workflow",
                "SplitNode: spawning two branches",
                "BranchA executed",
                "BranchB executed"
            ]
        );
    }
}
