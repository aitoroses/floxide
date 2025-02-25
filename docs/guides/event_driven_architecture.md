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

- Receives events from the event source
- Processes events through a chain of nodes
- Maintains state between events using context
- Handles errors and retries gracefully

```rust
let workflow = EventDrivenWorkflow::new()
    .source(source)
    .node("process_temperature", process_temperature)
    .node("check_threshold", check_threshold)
    .node("alert", alert_handler);
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

- Execute specific actions based on event processing
- Can spawn additional workflows if needed
- Report results back to the main workflow

```rust
// Add nodes for different temperature classifications
let normal_handler = NormalTempHandler::new();
let high_handler = HighTempHandler::new();
let low_handler = LowTempHandler::new();
let critical_handler = CriticalTempHandler::new();
```

### 5. Context Management

The context in event-driven workflows needs to handle:

- Event history and aggregation
- State persistence between events
- Configuration and thresholds
- Error tracking and recovery state

```rust
// The context maintains state across the workflow
struct MonitoringContext {
    temperature_history: HashMap<String, Vec<f32>>,
    alerts: Vec<String>,
    average_temperatures: HashMap<String, f32>,
}
```

### 6. Feedback Loop

The feedback loop enables:

- Dynamic threshold adjustments
- Learning from historical data
- Adaptive event processing
- System health monitoring

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

## Best Practices

1. **Error Handling**
   - Implement proper error recovery mechanisms
   - Use retries with exponential backoff
   - Log errors for debugging and monitoring

2. **State Management**
   - Keep state minimal and focused
   - Consider persistence for critical state
   - Use atomic operations when updating state

3. **Performance Optimization**
   - Buffer events appropriately
   - Use async processing where beneficial
   - Implement backpressure mechanisms

4. **Testing**
   - Test event sources independently
   - Mock events for workflow testing
   - Verify error handling paths
   - Test state persistence and recovery

5. **Monitoring**
   - Track event processing latency
   - Monitor queue depths
   - Set up alerting for anomalies
   - Log important state transitions

6. **Timeout Handling**
   - Set appropriate timeouts for event processing
   - Implement deadletter queues
   - Handle slow event sources gracefully
   - Clean up resources on timeout

7. **Context Management**
   - Define clear boundaries for context data
   - Implement proper serialization
   - Handle context versioning
   - Clean up stale context data

## Integration with Standard Workflows

Event-driven workflows can be integrated with standard request-response workflows:

1. Use event sources as triggers for standard workflows
2. Convert workflow results into events
3. Implement hybrid patterns for complex use cases

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
- [View the complete example](../examples/event-driven-workflow.md)
