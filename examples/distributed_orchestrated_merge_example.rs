// examples/distributed_orchestrated_merge_example.rs
// Demonstrates distributed orchestration and worker pool with split/merge/hold using Floxide

use floxide::distributed::event_log::EventLog;
use floxide::*;
use floxide::{
    distributed::{
        InMemoryContextStore, InMemoryErrorStore, InMemoryLivenessStore, InMemoryMetricsStore,
        InMemoryRunInfoStore, InMemoryWorkItemStateStore, InMemoryWorkQueue, OrchestratorBuilder,
        RunInfo, RunStatus, WorkerBuilder, WorkerPool,
    },
    merge::Fixed,
};
use floxide_macros::{node, workflow, Merge};
use serde::{Deserialize, Serialize};
use tokio::time::error::Elapsed;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeEvent {
    ValueReceived(i32),
    Merged(Vec<i32>),
    Log(String),
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct MergeState {
    pub values: Vec<i32>,
    pub logs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, Merge)]
pub struct MergeContext {
    pub event_log: EventLog<MergeEvent>,
    pub expected: Fixed<usize>,
    pub random_fail_chance: Fixed<f64>,
}

impl MergeContext {
    pub fn replay(&self) -> MergeState {
        self.event_log
            .apply_all_default(|event, state: &mut MergeState| match event {
                MergeEvent::ValueReceived(v) => state.values.push(*v),
                MergeEvent::Merged(vals) => state.values = vals.clone(),
                MergeEvent::Log(msg) => state.logs.push(msg.clone()),
            })
    }
}

// --- MergeNode using node! macro ---
node! {
    pub struct MergeNode {};
    context = MergeContext;
    input = i32;
    output = Vec<i32>;
    |ctx, input| {
        if rand::random::<f64>() < *ctx.random_fail_chance {
            ctx.event_log.append(MergeEvent::Log("random failure in MergeNode".to_string()));
            return Ok(Transition::Abort(FloxideError::Generic("random failure".to_string())));
        }
        ctx.event_log.append(MergeEvent::ValueReceived(input));
        let state = ctx.replay();
        if state.values.len() < *ctx.expected {
            Ok(Transition::Hold)
        } else {
            ctx.event_log.append(MergeEvent::Merged(state.values.clone()));
            Ok(Transition::Next(state.values))
        }
    }
}

// --- TerminalNode using node! macro ---
node! {
    pub struct TerminalNode {};
    context = MergeContext;
    input = Vec<i32>;
    output = Vec<i32>;
    |ctx, input| {
        if rand::random::<f64>() < *ctx.random_fail_chance {
            ctx.event_log.append(MergeEvent::Log("random failure in TerminalNode".to_string()));
            return Ok(Transition::Abort(FloxideError::Generic("random failure".to_string())));
        }
        ctx.event_log.append(MergeEvent::Log(format!("Merged values: {:?}", input)));
        println!("Merged values: {:?}", input);
        let mut output = input.clone();
        output.sort();
        Ok(Transition::Next(output))
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
        terminal => {};
    }
}

async fn run_distributed_orchestrated_merge() -> Result<RunInfo, Box<dyn std::error::Error>> {
    // --- Distributed setup ---
    let ctx = MergeContext {
        event_log: EventLog::new(),
        expected: Fixed::new(10),
        random_fail_chance: Fixed::new(0.3),
    };
    let wf_ctx = WorkflowCtx::new(ctx);

    // Build workflow
    let wf = MergeWorkflow {
        split: SplitNode::new(|n| (0..10).map(|x| x * n).collect()),
        merge: MergeNode {},
        terminal: TerminalNode {},
    };

    let queue: InMemoryWorkQueue<MergeWorkflowWorkItem> = InMemoryWorkQueue::default();
    let context_store: InMemoryContextStore<MergeContext> = InMemoryContextStore::default();

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
        .context_store(context_store.clone())
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
        .context_store(context_store)
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

        let context = orchestrator.context(&run_id).await?;
        println!("Context: {:#?}", context);
        let state = context.replay();
        println!("Replay state: {:#?}", state);

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
            Ok(Ok(ref info)) if info.status == RunStatus::Completed => {
                print_stats().await?;
                break Ok(info.clone());
            }
            Ok(Ok(ref info))
                if info.status == RunStatus::Failed
                    || matches!(final_status, Err(Elapsed { .. })) =>
            {
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
            Ok(Ok(ref info)) if info.status == RunStatus::Cancelled => {
                print_stats().await?;
                break Err(FloxideError::Generic("run cancelled".to_string()).into());
            }
            Ok(Ok(ref info)) if info.status == RunStatus::Paused => {
                print_stats().await?;
                break Err(FloxideError::Generic("run paused".to_string()).into());
            }
            Ok(Ok(ref info)) if info.status == RunStatus::Running => {
                print_stats().await?;
                break Err(FloxideError::Generic("run running".to_string()).into());
            }
            Ok(Err(ref e)) => {
                print_stats().await?;
                break Err(e.clone().into());
            }
            Err(Elapsed { .. }) => {
                print_stats().await?;
                println!("Timeout reached, resuming run");
                orchestrator.resume(&run_id).await?;
                final_status = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    orchestrator
                        .wait_for_completion(&run_id, std::time::Duration::from_millis(100)),
                )
                .await;
            }
            _ => {
                print_stats().await?;
                break Err(FloxideError::Generic("unexpected status".to_string()).into());
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
    use serde_json::json;

    use super::*;
    #[tokio::test]
    async fn test_distributed_orchestrated_merge() {
        let info = run_distributed_orchestrated_merge()
            .await
            .expect("distributed orchestrated merge example failed");
        assert_eq!(info.status, RunStatus::Completed);
        let expected = json!(vec![0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
        assert_eq!(info.output, Some(expected));
    }
}
