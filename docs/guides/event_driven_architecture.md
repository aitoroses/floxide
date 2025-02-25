# Event-Driven Workflow Architecture

This guide explains the event-driven workflow architecture in the Flow Framework, as implemented in the temperature monitoring example.

## Overview

The event-driven workflow pattern is designed to handle asynchronous, unpredictable events and process them through a directed graph of nodes. Unlike traditional request-response workflows, event-driven workflows continue processing indefinitely until a termination condition is met.

## Architecture Diagram

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────────┐
│                 │     │                 │     │                     │
│  Event Sources  │────▶│  Event-Driven   │────▶│  Handler Workflows  │
│  (Sensors)      │     │  Workflow       │     │  (Actions)          │
│                 │     │                 │     │                     │
└─────────────────┘     └─────────────────┘     └─────────────────────┘
                              │                           │
                              │                           │
                              ▼                           ▼
                        ┌──────────────┐          ┌─────────────────┐
                        │              │          │                 │
                        │   Context    │◀─────────│    Feedback     │
                        │  Management  │          │     Loop        │
                        │              │          │                 │
                        └──────────────┘          └─────────────────┘
```

## Temperature Monitoring Example

In the temperature monitoring example, the architecture is implemented as follows:

### 1. Event Sources

- Multiple temperature sensors (simulated)
- Send events to a channel-based event source
- Events are asynchronous and unpredictable

```rust
// Create the event source with a buffer capacity of 100 events
let (source, sender) = ChannelEventSource::new(100);
```

### 2. Event-Driven Workflow

- Receives events from sources
- Routes events to appropriate processors
- Contains an event classification node

```rust
// Create the event-driven workflow with a termination action of Complete
let mut workflow = EventDrivenWorkflow::new(
    source.clone(),
    TempAction::Complete,
);

// Add the classifier node to the workflow
workflow.add_node(classifier.clone());
```

### 3. Event Classification

- Analyzes temperature events
- Categorizes them based on thresholds
- Returns appropriate actions

```rust
// The classifier processes events and returns actions
let classifier = TemperatureClassifier::new(30.0, 10.0, 40.0);
```

### 4. Handler Workflows

- Specialized nodes for different temperature ranges
- Trigger appropriate actions based on classification
- Can terminate the workflow when needed

```rust
// Add nodes for different temperature classifications
let normal_handler = NormalTempHandler::new();
let high_handler = HighTempHandler::new();
let low_handler = LowTempHandler::new();
let critical_handler = CriticalTempHandler::new();
```

### 5. Context Management

- Maintains state across events
- Records temperature history
- Tracks alerts and statistics

```rust
// The context maintains state across the workflow
struct MonitoringContext {
    temperature_history: HashMap<String, Vec<f32>>,
    alerts: Vec<String>,
    average_temperatures: HashMap<String, f32>,
}
```

## Flow of Events

1. **Event Generation**: Sensors generate temperature readings
2. **Event Transmission**: Readings are sent to the event-driven workflow
3. **Event Classification**: The classifier node categorizes the temperature
4. **Action Routing**: Based on the classification, the workflow routes to the appropriate handler
5. **Action Execution**: The handler performs the necessary actions
6. **State Update**: The context is updated with new information
7. **Continuation/Termination**: The workflow continues or terminates based on conditions

## Implementation Considerations

When implementing an event-driven workflow, consider the following:

1. **Event Source Design**: How events are generated and retrieved
2. **Event Classification Logic**: Rules for categorizing events
3. **Action Types**: The set of possible actions that can result from events
4. **Routing Logic**: How events are routed through the workflow
5. **Termination Conditions**: When and how the workflow should terminate
6. **Timeout Handling**: How to handle hanging or slow event sources
7. **Context Management**: What state needs to be maintained across events

## Integration with Standard Workflows

Event-driven workflows can be integrated with standard workflows using adapters:

```rust
// Create a nested event-driven workflow adapter
let nested_workflow = NestedEventDrivenWorkflow::new(
    workflow.clone(),
    TempAction::Complete,
    TempAction::Timeout,
);
```

This allows for combining both synchronous and asynchronous processing patterns.

## References

- [ADR-0009: Event-Driven Workflow Pattern](../adrs/0009-event-driven-workflow-pattern.md)
- [Example: Temperature Monitoring System](../../examples/examples/event_driven_workflow.rs)
