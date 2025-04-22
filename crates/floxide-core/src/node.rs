use crate::transition::Transition;
use async_trait::async_trait;

/// A node takes an input and a context, and returns a transition.
#[async_trait]
pub trait Node: Send + Sync + 'static {
    type Input : Send + 'static;
    type Output: Send + 'static;

    async fn process<C>(
        &self,
        ctx: &C,
        input: Self::Input
    ) -> Result<Transition<Self::Output>, crate::error::FloxideError>
    where Self: Sized + Node, C: Clone + Send + Sync + 'static;
}