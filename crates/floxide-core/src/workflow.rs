use std::fmt::Debug;

// crates/floxide-core/src/workflow.rs
use async_trait::async_trait;
use crate::{error::FloxideError, Node, Transition, WorkflowCtx};

#[async_trait]
pub trait Workflow<C>: Debug + Clone + Send + Sync
where
    C: Debug + Clone + Send + Sync,
{
    /// Input type for the workflow
    type Input: Send + Sync;
    /// Output type returned by the workflow's terminal branch
    type Output: Send + Sync;
    /// Workflow-specific work item type (macro-generated enum)
    type WorkItem: Send + Sync + Clone + Debug + 'static;

    /// Execute the workflow, returning the output of the terminal branch
    async fn run<'a>(
        &'a self,
        ctx: &'a crate::WorkflowCtx<C>,
        input: Self::Input,
    ) -> Result<Self::Output, FloxideError>;

    /// Process a work item, returning the next work item to be processed.
    async fn process_work_item<'a>(
        &'a self,
        ctx: &'a crate::WorkflowCtx<C>,
        item: Self::WorkItem,
        __q: &mut std::collections::VecDeque<Self::WorkItem>,
    ) -> Result<Option<Self::Output>, FloxideError>;

    /// Execute the workflow, checkpointing state after each step.
    async fn run_with_checkpoint<CS: crate::checkpoint::CheckpointStore<C, Self::WorkItem> + Send + Sync>(
        &self,
        ctx: &crate::WorkflowCtx<C>,
        input: Self::Input,
        store: &CS,
        id: &str,
    ) -> Result<Self::Output, FloxideError>;

    /// Resume a workflow run from its last checkpoint; context and queue are restored from store.
    async fn resume<CS: crate::checkpoint::CheckpointStore<C, Self::WorkItem> + Send + Sync>(
        &self,
        store: &CS,
        id: &str,
    ) -> Result<Self::Output, FloxideError>;

    /// Orchestrator: seed the distributed workflow (checkpoint + queue) but do not execute steps.
    async fn start_distributed<CS, Q>(
        &self,
        ctx: &crate::WorkflowCtx<C>,
        input: Self::Input,
        store: &CS,
        queue: &Q,
        id: &str,
    ) -> Result<(), FloxideError>
    where
        CS: crate::checkpoint::CheckpointStore<C, Self::WorkItem> + Send + Sync,
        Q: crate::distributed::WorkQueue<Self::WorkItem> + Send + Sync;

    /// Worker: perform one distributed step (dequeue, process, enqueue successors, persist).
    async fn step_distributed<CS, Q>(
        &self,
        store: &CS,
        queue: &Q,
        worker_id: usize,
    ) -> Result<Option<(String, Self::Output)>, FloxideError>
    where
        CS: crate::checkpoint::CheckpointStore<C, Self::WorkItem> + Send + Sync,
        Q: crate::distributed::WorkQueue<Self::WorkItem> + Send + Sync;

    /// Export the workflow definition as a Graphviz DOT string.
    fn to_dot(&self) -> &'static str;
}

/// CompositeNode wraps a Workflow so it implements Node
#[derive(Clone, Debug)]
pub struct CompositeNode<C, W> {
    ctx: WorkflowCtx<C>,
    workflow: W,
}

impl<C: Clone, W> CompositeNode<C, W> {
    pub fn new(workflow: W, ctx: &WorkflowCtx<C>) -> Self { CompositeNode { ctx: ctx.clone(), workflow } }
}

#[async_trait]
impl<C, W> Node<C> for CompositeNode<C, W>
where
    C: Debug + Clone + Send + Sync,
    W: Workflow<C> + Clone + Send + Sync,
    W::Input: Send + Sync,
    W::Output: Send + Sync,
{
    type Input = W::Input;
    type Output = W::Output;

    async fn process(
        &self,
        ctx: &C,
        input: Self::Input,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let mut w_ctx = self.ctx.clone();
        w_ctx.store = ctx.clone();
        let out = self.workflow.run(&w_ctx, input).await?;
        Ok(Transition::Next(out))
    }
}