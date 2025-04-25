use crate::workflow::Workflow;
use crate::checkpoint::CheckpointStore;
use crate::context::WorkflowCtx;
use crate::error::FloxideError;
use crate::distributed::{RunStatus, RunInfo, WorkflowError, RunMetrics, WorkQueue, RunInfoStore, MetricsStore, ErrorStore, LivenessStore, LivenessStatus, RunInfoError, LivenessStoreError};
use std::marker::PhantomData;
use std::fmt::Debug;
use uuid;
use chrono::Utc;
use std::time::Duration;

pub struct DistributedOrchestrator<W, C, Q, S, RIS, MS, ES, LS>
where
    W: Workflow<C>,
    C: Debug + Clone + Send + Sync,
    Q: WorkQueue<W::WorkItem> + Send + Sync,
    S: CheckpointStore<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
{
    workflow: W,
    queue: Q,
    store: S,
    run_info_store: RIS,
    metrics_store: MS,
    error_store: ES,
    liveness_store: LS,
    phantom: PhantomData<C>,
}

impl<W, C, Q, S, RIS, MS, ES, LS> DistributedOrchestrator<W, C, Q, S, RIS, MS, ES, LS>
where
    W: Workflow<C>,
    Q: WorkQueue<W::WorkItem> + Send + Sync,
    S: CheckpointStore<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    C: std::fmt::Debug + Clone + Send + Sync,
{
    /// Create a new orchestrator.
    pub fn new(
        workflow: W,
        queue: Q,
        store: S,
        run_info_store: RIS,
        metrics_store: MS,
        error_store: ES,
        liveness_store: LS,
    ) -> Self {
        Self { workflow, queue, store, run_info_store, metrics_store, error_store, liveness_store, phantom: PhantomData }
    }

    /// Start a new workflow run. Returns a run_id.
    ///
    /// Note: Requires `uuid` crate in Cargo.toml: uuid = { version = "1", features = ["v4"] }
    pub async fn start_run(&self, ctx: &WorkflowCtx<C>, input: W::Input)
        -> Result<String, FloxideError>
    {
        // Generate a unique run_id
        let run_id = uuid::Uuid::new_v4().to_string();
        // Seed the workflow (creates checkpoint and enqueues first work item)
        self.workflow
            .start_distributed(ctx, input, &self.store, &self.queue, &run_id)
            .await?;
        // Insert run info into run_info_store
        let run_info = RunInfo {
            run_id: run_id.clone(),
            status: RunStatus::Running,
            started_at: Utc::now(),
            finished_at: None,
        };
        self.run_info_store.insert_run(run_info).await.map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
        Ok(run_id)
    }

    /// Query the status of a run.
    pub async fn status(&self, run_id: &str) -> Result<RunStatus, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        match self.run_info_store.get_run(run_id).await {
            Ok(Some(info)) => Ok(info.status),
            Ok(None) => Err(FloxideError::NotStarted),
            Err(e) => Err(FloxideError::Generic(format!("run_info_store error: {e}"))),
        }
    }

    /// List all runs (optionally filter by status).
    pub async fn list_runs(&self, filter: Option<RunStatus>) -> Result<Vec<RunInfo>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.run_info_store.list_runs(filter).await.map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))
    }

    /// Cancel a run.
    pub async fn cancel(&self, run_id: &str) -> Result<(), FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.run_info_store.update_status(run_id, RunStatus::Cancelled)
            .await
            .map_err(|e| match e {
                RunInfoError::NotFound => FloxideError::NotStarted,
                e => FloxideError::Generic(format!("run_info_store error: {e}")),
            })
    }

    /// Pause a run.
    pub async fn pause(&self, run_id: &str) -> Result<(), FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.run_info_store.update_status(run_id, RunStatus::Paused)
            .await
            .map_err(|e| match e {
                RunInfoError::NotFound => FloxideError::NotStarted,
                e => FloxideError::Generic(format!("run_info_store error: {e}")),
            })
    }

    /// Resume a run.
    pub async fn resume(&self, run_id: &str) -> Result<(), FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.run_info_store.update_status(run_id, RunStatus::Running)
            .await
            .map_err(|e| match e {
                RunInfoError::NotFound => FloxideError::NotStarted,
                e => FloxideError::Generic(format!("run_info_store error: {e}")),
            })
    }

    /// Get all errors for a run.
    pub async fn errors(&self, run_id: &str) -> Result<Vec<WorkflowError>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.error_store.get_errors(run_id).await.map_err(|e| FloxideError::Generic(format!("error_store error: {e}")))
    }

    /// Get progress/metrics for a run.
    pub async fn metrics(&self, run_id: &str) -> Result<RunMetrics, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        match self.metrics_store.get_metrics(run_id).await {
            Ok(Some(metrics)) => Ok(metrics),
            Ok(None) => Err(FloxideError::NotStarted),
            Err(e) => Err(FloxideError::Generic(format!("metrics_store error: {e}"))),
        }
    }

    /// Check liveness status of a list of workers.
    pub async fn check_worker_liveness(&self, worker_ids: &[usize], threshold: Duration) -> Vec<(usize, LivenessStatus)> {
        let now = Utc::now();
        let mut statuses = Vec::new();
        for &worker_id in worker_ids {
            let status = match self.liveness_store.get_heartbeat(worker_id).await {
                Ok(Some(ts)) => {
                    let elapsed = now.signed_duration_since(ts).to_std().unwrap_or(Duration::MAX);
                    if elapsed < threshold {
                        LivenessStatus::Alive
                    } else if elapsed < threshold * 3 {
                        LivenessStatus::Stale
                    } else {
                        LivenessStatus::Dead
                    }
                }
                Ok(None) => LivenessStatus::Dead,
                Err(_) => LivenessStatus::Dead,
            };
            statuses.push((worker_id, status));
        }
        statuses
    }

    /// List all known worker IDs.
    pub async fn list_workers(&self) -> Result<Vec<usize>, LivenessStoreError> {
        self.liveness_store.list_workers().await
    }

    /// List all worker health info.
    pub async fn list_worker_health(&self) -> Result<Vec<crate::distributed::WorkerHealth>, LivenessStoreError> {
        self.liveness_store.list_health().await
    }

    /// Mark a run as completed and set finished_at timestamp.
    pub async fn complete_run(&self, run_id: &str) -> Result<(), FloxideError> {
        let now = chrono::Utc::now();
        self.run_info_store.update_status(run_id, RunStatus::Completed).await.map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
        self.run_info_store.update_finished_at(run_id, now).await.map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
        Ok(())
    }

    /// Wait for a run to reach a terminal state (Completed, Failed, or Cancelled).
    pub async fn wait_for_completion(&self, run_id: &str, poll_interval: std::time::Duration) -> Result<RunStatus, FloxideError> {
        loop {
            let status = self.status(run_id).await?;
            match status {
                RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => return Ok(status),
                _ => tokio::time::sleep(poll_interval).await,
            }
        }
    }
}

pub struct OrchestratorBuilder<W, C, Q, S, RIS, MS, ES, LS> {
    workflow: Option<W>,
    queue: Option<Q>,
    store: Option<S>,
    run_info_store: Option<RIS>,
    metrics_store: Option<MS>,
    error_store: Option<ES>,
    liveness_store: Option<LS>,
    _phantom: std::marker::PhantomData<C>,
}

impl<W, C, Q, S, RIS, MS, ES, LS> OrchestratorBuilder<W, C, Q, S, RIS, MS, ES, LS> {
    pub fn new() -> Self {
        Self {
            workflow: None,
            queue: None,
            store: None,
            run_info_store: None,
            metrics_store: None,
            error_store: None,
            liveness_store: None,
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn workflow(mut self, workflow: W) -> Self { self.workflow = Some(workflow); self }
    pub fn queue(mut self, queue: Q) -> Self { self.queue = Some(queue); self }
    pub fn store(mut self, store: S) -> Self { self.store = Some(store); self }
    pub fn run_info_store(mut self, ris: RIS) -> Self { self.run_info_store = Some(ris); self }
    pub fn metrics_store(mut self, ms: MS) -> Self { self.metrics_store = Some(ms); self }
    pub fn error_store(mut self, es: ES) -> Self { self.error_store = Some(es); self }
    pub fn liveness_store(mut self, ls: LS) -> Self { self.liveness_store = Some(ls); self }
    pub fn build(self) -> Result<DistributedOrchestrator<W, C, Q, S, RIS, MS, ES, LS>, String>
    where
        W: Workflow<C>,
        C: std::fmt::Debug + Clone + Send + Sync,
        Q: WorkQueue<W::WorkItem> + Send + Sync,
        S: CheckpointStore<C, W::WorkItem> + Send + Sync,
        RIS: crate::distributed::RunInfoStore + Send + Sync,
        MS: crate::distributed::MetricsStore + Send + Sync,
        ES: crate::distributed::ErrorStore + Send + Sync,
        LS: crate::distributed::LivenessStore + Send + Sync,
    {
        Ok(DistributedOrchestrator {
            workflow: self.workflow.ok_or("workflow is required")?,
            queue: self.queue.ok_or("queue is required")?,
            store: self.store.ok_or("store is required")?,
            run_info_store: self.run_info_store.ok_or("run_info_store is required")?,
            metrics_store: self.metrics_store.ok_or("metrics_store is required")?,
            error_store: self.error_store.ok_or("error_store is required")?,
            liveness_store: self.liveness_store.ok_or("liveness_store is required")?,
            phantom: std::marker::PhantomData,
        })
    }
} 