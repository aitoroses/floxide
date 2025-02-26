# floxide-reactive API Reference

The `floxide-reactive` crate provides support for reactive patterns in the Floxide framework, enabling nodes to respond to changes in external data sources using a stream-based approach.

## Core Types

### ReactiveNode

```rust
#[async_trait]
pub trait ReactiveNode<Change, Context, Action>: Send + Sync
where
    Change: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    /// Set up a stream of changes to watch
    async fn watch(&self) -> Result<impl Stream<Item = Change> + Send, FloxideError>;

    /// React to a detected change
    async fn react_to_change(
        &self,
        change: Change,
        context: &mut Context,
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

The `ReactiveNode` trait is the core abstraction for reactive patterns, providing:
- Stream-based change detection
- Context-aware change handling
- Integration with the workflow engine

### ReactiveError

```rust
pub enum ReactiveError {
    WatchError(String),
    StreamClosed,
    ConnectionError(String),
    ResourceNotFound(String),
}
```

Specialized error types for reactive operations, handling:
- Resource watching failures
- Stream lifecycle issues
- Connection problems
- Resource availability

## Built-in Implementations

### FileWatcherNode

```rust
pub struct FileWatcherNode<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    // ...
}
```

A specialized node for file system monitoring:
- File modification detection
- Configurable polling intervals
- Custom change handlers
- Metadata tracking

### CustomReactiveNode

```rust
pub struct CustomReactiveNode<Change, Context, Action, WatchFn, ReactFn>
where
    Change: Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    // ...
}
```

A flexible implementation for custom reactive patterns:
- User-defined watch functions
- Custom change reaction logic
- Configurable node identity

## Stream Management

### ReactiveNodeAdapter

```rust
pub struct ReactiveNodeAdapter<R, Change, Context, Action>
where
    R: ReactiveNode<Change, Context, Action>,
{
    // ...
}
```

Adapts reactive nodes to the standard Node interface:
- Buffering with backpressure handling
- Stream lifecycle management
- Background processing

### Buffering and Backpressure

The implementation includes configurable buffering to handle rapid changes:

```rust
// Configure buffer size
let node = reactive_node
    .with_buffer_size(100)
    .with_backoff(Duration::from_millis(100));
```

## Usage Examples

### File Watching

```rust
use floxide_reactive::{FileWatcherNode, FileChange};

let watcher = FileWatcherNode::new("config.toml")
    .with_poll_interval(Duration::from_secs(5))
    .with_change_handler(|change: FileChange, ctx: &mut Context| async move {
        match change {
            FileChange::Modified(path) => {
                println!("File modified: {}", path.display());
                Ok(DefaultAction::Next)
            }
            FileChange::NotFound => {
                println!("File not found");
                Ok(DefaultAction::Stop)
            }
        }
    });

let mut workflow = Workflow::new(watcher);
workflow.run(context).await?;
```

### Custom Reactive Pattern

```rust
use floxide_reactive::CustomReactiveNode;

let node = CustomReactiveNode::new(
    // Watch function returns a stream of changes
    || async {
        let (tx, rx) = mpsc::channel(100);
        // Set up change detection...
        Ok(ReceiverStream::new(rx).boxed())
    },
    // React function handles changes
    |change, ctx| async move {
        println!("Processing change: {:?}", change);
        Ok(DefaultAction::Next)
    }
);
```

## Best Practices

1. **Stream Management**
   - Configure appropriate buffer sizes based on change frequency
   - Implement proper backpressure handling
   - Clean up resources when streams are dropped
   - Handle stream closure gracefully

2. **Error Handling**
   - Handle transient failures with retries
   - Provide clear error context
   - Clean up resources on error
   - Log relevant error details

3. **Resource Usage**
   - Monitor memory usage with large streams
   - Use appropriate polling intervals
   - Implement proper cleanup
   - Consider system resource limits

4. **Testing**
   - Test stream handling
   - Verify error scenarios
   - Check resource cleanup
   - Test backpressure handling

## See Also

- [ADR-0017: ReactiveNode Implementation](../adrs/0017-reactive-node-implementation.md)
- [Reactive Node Example](../examples/reactive-node.md)
- [Event-Driven Architecture](../guides/event_driven_architecture.md)
