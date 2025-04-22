use std::fmt::Debug;

// crates/floxide-core/src/workflow.rs
use async_trait::async_trait;
use crate::{context::WorkflowCtx, error::FloxideError, Node, Transition};

#[async_trait]
pub trait Workflow: Debug + Clone + Send + Sync + 'static {
    type Input: Send + 'static;
    /// Output type returned by the workflow's terminal branch
    type Output: Send + 'static;
    /// Execute the workflow, returning the output of the terminal branch
    async fn run<D>(
        &mut self,
        ctx: &mut WorkflowCtx<D>,
        input: Self::Input
    ) -> Result<Self::Output, FloxideError>
    where D: Clone + Send + Sync + 'static;
}

/// CompositeNode wraps a Workflow so it implements Node
#[derive(Clone, Debug)]
pub struct CompositeNode<W> {
    workflow: W,
}

impl<W> CompositeNode<W> {
    pub fn new(workflow: W) -> Self { CompositeNode { workflow } }
}

#[async_trait]
impl<W> Node for CompositeNode<W>
where
    W: Workflow + Clone + Send + Sync + 'static,
    W::Input: Send + 'static,
    W::Output: Send + 'static,
{
    type Input = W::Input;
    type Output = W::Output;
    async fn process<D>(
        &self,
        ctx: &mut WorkflowCtx<D>,
        input: Self::Input,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where D: Clone + Send + Sync + 'static {
        let mut inner = self.workflow.clone();
        let out = inner.run(ctx, input).await?;
        Ok(Transition::Next(out))
    }
}