use crate::node::Node;
use crate::transition::Transition;
use crate::error::FloxideError;
use async_trait::async_trait;
use tokio::task;
use futures::future::join_all;
use std::vec::Vec;

/// A node adapter that runs an inner node on a batch of inputs, collecting outputs in parallel
#[derive(Clone, Debug)]
pub struct BatchNode<N> {
    pub node: N,
    pub batch_size: usize,
}

impl<N> BatchNode<N> {
    /// Wraps an existing node into a batch adapter with a batch size
    pub fn new(node: N, batch_size: usize) -> Self {
        BatchNode { node, batch_size }
    }
}

impl<N> BatchNode<N>
where
    N: Node + Clone + Send + Sync + 'static,
    <N as Node>::Input: Clone + Send + 'static,
    <N as Node>::Output: Send + 'static,
{
    /// Process a batch of inputs, where the associated Input/Output types are Vecs
pub async fn process_batch<C>(
        &self,
        // take ownership of the context so it can be cloned into blocking tasks
        ctx: C,
        inputs: <Self as Node>::Input,
    ) -> Result<<Self as Node>::Output, FloxideError>
    where
        C: Clone + Send + Sync + 'static,
    {
        let mut outputs = Vec::new();
        let node = self.node.clone();
        let ctx_clone = ctx.clone();
        let mut tasks = Vec::new();

        for input in inputs.into_iter() {
            let node = node.clone();
            // clone the context reference (does not require C: Clone)
            let ctx = ctx_clone.clone();
            let task = task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async move {
                    node.process(&ctx, input).await
                })
            });
            tasks.push(task);

            if tasks.len() >= self.batch_size {
                let results = join_all(tasks).await;
                tasks = Vec::new();
                for res in results {
                    match res {
                        Ok(Ok(Transition::Next(o))) => outputs.push(o),
                        Ok(Ok(Transition::Finish)) => {},
                        Ok(Ok(Transition::Abort(e))) => return Err(e),
                        Ok(Err(e)) => return Err(e),
                        Err(e) => return Err(FloxideError::Generic(format!("Join error: {e}"))),
                    }
                }
            }
        }

        if !tasks.is_empty() {
            let results = join_all(tasks).await;
            for res in results {
                match res {
                    Ok(Ok(Transition::Next(o))) => outputs.push(o),
                    Ok(Ok(Transition::Finish)) => {},
                    Ok(Ok(Transition::Abort(e))) => return Err(e),
                    Ok(Err(e)) => return Err(e),
                    Err(e) => return Err(FloxideError::Generic(format!("Join error: {e}"))),
                }
            }
        }

        Ok(outputs)
    }
}

#[async_trait]
impl<N> Node for BatchNode<N>
where
    N: Node + Clone + Send + Sync + 'static,
    <N as Node>::Input: Clone + Send + 'static,
    <N as Node>::Output: Send + 'static,
{
    type Input = Vec<<N as Node>::Input>;
    type Output = Vec<<N as Node>::Output>;

    async fn process<C>(
        &self,
        ctx: &C,
        inputs: Self::Input,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Clone + Send + Sync + 'static,
    {
        // clone the owned context for batch processing
        // clone the owned context for batch processing
        let outputs = self.process_batch((*ctx).clone(), inputs).await?;
        Ok(Transition::Next(outputs))
    }
}
