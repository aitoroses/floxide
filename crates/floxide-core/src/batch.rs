use crate::context::Context;
use crate::error::FloxideError;
use crate::node::Node;
use crate::transition::Transition;
use async_trait::async_trait;
use futures::future::join_all;
use std::marker::PhantomData;
use std::vec::Vec;
use tokio::task;
use tracing;

/// A node adapter that runs an inner node on a batch of inputs, collecting outputs in parallel
#[derive(Clone, Debug)]
pub struct BatchNode<C: Context, N: Node<C>> {
    pub node: N,
    pub batch_size: usize,
    _phantom: PhantomData<C>,
}

impl<C: Context, N: Node<C>> BatchNode<C, N> {
    /// Wraps an existing node into a batch adapter with a batch size
    pub fn new(node: N, batch_size: usize) -> Self {
        BatchNode {
            node,
            batch_size,
            _phantom: PhantomData,
        }
    }
}

impl<C: Context, N: Node<C>> BatchNode<C, N> {
    /// Process a batch of inputs, where the associated Input/Output types are Vecs
    pub async fn process_batch(
        &self,
        // take ownership of the context so it can be cloned into blocking tasks
        ctx: C,
        inputs: <Self as Node<C>>::Input,
    ) -> Result<<Self as Node<C>>::Output, FloxideError>
    where
        C: Context + 'static,
        N: Node<C> + Clone + Send + Sync + 'static,
        <N as Node<C>>::Input: Clone + Send + 'static,
        <N as Node<C>>::Output: Send + 'static,
    {
        use tracing::{debug, error};
        debug!(
            batch_size = self.batch_size,
            num_inputs = inputs.len(),
            "Starting batch processing"
        );
        let mut outputs = Vec::new();
        let node = self.node.clone();
        let ctx_clone = ctx.clone();
        let mut tasks = Vec::new();

        for input in inputs.into_iter() {
            let node = node.clone();
            // clone the context reference (does not require C: Clone)
            let ctx = ctx_clone.clone();
            let task = task::spawn_blocking(move || {
                tokio::runtime::Handle::current()
                    .block_on(async move { node.process(&ctx, input).await })
            });
            tasks.push(task);

            if tasks.len() >= self.batch_size {
                debug!(current_batch = tasks.len(), "Processing batch");
                let results = join_all(tasks).await;
                tasks = Vec::new();
                for res in results {
                    match res {
                        Ok(Ok(Transition::Next(o))) => outputs.push(o),
                        Ok(Ok(Transition::NextAll(os))) => outputs.extend(os),
                        Ok(Ok(Transition::Hold)) => {}
                        Ok(Ok(Transition::Abort(e))) => {
                            error!(?e, "Node aborted during batch");
                            return Err(e);
                        }
                        Ok(Err(e)) => {
                            error!(?e, "Node errored during batch");
                            return Err(e);
                        }
                        Err(e) => {
                            error!(?e, "Join error during batch");
                            return Err(FloxideError::Generic(format!("Join error: {e}")));
                        }
                    }
                }
            }
        }

        if !tasks.is_empty() {
            debug!(final_batch = tasks.len(), "Processing final batch");
            let results = join_all(tasks).await;
            for res in results {
                match res {
                    Ok(Ok(Transition::Next(o))) => outputs.push(o),
                    Ok(Ok(Transition::NextAll(os))) => outputs.extend(os),
                    Ok(Ok(Transition::Hold)) => {}
                    Ok(Ok(Transition::Abort(e))) => {
                        error!(?e, "Node aborted during final batch");
                        return Err(e);
                    }
                    Ok(Err(e)) => {
                        error!(?e, "Node errored during final batch");
                        return Err(e);
                    }
                    Err(e) => {
                        error!(?e, "Join error during final batch");
                        return Err(FloxideError::Generic(format!("Join error: {e}")));
                    }
                }
            }
        }

        debug!(num_outputs = outputs.len(), "Batch processing complete");
        Ok(outputs)
    }
}

#[async_trait]
impl<C, N> Node<C> for BatchNode<C, N>
where
    C: Context + 'static,
    N: Node<C> + Clone + Send + Sync + 'static,
    <N as Node<C>>::Input: Clone + Send + 'static,
    <N as Node<C>>::Output: Send + 'static,
{
    type Input = Vec<<N as Node<C>>::Input>;
    type Output = Vec<<N as Node<C>>::Output>;

    async fn process(
        &self,
        ctx: &C,
        inputs: Self::Input,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        let outputs = self.process_batch((*ctx).clone(), inputs).await?;
        Ok(Transition::Next(outputs))
    }
}
