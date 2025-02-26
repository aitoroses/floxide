# ADR-0017: ReactiveNode Implementation

## Status

Proposed

## Date

2025-02-25

## Context

As part of the implementation of async extension patterns outlined in [ADR-0016](0016-transform-node-and-async-extensions.md), we need to implement a `ReactiveNode` that can respond to changes in external data sources using a stream-based approach. This pattern is valuable for workflows that need to monitor and react to external changes without constant polling.

Reactive programming is a paradigm that deals with asynchronous data streams and the propagation of changes. In the context of our workflow system, we need a node type that can:

1. Watch external data sources for changes
2. React to those changes by executing business logic
3. Produce appropriate routing actions based on the changes
4. Integrate with the core workflow engine

The challenges include:

- Managing long-lived connections to data sources
- Handling connection failures and retries
- Converting external change events into workflow actions
- Maintaining state between change events
- Providing a clean abstraction over various types of data sources

## Decision

We will implement a `ReactiveNode` trait and supporting infrastructure in a new `floxide-reactive` crate with the following components:

### 1. Core Trait and Types

The main `ReactiveNode` trait will provide a stream-based interface for watching and reacting to changes:

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
        ctx: &mut Context,
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

### 2. Adapter for Core Node Integration

A `ReactiveNodeAdapter` that allows a `ReactiveNode` to be used as a standard `Node`:

```rust
pub struct ReactiveNodeAdapter<R, Change, Context, Action>
where
    R: ReactiveNode<Change, Context, Action>,
    Change: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
{
    node: Arc<R>,
    buffer_size: usize,
    _phantom: PhantomData<(Change, Context, Action)>,
}
```

### 3. Concrete Implementations

We will provide two concrete implementations:

1. `FileWatcherNode`: A simple reactive node that watches a file system path for changes
2. `CustomReactiveNode`: A flexible implementation that allows using closures to define watch and react behavior

### 4. Extension Trait for Actions

A `ReactiveActionExt` trait to define common reactive actions:

```rust
pub trait ReactiveActionExt: ActionType {
    fn change_detected() -> Self;
    fn no_change() -> Self;
    fn is_change_detected(&self) -> bool;
    fn is_no_change(&self) -> bool;
}
```

### 5. Error Types

Specific error types for reactive operations:

```rust
pub enum ReactiveError {
    WatchError(String),
    StreamClosed,
    ConnectionError(String),
    ResourceNotFound(String),
}
```

## Consequences

### Advantages

1. **Event-Driven Architecture**: Enables truly event-driven workflows that respond to external changes
2. **Resource Efficiency**: Avoids constant polling for changes by using reactive streams
3. **Separation of Concerns**: Clear separation between watching for changes and reacting to them
4. **Flexibility**: The generic design allows reacting to any type of change from any data source
5. **Integration**: Seamless integration with the core workflow engine via the adapter pattern

### Disadvantages

1. **Complexity**: Introduces additional complexity with stream management and background tasks
2. **Resource Management**: Long-lived connections require careful resource management
3. **Error Handling**: More complex error handling is needed for connection issues and recovery
4. **Multiple Executions**: ReactiveNode may execute multiple times in response to rapid changes
5. **Testing Challenges**: Testing reactive code is more complex than synchronous code

## Implementation Details

### Background Tasks and Resource Management

The `ReactiveNodeAdapter` will spawn background tasks to watch for changes, requiring careful management of task lifetimes and proper cleanup:

```rust
// Start a background task to watch for changes and process them
tokio::spawn(async move {
    match node_clone.watch().await {
        Ok(mut change_stream) => {
            while let Some(change) = change_stream.next().await {
                // Process change and send action...
            }
        }
        Err(e) => {
            warn!("Failed to set up watch stream: {}", e);
        }
    }
});
```

### Buffering and Backpressure

The implementation will include configurable buffering to handle backpressure when changes occur more rapidly than they can be processed:

```rust
pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
}
```

### State Handling

Contexts will need to maintain state between change events. The implementation will support cloneable contexts to allow state sharing with background tasks:

```rust
// Context is cloned for the background task
let ctx_clone = ctx.clone();
```

## Alternatives Considered

### 1. Using Callbacks Instead of Streams

We could have used a callback-based approach instead of streams:

```rust
async fn on_change(&self, callback: impl Fn(Change) -> Result<Action, FloxideError>);
```

This would be simpler in some ways but less flexible and harder to integrate with other async code. Streams provide better composition and more control over backpressure.

### 2. Polling-Based Approach

We could have used a polling-based approach instead of reactive streams:

```rust
async fn check_for_changes(&self) -> Result<Option<Change>, FloxideError>;
```

This would be simpler to implement but less efficient and less idiomatic for truly reactive patterns.

### 3. Using Event Emitters

We could have used an event emitter pattern instead of streams:

```rust
fn subscribe(&self, event_emitter: &EventEmitter<Change>);
```

This approach is common in many event-driven frameworks but would require building a custom event emitter system and doesn't leverage Rust's existing stream ecosystem as well.

## Related ADRs

- [ADR-0003: Core Framework Abstractions](0003-core-framework-abstractions.md)
- [ADR-0004: Async Runtime Selection](0004-async-runtime-selection.md)
- [ADR-0015: Node Abstraction Hierarchy](0015-node-abstraction-hierarchy.md)
- [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md)

## References

- [Tokio Documentation](https://tokio.rs/)
- [futures-rs Documentation](https://docs.rs/futures/latest/futures/)
- [Reactive Streams Specification](https://www.reactive-streams.org/)
- [Reactive Programming Patterns](https://www.reactivemanifesto.org/)
