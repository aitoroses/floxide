use crate::context::Context;
use crate::distributed::{
    ContextStore, ErrorStore, LivenessStore, MetricsStore, RunInfoStore, RunStatus,
    WorkItemStateStore, WorkItemStatus, WorkQueue, WorkerHealth, WorkerStatus, WorkflowError,
};
use crate::error::FloxideError;
use crate::retry::{BackoffStrategy, RetryError, RetryPolicy};
use crate::workflow::Workflow;
use async_trait::async_trait;
use rand::Rng;
use serde_json;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use tracing::error;

use super::{ItemProcessedOutcome, StepCallbacks};

/// A distributed workflow worker that polls a work queue, processes workflow steps, and updates state in distributed stores.
///
/// Use [`run_once`] to process a single work item, or [`run_forever`] to continuously poll for work.
#[derive(Clone)]
pub struct DistributedWorker<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync + 'static,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    workflow: W,
    queue: Q,
    context_store: CS,
    run_info_store: RIS,
    metrics_store: MS,
    error_store: ES,
    liveness_store: LS,
    work_item_state_store: WISS,
    retry_policy: Option<RetryPolicy>,
    idle_sleep_duration: Duration,
    idle_sleep_jitter: Duration,
    phantom: PhantomData<C>,
}

impl<W, C, Q, RIS, MS, ES, LS, WISS, CS> DistributedWorker<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static> + 'static,
    C: Context + crate::merge::Merge + Default + 'static,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync + Clone,
    RIS: RunInfoStore + Send + Sync + Clone + 'static,
    MS: MetricsStore + Send + Sync + Clone + 'static,
    ES: ErrorStore + Send + Sync + Clone + 'static,
    LS: LivenessStore + Send + Sync + Clone + 'static,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync + Clone + 'static,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
    Self: Clone,
{
    /// Create a new distributed worker with all required stores and workflow.
    ///
    /// See [`WorkerBuilder`] for ergonomic construction with defaults.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workflow: W,
        queue: Q,
        context_store: CS,
        run_info_store: RIS,
        metrics_store: MS,
        error_store: ES,
        liveness_store: LS,
        work_item_state_store: WISS,
    ) -> Self {
        Self {
            workflow,
            queue,
            context_store,
            run_info_store,
            metrics_store,
            error_store,
            liveness_store,
            work_item_state_store,
            retry_policy: None,
            idle_sleep_duration: Duration::from_millis(100),
            idle_sleep_jitter: Duration::from_millis(50),
            phantom: PhantomData,
        }
    }

    /// Set a retry policy for all work items.
    pub fn set_retry_policy(&mut self, policy: RetryPolicy) {
        self.retry_policy = Some(policy);
    }

    #[allow(clippy::type_complexity)]
    fn build_callbacks(
        &self,
        worker_id: usize,
    ) -> Arc<StepCallbacksImpl<C, W, Q, RIS, MS, ES, LS, WISS, CS>> {
        let cloned_worker = self.clone();
        Arc::new(StepCallbacksImpl {
            worker: Arc::new(cloned_worker),
            worker_id,
        })
    }

    // --- Callback-style state update methods ---
    /// Called when a work item is about to be processed. Only allows Pending items to proceed.
    /// Returns Err if the item is not in a processable state.
    async fn on_started_state_updates(
        &self,
        worker_id: usize,
        run_id: &str,
        work_item: &W::WorkItem,
    ) -> Result<(), FloxideError> {
        let mut health = self
            .liveness_store
            .get_health(worker_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        health.status = WorkerStatus::InProgress;
        health.current_work_item = Some(format!("{:?}", work_item));
        health.current_work_item_run_id = Some(run_id.to_string());
        self.liveness_store
            .update_health(worker_id, health)
            .await
            .ok();
        let item_state = self
            .work_item_state_store
            .get_status(run_id, work_item)
            .await;
        match item_state {
            Ok(WorkItemStatus::Pending) => {
                // Normal path: transition to InProgress and increment attempts
                self.work_item_state_store
                    .set_status(run_id, work_item, WorkItemStatus::InProgress)
                    .await
                    .ok();
                self.work_item_state_store
                    .increment_attempts(run_id, work_item)
                    .await
                    .ok();
                Ok(())
            }
            Ok(WorkItemStatus::InProgress) => {
                // [Distributed edge case]: Multiple workers may dequeue the same work item nearly simultaneously.
                // Only one will succeed in updating the state; the others will see it as already in progress or completed.
                // This is expected in distributed systems without strict distributed locking or atomic compare-and-set.
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is already in progress",
                    work_item
                );
                Err(FloxideError::Generic(format!(
                    "Work item {:?} is already in progress",
                    work_item
                )))
            }
            Ok(WorkItemStatus::Completed) => {
                // [Distributed edge case]: Multiple workers may attempt to process the same work item, but only one can complete it.
                // The others will see it as already completed. This is not a fatal error, but is expected in at-least-once delivery systems.
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is already completed",
                    work_item
                );
                Err(FloxideError::Generic(format!(
                    "Work item {:?} is already completed",
                    work_item
                )))
            }
            Ok(WorkItemStatus::Failed) => {
                tracing::error!(
                    worker_id,
                    run_id,
                    "Work item {:?} previously failed and should not be processed again",
                    work_item
                );
                Err(FloxideError::Generic(format!(
                    "Work item {:?} previously failed and should not be processed again",
                    work_item
                )))
            }
            Ok(WorkItemStatus::WaitingRetry) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is waiting for retry backoff",
                    work_item
                );
                Err(FloxideError::Generic(format!(
                    "Work item {:?} is waiting for retry backoff",
                    work_item
                )))
            }
            Ok(WorkItemStatus::PermanentlyFailed) => {
                tracing::error!(
                    worker_id,
                    run_id,
                    "Work item {:?} is permanently failed and should not be processed again",
                    work_item
                );
                Err(FloxideError::Generic(format!(
                    "Work item {:?} is permanently failed and should not be processed again",
                    work_item
                )))
            }
            Err(e) => {
                tracing::error!(worker_id, run_id, "Error getting work item status: {:?}", e);
                Err(FloxideError::Generic(format!(
                    "Error getting work item status: {:?}",
                    e
                )))
            }
        }
    }

    /// Called when a terminal work item is processed successfully.
    async fn on_item_processed_success_terminal_state_updates(
        &self,
        worker_id: usize,
        run_id: &str,
        work_item: &W::WorkItem,
        output: &serde_json::Value,
    ) -> Result<(), FloxideError> {
        let status_result = self
            .work_item_state_store
            .get_status(run_id, work_item)
            .await;
        let status = status_result.ok(); // Get Option<WorkItemStatus>
        tracing::debug!(worker_id, run_id=%run_id, ?work_item, current_status=?status, "Processing successful terminal item");
        match status {
            Some(WorkItemStatus::Completed) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is already completed (terminal)",
                    work_item
                );
                return Ok(());
            }
            Some(WorkItemStatus::PermanentlyFailed) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is permanently failed (terminal)",
                    work_item
                );
                return Ok(());
            }
            _ => {
                self.work_item_state_store
                    .set_status(run_id, work_item, WorkItemStatus::Completed)
                    .await
                    .ok();
            }
        }
        let mut metrics = self
            .metrics_store
            .get_metrics(run_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        metrics.completed += 1;
        metrics.total_work_items += 1;
        self.metrics_store
            .update_metrics(run_id, metrics)
            .await
            .ok();
        let now = chrono::Utc::now();
        tracing::debug!(worker_id, run_id=%run_id, "Attempting to set run status to Completed");
        self.run_info_store
            .update_status(run_id, RunStatus::Completed)
            .await
            .map_err(|e| {
                FloxideError::Generic(format!("Failed to set run status to Completed: {}", e))
            })?;
        // Set the output field
        self.run_info_store
            .update_output(run_id, output.clone())
            .await
            .map_err(|e| FloxideError::Generic(format!("Failed to set run output: {}", e)))?;
        self.run_info_store
            .update_finished_at(run_id, now)
            .await
            .map_err(|e| FloxideError::Generic(format!("Failed to set run finished at: {}", e)))?;
        Ok(())
    }

    /// Called when a non-terminal work item is processed successfully.
    async fn on_item_processed_success_non_terminal_state_updates(
        &self,
        worker_id: usize,
        run_id: &str,
        work_item: &W::WorkItem,
    ) -> Result<(), FloxideError> {
        let status_result = self
            .work_item_state_store
            .get_status(run_id, work_item)
            .await;
        let status = status_result.ok(); // Get Option<WorkItemStatus>
        tracing::debug!(worker_id, run_id=%run_id, ?work_item, current_status=?status, "Processing successful non-terminal item");
        match status {
            Some(WorkItemStatus::Completed) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is already completed (non-terminal)",
                    work_item
                );
                return Ok(());
            }
            Some(WorkItemStatus::PermanentlyFailed) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is permanently failed (non-terminal)",
                    work_item
                );
                return Ok(());
            }
            _ => {
                self.work_item_state_store
                    .set_status(run_id, work_item, WorkItemStatus::Completed)
                    .await
                    .ok();
            }
        }
        let mut metrics = self
            .metrics_store
            .get_metrics(run_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        metrics.total_work_items += 1;
        self.metrics_store
            .update_metrics(run_id, metrics)
            .await
            .ok();
        Ok(())
    }

    /// Called when a work item processing returns an error.
    async fn on_item_processed_error_state_updates(
        &self,
        worker_id: usize,
        run_id: &str,
        work_item: &W::WorkItem,
        e: &FloxideError,
    ) -> Result<(), FloxideError> {
        let status = self
            .work_item_state_store
            .get_status(run_id, work_item)
            .await
            .ok();
        match status {
            Some(WorkItemStatus::Completed) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is already completed (error)",
                    work_item
                );
                return Ok(());
            }
            Some(WorkItemStatus::PermanentlyFailed) => {
                tracing::warn!(
                    worker_id,
                    run_id,
                    "Work item {:?} is permanently failed (error)",
                    work_item
                );
                return Ok(());
            }
            _ => {}
        }
        let mut health = self
            .liveness_store
            .get_health(worker_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        health.error_count += 1;
        let policy = self.retry_policy.as_ref();
        let attempt = self
            .work_item_state_store
            .get_attempts(run_id, work_item)
            .await
            .unwrap_or(0) as usize;
        let should_retry = policy.map(|p| p.should_retry(e, attempt)).unwrap_or(false);
        let max_attempts = policy.map(|p| p.max_attempts).unwrap_or(5);
        let mut is_permanent = false;
        if should_retry {
            health.status = WorkerStatus::Retrying(attempt, max_attempts);
            self.work_item_state_store
                .set_status(run_id, work_item, WorkItemStatus::Failed)
                .await
                .ok();
            let attempts = self
                .work_item_state_store
                .get_attempts(run_id, work_item)
                .await
                .unwrap_or(0);
            if attempts >= max_attempts as u32 {
                self.work_item_state_store
                    .set_status(run_id, work_item, WorkItemStatus::PermanentlyFailed)
                    .await
                    .ok();
                is_permanent = true;
            } else {
                // Set to WaitingRetry for backoff
                tracing::debug!(worker_id, run_id=%run_id, ?work_item, attempt, "Setting item status to WaitingRetry");
                self.work_item_state_store
                    .set_status(run_id, work_item, WorkItemStatus::WaitingRetry)
                    .await
                    .ok();
            }
        } else {
            self.work_item_state_store
                .set_status(run_id, work_item, WorkItemStatus::PermanentlyFailed)
                .await
                .ok();
            is_permanent = true;
        }
        self.liveness_store
            .update_health(worker_id, health)
            .await
            .ok();
        // Record error
        let work_item_str = format!("{:?}", work_item);
        let workflow_error = WorkflowError {
            work_item: work_item_str,
            error: format!("{:?}", e),
            attempt,
            timestamp: chrono::Utc::now(),
        };
        self.error_store
            .record_error(run_id, workflow_error)
            .await
            .ok();
        // Update metrics
        let mut metrics = self
            .metrics_store
            .get_metrics(run_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if should_retry && !is_permanent {
            metrics.retries += 1;
        } else {
            metrics.failed += 1;
        }
        self.metrics_store
            .update_metrics(run_id, metrics)
            .await
            .ok();
        // On permanent failure, mark run as failed and purge
        if is_permanent {
            let now = chrono::Utc::now();
            self.run_info_store
                .update_status(run_id, RunStatus::Failed)
                .await
                .ok();
            self.run_info_store
                .update_finished_at(run_id, now)
                .await
                .ok();
        }
        // Retry or break: on retry, re-enqueue the failed work item
        if should_retry && !is_permanent {
            if let Some(policy) = policy {
                let queue = self.queue.clone();
                let run_id = run_id.to_string();
                let work_item = work_item.clone();
                let work_item_state_store = self.work_item_state_store.clone();
                let backoff = policy.backoff_duration(attempt);
                tracing::debug!(
                    worker_id,
                    run_id = %run_id,
                    ?work_item,
                    ?backoff,
                    "Spawning task to re-enqueue work item after backoff"
                );
                tokio::spawn(async move {
                    let task_run_id = run_id.clone();
                    let task_work_item = work_item.clone();
                    tracing::debug!(run_id = %task_run_id, work_item = ?task_work_item, ?backoff, "Retry task SPAWNED, will sleep");
                    tokio::time::sleep(backoff).await;
                    tracing::debug!(run_id = %task_run_id, work_item = ?task_work_item, "Retry task AWAKE after sleep");

                    // Set to Pending before re-enqueue
                    tracing::debug!(run_id = %task_run_id, work_item = ?task_work_item, "Retry task attempting to set item status to Pending");
                    match work_item_state_store
                        .set_status(&task_run_id, &task_work_item, WorkItemStatus::Pending)
                        .await
                    {
                        Ok(_) => {
                            tracing::debug!(run_id = %task_run_id, work_item = ?task_work_item, "Retry task successfully set item status to Pending");
                        }
                        Err(e) => {
                            tracing::error!(
                                run_id = %task_run_id,
                                work_item = ?task_work_item,
                                error = %e,
                                "Retry task FAILED to set status to Pending"
                            );
                            // Optionally, decide if we should still attempt enqueue or just return
                            return;
                        }
                    }

                    tracing::debug!(run_id = %task_run_id, work_item = ?task_work_item, "Retry task attempting enqueue");
                    match queue.enqueue(&task_run_id, task_work_item.clone()).await {
                        Ok(_) => {
                            tracing::debug!(run_id = %task_run_id, work_item = ?task_work_item, "Retry task successfully enqueued work item");
                        }
                        Err(e) => {
                            tracing::error!(
                                run_id = %task_run_id,
                                work_item = ?task_work_item,
                                error = %e,
                                "Retry task FAILED to enqueue work item!"
                            );
                            // CRITICAL: The item failed to re-enqueue. It's now potentially lost.
                            // Consider adding logic here to signal this failure more robustly,
                            // maybe update the WorkItemStatus to a specific 'EnqueueFailed' state,
                            // or alert an external monitoring system.
                        }
                    }
                });
            }
        }
        Ok(())
    }

    /// Update the worker's health to idle.
    async fn on_idle_state_updates(&self, worker_id: usize) -> Result<(), FloxideError> {
        let mut health = self
            .liveness_store
            .get_health(worker_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        health.status = WorkerStatus::Idle;
        health.current_work_item = None;
        health.current_work_item_run_id = None;
        self.liveness_store
            .update_health(worker_id, health)
            .await
            .ok();
        Ok(())
    }

    /// Check if the worker is permanently failed and should stop.
    async fn can_worker_continue(&self, worker_id: usize) -> bool {
        let health = self
            .liveness_store
            .get_health(worker_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        matches!(health.status, WorkerStatus::Idle)
    }

    /// Process a single work item from the queue, updating all distributed state.
    ///
    /// Returns `Ok(Some((run_id, output)))` if a work item was processed, `Ok(None)` if no work was available, or `Err` on permanent failure.
    ///
    /// # Instrumentation
    /// This method is instrumented with `tracing` for async span tracking.
    #[tracing::instrument(skip(self))]
    pub async fn run_once(
        &self,
        worker_id: usize,
    ) -> Result<Option<(String, W::Output)>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        if !self.can_worker_continue(worker_id).await {
            tracing::debug!(worker_id, "Worker is permanently failed, skipping work");
            return Ok(None);
        }
        self.heartbeat(worker_id).await;
        match self
            .workflow
            .step_distributed(
                &self.context_store,
                &self.queue,
                worker_id,
                self.build_callbacks(worker_id),
            )
            .await
        {
            Ok(Some((run_id, output))) => {
                self.on_idle_state_updates(worker_id).await?;
                Ok(Some((run_id, output)))
            }
            Ok(None) => {
                self.on_idle_state_updates(worker_id).await?;
                Ok(None)
            }
            Err(e) => {
                self.on_idle_state_updates(worker_id).await?;
                Err(e.error)
            }
        }
    }

    /// Continuously poll for work and process items, sleeping briefly when idle or on error.
    ///
    /// This method never returns and is suitable for running in a background task.
    ///
    /// # Instrumentation
    /// This method is instrumented with `tracing` for async span tracking.
    ///
    /// Note: Returns [`std::convert::Infallible`] for compatibility with stable Rust (instead of the experimental `!` type).
    #[tracing::instrument(skip(self))]
    pub async fn run_forever(&self, worker_id: usize) -> std::convert::Infallible
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        let base_sleep_ms = self.idle_sleep_duration.as_millis() as u64;
        // Jitter range is +/- half the jitter duration
        let jitter_range_ms = (self.idle_sleep_jitter.as_millis() / 2) as i64;

        loop {
            match self.run_once(worker_id).await {
                Ok(Some((_run_id, _output))) => {
                    // Work was done, continue immediately
                }
                Ok(None) => {
                    // No work available, sleep before polling again
                    let jitter_ms =
                        rand::thread_rng().gen_range(-jitter_range_ms..=jitter_range_ms);
                    let sleep_ms = ((base_sleep_ms as i64) + jitter_ms).max(0) as u64;
                    let sleep_duration = Duration::from_millis(sleep_ms);
                    sleep(sleep_duration).await;
                }
                Err(e) => {
                    error!(worker_id, error = ?e, "Worker encountered error in run_once");
                    let jitter_ms =
                        rand::thread_rng().gen_range(-jitter_range_ms..=jitter_range_ms);
                    let sleep_ms = ((base_sleep_ms as i64) + jitter_ms).max(0) as u64;
                    let sleep_duration = Duration::from_millis(sleep_ms);
                    sleep(sleep_duration).await;
                }
            }
        }
    }

    /// Heartbeat: update liveness store with current timestamp and update health.
    ///
    /// # Instrumentation
    /// This method is instrumented with `tracing` for async span tracking.
    #[tracing::instrument(skip(self))]
    pub async fn heartbeat(&self, worker_id: usize)
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        let now = chrono::Utc::now();
        let _ = self.liveness_store.update_heartbeat(worker_id, now).await;
        // Fetch and update health
        let mut health = self
            .liveness_store
            .get_health(worker_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        health.last_heartbeat = now;
        let _ = self.liveness_store.update_health(worker_id, health).await;
    }
}

pub struct WorkerBuilder<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    workflow: Option<W>,
    queue: Option<Q>,
    context_store: Option<CS>,
    run_info_store: Option<RIS>,
    metrics_store: Option<MS>,
    error_store: Option<ES>,
    liveness_store: Option<LS>,
    work_item_state_store: Option<WISS>,
    retry_policy: Option<RetryPolicy>,
    idle_sleep_duration: Option<Duration>,
    idle_sleep_jitter: Option<Duration>,
    _phantom: std::marker::PhantomData<C>,
}

impl<W, C, Q, RIS, MS, ES, LS, WISS, CS> WorkerBuilder<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    pub fn new() -> Self {
        Self {
            workflow: None,
            queue: None,
            context_store: None,
            run_info_store: None,
            metrics_store: None,
            error_store: None,
            liveness_store: None,
            work_item_state_store: None,
            retry_policy: None,
            idle_sleep_duration: None,
            idle_sleep_jitter: None,
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
    pub fn context_store(mut self, context_store: CS) -> Self {
        self.context_store = Some(context_store);
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
    pub fn work_item_state_store(mut self, wiss: WISS) -> Self {
        self.work_item_state_store = Some(wiss);
        self
    }
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }
    pub fn idle_sleep_duration(mut self, duration: Duration) -> Self {
        self.idle_sleep_duration = Some(duration);
        self
    }
    pub fn idle_sleep_jitter(mut self, jitter: Duration) -> Self {
        self.idle_sleep_jitter = Some(jitter);
        self
    }
    #[allow(clippy::type_complexity)]
    pub fn build(self) -> Result<DistributedWorker<W, C, Q, RIS, MS, ES, LS, WISS, CS>, String>
    where
        W: Workflow<C, WorkItem: 'static>,
        C: std::fmt::Debug + Clone + Send + Sync,
        Q: WorkQueue<C, W::WorkItem> + Send + Sync,
        RIS: RunInfoStore + Send + Sync,
        MS: MetricsStore + Send + Sync,
        ES: ErrorStore + Send + Sync,
        LS: LivenessStore + Send + Sync,
        WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
        CS: ContextStore<C> + Send + Sync + Clone + 'static,
    {
        Ok(DistributedWorker {
            workflow: self.workflow.ok_or("workflow is required")?,
            queue: self.queue.ok_or("queue is required")?,
            context_store: self.context_store.ok_or("context_store is required")?,
            run_info_store: self.run_info_store.ok_or("run_info_store is required")?,
            metrics_store: self.metrics_store.ok_or("metrics_store is required")?,
            error_store: self.error_store.ok_or("error_store is required")?,
            liveness_store: self.liveness_store.ok_or("liveness_store is required")?,
            work_item_state_store: self
                .work_item_state_store
                .ok_or("work_item_state_store is required")?,
            retry_policy: Some(self.retry_policy.unwrap_or_else(|| {
                RetryPolicy::new(
                    5,
                    std::time::Duration::from_millis(1000),
                    std::time::Duration::from_secs(10),
                    BackoffStrategy::Exponential,
                    RetryError::All,
                )
            })),
            idle_sleep_duration: self
                .idle_sleep_duration
                .unwrap_or(Duration::from_millis(100)),
            idle_sleep_jitter: self.idle_sleep_jitter.unwrap_or(Duration::from_millis(50)),
            phantom: std::marker::PhantomData,
        })
    }
}

impl<W, C, Q, RIS, MS, ES, LS, WISS, CS> Default
    for WorkerBuilder<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A pool of distributed workflow workers, each running in its own async task.
///
/// The pool manages worker lifecycles, graceful shutdown, and health reporting.
#[allow(clippy::type_complexity)]
pub struct WorkerPool<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    worker: DistributedWorker<W, C, Q, RIS, MS, ES, LS, WISS, CS>,
    num_workers: usize,
    handles: Vec<JoinHandle<()>>,
    cancel_tokens: Vec<CancellationToken>,
}

impl<W, C, Q, RIS, MS, ES, LS, WISS, CS> WorkerPool<W, C, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static> + 'static,
    C: Context + crate::merge::Merge + Default + 'static,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync + Clone + 'static,
    RIS: RunInfoStore + Send + Sync + Clone + 'static,
    MS: MetricsStore + Send + Sync + Clone + 'static,
    ES: ErrorStore + Send + Sync + Clone + 'static,
    LS: LivenessStore + Send + Sync + Clone + 'static,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync + Clone + 'static,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    /// Create a new worker pool with the given worker and number of workers.
    pub fn new(
        worker: DistributedWorker<W, C, Q, RIS, MS, ES, LS, WISS, CS>,
        num_workers: usize,
    ) -> Self {
        Self {
            worker,
            num_workers,
            handles: Vec::new(),
            cancel_tokens: Vec::new(),
        }
    }

    /// Start all workers in the pool. Each worker runs in its own async task.
    pub fn start(&mut self) {
        for worker_id in 0..self.num_workers {
            let cancel_token = CancellationToken::new();
            let cancel_token_child = cancel_token.child_token();
            let worker = self.worker.clone();
            let handle = tokio::spawn(async move {
                let token = cancel_token_child;
                tokio::select! {
                    _ = worker.run_forever(worker_id) => {},
                    _ = token.cancelled() => {},
                }
            });
            self.handles.push(handle);
            self.cancel_tokens.push(cancel_token);
        }
    }

    /// Gracefully stop all workers by signalling cancellation and waiting for them to finish.
    pub async fn stop(&mut self) {
        for token in &self.cancel_tokens {
            token.cancel();
        }
        for handle in self.handles.drain(..) {
            let _ = handle.await;
        }
    }

    /// Wait for all workers to finish.
    pub async fn join(&mut self) {
        for handle in self.handles.drain(..) {
            let _ = handle.await;
        }
    }

    /// Get health/status of all workers from the liveness store.
    pub async fn health(&self) -> Vec<WorkerHealth> {
        self.worker
            .liveness_store
            .list_health()
            .await
            .unwrap_or_default()
    }
}

#[allow(clippy::type_complexity)]
struct StepCallbacksImpl<
    C: Context + crate::merge::Merge + Default,
    W: Workflow<C>,
    Q,
    RIS,
    MS,
    ES,
    LS,
    WISS,
    CS,
> where
    W: Workflow<C, WorkItem: 'static>,
    C: Context + crate::merge::Merge + Default,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync,
    RIS: RunInfoStore + Send + Sync,
    MS: MetricsStore + Send + Sync,
    ES: ErrorStore + Send + Sync,
    LS: LivenessStore + Send + Sync,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    worker: Arc<DistributedWorker<W, C, Q, RIS, MS, ES, LS, WISS, CS>>,
    worker_id: usize,
}

#[async_trait]
impl<C, W, Q, RIS, MS, ES, LS, WISS, CS> StepCallbacks<C, W>
    for StepCallbacksImpl<C, W, Q, RIS, MS, ES, LS, WISS, CS>
where
    W: Workflow<C, WorkItem: 'static> + 'static,
    C: Context + crate::merge::Merge + Default + 'static,
    Q: WorkQueue<C, W::WorkItem> + Send + Sync + Clone,
    RIS: RunInfoStore + Send + Sync + Clone + 'static,
    MS: MetricsStore + Send + Sync + Clone + 'static,
    ES: ErrorStore + Send + Sync + Clone + 'static,
    LS: LivenessStore + Send + Sync + Clone + 'static,
    WISS: WorkItemStateStore<W::WorkItem> + Send + Sync + Clone + 'static,
    CS: ContextStore<C> + Send + Sync + Clone + 'static,
{
    async fn on_started(&self, run_id: String, item: W::WorkItem) -> Result<(), FloxideError> {
        if let Err(e) = self
            .worker
            .on_started_state_updates(self.worker_id, &run_id, &item)
            .await
        {
            tracing::error!(worker_id = self.worker_id, run_id = %run_id, "on_started_state_updates failed: {:?}", e);
        }
        Ok(())
    }
    async fn on_item_processed(
        &self,
        run_id: String,
        item: W::WorkItem,
        outcome: ItemProcessedOutcome,
    ) -> Result<(), FloxideError> {
        let result = match outcome {
            ItemProcessedOutcome::SuccessTerminal(output) => {
                self.worker
                    .on_item_processed_success_terminal_state_updates(
                        self.worker_id,
                        &run_id,
                        &item,
                        &output,
                    )
                    .await
            }
            ItemProcessedOutcome::SuccessNonTerminal => {
                self.worker
                    .on_item_processed_success_non_terminal_state_updates(
                        self.worker_id,
                        &run_id,
                        &item,
                    )
                    .await
            }
            ItemProcessedOutcome::Error(e) => {
                self.worker
                    .on_item_processed_error_state_updates(self.worker_id, &run_id, &item, &e)
                    .await
            }
        };
        if let Err(e) = result {
            tracing::error!(worker_id = self.worker_id, run_id = %run_id, "on_item_processed_state_updates failed: {:?}", e);
        }
        Ok(())
    }
}
