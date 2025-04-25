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
use std::fmt::Debug;

// crates/floxide-core/src/workflow.rs
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::context::Context;
use crate::error::FloxideError;
use crate::distributed::{StepError, WorkQueue};
use crate::CheckpointStore;

/// Trait for a workflow work item.
///
/// Implementations provide a way to serialize and deserialize work items.
pub trait WorkItem: Debug + Send + Sync + Serialize + DeserializeOwned + Clone {}

impl<T: Debug + Send + Sync + Serialize + DeserializeOwned + Clone> WorkItem for T {}

/// Trait for a workflow.
///
#[async_trait]
pub trait Workflow<C: Context>: Debug + Clone + Send + Sync
{
    /// Input type for the workflow
    type Input: Send + Sync + Serialize + DeserializeOwned;
    /// Output type returned by the workflow's terminal branch
    type Output: Send + Sync + Serialize + DeserializeOwned;
    /// Workflow-specific work item type (macro-generated enum)
    type WorkItem: WorkItem;

    /// Execute the workflow, returning the output of the terminal branch.
    ///
    /// This runs the workflow from the start node to a terminal node in a single process, without checkpointing or distributed coordination.
    /// Use this for simple, local workflows where failure recovery and distributed scaling are not needed.
    ///
    /// # Arguments
    /// * `ctx` - The workflow context, containing shared state for all nodes.
    /// * `input` - The input to the workflow's start node.
    ///
    /// # Returns
    /// * `Ok(Self::Output)` - The output of the terminal node if the workflow completes successfully.
    /// * `Err(FloxideError)` - If any node returns an error or aborts.
    async fn run<'a>(
        &'a self,
        ctx: &'a crate::WorkflowCtx<C>,
        input: Self::Input,
    ) -> Result<Self::Output, FloxideError>;

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
        __q: &mut std::collections::VecDeque<Self::WorkItem>,
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
    ) -> Result<Self::Output, FloxideError>;

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
    ) -> Result<Self::Output, FloxideError>;

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
        Q: WorkQueue<C, Self::WorkItem> + Send + Sync;

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
    ) -> Result<Option<(String, Self::Output)>, StepError<Self::WorkItem>>
    where
        CS: crate::checkpoint::CheckpointStore<C, Self::WorkItem> + Send + Sync,
        Q: crate::distributed::WorkQueue<C, Self::WorkItem> + Send + Sync;

    /// Export the workflow definition as a Graphviz DOT string.
    ///
    /// This method returns a static DOT-format string representing the workflow graph, for visualization or debugging.
    fn to_dot(&self) -> &'static str;
}