use crate::transition::Transition;
use async_trait::async_trait;

/// A node takes an input and a context, and returns a transition.
#[async_trait]
pub trait Node<C=()>: Send + Sync + 'static
where
    C: Clone + Send + Sync + 'static,
{
    /// Input type for the node
    type Input: Send + 'static;
    /// Output type produced by the node
    type Output: Send + 'static;

    /// Process an input value within the given context, producing a transition
    async fn process(
        &self,
        ctx: &C,
        input: Self::Input,
    ) -> Result<Transition<Self::Output>, crate::error::FloxideError>;
}