# flowrs-event API Reference

This document provides a reference for the `flowrs-event` crate, which extends the Flowrs framework with event-driven capabilities.

## Overview

The `flowrs-event` crate provides extensions to the core Flowrs framework for building event-driven workflows, including:

- Event-driven node abstractions
- Event source implementations
- Event-driven workflow orchestration
- Integration with standard workflows

## Core Modules

### Event Module

The Event module provides the core event abstractions:

```rust
use flowrs_event::event::{Event, EventAction};

// Define a custom event type
#[derive(Clone)]
struct SensorEvent {
    id: String,
    value: f64,
}

// Implement the Event trait
impl Event for SensorEvent {}
```

### EventDrivenNode Module

The EventDrivenNode module provides the core node abstractions for event-driven workflows:

```rust
use flowrs_event::node::{EventDrivenNode, NodeId};
use async_trait::async_trait;

struct MyEventNode {
    id: NodeId,
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for MyEventNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FlowrsError> {
        // Wait for an event
        // ...
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction<SensorEvent>, FlowrsError> {
        // Process the event
        // ...
        Ok(EventAction::Route("next_node".to_string(), event))
    }
}
```

### EventSource Module

The EventSource module provides abstractions and implementations for event sources:

```rust
use flowrs_event::source::{EventSource, ChannelEventSource};
use tokio::sync::mpsc;

// Create a channel-based event source
let (tx, rx) = mpsc::channel::<SensorEvent>(100);
let source = ChannelEventSource::new(rx);

// Send an event
tx.send(SensorEvent { id: "sensor1".to_string(), value: 42.0 }).await?;
```

### Workflow Module

The Workflow module provides the event-driven workflow orchestration:

```rust
use flowrs_event::workflow::EventDrivenWorkflow;

// Create an event-driven workflow
let mut workflow = EventDrivenWorkflow::<SensorEvent>::new();

// Add nodes and configure routes
let node_id = workflow.add_node(MyEventNode { id: NodeId::new() });
workflow.set_initial_node(node_id);
workflow.set_route("next_node", node_id); // Loop back to the same node

// Run the workflow
workflow.run().await?;
```

## Key Types

### Event Trait

The core trait for event types:

```rust
pub trait Event: Clone + Send + 'static {}
```

### EventDrivenNode Trait

The core trait for event-driven nodes:

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
    async fn process_event(&self, event: E) -> Result<EventAction<E>, FlowrsError>;
}
```

### EventSource Trait

The core trait for event sources:

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

### EventAction Enum

The action returned by event-driven nodes:

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

### EventDrivenWorkflow

The workflow orchestrator for event-driven nodes:

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
```

## Usage Examples

### Basic Event-Driven Workflow

```rust
use flowrs_core::prelude::*;
use flowrs_event::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

// Define an event type
#[derive(Clone)]
struct SensorEvent {
    id: String,
    value: f64,
}

impl Event for SensorEvent {}

// Create an event source
let (tx, rx) = mpsc::channel::<SensorEvent>(100);
let source = Arc::new(ChannelEventSource::new(rx));

// Create an event-driven node
struct SensorNode {
    id: NodeId,
    source: Arc<dyn EventSource<SensorEvent>>,
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for SensorNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FlowrsError> {
        self.source.next_event().await
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction<SensorEvent>, FlowrsError> {
        println!("Sensor {}: {}", event.id, event.value);

        if event.value > 100.0 {
            Ok(EventAction::Terminate)
        } else {
            Ok(EventAction::Route("self".to_string(), event))
        }
    }
}

// Create and run the workflow
#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    let mut workflow = EventDrivenWorkflow::<SensorEvent>::new();

    let node = SensorNode {
        id: NodeId::new(),
        source: Arc::clone(&source),
    };

    let node_id = workflow.add_node(node);
    workflow.set_initial_node(node_id);
    workflow.set_route("self", node_id);

    // Run the workflow in a separate task
    let workflow_handle = tokio::spawn(async move {
        workflow.run().await
    });

    // Send events
    tx.send(SensorEvent { id: "sensor1".to_string(), value: 42.0 }).await?;
    tx.send(SensorEvent { id: "sensor1".to_string(), value: 75.0 }).await?;
    tx.send(SensorEvent { id: "sensor1".to_string(), value: 120.0 }).await?;

    // Wait for the workflow to complete
    workflow_handle.await??;

    Ok(())
}
```

### Integration with Standard Workflows

```rust
use flowrs_core::prelude::*;
use flowrs_event::prelude::*;
use flowrs_event::adapter::EventDrivenNodeAdapter;

// Create an event-driven node adapter
let event_node = SensorNode {
    id: NodeId::new(),
    source: Arc::clone(&source),
};

let adapter = EventDrivenNodeAdapter::new(event_node);

// Use the adapter in a standard workflow
let mut workflow = Workflow::new();
workflow.add_node("event_node", adapter);
workflow.set_initial_node("event_node");

// Run the standard workflow
workflow.run(&mut context).await?;
```

## Best Practices

When using the `flowrs-event` crate, consider these best practices:

1. **Event Design**: Design event types to be small, focused, and immutable.
2. **Error Handling**: Properly handle errors in event processing, especially for external event sources.
3. **Timeouts**: Use timeouts to prevent workflows from hanging indefinitely.
4. **Resource Management**: Ensure event sources are properly closed when no longer needed.
5. **Backpressure**: Implement backpressure mechanisms for high-volume event sources.

## Related Documentation

- [Event-Driven Workflow Pattern](../architecture/event-driven-workflow-pattern.md)
- [Event-Driven Workflow Example](../examples/event-driven-workflow.md)
