// examples/distributed_orchestrated_merge_example.rs
// Demonstrates distributed orchestration and worker pool with split/merge/hold using Floxide

use floxide::{
    checkpoint::InMemoryCheckpointStore,
    context::SharedState,
    distributed::{
        InMemoryErrorStore, InMemoryLivenessStore, InMemoryMetricsStore, InMemoryRunInfoStore, InMemoryWorkQueue, OrchestratorBuilder, RunStatus, WorkerBuilder, WorkerPool
    },
};
use floxide_core::*;
use floxide_macros::{node, workflow};
use serde::{Deserialize, Serialize};

// --- Context ---
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct MergeContext {
    values: SharedState<Vec<i32>>,
    expected: usize,
}

// --- MergeNode using node! macro ---
node! {
    pub struct MergeNode {};
    context = MergeContext;
    input = i32;
    output = Vec<i32>;
    |ctx, input| {
        let mut vals = ctx.values.get().await;
        vals.push(input);
        if vals.len() < ctx.expected {
            Ok(Transition::Hold)
        } else {
            let merged = vals.clone();
            Ok(Transition::Next(merged))
        }
    }

}

// --- TerminalNode using node! macro ---
node! {
    pub struct TerminalNode {};
    context = MergeContext;
    input = Vec<i32>;
    output = Vec<i32>;
    |_ctx, input| {
        println!("Merged values: {:?}", input);
        Ok(Transition::Next(input))
    }
}

// --- Workflow ---
workflow! {
    pub struct MergeWorkflow {
        split: SplitNode<i32, i32, fn(i32) -> Vec<i32>>,
        merge: MergeNode,
        terminal: TerminalNode,
    }
    context = MergeContext;
    start = split;
    edges {
        split => { [merge] };
        merge => { [terminal] };
        terminal => { [] };
    }
}

async fn run_distributed_orchestrated_merge() -> Result<RunStatus, Box<dyn std::error::Error>> {
    // --- Distributed setup ---
    let ctx = MergeContext {
        values: SharedState::new(Vec::new()),
        expected: 3,
    };
    let wf_ctx = WorkflowCtx::new(ctx);

    // Build workflow
    let wf = MergeWorkflow {
        split: SplitNode::new(|n| vec![n - 1, n, n + 1]),
        merge: MergeNode {},
        terminal: TerminalNode {},
    };

    let queue: InMemoryWorkQueue<MergeWorkflowWorkItem> = InMemoryWorkQueue::default();
    let checkpoint_store: InMemoryCheckpointStore<MergeContext, MergeWorkflowWorkItem> =
        InMemoryCheckpointStore::default();

    let run_info_store: InMemoryRunInfoStore = InMemoryRunInfoStore::default();
    let metrics_store: InMemoryMetricsStore = InMemoryMetricsStore::default();
    let error_store: InMemoryErrorStore = InMemoryErrorStore::default();
    let liveness_store: InMemoryLivenessStore = InMemoryLivenessStore::default();

    // Orchestrator with in-memory defaults
    let orchestrator = OrchestratorBuilder::new()
        .workflow(wf.clone())
        .queue(queue.clone())
        .checkpoint_store(checkpoint_store.clone())
        .run_info_store(run_info_store.clone())
        .metrics_store(metrics_store.clone())
        .error_store(error_store.clone())
        .liveness_store(liveness_store.clone())
        .build()
        .unwrap();

    // Start the run
    let run_id = orchestrator.start_run(&wf_ctx, 10).await?;

    // Worker with in-memory defaults
    let worker = WorkerBuilder::new()
        .workflow(wf)
        .queue(queue)
        .checkpoint_store(checkpoint_store)
        .run_info_store(run_info_store)
        .metrics_store(metrics_store)
        .error_store(error_store)
        .liveness_store(liveness_store)
        .build()
        .unwrap();

    // Worker pool
    let mut pool = WorkerPool::new(worker, 3);
    pool.start();

    // Wait for completion
    let status = orchestrator
        .wait_for_completion(&run_id, std::time::Duration::from_millis(100))
        .await?;

    // Print final context
    println!("Distributed merge workflow completed!");

    Ok(status)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_distributed_orchestrated_merge().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_distributed_orchestrated_merge() {
        let status = run_distributed_orchestrated_merge()
            .await
            .expect("distributed orchestrated merge example failed");
        assert_eq!(status, RunStatus::Completed);
    }
}
