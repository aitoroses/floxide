//! # Workflow Execution Modes in Floxide
//!
//! Floxide workflows can be executed in several modes, each suited to different use cases:
//!
//! ## 1. Monolithic (Single-Process) Execution
//! - **Method:** [`run`]
//! - **Description:** Runs the entire workflow from start to finish in a single process, with no checkpointing or distributed coordination.
//! - **Use Case:** Simple, local workflows where failure recovery and distributed scaling are not needed.
//!
//! ## 2. Checkpointed (Resumable) Execution
//! - **Method:** [`run_with_checkpoint`]
//! - **Description:** Runs the workflow, checkpointing state after each step. If interrupted, can be resumed from the last checkpoint.
//! - **Use Case:** Long-running workflows, or those that need to recover from process crashes or restarts.
//!
//! ## 3. Resuming from Checkpoint
//! - **Method:** [`resume`]
//! - **Description:** Resumes a workflow run from its last checkpoint, restoring context and queue from the store.
//! - **Use Case:** Recovery after failure, or continuing a workflow after a pause.
//!
//! ## 4. Distributed Execution Primitives
//! - **a. Orchestration/Seeding**
//!   - **Method:** [`start_distributed`]
//!   - **Description:** Seeds the distributed workflow by checkpointing the initial state and enqueuing the first work item(s). Does not execute any steps.
//!   - **Use Case:** Used by an orchestrator to start a new distributed workflow run, preparing it for workers to process.
//! - **b. Worker Step**
//!   - **Method:** [`step_distributed`]
//!   - **Description:** A distributed worker dequeues a work item, loads the latest checkpoint, processes the node, enqueues successors, and persists state. Returns output if a terminal node is reached.
//!   - **Use Case:** Used by distributed workers to process workflow steps in parallel, with coordination via distributed queue and checkpoint store.
//!
//! These distributed methods are the core primitives for building scalable, fault-tolerant workflow systems in Floxide.
//!
use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::sync::Arc;

// crates/floxide-core/src/workflow.rs
use crate::context::Context;
use crate::distributed::{StepCallbacks, StepError, WorkQueue};
use crate::error::FloxideError;
use crate::{Checkpoint, CheckpointStore};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{debug, info, span, Level};

/// Trait for a workflow work item.
///
/// Implementations provide a way to serialize and deserialize work items, and
/// track unique instances within a workflow run.
pub trait WorkItem:
    Debug + Display + Send + Sync + Serialize + DeserializeOwned + Clone + PartialEq + Eq
{
    /// Returns a unique identifier for this work item instance.
    fn instance_id(&self) -> String;
    /// Returns true if this work item is a terminal node (no successors)
    fn is_terminal(&self) -> bool;
}

/// Trait for a workflow.
///
#[async_trait]
pub trait Workflow<C: Context>: Debug + Clone + Send + Sync {
    /// Input type for the workflow
    type Input: Send + Sync + Serialize + DeserializeOwned;
    /// Output type returned by the workflow's terminal branch
    type Output: Send + Sync + Serialize + DeserializeOwned;
    /// Workflow-specific work item type (macro-generated enum)
    type WorkItem: WorkItem;

    /// Name of the workflow, used for logging and tracing.
    fn name(&self) -> &'static str;

    /// Create the initial work item for the workflow start node.
    fn start_work_item(&self, input: Self::Input) -> Self::WorkItem;

    /// Execute the workflow, returning the output of the terminal branch.
    ///
    /// Default implementation dispatches work items via `process_work_item`.
    async fn run<'a>(
        &'a self,
        ctx: &'a crate::WorkflowCtx<C>,
        input: Self::Input,
    ) -> Result<Self::Output, FloxideError> {
        let span = span!(Level::INFO, "workflow_run", workflow = self.name());
        let _enter = span.enter();
        let mut queue: VecDeque<Self::WorkItem> = VecDeque::new();
        queue.push_back(self.start_work_item(input));
        while let Some(item) = queue.pop_front() {
            debug!(?item, queue_len = queue.len(), "Processing work item");
            if let Some(output) = self.process_work_item(ctx, item, &mut queue).await? {
                return Ok(output);
            }
            debug!(queue_len = queue.len(), "Queue state after processing");
        }
        unreachable!("Workflow did not reach terminal branch");
    }

    /// Process a single work item, returning the next work item to be processed if any.
    ///
    /// This is the core primitive for distributed execution: given a work item (node + input),
    /// processes it and enqueues any successor work items. If the work item is a terminal node,
    /// returns its output.
    ///
    /// # Arguments
    /// * `ctx` - The workflow context.
    /// * `item` - The work item to process (node + input).
    /// * `__q` - The work queue for successor items (used internally).
    ///
    /// # Returns
    /// * `Ok(Some(Self::Output))` - If this work item was a terminal node and produced output.
    /// * `Ok(None)` - If more work remains (successors enqueued).
    /// * `Err(FloxideError)` - If the node processing failed or aborted.
    async fn process_work_item<'a>(
        &'a self,
        ctx: &'a crate::WorkflowCtx<C>,
        item: Self::WorkItem,
        queue: &mut std::collections::VecDeque<Self::WorkItem>,
    ) -> Result<Option<Self::Output>, FloxideError>;

    /// Execute the workflow with checkpointing, saving state after each step.
    ///
    /// This allows the workflow to be resumed after interruption or failure.
    ///
    /// # Arguments
    /// * `ctx` - The workflow context.
    /// * `input` - The input to the workflow's start node.
    /// * `store` - The checkpoint store to persist state.
    /// * `id` - The unique run ID for this workflow execution.
    ///
    /// # Returns
    /// * `Ok(Self::Output)` - The output of the terminal node if the workflow completes successfully.
    /// * `Err(FloxideError)` - If any node returns an error or aborts.
    async fn run_with_checkpoint<CS: CheckpointStore<C, Self::WorkItem> + Send + Sync>(
        &self,
        ctx: &crate::WorkflowCtx<C>,
        input: Self::Input,
        store: &CS,
        id: &str,
    ) -> Result<Self::Output, FloxideError> {
        use std::collections::VecDeque;
        let span = span!(
            Level::INFO,
            "workflow_run_with_checkpoint",
            workflow = self.name(),
            run_id = id
        );
        let _enter = span.enter();
        // load existing checkpoint or start new
        let mut cp: Checkpoint<C, Self::WorkItem> = match store
            .load(id)
            .await
            .map_err(|e| FloxideError::Generic(e.to_string()))?
        {
            Some(saved) => {
                debug!("Loaded existing checkpoint");
                saved
            }
            None => {
                debug!("No checkpoint found, starting new");
                let mut init_q = VecDeque::new();
                init_q.push_back(self.start_work_item(input));
                Checkpoint::new(ctx.store.clone(), init_q)
            }
        };
        let mut queue = cp.queue.clone();
        if queue.is_empty() {
            info!("Workflow already completed (empty queue)");
            return Err(FloxideError::AlreadyCompleted);
        }
        while let Some(item) = queue.pop_front() {
            debug!(?item, queue_len = queue.len(), "Processing work item");
            if let Some(output) = self.process_work_item(ctx, item, &mut queue).await? {
                return Ok(output);
            }
            debug!(queue_len = queue.len(), "Queue state after processing");
            cp.context = ctx.store.clone();
            cp.queue = queue.clone();
            store
                .save(id, &cp)
                .await
                .map_err(|e| FloxideError::Generic(e.to_string()))?;
            debug!("Checkpoint saved");
        }
        unreachable!("Workflow did not reach terminal branch");
    }

    /// Resume a workflow run from its last checkpoint, restoring context and queue from the store.
    ///
    /// # Arguments
    /// * `store` - The checkpoint store containing saved state.
    /// * `id` - The unique run ID for this workflow execution.
    ///
    /// # Returns
    /// * `Ok(Self::Output)` - The output of the terminal node if the workflow completes successfully.
    /// * `Err(FloxideError)` - If any node returns an error or aborts, or if no checkpoint is found.
    async fn resume<CS: CheckpointStore<C, Self::WorkItem> + Send + Sync>(
        &self,
        store: &CS,
        id: &str,
    ) -> Result<Self::Output, FloxideError> {
        use std::collections::VecDeque;
        let span = span!(
            Level::INFO,
            "workflow_resume",
            workflow = self.name(),
            checkpoint_id = id
        );
        let _enter = span.enter();
        // load persisted checkpoint or error
        let mut cp = store
            .load(id)
            .await
            .map_err(|e| FloxideError::Generic(e.to_string()))?
            .ok_or_else(|| FloxideError::NotStarted)?;
        debug!("Loaded checkpoint for resume");
        let wf_ctx = crate::WorkflowCtx::new(cp.context.clone());
        let ctx = &wf_ctx;
        let mut queue: VecDeque<Self::WorkItem> = cp.queue.clone();
        if queue.is_empty() {
            info!("Workflow already completed (empty queue)");
            return Err(FloxideError::AlreadyCompleted);
        }
        // If the queue contains exactly one item and it is terminal, treat as already completed
        if queue.len() == 1 && queue.front().map(|item| item.is_terminal()).unwrap_or(false) {
            info!("Workflow already completed (terminal node in queue)");
            return Err(FloxideError::AlreadyCompleted);
        }
        while let Some(item) = queue.pop_front() {
            debug!(?item, queue_len = queue.len(), "Processing work item");
            if let Some(output) = self.process_work_item(ctx, item, &mut queue).await? {
                return Ok(output);
            }
            cp.context = ctx.store.clone();
            cp.queue = queue.clone();
            store
                .save(id, &cp)
                .await
                .map_err(|e| FloxideError::Generic(e.to_string()))?;
            debug!("Checkpoint saved");
            debug!(queue_len = queue.len(), "Queue state after processing");
        }
        unreachable!("Workflow did not reach terminal branch");
    }

    /// Orchestrator primitive: seed the distributed workflow (checkpoint + queue) but do not execute steps.
    ///
    /// This method is used to initialize a distributed workflow run, creating the initial checkpoint and enqueuing the first work item(s).
    /// No workflow steps are executed by this method; workers will process the steps via `step_distributed`.
    ///
    /// # Arguments
    /// * `ctx` - The workflow context.
    /// * `input` - The input to the workflow's start node.
    /// * `store` - The checkpoint store to persist state.
    /// * `queue` - The distributed work queue.
    /// * `id` - The unique run ID for this workflow execution.
    ///
    /// # Returns
    /// * `Ok(())` - If the workflow was successfully seeded.
    /// * `Err(FloxideError)` - If checkpointing or queueing failed.
    async fn start_distributed<CS, Q>(
        &self,
        ctx: &crate::WorkflowCtx<C>,
        input: Self::Input,
        store: &CS,
        queue: &Q,
        id: &str,
    ) -> Result<(), FloxideError>
    where
        CS: CheckpointStore<C, Self::WorkItem> + Send + Sync,
        Q: WorkQueue<C, Self::WorkItem> + Send + Sync,
    {
        use std::collections::VecDeque;
        let seed_span =
            span!(Level::DEBUG, "start_distributed", workflow = self.name(), run_id = %id);
        let _enter = seed_span.enter();
        debug!(run_id = %id, "start_distributed seeding");
        // seed initial checkpoint and queue if not present
        if store
            .load(id)
            .await
            .map_err(|e| FloxideError::Generic(e.to_string()))?
            .is_none()
        {
            let item = self.start_work_item(input);
            let mut init_q = VecDeque::new();
            init_q.push_back(item.clone());
            let cp0 = Checkpoint::new(ctx.store.clone(), init_q);
            store
                .save(id, &cp0)
                .await
                .map_err(|e| FloxideError::Generic(e.to_string()))?;
            queue
                .enqueue(id, item)
                .await
                .map_err(|e| FloxideError::Generic(e.to_string()))?;
        }
        Ok(())
    }

    /// Worker primitive: perform one distributed step (dequeue, process, enqueue successors, persist).
    ///
    /// This method is called by distributed workers to process a single work item for any workflow run.
    /// It loads the latest checkpoint, processes the node, enqueues successors, and persists state.
    /// If a terminal node is reached, returns the output.
    ///
    /// # Arguments
    /// * `store` - The checkpoint store containing saved state.
    /// * `queue` - The distributed work queue.
    /// * `worker_id` - The unique ID of the worker processing this step.
    ///
    /// # Returns
    /// * `Ok(Some((run_id, output)))` - If a terminal node was processed and output produced.
    /// * `Ok(None)` - If more work remains for this run.
    /// * `Err(StepError)` - If processing failed or checkpointing/queueing failed.
    async fn step_distributed<CS, Q>(
        &self,
        store: &CS,
        queue: &Q,
        worker_id: usize,
        callbacks: Arc<dyn StepCallbacks<C, Self>>,
    ) -> Result<Option<(String, Self::Output)>, StepError<Self::WorkItem>>
    where
        C: 'static,
        CS: crate::checkpoint::CheckpointStore<C, Self::WorkItem> + Send + Sync,
        Q: crate::distributed::WorkQueue<C, Self::WorkItem> + Send + Sync,
    {
        use crate::distributed::ItemProcessedOutcome;
        use tracing::{debug, span, Level};
        // dequeue one item
        let work = queue.dequeue().await.map_err(|e| StepError {
            error: FloxideError::Generic(e.to_string()),
            run_id: None,
            work_item: None,
        })?;
        let (run_id, item) = match work {
            None => return Ok(None),
            Some((rid, it)) => (rid, it),
        };
        let step_span = span!(Level::DEBUG, "step_distributed",
            workflow = self.name(), run_id = %run_id, worker = worker_id);
        let _enter = step_span.enter();
        debug!(worker = worker_id, run_id = %run_id, ?item, "Worker dequeued item");
        // Call on_started and abort if it returns an error
        let on_started_result = callbacks.on_started(run_id.clone(), item.clone()).await;
        if let Err(e) = on_started_result {
            return Err(StepError {
                error: FloxideError::Generic(format!("on_started_state_updates failed: {:?}", e)),
                run_id: Some(run_id.clone()),
                work_item: Some(item.clone()),
            });
        }
        // load checkpoint
        let mut cp = store
            .load(&run_id)
            .await
            .map_err(|e| StepError {
                error: FloxideError::Generic(e.to_string()),
                run_id: Some(run_id.clone()),
                work_item: Some(item.clone()),
            })?
            .ok_or_else(|| StepError {
                error: FloxideError::NotStarted,
                run_id: Some(run_id.clone()),
                work_item: Some(item.clone()),
            })?;
        debug!(worker = worker_id, run_id = %run_id, queue_len = cp.queue.len(), "Loaded checkpoint");
        let wf_ctx = crate::WorkflowCtx::new(cp.context.clone());
        let ctx_ref = &wf_ctx;
        let mut local_q = cp.queue.clone();
        let _ = local_q.pop_front();
        let old_tail = local_q.clone();
        let process_result = self
            .process_work_item(ctx_ref, item.clone(), &mut local_q)
            .await;
        match process_result {
            Ok(Some(out)) => {
                cp.context = wf_ctx.store.clone();
                cp.queue = local_q.clone();
                store.save(&run_id, &cp).await.map_err(|e| StepError {
                    error: FloxideError::Generic(e.to_string()),
                    run_id: Some(run_id.clone()),
                    work_item: Some(item.clone()),
                })?;
                debug!(worker = worker_id, run_id = %run_id, queue_len = cp.queue.len(), "Checkpoint saved (terminal)");
                let on_item_processed_result = callbacks
                    .on_item_processed(
                        run_id.clone(),
                        item.clone(),
                        ItemProcessedOutcome::SuccessTerminal,
                    )
                    .await;
                if let Err(e) = on_item_processed_result {
                    return Err(StepError {
                        error: e,
                        run_id: Some(run_id.clone()),
                        work_item: Some(item.clone()),
                    });
                }
                return Ok(Some((run_id.clone(), out)));
            }
            Ok(None) => {
                let mut appended = local_q.clone();
                for _ in 0..old_tail.len() {
                    let _ = appended.pop_front();
                }
                for succ in appended.iter() {
                    queue
                        .enqueue(&run_id, succ.clone())
                        .await
                        .map_err(|e| StepError {
                            error: FloxideError::Generic(e.to_string()),
                            run_id: Some(run_id.clone()),
                            work_item: Some(item.clone()),
                        })?;
                }
                cp.context = wf_ctx.store.clone();
                cp.queue = local_q.clone();
                store.save(&run_id, &cp).await.map_err(|e| StepError {
                    error: FloxideError::Generic(e.to_string()),
                    run_id: Some(run_id.clone()),
                    work_item: Some(item.clone()),
                })?;
                debug!(worker = worker_id, run_id = %run_id, queue_len = cp.queue.len(), "Checkpoint saved");
                let on_item_processed_result = callbacks
                    .on_item_processed(
                        run_id.clone(),
                        item.clone(),
                        ItemProcessedOutcome::SuccessNonTerminal,
                    )
                    .await;
                if let Err(e) = on_item_processed_result {
                    return Err(StepError {
                        error: e,
                        run_id: Some(run_id.clone()),
                        work_item: Some(item.clone()),
                    });
                }
                return Ok(None);
            }
            Err(e) => {
                let on_item_processed_result = callbacks
                    .on_item_processed(
                        run_id.clone(),
                        item.clone(),
                        ItemProcessedOutcome::Error(e.clone()),
                    )
                    .await;
                if let Err(e) = on_item_processed_result {
                    return Err(StepError {
                        error: e,
                        run_id: Some(run_id.clone()),
                        work_item: Some(item.clone()),
                    });
                }
                Err(StepError {
                    error: e,
                    run_id: Some(run_id),
                    work_item: Some(item),
                })
            }
        }
    }

    /// Export the workflow definition as a Graphviz DOT string.
    ///
    /// This method returns a static DOT-format string representing the workflow graph, for visualization or debugging.
    fn to_dot(&self) -> &'static str;
}
