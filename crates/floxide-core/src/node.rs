use crate::{context::Context, transition::Transition};
use async_trait::async_trait;

/// A node takes an input and a context, and returns a transition.
#[async_trait]
pub trait Node<C: Context = ()>: Send + Sync {
    /// Input type for the node
    type Input: Send + Sync;
    /// Output type produced by the node
    type Output: Send + Sync;

    /// Process an input value within the given context, producing a transition
    async fn process(
        &self,
        ctx: &C,
        input: Self::Input,
    ) -> Result<Transition<Self::Output>, crate::error::FloxideError>;
}
