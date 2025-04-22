use crate::transition::Transition;
use crate::context::WorkflowCtx;
use async_trait::async_trait;

/// Un nodo toma `Input` + `&mut WorkflowCtx` y produce un `Transition<Self>`
#[async_trait]
pub trait Node: Send + Sync + 'static {
    type Input : Send + 'static;
    type Output: Send + 'static;

    async fn process<C>(
        &self,
        ctx: &mut WorkflowCtx<C>,
        input: Self::Input
    ) -> Result<Transition<Self::Output>, crate::error::FloxideError>
    where Self: Sized + Node, C: Clone + Send + Sync + 'static;
}