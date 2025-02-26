# Event-Driven Workflow Example

This document provides a complete example of using event-driven workflow capabilities in the Floxide framework.

## Overview

Event-driven workflows allow you to build reactive systems that respond to external events in real-time. This example demonstrates how to create and use event-driven nodes to process events from various sources.

## Prerequisites

Before running this example, ensure you have the following dependencies in your `Cargo.toml`:

```toml
[dependencies]
floxide-core = "0.1.0"
floxide-transform = "0.1.0"
floxide-event = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
```

## Example Implementation

### Step 1: Define Event Types

First, define the event types for your event-driven workflow:

```rust
use floxide_core::prelude::*;
use floxide_event::prelude::*;
use chrono::{DateTime, Utc};
use std::sync::Arc;

// Define a sensor event type
#[derive(Clone, Debug)]
struct SensorEvent {
    id: String,
    value: f64,
    timestamp: DateTime<Utc>,
}

// Implement the Event trait for the sensor event
impl Event for SensorEvent {}
```

### Step 2: Create Event Sources

Next, create event sources that will provide events to your workflow:

```rust
use tokio::sync::mpsc;

// Create channel-based event sources
let (sensor_tx, sensor_rx) = mpsc::channel::<SensorEvent>(100);
let sensor_source = ChannelEventSource::new(sensor_rx);

// Create a shared event source that can be used by multiple nodes
let shared_sensor_source = Arc::new(sensor_source);
```

### Step 3: Implement Event-Driven Nodes

Now, implement event-driven nodes that will process the events:

```rust
// Create a sensor monitor node
struct SensorMonitorNode {
    id: NodeId,
    event_source: Arc<dyn EventSource<SensorEvent>>,
    threshold: f64,
}

impl SensorMonitorNode {
    fn new(id_str: &str, event_source: Arc<dyn EventSource<SensorEvent>>, threshold: f64) -> Self {
        Self {
            id: NodeId::from_string(id_str),
            event_source,
            threshold,
        }
    }
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for SensorMonitorNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FloxideError> {
        self.event_source.next_event().await
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction<SensorEvent>, FloxideError> {
        println!("Processing sensor event: id={}, value={}", event.id, event.value);

        // Route based on the sensor value
        if event.value > self.threshold {
            println!("High value detected, routing to alert handler");
            Ok(EventAction::Route("alert_handler".to_string(), event))
        } else {
            println!("Normal value, routing to standard handler");
            Ok(EventAction::Route("standard_handler".to_string(), event))
        }
    }
}

// Create an alert handler node
struct AlertHandlerNode {
    id: NodeId,
}

impl AlertHandlerNode {
    fn new(id_str: &str) -> Self {
        Self {
            id: NodeId::from_string(id_str),
        }
    }
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for AlertHandlerNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FloxideError> {
        // This node doesn't wait for events directly, it receives them from the workflow
        // In a real implementation, you might want to add a timeout here
        Err(FloxideError::EventSourceClosed)
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction<SensorEvent>, FloxideError> {
        println!("ALERT: High sensor value detected: id={}, value={}", event.id, event.value);

        // Send an alert notification (in a real system)
        // ...

        // Continue the workflow
        Ok(EventAction::Route("logging".to_string(), event))
    }
}

// Create a standard handler node
struct StandardHandlerNode {
    id: NodeId,
}

impl StandardHandlerNode {
    fn new(id_str: &str) -> Self {
        Self {
            id: NodeId::from_string(id_str),
        }
    }
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for StandardHandlerNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FloxideError> {
        // This node doesn't wait for events directly, it receives them from the workflow
        Err(FloxideError::EventSourceClosed)
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction<SensorEvent>, FloxideError> {
        println!("Standard processing for sensor: id={}, value={}", event.id, event.value);

        // Process the event (in a real system)
        // ...

        // Continue the workflow
        Ok(EventAction::Route("logging".to_string(), event))
    }
}

// Create a logging node
struct LoggingNode {
    id: NodeId,
}

impl LoggingNode {
    fn new(id_str: &str) -> Self {
        Self {
            id: NodeId::from_string(id_str),
        }
    }
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for LoggingNode {
    fn id(&self) -> NodeId {
        self.id
    }

    async fn wait_for_event(&self) -> Result<SensorEvent, FloxideError> {
        // This node doesn't wait for events directly, it receives them from the workflow
        Err(FloxideError::EventSourceClosed)
    }

    async fn process_event(&self, event: SensorEvent) -> Result<EventAction<SensorEvent>, FloxideError> {
        println!("Logging event: id={}, value={}, timestamp={}",
                 event.id, event.value, event.timestamp);

        // In a real system, you might log to a database or file
        // ...

        // Return to the monitor node to wait for the next event
        Ok(EventAction::Route("monitor".to_string(), event))
    }
}
```

### Step 4: Create and Configure the Workflow

Now, create and configure the event-driven workflow:

```rust
// Create the event-driven workflow
let mut workflow = EventDrivenWorkflow::<SensorEvent>::new();

// Create the nodes
let monitor_node = SensorMonitorNode::new("monitor", shared_sensor_source, 100.0);
let alert_handler = AlertHandlerNode::new("alert_handler");
let standard_handler = StandardHandlerNode::new("standard_handler");
let logging_node = LoggingNode::new("logging");

// Add nodes to the workflow
let monitor_id = workflow.add_node(monitor_node);
let alert_id = workflow.add_node(alert_handler);
let standard_id = workflow.add_node(standard_handler);
let logging_id = workflow.add_node(logging_node);

// Configure routes
workflow.set_initial_node(monitor_id);
workflow.set_route("alert_handler", alert_id);
workflow.set_route("standard_handler", standard_id);
workflow.set_route("logging", logging_id);
workflow.set_route("monitor", monitor_id);

// Set a timeout for the workflow (optional)
workflow.set_timeout(std::time::Duration::from_secs(300)); // 5 minutes
```

### Step 5: Run the Workflow and Send Events

Finally, run the workflow and send events to it:

```rust
#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    // Create and configure the workflow (as shown above)
    // ...

    // Run the workflow in a separate task
    let workflow_handle = tokio::spawn(async move {
        if let Err(e) = workflow.run().await {
            eprintln!("Workflow error: {}", e);
        }
    });

    // Send some test events
    for i in 1..=5 {
        let value = if i % 2 == 0 { 120.0 } else { 80.0 };

        let event = SensorEvent {
            id: format!("sensor-{}", i),
            value,
            timestamp: Utc::now(),
        };

        println!("Sending event: id={}, value={}", event.id, event.value);
        sensor_tx.send(event).await.unwrap();

        // Wait a bit between events
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Wait a bit for processing to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // In a real application, you might wait for user input to terminate
    // or use a signal handler to gracefully shut down

    Ok(())
}
```

## Running the Example

To run this example:

1. Create a new Rust project with the dependencies listed above
2. Copy the code into your `src/main.rs` file
3. Run the example with `cargo run`

You should see output similar to:

```
Sending event: id=sensor-1, value=80.0
Processing sensor event: id=sensor-1, value=80.0
Normal value, routing to standard handler
Standard processing for sensor: id=sensor-1, value=80.0
Logging event: id=sensor-1, value=80.0, timestamp=2024-02-25T12:34:56.789012Z

Sending event: id=sensor-2, value=120.0
Processing sensor event: id=sensor-2, value=120.0
High value detected, routing to alert handler
ALERT: High sensor value detected: id=sensor-2, value=120.0
Logging event: id=sensor-2, value=120.0, timestamp=2024-02-25T12:34:57.789012Z

...
```

## Advanced Techniques

### Custom Event Sources

You can create custom event sources for different types of event producers:

```rust
// Create a WebSocket event source
struct WebSocketEventSource<E> {
    // WebSocket connection details
    // ...
    _phantom: PhantomData<E>,
}

#[async_trait]
impl<E> EventSource<E> for WebSocketEventSource<E>
where
    E: Event + Send + 'static,
    for<'de> E: serde::Deserialize<'de>,
{
    async fn next_event(&self) -> Result<E, FloxideError> {
        // Wait for and parse the next WebSocket message
        // ...
    }

    async fn has_more_events(&self) -> Result<bool, FloxideError> {
        // Check if the WebSocket connection is still open
        // ...
    }
}
```

### Event Filtering

You can add filtering capabilities to your event-driven nodes:

```rust
struct FilteredSensorNode {
    id: NodeId,
    event_source: Arc<dyn EventSource<SensorEvent>>,
    filter: Box<dyn Fn(&SensorEvent) -> bool + Send + Sync>,
}

impl FilteredSensorNode {
    fn new(
        id_str: &str,
        event_source: Arc<dyn EventSource<SensorEvent>>,
        filter: impl Fn(&SensorEvent) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: NodeId::from_string(id_str),
            event_source,
            filter: Box::new(filter),
        }
    }
}

#[async_trait]
impl EventDrivenNode<SensorEvent> for FilteredSensorNode {
    // ... implementation with filtering logic
}

// Usage
let temperature_filter = |event: &SensorEvent| event.id.starts_with("temp-");
let filtered_node = FilteredSensorNode::new("temp_monitor", shared_source, temperature_filter);
```

## Conclusion

This example demonstrates how to use event-driven workflows in the Floxide framework to build reactive systems. By leveraging the `EventDrivenNode`, `EventSource`, and `EventDrivenWorkflow` abstractions, you can create powerful and flexible event processing pipelines.

For more information on event-driven workflows, refer to the [Event-Driven Workflow Pattern](../architecture/event-driven-workflow-pattern.md) documentation.
