use crate::context::{Context, WorkflowCtx};
use crate::distributed::context_store::ContextStore;
use crate::distributed::{
    ErrorStore, LivenessStatus, LivenessStore, LivenessStoreError, MetricsStore, RunInfo,
    RunInfoError, RunInfoStore, RunMetrics, RunStatus, WorkItemStateStore, WorkQueue,
    WorkflowError,
};
use crate::error::FloxideError;
use crate::workflow::Workflow;
use chrono::Utc;
use std::marker::PhantomData;
use std::time::Duration;
use uuid;

use super::{WorkItemState, WorkItemStateStoreError, WorkItemStatus, WorkerHealth};

pub struct DistributedOrchestrator<W, C, Q, RIS, MS, ES, LS, WIS, CS>
where
    W: Workflow<C>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WIS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync,
{
    workflow: W,
    queue: Q,
    run_info_store: RIS,
    metrics_store: MS,
    error_store: ES,
    liveness_store: LS,
    work_item_state_store: WIS,
    context_store: CS,
    phantom: PhantomData<C>,
}

impl<W, C, Q, RIS, MS, ES, LS, WIS, CS> DistributedOrchestrator<W, C, Q, RIS, MS, ES, LS, WIS, CS>
where
    W: Workflow<C>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WIS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync,
{
    /// Create a new orchestrator.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workflow: W,
        queue: Q,
        run_info_store: RIS,
        metrics_store: MS,
        error_store: ES,
        liveness_store: LS,
        work_item_state_store: WIS,
        context_store: CS,
    ) -> Self {
        Self {
            workflow,
            queue,
            run_info_store,
            metrics_store,
            error_store,
            liveness_store,
            work_item_state_store,
            context_store,
            phantom: PhantomData,
        }
    }

    /// Start a new workflow run. Returns a run_id.
    ///
    /// Note: Requires `uuid` crate in Cargo.toml: uuid = { version = "1", features = ["v4"] }
    pub async fn start_run(
        &self,
        ctx: &WorkflowCtx<C>,
        input: W::Input,
    ) -> Result<String, FloxideError> {
        // Generate a unique run_id
        let run_id = uuid::Uuid::new_v4().to_string();
        // Seed the workflow (creates context and enqueues first work item)
        self.workflow
            .start_distributed(ctx, input, &self.context_store, &self.queue, &run_id)
            .await?;
        // Insert run info into run_info_store
        let run_info = RunInfo {
            run_id: run_id.clone(),
            status: RunStatus::Running,
            started_at: Utc::now(),
            finished_at: None,
            output: None,
        };
        self.run_info_store
            .insert_run(run_info)
            .await
            .map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
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
        self.run_info_store
            .list_runs(filter)
            .await
            .map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))
    }

    /// Cancel a run.
    pub async fn cancel(&self, run_id: &str) -> Result<(), FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.run_info_store
            .update_status(run_id, RunStatus::Cancelled)
            .await
            .map_err(|e| match e {
                RunInfoError::NotFound => FloxideError::NotStarted,
                e => FloxideError::Generic(format!("run_info_store error: {e}")),
            })?;
        let now = chrono::Utc::now();
        self.run_info_store
            .update_finished_at(run_id, now)
            .await
            .map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
        self.queue
            .purge_run(run_id)
            .await
            .map_err(|e| FloxideError::Generic(format!("work_queue error: {e}")))?;
        Ok(())
    }

    /// Pause a run.
    pub async fn pause(&self, run_id: &str) -> Result<(), FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.run_info_store
            .update_status(run_id, RunStatus::Paused)
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
        let run_info = self
            .run_info_store
            .get_run(run_id)
            .await
            .map_err(|e| match e {
                RunInfoError::NotFound => FloxideError::NotStarted,
                e => FloxideError::Generic(format!("run_info_store error: {e}")),
            })?;

        if run_info.is_none() {
            return Err(FloxideError::NotStarted);
        }

        match run_info.unwrap().status {
            // If already running, do nothing
            RunStatus::Running => Ok(()),
            RunStatus::Failed => {
                // Fetch pending work too
                let pending_work = self
                    .pending_work(run_id)
                    .await
                    .map_err(|e| FloxideError::Generic(format!("work_queue error: {e}")))?;

                // Re-establish the work queue from the work item state store
                for item in self.list_work_items(run_id).await.map_err(|e| {
                    FloxideError::Generic(format!("work_item_state_store error: {e}"))
                })? {
                    if item.status != WorkItemStatus::Completed
                        && !pending_work.contains(&item.work_item)
                    {
                        self.work_item_state_store
                            .set_status(run_id, &item.work_item, WorkItemStatus::Pending)
                            .await
                            .map_err(|e| {
                                FloxideError::Generic(format!("work_item_state_store error: {e}"))
                            })?;
                        self.work_item_state_store
                            .reset_attempts(run_id, &item.work_item)
                            .await
                            .map_err(|e| {
                                FloxideError::Generic(format!("work_item_state_store error: {e}"))
                            })?;
                        self.queue
                            .enqueue(run_id, item.work_item.clone())
                            .await
                            .map_err(|e| FloxideError::Generic(format!("work_queue error: {e}")))?;
                    }
                }

                // Reset run status to Running
                self.run_info_store
                    .update_status(run_id, RunStatus::Running)
                    .await
                    .map_err(|e| match e {
                        RunInfoError::NotFound => FloxideError::NotStarted,
                        e => FloxideError::Generic(format!("run_info_store error: {e}")),
                    })
            }
            RunStatus::Completed => Err(FloxideError::Generic("run already completed".to_string())),
            RunStatus::Cancelled => Err(FloxideError::AlreadyCompleted),
            RunStatus::Paused => {
                // Change the status of all work items to Pending
                for item in self.list_work_items(run_id).await.map_err(|e| {
                    FloxideError::Generic(format!("work_item_state_store error: {e}"))
                })? {
                    self.work_item_state_store
                        .set_status(run_id, &item.work_item, WorkItemStatus::Pending)
                        .await
                        .map_err(|e| {
                            FloxideError::Generic(format!("work_item_state_store error: {e}"))
                        })?;
                }
                // Reset run status to Running
                self.run_info_store
                    .update_status(run_id, RunStatus::Running)
                    .await
                    .map_err(|e| match e {
                        RunInfoError::NotFound => FloxideError::NotStarted,
                        e => FloxideError::Generic(format!("run_info_store error: {e}")),
                    })
            }
        }
    }

    /// Get all errors for a run.
    pub async fn errors(&self, run_id: &str) -> Result<Vec<WorkflowError>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.error_store
            .get_errors(run_id)
            .await
            .map_err(|e| FloxideError::Generic(format!("error_store error: {e}")))
    }

    // Get liveness status for a run.
    pub async fn liveness(&self) -> Result<Vec<WorkerHealth>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.liveness_store
            .list_health()
            .await
            .map_err(|e| FloxideError::Generic(format!("liveness_store error: {e}")))
    }

    // Get context for a run.
    pub async fn context(&self, run_id: &str) -> Result<C, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        match self.context_store.get(run_id).await {
            Ok(Some(context)) => Ok(context),
            Ok(None) => Err(FloxideError::NotStarted),
            Err(e) => Err(FloxideError::Generic(format!("context_store error: {e}"))),
        }
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

    /// Get pending work for a run.
    pub async fn pending_work(&self, run_id: &str) -> Result<Vec<W::WorkItem>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.queue
            .pending_work(run_id)
            .await
            .map_err(|e| FloxideError::Generic(format!("work_queue error: {e}")))
    }

    /// Check liveness status of a list of workers.
    pub async fn check_worker_liveness(
        &self,
        worker_ids: &[usize],
        threshold: Duration,
    ) -> Vec<(usize, LivenessStatus)> {
        let now = Utc::now();
        let mut statuses = Vec::new();
        for &worker_id in worker_ids {
            let status = match self.liveness_store.get_heartbeat(worker_id).await {
                Ok(Some(ts)) => {
                    let elapsed = now
                        .signed_duration_since(ts)
                        .to_std()
                        .unwrap_or(Duration::MAX);
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
    pub async fn list_worker_health(
        &self,
    ) -> Result<Vec<crate::distributed::WorkerHealth>, LivenessStoreError> {
        self.liveness_store.list_health().await
    }

    /// Get all work items for a run.
    pub async fn list_work_items(
        &self,
        run_id: &str,
    ) -> Result<Vec<WorkItemState<W::WorkItem>>, WorkItemStateStoreError> {
        self.work_item_state_store.get_all(run_id).await
    }

    /// Mark a run as completed and set finished_at timestamp.
    pub async fn complete_run(&self, run_id: &str) -> Result<(), FloxideError> {
        let now = chrono::Utc::now();
        self.run_info_store
            .update_status(run_id, RunStatus::Completed)
            .await
            .map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
        self.run_info_store
            .update_finished_at(run_id, now)
            .await
            .map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
        Ok(())
    }

    /// Wait for a run to reach a terminal state (Completed, Failed, or Cancelled).
    pub async fn wait_for_completion(
        &self,
        run_id: &str,
        poll_interval: std::time::Duration,
    ) -> Result<RunInfo, FloxideError> {
        loop {
            let status = self
                .run_info_store
                .get_run(run_id)
                .await
                .map_err(|e| FloxideError::Generic(format!("run_info_store error: {e}")))?;
            if let Some(info) = status {
                match info.status {
                    RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => {
                        return Ok(info)
                    }
                    _ => tokio::time::sleep(poll_interval).await,
                }
            } else {
                return Err(FloxideError::NotStarted);
            }
        }
    }
}

pub struct OrchestratorBuilder<W, C, Q, RIS, MS, ES, LS, WIS, CS>
where
    W: Workflow<C>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WIS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync,
{
    workflow: Option<W>,
    queue: Option<Q>,
    run_info_store: Option<RIS>,
    metrics_store: Option<MS>,
    error_store: Option<ES>,
    liveness_store: Option<LS>,
    work_item_state_store: Option<WIS>,
    context_store: Option<CS>,
    _phantom: std::marker::PhantomData<C>,
}

impl<W, C, Q, RIS, MS, ES, LS, WIS, CS> OrchestratorBuilder<W, C, Q, RIS, MS, ES, LS, WIS, CS>
where
    W: Workflow<C>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WIS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            workflow: None,
            queue: None,
            run_info_store: None,
            metrics_store: None,
            error_store: None,
            liveness_store: None,
            work_item_state_store: None,
            context_store: None,
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn workflow(mut self, workflow: W) -> Self {
        self.workflow = Some(workflow);
        self
    }
    pub fn queue(mut self, queue: Q) -> Self {
        self.queue = Some(queue);
        self
    }
    pub fn run_info_store(mut self, ris: RIS) -> Self {
        self.run_info_store = Some(ris);
        self
    }
    pub fn metrics_store(mut self, ms: MS) -> Self {
        self.metrics_store = Some(ms);
        self
    }
    pub fn error_store(mut self, es: ES) -> Self {
        self.error_store = Some(es);
        self
    }
    pub fn liveness_store(mut self, ls: LS) -> Self {
        self.liveness_store = Some(ls);
        self
    }
    pub fn work_item_state_store(mut self, wiss: WIS) -> Self {
        self.work_item_state_store = Some(wiss);
        self
    }
    pub fn context_store(mut self, context_store: CS) -> Self {
        self.context_store = Some(context_store);
        self
    }
    #[allow(clippy::type_complexity)]
    pub fn build(self) -> Result<DistributedOrchestrator<W, C, Q, RIS, MS, ES, LS, WIS, CS>, String>
    where
        W: Workflow<C>,
        C: Context + crate::merge::Merge + Default,
        Q: WorkQueue<C, W::WorkItem> + Send + Sync,
        RIS: RunInfoStore + Send + Sync,
        MS: MetricsStore + Send + Sync,
        ES: ErrorStore + Send + Sync,
        LS: LivenessStore + Send + Sync,
        WIS: WorkItemStateStore<W::WorkItem> + Send + Sync,
        CS: ContextStore<C> + Send + Sync,
    {
        Ok(DistributedOrchestrator {
            workflow: self.workflow.ok_or("workflow is required")?,
            queue: self.queue.ok_or("queue is required")?,
            run_info_store: self.run_info_store.ok_or("run_info_store is required")?,
            metrics_store: self.metrics_store.ok_or("metrics_store is required")?,
            error_store: self.error_store.ok_or("error_store is required")?,
            liveness_store: self.liveness_store.ok_or("liveness_store is required")?,
            work_item_state_store: self
                .work_item_state_store
                .ok_or("work_item_state_store is required")?,
            context_store: self.context_store.ok_or("context_store is required")?,
            phantom: std::marker::PhantomData,
        })
    }
}

impl<W, C, Q, RIS, MS, ES, LS, WIS, CS> Default
    for OrchestratorBuilder<W, C, Q, RIS, MS, ES, LS, WIS, CS>
where
    W: Workflow<C>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WIS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}
