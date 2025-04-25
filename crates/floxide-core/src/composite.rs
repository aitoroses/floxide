use crate::{Workflow, Node, Transition, WorkflowCtx, error::FloxideError};
use std::fmt::Debug;
use async_trait::async_trait;

/// CompositeNode wraps a Workflow so it implements Node
#[derive(Clone, Debug)]
pub struct CompositeNode<C, W> {
    ctx: WorkflowCtx<C>,
    workflow: W,
}

impl<C: Clone, W> CompositeNode<C, W> {
    pub fn new(workflow: W, ctx: &WorkflowCtx<C>) -> Self {
        CompositeNode { ctx: ctx.clone(), workflow }
    }
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