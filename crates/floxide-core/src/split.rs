use crate::node::Node;
use crate::transition::Transition;
use crate::error::FloxideError;
use async_trait::async_trait;
use std::marker::PhantomData;

/// A node that splits its input into multiple outputs using the provided function.
///
/// Given an input `I`, the splitter function returns `Vec<O>`, and each element
/// is emitted via a `Transition::NextAll` to the workflow engine.
#[derive(Clone, Debug)]
pub struct SplitNode<I, O, F>
where
    F: Fn(I) -> Vec<O> + Send + Sync,
{
    splitter: F,
    _phantom: PhantomData<(I, O)>,
}

impl<I, O, F> SplitNode<I, O, F>
where
    F: Fn(I) -> Vec<O> + Send + Sync,
{
    /// Create a new SplitNode from a function that maps an input to a Vec of outputs.
    pub fn new(splitter: F) -> Self {
        SplitNode { splitter, _phantom: PhantomData }
    }
}

#[async_trait]
impl<C, I, O, F> Node<C> for SplitNode<I, O, F>
where
    C: Send + Sync + 'static,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: Fn(I) -> Vec<O> + Send + Sync,
{
    /// Input type for the split node.
    type Input = I;
    /// Output type produced by the split node.
    type Output = O;

    /// Process an input value, producing multiple outputs via NextAll transition.
    async fn process(
        &self,
        _ctx: &C,
        input: I,
    ) -> Result<Transition<O>, FloxideError> {
        let outputs = (self.splitter)(input);
        Ok(Transition::NextAll(outputs))
    }
}