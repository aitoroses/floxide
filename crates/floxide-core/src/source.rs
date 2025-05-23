//! Abstraction for value-producing (source) nodes: nodes with Input=() that emit a stream of outputs.
use crate::context::Context;
use crate::error::FloxideError;
use crate::workflow::Workflow;
use crate::WorkflowCtx;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;

// -----------------------------------------------------------------------------
/// A channel source: external code can send values in, and this source
/// will drive a workflow for each received item until the channel closes.
#[derive(Debug, Clone)]
pub struct Source<C, O> {
    receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<O>>>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Context, O> Source<C, O> {
    /// Wrap an existing receiver into a Source
    pub fn new(rx: tokio::sync::mpsc::Receiver<O>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(rx)),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<C, O> Source<C, O>
where
    C: Context,
    O: Send + Sync,
{
    /// Drive the provided workflow by pulling items from the channel and
    /// invoking `wf.run(ctx, item)` for each until the channel closes.
    pub async fn run<W>(&self, wf: &W, ctx: &WorkflowCtx<C>) -> Result<(), FloxideError>
    where
        W: Workflow<C, Input = O>,
    {
        let mut rx = self.receiver.lock().await;
        while let Some(item) = rx.recv().await {
            wf.run(ctx, item).await?;
        }
        Ok(())
    }
}

/// Create a channel-backed source node and its sender handle.
///
/// `capacity` sets the mpsc buffer size. Returns `(sender, source_node)`.
pub fn source<C, O>(capacity: usize) -> (tokio::sync::mpsc::Sender<O>, Source<C, O>)
where
    C: Context,
    O: Send + Sync,
{
    let (tx, rx) = tokio::sync::mpsc::channel(capacity);
    (tx, Source::new(rx))
}
