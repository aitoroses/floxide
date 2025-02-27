# ADR-0009: Event-Driven Workflow Pattern

## Status

Accepted

## Date

2025-02-27

## Context

The Flow Framework is primarily designed around synchronous, request-response style workflows where nodes are executed in a predefined sequence. However, many real-world systems require handling of asynchronous events where the timing of events is unpredictable.

Examples include:

- IoT sensor monitoring systems
- Message queue processors
- User interaction workflows
- Long-running background processes

The framework needs to support event-driven workflows where nodes can wait for external events and then process them according to workflow rules.

## Decision

We will extend the Flow Framework with a dedicated event subsystem by introducing:

1. **EventDrivenNode Trait**: A specialized node trait for handling events with these key methods:

   - `wait_for_event()`: Waits for an external event to arrive
   - `process_event()`: Processes the event and returns an action
   - `id()`: Returns the node's unique identifier

2. **Event Sources**: Implementations of event sources such as:

   - `ChannelEventSource`: Receives events from a Tokio MPSC channel
   - (Future) Other event sources for websockets, HTTP, etc.

3. **EventDrivenWorkflow**: A specialized workflow that:

   - Contains a collection of event-driven nodes
   - Routes events between nodes based on actions
   - Executes until a termination condition is met
   - Supports timeouts for bounded execution

4. **Integration with Standard Workflows**: Adapters to:

   - Use event-driven nodes within standard workflows (`EventDrivenNodeAdapter`)
   - Use event-driven workflows as nodes in standard workflows (`NestedEventDrivenWorkflow`)

5. **EventActionExt Trait**: An extension trait providing common event-related actions:
   - `terminate()`: Creates a termination action
   - `timeout()`: Creates a timeout action

## Workflow Pattern

The event-driven workflow follows this pattern:

1. **Setup Phase**:

   - Create event sources
   - Create event processors
   - Configure the workflow routing

2. **Execution Phase**:

   - The workflow starts with an initial event source node
   - The source waits for an external event
   - When an event arrives, it's passed to the next node based on routing rules
   - The node processes the event and returns an action
   - The action determines the next node to route to
   - This continues until a termination action is received or a timeout occurs

3. **Termination Phase**:
   - The workflow terminates when a node returns a designated termination action
   - Alternatively, the workflow can time out if configured with a timeout

## Key Benefits

1. **Non-blocking Event Handling**: Allows nodes to wait for events without blocking the thread
2. **Dynamic Routing**: Routes events based on their content/classification
3. **Integration with Standard Workflows**: Can be used seamlessly with regular workflows
4. **Timeout Support**: Prevents workflows from hanging indefinitely
5. **Stateful Processing**: Maintains context between events

## Implementation Approach

We have implemented this pattern in the `floxide-event` crate, which provides:

1. The core traits and interfaces for event-driven nodes
2. Basic event source implementations (channel-based)
3. Adapters for integrating with standard workflows
4. Workflows for orchestrating event-driven nodes

The implementation has been designed to be:

- **Type-safe**: Using Rust's type system to ensure correctness
- **Async-first**: Built around Tokio's async ecosystem
- **Composable**: Can be used alongside other framework components
- **Extensible**: New event sources can be added without modifying the core

## Example Use Cases

1. **IoT Monitoring**: Processing temperature/humidity readings from sensors
2. **Chat Application**: Handling incoming messages in a chat system
3. **Job Queue**: Processing jobs from a distributed queue
4. **User Session**: Handling events during a user session

## Consequences

### Positive

- Enables event-driven application patterns within the Flow Framework
- Allows for integration with external event sources
- Maintains type safety throughout the event pipeline
- Fits naturally into the existing node/action framework model

### Negative

- Adds complexity to the framework
- Requires understanding of both workflow and event concepts
- Might increase the learning curve for new users

### Neutral

- Requires additional testing of asynchronous patterns
- May need refinement as real-world usage patterns emerge
