use std::fmt::Debug;

// crates/floxide-core/src/workflow.rs
use async_trait::async_trait;
use crate::{error::FloxideError, Node, Transition};

#[async_trait]
pub trait Workflow<C>: Debug + Clone + Send + Sync + 'static
where
    C: Clone + Send + Sync + 'static,
{
    /// Input type for the workflow
    type Input: Send + 'static;
    /// Output type returned by the workflow's terminal branch
    type Output: Send + 'static;

    /// Execute the workflow, returning the output of the terminal branch
    async fn run(
        &self,
        ctx: &C,
        input: Self::Input,
    ) -> Result<Self::Output, FloxideError>;
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
impl<C, W> Node<C> for CompositeNode<W>
where
    C: Clone + Send + Sync + 'static,
    W: Workflow<C> + Clone + Send + Sync + 'static,
    W::Input: Send + 'static,
    W::Output: Send + 'static,
{
    type Input = W::Input;
    type Output = W::Output;

    async fn process(
        &self,
        ctx: &C,
        input: Self::Input,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let inner = self.workflow.clone();
        let out = inner.run(ctx, input).await?;
        Ok(Transition::Next(out))
    }
}