use crate::workflow::Workflow;
use crate::checkpoint::CheckpointStore;
use crate::distributed::{WorkQueue, RunInfoStore, MetricsStore, ErrorStore, WorkflowError, LivenessStore, WorkerHealth};
use crate::error::FloxideError;
use std::fmt::Debug;
use std::marker::PhantomData;
use tokio::time::{sleep, Duration};
use tracing::error;
use crate::retry::{RetryPolicy, BackoffStrategy, RetryError};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// A distributed workflow worker that polls a work queue, processes workflow steps, and updates state in distributed stores.
///
/// # Example
///
/// ```rust
/// use floxide_core::distributed::{DistributedWorker, InMemoryWorkQueue, InMemoryRunInfoStore, InMemoryMetricsStore, InMemoryErrorStore, InMemoryLivenessStore};
/// // ... setup workflow, context, and stores ...
/// let worker = DistributedWorker::new(
///     workflow,
///     InMemoryWorkQueue::default(),
///     checkpoint_store,
///     InMemoryRunInfoStore::default(),
///     InMemoryMetricsStore::default(),
///     InMemoryErrorStore::default(),
///     InMemoryLivenessStore::default(),
/// );
/// ```
///
/// Use [`run_once`] to process a single work item, or [`run_forever`] to continuously poll for work.
#[derive(Clone)]
pub struct DistributedWorker<W, C, Q, S, RIS, MS, ES, LS>
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
    checkpoint_store: S,
    run_info_store: RIS,
    metrics_store: MS,
    error_store: ES,
    liveness_store: LS,
    retry_policy: Option<RetryPolicy>,
    phantom: PhantomData<C>,
}

impl<W, C, Q, S, RIS, MS, ES, LS> DistributedWorker<W, C, Q, S, RIS, MS, ES, LS>
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
    /// Create a new distributed worker with all required stores and workflow.
    ///
    /// See [`WorkerBuilder`] for ergonomic construction with defaults.
    pub fn new(workflow: W, queue: Q, checkpoint_store: S, run_info_store: RIS, metrics_store: MS, error_store: ES, liveness_store: LS) -> Self {
        Self {
            workflow, queue, checkpoint_store, run_info_store, metrics_store, error_store, liveness_store,
            retry_policy: None, phantom: PhantomData,
        }
    }

    /// Set a retry policy for all work items.
    pub fn set_retry_policy(&mut self, policy: RetryPolicy) {
        self.retry_policy = Some(policy);
    }

    /// Process a single work item from the queue, updating all distributed state.
    ///
    /// Returns `Ok(Some((run_id, output)))` if a work item was processed, `Ok(None)` if no work was available, or `Err` on permanent failure.
    ///
    /// # Instrumentation
    /// This method is instrumented with `tracing` for async span tracking.
    #[tracing::instrument(skip(self))]
    pub async fn run_once(&self, worker_id: usize) -> Result<Option<(String, W::Output)>, FloxideError>
    where
        C: std::fmt::Debug + Clone + Send + Sync,
    {
        self.heartbeat(worker_id).await;
        let policy = self.retry_policy.as_ref();
        let max_attempts = policy.map(|p| p.max_attempts).unwrap_or(1);
        let mut attempt = 1;
        let mut last_err;
        loop {
            // Check run status before processing
            // Peek a work item to get the run_id
            let (run_id, _work_item) = match self.queue.peek().await {
                Ok(Some(pair)) => pair,
                Ok(None) => return Ok(None),
                Err(e) => return Err(FloxideError::Generic(format!("work queue error: {e}"))),
            };
            // Check run status
            if let Ok(Some(info)) = self.run_info_store.get_run(&run_id).await {
                use crate::distributed::RunStatus;
                if info.status != RunStatus::Running {
                    // Skip processing if not running
                    return Ok(None);
                }
            }
            // Process the work item
            match self.workflow.step_distributed(&self.checkpoint_store, &self.queue, worker_id).await {
                Ok(Some((run_id, output))) => {
                    // On success, clear custom_status in health
                    let mut health = self.liveness_store.get_health(worker_id).await.ok().flatten().unwrap_or_else(|| WorkerHealth {
                        worker_id,
                        last_heartbeat: chrono::Utc::now(),
                        error_count: 0,
                        custom_status: None,
                    });
                    health.custom_status = None;
                    self.liveness_store.update_health(worker_id, health).await.ok();
                    // Update metrics: increment completed and total_work_items
                    let mut metrics = self.metrics_store.get_metrics(&run_id).await.ok().flatten().unwrap_or_default();
                    metrics.completed += 1;
                    metrics.total_work_items += 1;
                    self.metrics_store.update_metrics(&run_id, metrics).await.ok();
                    // Mark run as completed only when Ok(Some(...)) is returned (terminal node)
                    let now = chrono::Utc::now();
                    let _ = self.run_info_store.update_status(&run_id, crate::distributed::RunStatus::Completed).await;
                    let _ = self.run_info_store.update_finished_at(&run_id, now).await;
                    return Ok(Some((run_id, output)));
                }
                Ok(None) => {
                    // Update health and metrics for in-progress work
                    let mut health = self.liveness_store.get_health(worker_id).await.ok().flatten().unwrap_or_else(|| WorkerHealth {
                        worker_id,
                        last_heartbeat: chrono::Utc::now(),
                        error_count: 0,
                        custom_status: None,
                    });
                    health.custom_status = Some("in progress".to_string());
                    self.liveness_store.update_health(worker_id, health).await.ok();
                    let mut metrics = self.metrics_store.get_metrics(&run_id).await.ok().flatten().unwrap_or_default();
                    metrics.total_work_items += 1;
                    self.metrics_store.update_metrics(&run_id, metrics).await.ok();
                    return Ok(None);
                }
                Err(e) => {
                    last_err = e;
                    // Update error_count and custom_status in health
                    let mut health = self.liveness_store.get_health(worker_id).await.ok().flatten().unwrap_or_else(|| WorkerHealth {
                        worker_id,
                        last_heartbeat: chrono::Utc::now(),
                        error_count: 0,
                        custom_status: None,
                    });
                    health.error_count += 1;
                    if let Some(policy) = policy {
                        if policy.should_retry(&last_err.error, attempt) {
                            health.custom_status = Some(format!("retrying (attempt {}/{})", attempt, max_attempts));
                        } else {
                            health.custom_status = Some("permanently failed".to_string());
                        }
                    } else {
                        health.custom_status = Some("permanently failed".to_string());
                    }
                    self.liveness_store.update_health(worker_id, health).await.ok();
                    // Record error in error_store
                    let run_id = last_err.run_id.clone().unwrap_or_else(|| format!("worker-{}-unknown-run", worker_id));
                    let work_item = last_err.work_item.as_ref().map(|w| format!("{:?}", w)).unwrap_or_else(|| format!("worker_id={}", worker_id));
                    let workflow_error = WorkflowError {
                        work_item,
                        error: format!("{:?}", last_err),
                        attempt,
                        timestamp: chrono::Utc::now(),
                    };
                    let _ = self.error_store.record_error(&run_id, workflow_error).await;
                    // Update metrics: increment failed or retries
                    let mut metrics = self.metrics_store.get_metrics(&run_id).await.ok().flatten().unwrap_or_default();
                    if let Some(policy) = policy {
                        if policy.should_retry(&last_err.error, attempt) {
                            metrics.retries += 1;
                        } else {
                            metrics.failed += 1;
                        }
                    } else {
                        metrics.failed += 1;
                    }
                    self.metrics_store.update_metrics(&run_id, metrics).await.ok();
                    // On permanent failure, mark run as failed
                    if let Some(policy) = policy {
                        if !policy.should_retry(&last_err.error, attempt) {
                            let now = chrono::Utc::now();
                            self.run_info_store.update_status(&run_id, crate::distributed::RunStatus::Failed).await.ok();
                            self.run_info_store.update_finished_at(&run_id, now).await.ok();
                            self.queue.purge_run(&run_id).await.ok();
                        }
                    } else {
                        let now = chrono::Utc::now();
                        self.run_info_store.update_status(&run_id, crate::distributed::RunStatus::Failed).await.ok();
                        self.run_info_store.update_finished_at(&run_id, now).await.ok();
                        self.queue.purge_run(&run_id).await.ok();
                    }
                    // Retry or break
                    if let Some(policy) = policy {
                        if policy.should_retry(&last_err.error, attempt) {
                            let backoff = policy.backoff_duration(attempt);
                            tokio::time::sleep(backoff).await;
                            attempt += 1;
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        Err(last_err.error)
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
        loop {
            match self.run_once(worker_id).await {
                Ok(Some((_run_id, _output))) => {
                    // Work was done, continue immediately
                    // Optionally: log or metrics
                }
                Ok(None) => {
                    // No work available, sleep before polling again
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!(worker_id, error = ?e, "Worker encountered error in run_once");
                    sleep(Duration::from_millis(100)).await;
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
        let mut health = self.liveness_store.get_health(worker_id).await.ok().flatten().unwrap_or_else(|| WorkerHealth {
            worker_id,
            last_heartbeat: now,
            error_count: 0,
            custom_status: None,
        });
        health.last_heartbeat = now;
        let _ = self.liveness_store.update_health(worker_id, health).await;
    }
}

pub struct WorkerBuilder<W, C, Q, S, RIS, MS, ES, LS> {
    workflow: Option<W>,
    queue: Option<Q>,
    checkpoint_store: Option<S>,
    run_info_store: Option<RIS>,
    metrics_store: Option<MS>,
    error_store: Option<ES>,
    liveness_store: Option<LS>,
    retry_policy: Option<RetryPolicy>,
    _phantom: std::marker::PhantomData<C>,
}

impl<W, C, Q, S, RIS, MS, ES, LS> WorkerBuilder<W, C, Q, S, RIS, MS, ES, LS> {
    pub fn new() -> Self {
        Self {
            workflow: None,
            queue: None,
            checkpoint_store: None,
            run_info_store: None,
            metrics_store: None,
            error_store: None,
            liveness_store: None,
            retry_policy: None,
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn workflow(mut self, workflow: W) -> Self { self.workflow = Some(workflow); self }
    pub fn queue(mut self, queue: Q) -> Self { self.queue = Some(queue); self }
    pub fn checkpoint_store(mut self, checkpoint_store: S) -> Self { self.checkpoint_store = Some(checkpoint_store); self }
    pub fn run_info_store(mut self, ris: RIS) -> Self { self.run_info_store = Some(ris); self }
    pub fn metrics_store(mut self, ms: MS) -> Self { self.metrics_store = Some(ms); self }
    pub fn error_store(mut self, es: ES) -> Self { self.error_store = Some(es); self }
    pub fn liveness_store(mut self, ls: LS) -> Self { self.liveness_store = Some(ls); self }
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self { self.retry_policy = Some(policy); self }
    pub fn with_in_memory_defaults() -> WorkerBuilder<W, C, Q, S, crate::distributed::InMemoryRunInfoStore, crate::distributed::InMemoryMetricsStore, crate::distributed::InMemoryErrorStore, crate::distributed::InMemoryLivenessStore> {
        WorkerBuilder {
            workflow: None,
            queue: None,
            checkpoint_store: None,
            run_info_store: Some(crate::distributed::InMemoryRunInfoStore::default()),
            metrics_store: Some(crate::distributed::InMemoryMetricsStore::default()),
            error_store: Some(crate::distributed::InMemoryErrorStore::default()),
            liveness_store: Some(crate::distributed::InMemoryLivenessStore::default()),
            retry_policy: None,
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn build(self) -> Result<DistributedWorker<W, C, Q, S, RIS, MS, ES, LS>, String>
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
        Ok(DistributedWorker {
            workflow: self.workflow.ok_or("workflow is required")?,
            queue: self.queue.ok_or("queue is required")?,
            checkpoint_store: self.checkpoint_store.ok_or("checkpoint_store is required")?,
            run_info_store: self.run_info_store.ok_or("run_info_store is required")?,
            metrics_store: self.metrics_store.ok_or("metrics_store is required")?,
            error_store: self.error_store.ok_or("error_store is required")?,
            liveness_store: self.liveness_store.ok_or("liveness_store is required")?,
            retry_policy: Some(self.retry_policy.unwrap_or_else(|| RetryPolicy::new(
                5,
                std::time::Duration::from_millis(100),
                std::time::Duration::from_secs(2),
                BackoffStrategy::Exponential,
                RetryError::All,
            ))),
            phantom: std::marker::PhantomData,
        })
    }
}

/// A pool of distributed workflow workers, each running in its own async task.
///
/// The pool manages worker lifecycles, graceful shutdown, and health reporting.
pub struct WorkerPool<W, C, Q, S, RIS, MS, ES, LS>
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
    worker: DistributedWorker<W, C, Q, S, RIS, MS, ES, LS>,
    num_workers: usize,
    handles: Vec<JoinHandle<()>>,
    cancel_tokens: Vec<CancellationToken>,
}

impl<W, C, Q, S, RIS, MS, ES, LS> WorkerPool<W, C, Q, S, RIS, MS, ES, LS>
where
    W: Workflow<C> + 'static,
    C: Debug + Clone + Send + Sync + 'static,
    Q: WorkQueue<W::WorkItem> + Send + Sync + Clone + 'static,
    S: CheckpointStore<C, W::WorkItem> + Send + Sync + Clone + 'static,
    RIS: RunInfoStore + Send + Sync + Clone + 'static,
    MS: MetricsStore + Send + Sync + Clone + 'static,
    ES: ErrorStore + Send + Sync + Clone + 'static,
    LS: LivenessStore + Send + Sync + Clone + 'static,
{
    /// Create a new worker pool with the given worker and number of workers.
    pub fn new(worker: DistributedWorker<W, C, Q, S, RIS, MS, ES, LS>, num_workers: usize) -> Self {
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

    /// Gracefully stop all workers by signalling cancellation.
    pub async fn stop(&mut self) {
        for token in &self.cancel_tokens {
            token.cancel();
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
        self.worker.liveness_store.list_health().await.unwrap_or_default()
    }
} 