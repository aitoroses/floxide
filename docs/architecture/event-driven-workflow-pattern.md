# Event-Driven Workflow Pattern

This document describes the event-driven workflow pattern in the Flowrs framework.

## Overview

The event-driven workflow pattern extends the Flowrs framework to support asynchronous, event-based processing. This pattern is essential for building reactive systems that respond to external events in real-time.

## Core Concepts

The event-driven workflow pattern is built around several key concepts:

1. **EventDrivenNode**: A specialized node that waits for and processes events
2. **Event Sources**: Components that provide events from external systems
3. **EventDrivenWorkflow**: A workflow that orchestrates event-driven nodes
4. **Event Routing**: Mechanisms for routing events between nodes

## EventDrivenNode

The `EventDrivenNode` trait defines how nodes that respond to events should behave:

```rust
#[async_trait]
pub trait EventDrivenNode<E>: Send + Sync + 'static
where
    E: Event + Send + 'static,
{
    /// Get the node's unique identifier
    fn id(&self) -> NodeId;

    /// Wait for an event to arrive
    async fn wait_for_event(&self) -> Result<E, FlowrsError>;

    /// Process an event and return an action
    async fn process_event(&self, event: E) -> Result<EventAction, FlowrsError>;
}
```

A concrete implementation of `EventDrivenNode` typically connects to an event source and processes events:

```rust
pub struct SensorMonitorNode {
    id: NodeId,
    event_source: Arc<dyn EventSource<SensorEvent>>,
    threshold: f64,
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for SensorMonitorNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FlowrsError> {
        self.event_source.next_event().await
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction, FlowrsError> {
        if event.value > self.threshold {
            Ok(EventAction::Route("high_value_handler", event))
        } else {
            Ok(EventAction::Route("standard_handler", event))
        }
    }
}
```

## Event Sources

Event sources provide events from external systems:

```rust
#[async_trait]
pub trait EventSource<E>: Send + Sync + 'static
where
    E: Event + Send + 'static,
{
    /// Get the next event from the source
    async fn next_event(&self) -> Result<E, FlowrsError>;

    /// Check if the source has more events
    async fn has_more_events(&self) -> Result<bool, FlowrsError>;
}
```

The framework provides several built-in event sources:

### ChannelEventSource

Uses Tokio MPSC channels to receive events:

```rust
pub struct ChannelEventSource<E>
where
    E: Event + Send + 'static,
{
    receiver: Mutex<mpsc::Receiver<E>>,
}

impl<E> ChannelEventSource<E>
where
    E: Event + Send + 'static,
{
    pub fn new(receiver: mpsc::Receiver<E>) -> Self {
        Self {
            receiver: Mutex::new(receiver),
        }
    }

    pub fn sender(&self) -> mpsc::Sender<E> {
        // Return a sender connected to this receiver
    }
}

#[async_trait]
impl<E> EventSource<E> for ChannelEventSource<E>
where
    E: Event + Send + 'static,
{
    async fn next_event(&self) -> Result<E, FlowrsError> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await.ok_or(FlowrsError::EventSourceClosed)
    }

    async fn has_more_events(&self) -> Result<bool, FlowrsError> {
        let receiver = self.receiver.lock().await;
        Ok(!receiver.is_closed())
    }
}
```

## EventDrivenWorkflow

The `EventDrivenWorkflow` orchestrates event-driven nodes:

```rust
pub struct EventDrivenWorkflow<E>
where
    E: Event + Send + Clone + 'static,
{
    nodes: HashMap<NodeId, Box<dyn EventDrivenNode<E>>>,
    routes: HashMap<String, NodeId>,
    initial_node: NodeId,
    timeout: Option<Duration>,
}

impl<E> EventDrivenWorkflow<E>
where
    E: Event + Send + Clone + 'static,
{
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            routes: HashMap::new(),
            initial_node: NodeId::new(),
            timeout: None,
        }
    }

    pub fn add_node(&mut self, node: impl EventDrivenNode<E> + 'static) -> NodeId {
        let id = node.id();
        self.nodes.insert(id, Box::new(node));
        id
    }

    pub fn set_route(&mut self, name: &str, target_node: NodeId) {
        self.routes.insert(name.to_string(), target_node);
    }

    pub fn set_initial_node(&mut self, node_id: NodeId) {
        self.initial_node = node_id;
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }

    pub async fn run(&self) -> Result<(), FlowrsError> {
        let mut current_node_id = self.initial_node;

        loop {
            let node = self.nodes.get(&current_node_id)
                .ok_or_else(|| FlowrsError::NodeNotFound(current_node_id))?;

            let event = node.wait_for_event().await?;
            let action = node.process_event(event).await?;

            match action {
                EventAction::Route(route, event) => {
                    if let Some(next_node_id) = self.routes.get(&route) {
                        current_node_id = *next_node_id;
                    } else {
                        return Err(FlowrsError::RouteNotFound(route));
                    }
                },
                EventAction::Terminate => {
                    break;
                },
            }
        }

        Ok(())
    }
}
```

## Event Routing

Events are routed between nodes based on the actions returned by `process_event`:

```rust
pub enum EventAction<E>
where
    E: Event + Send + 'static,
{
    /// Route the event to another node
    Route(String, E),

    /// Terminate the workflow
    Terminate,
}
```

## Integration with Standard Workflows

The event-driven pattern can be integrated with standard workflows:

### EventDrivenNodeAdapter

Adapts an event-driven node to be used in a standard workflow:

```rust
pub struct EventDrivenNodeAdapter<E, N>
where
    E: Event + Send + 'static,
    N: EventDrivenNode<E>,
{
    node: N,
    _phantom: PhantomData<E>,
}

impl<E, N> Node for EventDrivenNodeAdapter<E, N>
where
    E: Event + Send + 'static,
    N: EventDrivenNode<E>,
{
    type Context = EventContext<E>;

    async fn run(&self, ctx: &mut Self::Context) -> Result<NextAction, FlowrsError> {
        let event = self.node.wait_for_event().await?;
        let action = self.node.process_event(event).await?;

        // Convert EventAction to NextAction
        match action {
            EventAction::Route(route, event) => {
                ctx.set_event(event);
                Ok(NextAction::Route(route))
            },
            EventAction::Terminate => {
                Ok(NextAction::Terminate)
            },
        }
    }
}
```

### NestedEventDrivenWorkflow

Uses an event-driven workflow as a node in a standard workflow:

```rust
pub struct NestedEventDrivenWorkflow<E>
where
    E: Event + Send + Clone + 'static,
{
    workflow: EventDrivenWorkflow<E>,
}

impl<E> Node for NestedEventDrivenWorkflow<E>
where
    E: Event + Send + Clone + 'static,
{
    type Context = Context<()>;

    async fn run(&self, _ctx: &mut Self::Context) -> Result<NextAction, FlowrsError> {
        self.workflow.run().await?;
        Ok(NextAction::Continue)
    }
}
```

## Usage Example

Here's an example of using the event-driven workflow pattern:

```rust
// Define an event type
#[derive(Clone)]
struct SensorEvent {
    id: String,
    value: f64,
    timestamp: DateTime<Utc>,
}

impl Event for SensorEvent {}

// Create event sources
let (tx1, rx1) = mpsc::channel(100);
let (tx2, rx2) = mpsc::channel(100);

let source1 = ChannelEventSource::new(rx1);
let source2 = ChannelEventSource::new(rx2);

// Create event-driven nodes
let sensor_monitor = SensorMonitorNode::new("sensor1", Arc::new(source1), 100.0);
let high_value_handler = HighValueHandlerNode::new("high_value", Arc::new(source2));
let standard_handler = StandardHandlerNode::new("standard");

// Create and configure the workflow
let mut workflow = EventDrivenWorkflow::new();

let monitor_id = workflow.add_node(sensor_monitor);
let high_value_id = workflow.add_node(high_value_handler);
let standard_id = workflow.add_node(standard_handler);

workflow.set_initial_node(monitor_id);
workflow.set_route("high_value_handler", high_value_id);
workflow.set_route("standard_handler", standard_id);
workflow.set_timeout(Duration::from_secs(60));

// Run the workflow
tokio::spawn(async move {
    workflow.run().await.unwrap();
});

// Send events to the workflow
tx1.send(SensorEvent {
    id: "sensor1".to_string(),
    value: 150.0,
    timestamp: Utc::now(),
}).await.unwrap();
```

## Conclusion

The event-driven workflow pattern in the Flowrs framework provides a powerful and flexible approach to building reactive systems. By leveraging Rust's async capabilities and the framework's core abstractions, it enables efficient processing of asynchronous events while maintaining type safety and proper error handling.

For more detailed information on the event-driven workflow pattern, refer to the [Event-Driven Workflow Pattern ADR](../adrs/0009-event-driven-workflow-pattern.md).
