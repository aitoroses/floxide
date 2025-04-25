use crate::{context::Context, error::FloxideError, Node, Transition, Workflow, WorkflowCtx};
use std::fmt::Debug;
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
/// CompositeNode wraps a Workflow so it implements Node
#[derive(Clone, Debug)]
pub struct CompositeNode<C: Context, W> {
    ctx: WorkflowCtx<C>,
    workflow: W,
}

impl<C: Context, W> CompositeNode<C, W> {
    pub fn new(workflow: W, ctx: &WorkflowCtx<C>) -> Self {
        CompositeNode { ctx: ctx.clone(), workflow }
    }
}

#[async_trait]
impl<C: Context, W> Node<C> for CompositeNode<C, W>
where
    C: Serialize + DeserializeOwned + Debug + Clone + Send + Sync,
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