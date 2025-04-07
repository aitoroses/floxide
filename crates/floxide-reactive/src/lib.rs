//! Support for reactive patterns in the Floxide framework.
//!
//! This crate provides the `ReactiveNode` trait and related implementations for
//! handling reactive patterns that respond to changes in external data sources.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use floxide_core::{ActionType, DefaultAction, FloxideError, Node, NodeId, NodeOutcome};
use futures::future::{BoxFuture, Future};
use futures::{Stream, StreamExt};
use thiserror::Error;
use tokio::fs::metadata;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, warn};

/// Errors specific to reactive operations
#[derive(Debug, Error)]
pub enum ReactiveError {
    #[error("Failed to watch resource: {0}")]
    WatchError(String),

    #[error("Stream closed unexpectedly")]
    StreamClosed,

    #[error("Failed to connect to data source: {0}")]
    ConnectionError(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
}

/// A handler function for file changes
pub type ChangeHandlerFn<Context, Action> = Box<
    dyn Fn(FileChange, &mut Context) -> BoxFuture<'static, Result<Action, FloxideError>>
        + Send
        + Sync,
>;

/// Trait for nodes that react to changes in external data sources
#[async_trait]
pub trait ReactiveNode<Change, Context, Action>: Send + Sync
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    /// Set up a stream of changes to watch
    async fn watch(&self) -> Result<Box<dyn Stream<Item = Change> + Send + Unpin>, FloxideError>;

    /// React to a detected change
    async fn react_to_change(
        &self,
        change: Change,
        ctx: &mut Context,
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}

/// An adapter that allows a ReactiveNode to be used as a standard Node.
/// This adapter handles the streaming and change detection.
pub struct ReactiveNodeAdapter<R, Change, Context, Action>
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    R: ReactiveNode<Change, Context, Action> + Send + 'static,
{
    node: Arc<R>,
    buffer_size: usize,
    _phantom: PhantomData<(Change, Context, Action)>,
}

impl<R, Change, Context, Action> ReactiveNodeAdapter<R, Change, Context, Action>
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    R: ReactiveNode<Change, Context, Action> + Send + 'static,
{
    /// Create a new adapter for a reactive node
    pub fn new(node: R) -> Self {
        Self {
            node: Arc::new(node),
            buffer_size: 100, // Default buffer size
            _phantom: PhantomData,
        }
    }

    /// Set the buffer size for the change stream
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Start watching for changes and process them in the background
    pub async fn start_watching(
        &self,
        mut ctx: Context,
    ) -> Result<impl Stream<Item = Action> + Unpin, FloxideError> {
        let (tx, rx) = mpsc::channel(self.buffer_size);
        let node_clone = self.node.clone();

        // Start a background task to watch for changes and process them
        tokio::spawn(async move {
            match node_clone.watch().await {
                Ok(mut change_stream) => {
                    while let Some(change) = change_stream.next().await {
                        match node_clone.react_to_change(change, &mut ctx).await {
                            Ok(action) => {
                                if tx.send(action).await.is_err() {
                                    warn!("Receiver dropped, stopping reactive node");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Error processing change: {}", e);
                                // Continue watching despite errors
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to set up watch stream: {}", e);
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}

#[async_trait]
impl<R, Change, Context, Action> Node<Context, Action>
    for ReactiveNodeAdapter<R, Change, Context, Action>
where
    Change: Send + Sync + 'static,
    Context: Clone + Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    R: ReactiveNode<Change, Context, Action> + Send + 'static,
{
    type Output = Action;

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FloxideError> {
        // Create a clone of the context for the background task
        let ctx_clone = ctx.clone();

        // Start watching for changes
        let mut action_stream = self.start_watching(ctx_clone).await?;

        // Return the first action if available
        if let Some(action) = action_stream.next().await {
            Ok(NodeOutcome::RouteToAction(action))
        } else {
            Err(FloxideError::Other(
                "Reactive stream closed without producing any actions".to_string(),
            ))
        }
    }

    fn id(&self) -> NodeId {
        self.node.id()
    }
}

/// A node that watches a file for changes
pub struct FileWatcherNode<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    file_path: String,
    poll_interval: Duration,
    change_handler: Option<ChangeHandlerFn<Context, Action>>,
}

impl<Context, Action> FileWatcherNode<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    /// Create a new file watcher node
    pub fn new<P: Into<String>>(file_path: P) -> Self {
        Self {
            file_path: file_path.into(),
            poll_interval: Duration::from_secs(5), // Default 5 second interval
            change_handler: None,
        }
    }

    /// Create a new file watcher node with a specified ID
    pub fn with_id<P: Into<String>>(file_path: P) -> Self {
        Self {
            file_path: file_path.into(),
            poll_interval: Duration::from_secs(5),
            change_handler: None,
        }
    }

    /// Set the poll interval for the file watcher
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set the change handler for the file watcher node
    pub fn with_change_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(FileChange, &mut Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Action, FloxideError>> + Send + 'static,
    {
        self.change_handler = Some(Box::new(move |change, ctx| Box::pin(handler(change, ctx))));
        self
    }
}

/// Represents a file change detected by the FileWatcherNode
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path of the changed file
    pub path: String,
    /// Last modified timestamp in seconds since the epoch
    pub modified_time: u64,
    /// Size of the file in bytes
    pub size: u64,
}

/// Default implementations for FileWatcherNode with DefaultAction
pub trait DefaultReactiveNode<Context>: ReactiveNode<FileChange, Context, DefaultAction>
where
    Context: Send + Sync + 'static,
{
    /// Default implementation for react_to_change that returns "change_detected" action
    fn default_react_to_change(
        &self,
        change: FileChange,
        ctx: &mut Context,
    ) -> Result<DefaultAction, FloxideError>;
}

#[async_trait]
impl<Context, Action> ReactiveNode<FileChange, Context, Action> for FileWatcherNode<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    async fn watch(
        &self,
    ) -> Result<Box<dyn Stream<Item = FileChange> + Send + Unpin>, FloxideError> {
        let file_path = self.file_path.clone();
        let poll_interval = self.poll_interval;

        // Check if the file exists
        if !Path::new(&file_path).exists() {
            return Err(FloxideError::Other(format!(
                "File not found: {}",
                file_path
            )));
        }

        let (tx, rx) = mpsc::channel(10);

        // Start a background task to poll for file changes
        tokio::spawn(async move {
            let mut last_modified = 0;
            let mut last_size = 0;

            loop {
                match metadata(&file_path).await {
                    Ok(meta) => {
                        let modified = meta
                            .modified()
                            .unwrap_or_else(|_| std::time::SystemTime::now())
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let size = meta.len();

                        // Detect change in modified time or size
                        if modified > last_modified || size != last_size {
                            let change = FileChange {
                                path: file_path.clone(),
                                modified_time: modified,
                                size,
                            };

                            last_modified = modified;
                            last_size = size;

                            if tx.send(change).await.is_err() {
                                // Receiver dropped, stop watching
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error accessing file {}: {}", file_path, e);
                        // If file becomes inaccessible, stop watching
                        break;
                    }
                }

                sleep(poll_interval).await;
            }
        });

        Ok(Box::new(ReceiverStream::new(rx)))
    }

    /// Reacts to a file change and produces an action
    async fn react_to_change(
        &self,
        change: FileChange,
        context: &mut Context,
    ) -> Result<Action, FloxideError> {
        debug!("Reacting to file change: {:?}", change);

        if let Some(callback) = &self.change_handler {
            callback(change.clone(), context).await
        } else {
            // We can't create a generic Action directly
            Err(FloxideError::Other(
                "FileWatcherNode requires a change handler to create specific action types"
                    .to_string(),
            ))
        }
    }

    fn id(&self) -> NodeId {
        NodeId::new()
    }
}

/// A custom reactive node that uses a provided closure to create the watch stream
/// and react to changes.
pub struct CustomReactiveNode<Change, Context, Action, WatchFn, ReactFn>
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    WatchFn: Fn() -> Result<Box<dyn Stream<Item = Change> + Send + Unpin>, FloxideError>
        + Send
        + Sync
        + 'static,
    ReactFn: Fn(Change, &mut Context) -> Result<Action, FloxideError> + Send + Sync + 'static,
{
    id: NodeId,
    watch_fn: Arc<WatchFn>,
    react_fn: Arc<ReactFn>,
    _phantom: PhantomData<(Change, Context, Action)>,
}

impl<Change, Context, Action, WatchFn, ReactFn>
    CustomReactiveNode<Change, Context, Action, WatchFn, ReactFn>
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    WatchFn: Fn() -> Result<Box<dyn Stream<Item = Change> + Send + Unpin>, FloxideError>
        + Send
        + Sync
        + 'static,
    ReactFn: Fn(Change, &mut Context) -> Result<Action, FloxideError> + Send + Sync + 'static,
{
    /// Create a new custom reactive node
    pub fn new(watch_fn: WatchFn, react_fn: ReactFn) -> Self {
        Self {
            id: NodeId::new(),
            watch_fn: Arc::new(watch_fn),
            react_fn: Arc::new(react_fn),
            _phantom: PhantomData,
        }
    }

    /// Create a new custom reactive node with a specified ID
    pub fn with_id(id: impl Into<NodeId>, watch_fn: WatchFn, react_fn: ReactFn) -> Self {
        Self {
            id: id.into(),
            watch_fn: Arc::new(watch_fn),
            react_fn: Arc::new(react_fn),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Change, Context, Action, WatchFn, ReactFn> ReactiveNode<Change, Context, Action>
    for CustomReactiveNode<Change, Context, Action, WatchFn, ReactFn>
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    WatchFn: Fn() -> Result<Box<dyn Stream<Item = Change> + Send + Unpin>, FloxideError>
        + Send
        + Sync
        + 'static,
    ReactFn: Fn(Change, &mut Context) -> Result<Action, FloxideError> + Send + Sync + 'static,
{
    async fn watch(&self) -> Result<Box<dyn Stream<Item = Change> + Send + Unpin>, FloxideError> {
        (self.watch_fn)()
    }

    async fn react_to_change(
        &self,
        change: Change,
        ctx: &mut Context,
    ) -> Result<Action, FloxideError> {
        (self.react_fn)(change, ctx)
    }

    fn id(&self) -> NodeId {
        self.id.clone()
    }
}

// Extension trait to add helper methods to DefaultAction
pub trait ReactiveActionExt: ActionType {
    /// Create an action indicating a change was detected
    fn change_detected() -> Self;

    /// Create an action indicating no change was detected
    fn no_change() -> Self;

    /// Check if this is a change_detected action
    fn is_change_detected(&self) -> bool;

    /// Check if this is a no_change action
    fn is_no_change(&self) -> bool;
}

// Implement the extension trait for DefaultAction
impl ReactiveActionExt for DefaultAction {
    fn change_detected() -> Self {
        DefaultAction::Custom("change_detected".to_string())
    }

    fn no_change() -> Self {
        DefaultAction::Custom("no_change".to_string())
    }

    fn is_change_detected(&self) -> bool {
        matches!(self, DefaultAction::Custom(s) if s == "change_detected")
    }

    fn is_no_change(&self) -> bool {
        matches!(self, DefaultAction::Custom(s) if s == "no_change")
    }
}

/// A helper function to create an action from a change
pub fn action_from_change<Change, Action>(_change: &Change) -> Action
where
    Action: ActionType + ReactiveActionExt,
    Change: Send + Sync + 'static,
{
    // Default implementation just returns a change_detected action
    Action::change_detected()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    struct TestContext {
        values: Vec<String>,
    }

    #[tokio::test]
    async fn test_custom_reactive_node() {
        // Create a stream of test changes
        let changes = vec!["change1", "change2", "change3"];

        // Create a custom reactive node
        let node = CustomReactiveNode::<_, _, _, _, _>::new(
            || {
                let stream = stream::iter(vec!["change1", "change2", "change3"]);
                let boxed: Box<dyn Stream<Item = &'static str> + Send + Unpin> = Box::new(stream);
                Ok(boxed)
            },
            |change: &str, ctx: &mut TestContext| {
                ctx.values.push(change.to_string());
                Ok(DefaultAction::change_detected())
            },
        );

        // Create a test context
        let mut ctx = TestContext { values: Vec::new() };

        // Set up the node's watch stream
        let mut stream = node.watch().await.unwrap();

        // Process each change
        while let Some(change) = stream.next().await {
            node.react_to_change(change, &mut ctx).await.unwrap();
        }

        // Verify the context was updated correctly
        assert_eq!(ctx.values, changes);
    }
}
