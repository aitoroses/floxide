# ADR-0017: Event-Driven Node Pattern

## Status

Proposed

## Date

2025-02-27

## Context

The Floxide framework currently supports synchronous workflow execution where each node processes input and produces output through a direct call chain. While this model works well for deterministic, sequential workflows, it lacks support for:

1. Workflows that need to wait for external events without blocking system resources
2. Long-running workflows that respond to events as they arrive
3. Integration with event sources like message queues, webhooks, or system signals
4. Building reactive systems that can respond to changes in real-time

The existing node abstractions (`Node`, `LifecycleNode`, and `TransformNode`) all assume that processing is initiated by the workflow engine and completes within a single execution context. They don't provide a natural way to express nodes that wait for events or react to external stimuli.

Additionally, since the framework operates within a single process, we need an event-driven pattern that:

- Works efficiently within a shared memory space
- Can be nested within other workflows
- Provides clean integration with the existing workflow engine
- Maintains type safety and the functional approach of the framework

## Decision

We will implement the `EventDrivenNode` trait to enable event-driven workflows within the Floxide framework. This trait will be designed to:

1. Allow nodes to wait for events without blocking threads
2. Process events as they arrive
3. Integrate smoothly with the existing workflow system
4. Support nesting within larger workflows

### EventDrivenNode Trait

```rust
#[async_trait]
pub trait EventDrivenNode<Event, Context, Action>: Send + Sync
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    /// Wait for an external event to occur
    async fn wait_for_event(&self) -> Result<Event, FloxideError>;

    /// Process the received event and update context
    async fn process_event(
        &self,
        event: Event,
        ctx: &mut Context
    ) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}
```

### Event Sources

To provide flexibility in event sources, we'll define common event source adapters:

1. **Channel-based Events**: Using Tokio channels for in-process communication
2. **Timer Events**: For time-based triggers using interval or cron expressions
3. **External Source Adapters**: For connecting to external systems like message queues

```rust
// Channel-based event source
pub struct ChannelEventSource<Event> {
    receiver: mpsc::Receiver<Event>,
    id: NodeId,
}

impl<Event> ChannelEventSource<Event>
where
    Event: Send + 'static,
{
    pub fn new(capacity: usize) -> (Self, mpsc::Sender<Event>) {
        let (sender, receiver) = mpsc::channel(capacity);
        let id = Uuid::new_v4().to_string();
        (Self { receiver, id }, sender)
    }
}

#[async_trait]
impl<Event, Context, Action> EventDrivenNode<Event, Context, Action> for ChannelEventSource<Event>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    async fn wait_for_event(&self) -> Result<Event, FloxideError> {
        self.receiver.recv().await.ok_or_else(|| FloxideError::event_source(
            self.id(),
            "Event channel closed"
        ))
    }

    async fn process_event(
        &self,
        event: Event,
        _ctx: &mut Context
    ) -> Result<Action, FloxideError> {
        // Default implementation just forwards the event
        // Users should implement their own event processors
        Ok(Action::default())
    }

    fn id(&self) -> NodeId {
        self.id.clone()
    }
}
```

### EventDrivenWorkflow

To execute event-driven nodes, we'll introduce an `EventDrivenWorkflow` that:

1. Manages the event loop for event-driven nodes
2. Handles routing between event-driven nodes
3. Provides graceful shutdown capabilities
4. Integrates with the standard workflow system

```rust
pub struct EventDrivenWorkflow<Event, Context, Action>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    nodes: HashMap<NodeId, Arc<dyn EventDrivenNode<Event, Context, Action>>>,
    routes: HashMap<(NodeId, ActionType), NodeId>,
    initial_node: NodeId,
}

impl<Event, Context, Action> EventDrivenWorkflow<Event, Context, Action>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    pub fn new(initial_node: Arc<dyn EventDrivenNode<Event, Context, Action>>) -> Self {
        let initial_id = initial_node.id();
        let mut nodes = HashMap::new();
        nodes.insert(initial_id.clone(), initial_node);

        Self {
            nodes,
            routes: HashMap::new(),
            initial_node: initial_id,
        }
    }

    pub fn add_node(&mut self, node: Arc<dyn EventDrivenNode<Event, Context, Action>>) {
        let id = node.id();
        self.nodes.insert(id, node);
    }

    pub fn set_route(&mut self, from_id: &NodeId, action: &Action, to_id: &NodeId) {
        self.routes.insert((from_id.clone(), action.clone()), to_id.clone());
    }

    pub async fn execute(&self, ctx: &mut Context) -> Result<(), FloxideError> {
        let mut current_node_id = self.initial_node.clone();

        loop {
            let node = self.nodes.get(&current_node_id).ok_or_else(|| {
                FloxideError::node_not_found(current_node_id.clone())
            })?;

            // Wait for an event
            let event = node.wait_for_event().await?;

            // Process the event
            let action = node.process_event(event, ctx).await?;

            // Check for termination action
            if action == Action::terminate() {
                break;
            }

            // Route to the next node
            if let Some(next_node_id) = self.routes.get(&(current_node_id.clone(), action)) {
                current_node_id = next_node_id.clone();
            } else {
                // Stay on the same node if no route is defined for this action
            }
        }

        Ok(())
    }

    pub async fn execute_with_timeout(
        &self,
        ctx: &mut Context,
        timeout: Duration
    ) -> Result<(), FloxideError> {
        tokio::select! {
            result = self.execute(ctx) => result,
            _ = tokio::time::sleep(timeout) => {
                Err(FloxideError::timeout("Event-driven workflow execution timed out"))
            }
        }
    }
}
```

### Integration with Standard Workflows

We'll provide adapter patterns to integrate event-driven nodes with standard workflows:

```rust
pub struct EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    node: Arc<dyn EventDrivenNode<E, C, A>>,
    id: NodeId,
    timeout: Duration,
}

#[async_trait]
impl<E, C, A> Node<C, A> for EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(&self, ctx: &mut C) -> Result<NodeOutcome<Self::Output, A>, FloxideError> {
        // Wait for one event with timeout
        tokio::select! {
            result = self.node.wait_for_event() => {
                match result {
                    Ok(event) => {
                        let action = self.node.process_event(event, ctx).await?;
                        Ok(NodeOutcome::RouteToAction((), action))
                    },
                    Err(e) => Err(e),
                }
            }
            _ = tokio::time::sleep(self.timeout) => {
                Ok(NodeOutcome::RouteToAction((), A::timeout()))
            }
        }
    }
}
```

### Nested Event-Driven Workflows

To support nested event-driven workflows within larger workflows, we'll provide:

```rust
pub struct NestedEventDrivenWorkflow<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    workflow: Arc<EventDrivenWorkflow<E, C, A>>,
    id: NodeId,
    timeout: Option<Duration>,
}

#[async_trait]
impl<E, C, A> Node<C, A> for NestedEventDrivenWorkflow<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(&self, ctx: &mut C) -> Result<NodeOutcome<Self::Output, A>, FloxideError> {
        // Execute the workflow, optionally with a timeout
        let result = if let Some(timeout) = self.timeout {
            self.workflow.execute_with_timeout(ctx, timeout).await
        } else {
            self.workflow.execute(ctx).await
        };

        match result {
            Ok(()) => Ok(NodeOutcome::RouteToAction((), A::complete())),
            Err(e) => {
                if e.is_timeout() {
                    Ok(NodeOutcome::RouteToAction((), A::timeout()))
                } else {
                    Err(e)
                }
            }
        }
    }
}
```

## Consequences

### Advantages

1. **Event-Driven Workflows**: Enables building reactive systems that respond to events
2. **Resource Efficiency**: Nodes can wait for events without blocking threads
3. **Integration**: Clean integration with the existing workflow engine
4. **Nesting Support**: Event-driven workflows can be nested within larger workflows
5. **Type Safety**: Maintains the type safety of the framework
6. **Flexibility**: Works with various event sources (channels, timers, external systems)
7. **Composability**: Event-driven nodes can be combined with other node types

### Disadvantages

1. **Complexity**: Adds more abstractions to the framework
2. **Concurrency Challenges**: Event-driven code can be harder to reason about
3. **State Management**: Stateful event-driven nodes require careful management
4. **Learning Curve**: Users need to understand both workflows and event handling

### Implementation Plan

The implementation will be phased:

1. Phase 1: Core `EventDrivenNode` trait and basic channel-based event sources
2. Phase 2: Integration with standard workflows and nesting support
3. Phase 3: Advanced event sources (timers, external system adapters)
4. Phase 4: Expanded examples and documentation

## Alternatives Considered

### 1. Using Actors Instead of Event-Driven Nodes

We considered using an actor model (similar to Akka or Actix) where each node is an actor that processes messages. This would provide a different concurrency model but would add significant complexity to the framework.

### 2. Event Loops in Standard Nodes

We evaluated adding event loop capabilities to standard nodes, allowing them to optionally wait for events. This would avoid adding a new trait but would make the existing traits more complex and harder to use correctly.

### 3. External Event Processing

Another approach would be to handle events externally and feed them into the workflow through standard inputs. This would be simpler but would not provide the full reactivity benefits of true event-driven nodes.

### 4. Workflow-level Event Handling

Instead of event-driven nodes, we considered adding event handling at the workflow level only. This would be simpler but would not allow the fine-grained control of having event handling at the node level.

## Implementation Notes

- The `EventDrivenNode` trait will be implemented in the new `floxide-event` crate
- Channel-based event sources will use Tokio's MPSC channels for efficiency
- Event-driven workflows will be cancelable and support graceful shutdown
- Type parameters on event-driven nodes allow for custom event types per workflow
- Events must be `Send + 'static` to support async processing across threads

## Related ADRs

- [ADR-0003: Core Framework Abstractions](0003-core-framework-abstractions.md)
- [ADR-0004: Async Runtime Selection](0004-async-runtime-selection.md)
- [ADR-0015: Node Abstraction Hierarchy](0015-node-abstraction-hierarchy.md)
- [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md)

## References

- [Tokio: Asynchronous Programming for Rust](https://tokio.rs/)
- [The Reactive Manifesto](https://www.reactivemanifesto.org/)
- [Designing event-driven systems](https://www.confluent.io/designing-event-driven-systems/)
- [Event-driven architecture](https://en.wikipedia.org/wiki/Event-driven_architecture)
