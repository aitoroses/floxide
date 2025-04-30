// examples/redis_distributed_example.rs
// Redis-backed version of distributed_orchestrated_merge_example.rs

use floxide_core::distributed::event_log::EventLog;
use floxide_core::*;
use floxide_core::{
    distributed::{OrchestratorBuilder, RunInfo, RunStatus, WorkerBuilder, WorkerPool},
    merge::Fixed,
};
use floxide_macros::{node, workflow, Merge};
use floxide_redis::{
    RedisClient, RedisConfig, RedisContextStore, RedisErrorStore, RedisLivenessStore,
    RedisMetricsStore, RedisRunInfoStore, RedisWorkItemStateStore, RedisWorkQueue,
};
use serde::{Deserialize, Serialize};
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
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
        terminal => {};
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Start a Redis container using testcontainers async API
    let container = GenericImage::new("redis", "7.2.4")
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .with_network("bridge")
        .with_env_var("DEBUG", "1")
        .start()
        .await
        .expect("Failed to start Redis");
    let host_port = container.get_host_port_ipv4(6379).await.unwrap();
    let redis_url = format!("redis://127.0.0.1:{}/", host_port);
    tracing::info!("Started Redis testcontainer at {}", redis_url);

    // Patch the run_distributed_orchestrated_merge to accept a redis_url
    run_distributed_orchestrated_merge_with_url(&redis_url).await?;

    // Stop the Redis container
    container.rm().await.unwrap();
    Ok(())
}

async fn run_distributed_orchestrated_merge_with_url(
    redis_url: &str,
) -> Result<RunInfo, Box<dyn std::error::Error>> {
    // --- Redis setup ---
    let redis_config = RedisConfig::new(redis_url).with_key_prefix("floxide_example:");
    let redis_client = RedisClient::new(redis_config).await?;

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

    let queue = RedisWorkQueue::new(redis_client.clone());
    let context_store = RedisContextStore::new(redis_client.clone());
    let run_info_store = RedisRunInfoStore::new(redis_client.clone());
    let metrics_store = RedisMetricsStore::new(redis_client.clone());
    let error_store = RedisErrorStore::new(redis_client.clone());
    let liveness_store = RedisLivenessStore::new(redis_client.clone());
    let work_item_state_store = RedisWorkItemStateStore::new(redis_client.clone());

    // Orchestrator with Redis stores
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

    // Worker with Redis stores
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
    let mut pool = WorkerPool::new(worker, 20);
    pool.start();

    // Start the run
    let run_id: String = orchestrator.start_run(&wf_ctx, 10).await?;

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

        let metrics = orchestrator.metrics(&run_id).await?;
        println!("Metrics: {:#?}", metrics);

        // let errors = orchestrator.errors(&run_id).await?;
        // println!("Errors: {:#?}", errors);

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
    let mut retries = 0;
    let max_retries = 5;
    let final_status = loop {
        retries += 1;
        if retries > max_retries {
            break Err(FloxideError::Generic("max retries reached".to_string()).into());
        }
        match final_status {
            Ok(Ok(ref info)) if info.status == RunStatus::Completed => {
                print_stats().await?;
                break Ok(info.clone());
            }
            Ok(Ok(ref info))
                if info.status == RunStatus::Failed
                    || matches!(final_status, Err(Elapsed { .. })) =>
            {
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
    };

    pool.stop().await;
    final_status
}
