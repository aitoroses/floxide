// examples/distributed_orchestrated_merge_example.rs
// Demonstrates distributed orchestration and worker pool with split/merge/hold using Floxide

use floxide::{
    checkpoint::InMemoryCheckpointStore,
    context::SharedState,
    distributed::{
        InMemoryErrorStore, InMemoryLivenessStore, InMemoryMetricsStore, InMemoryRunInfoStore,
        InMemoryWorkItemStateStore, InMemoryWorkQueue, OrchestratorBuilder, RunStatus,
        WorkerBuilder, WorkerPool,
    },
};
use floxide_core::*;
use floxide_macros::{node, workflow};
use serde::{Deserialize, Serialize};
use tokio::time::error::Elapsed;

// --- Context ---
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct MergeContext {
    values: SharedState<Vec<i32>>,
    expected: usize,
    random_fail_chance: f64,
}

// --- MergeNode using node! macro ---
// Fails with some random chance
node! {
    pub struct MergeNode {};
    context = MergeContext;
    input = i32;
    output = Vec<i32>;
    |ctx, input| {
        if rand::random::<f64>() < ctx.random_fail_chance {
            return Ok(Transition::Abort(FloxideError::Generic("random failure".to_string())));
        }
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
// Fails with some random chance
node! {
    pub struct TerminalNode {};
    context = MergeContext;
    input = Vec<i32>;
    output = Vec<i32>;
    |ctx, input| {
        if rand::random::<f64>() < ctx.random_fail_chance {
            return Ok(Transition::Abort(FloxideError::Generic("random failure".to_string())));
        }
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
        expected: 10,
        random_fail_chance: 0.7,
    };
    let wf_ctx = WorkflowCtx::new(ctx);

    // Build workflow
    let wf = MergeWorkflow {
        split: SplitNode::new(|n| (0..10).map(|x| x * n).collect()),
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
    let work_item_state_store: InMemoryWorkItemStateStore<MergeWorkflowWorkItem> =
        InMemoryWorkItemStateStore::default();

    // Orchestrator with in-memory defaults
    let orchestrator = OrchestratorBuilder::new()
        .workflow(wf.clone())
        .queue(queue.clone())
        .checkpoint_store(checkpoint_store.clone())
        .run_info_store(run_info_store.clone())
        .metrics_store(metrics_store.clone())
        .error_store(error_store.clone())
        .liveness_store(liveness_store.clone())
        .work_item_state_store(work_item_state_store.clone())
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
        .work_item_state_store(work_item_state_store)
        .build()
        .unwrap();

    // Worker pool
    let mut pool = WorkerPool::new(worker, 3);
    pool.start();

    // Wait for completion
    let status = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        orchestrator.wait_for_completion(&run_id, std::time::Duration::from_millis(100)),
    )
    .await;

    let print_stats = || async {
        let run_info = orchestrator.list_runs(None).await?;
        println!("Run info: {:#?}", run_info);

        let checkpoint = orchestrator.checkpoint(&run_id).await?;
        println!("Checkpoint: {:#?}", checkpoint);

        let metrics = orchestrator.metrics(&run_id).await?;
        println!("Metrics: {:#?}", metrics);

        let errors = orchestrator.errors(&run_id).await?;
        println!("Errors: {:#?}", errors);

        let liveness = orchestrator.liveness().await?;
        println!("Liveness: {:#?}", liveness);

        let pending_work = orchestrator.pending_work(&run_id).await.unwrap_or_default();
        println!("Pending work: {:#?}", pending_work);

        let work_items = orchestrator.list_work_items(&run_id).await?;
        println!("Work items: {:#?}", work_items);

        Ok::<(), Box<dyn std::error::Error>>(())
    };

    println!("Status: {:#?}", status);

    let mut final_status = status;
    loop {
        match final_status {
            Ok(Ok(RunStatus::Completed)) => {
                print_stats().await?;
                break Ok(RunStatus::Completed);
            }
            Ok(Ok(RunStatus::Failed)) | Err(Elapsed { .. }) => {
                // timeout or other error
                print_stats().await?;
                println!("Resuming run");
                orchestrator.resume(&run_id).await?;
                final_status = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    orchestrator
                        .wait_for_completion(&run_id, std::time::Duration::from_millis(100)),
                )
                .await;
            }
            Ok(Ok(RunStatus::Cancelled)) => {
                print_stats().await?;
                break Err(FloxideError::Generic("run cancelled".to_string()).into());
            }
            Ok(Ok(RunStatus::Paused)) => {
                print_stats().await?;
                break Err(FloxideError::Generic("run paused".to_string()).into());
            }
            Ok(Ok(RunStatus::Running)) => {
                print_stats().await?;
                break Err(FloxideError::Generic("run running".to_string()).into());
            }
            Ok(Err(e)) => {
                print_stats().await?;
                break Err(e.into());
            }
        }
    }
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
