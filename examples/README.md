# Flow Framework Examples

This directory contains a collection of examples demonstrating the various capabilities and patterns of the Flow Framework. Each example illustrates different aspects of workflow creation, node implementation, and execution patterns.

## Running Examples

To run any example, use the following command from the root of the repository:

```bash
cargo run --example <example_name>
```

For instance, to run the event-driven workflow example:

```bash
cargo run --example event_driven_workflow
```

## Available Examples

### 1. Lifecycle Node (`lifecycle_node`)

Demonstrates the basic lifecycle of a node in the Flow Framework, including:

- Node creation and configuration
- The prep/exec/post lifecycle phases
- Error handling within nodes
- Basic workflow execution

### 2. Transform Node (`transform_node`)

Shows how to use transformation nodes to process and modify data within a workflow:

- Creating nodes that transform input data
- Composing multiple transformations
- Using generics for type-safe transformations
- Error handling during transformations

### 3. Order Processing (`order_processing`)

A business-oriented example that simulates an e-commerce order processing workflow:

- Multiple stages of order processing (validation, payment, fulfillment)
- Conditional branching based on order status
- Context sharing between workflow steps
- Error handling and recovery in business workflows

### 4. Batch Processing (`batch_processing`)

Demonstrates how to process collections of items in parallel using batch workflows:

- Efficient processing of multiple items
- Configuration of batch size and parallelism
- Aggregation of batch results
- Error handling in parallel execution contexts

### 5. Event-Driven Workflow (`event_driven_workflow`)

A comprehensive example of an event-driven system for temperature monitoring:

#### Overview

This example implements a temperature monitoring system that processes temperature events from multiple sensors and triggers appropriate actions based on the temperature readings. It demonstrates how to use the event-driven capabilities of the Flow Framework to handle asynchronous, unpredictable events.

#### Key Components

1. **Event Sources**:

   - Simulated temperature sensors generating temperature readings
   - Events transmitted through a channel-based event source

2. **Event Processing Nodes**:

   - `TemperatureClassifier`: Processes temperature events and classifies them as normal, high, low, or critical
   - Various handler nodes for different temperature classifications

3. **Context Management**:

   - `MonitoringContext`: Maintains temperature history, alerts, and average temperatures
   - Provides methods for recording temperatures and generating alerts

4. **Workflow Structure**:
   - An event-driven workflow that receives temperature events
   - A standard workflow for handling different temperature classifications
   - Integration between the two workflow types

#### Running the Example

When you run this example:

1. Three simulated temperature sensors will start generating random temperature readings
2. The event-driven workflow will receive these events and process them
3. Based on temperature thresholds, different handler nodes will be activated
4. Various actions will be taken (logging, simulated cooling/heating)
5. If a critical temperature is detected, the workflow will terminate

#### Learning Outcomes

This example demonstrates:

- How to create and use event-driven nodes and workflows
- How to integrate event-driven workflows with standard workflows
- How to implement custom action types for specialized routing
- How to handle timeouts and termination conditions
- How to maintain context across multiple events

#### Implementation Details

The example uses several advanced features of the Flow Framework:

- Channel-based event sources (`ChannelEventSource`)
- Custom action types implementing the `EventActionExt` trait
- Workflow routing based on event classification
- Timeout-based execution boundaries
- Nested workflow composition

## Advanced Concepts Demonstrated

These examples collectively showcase several advanced concepts in the Flow Framework:

1. **Node Lifecycle Management**: How nodes are prepared, executed, and cleaned up
2. **Workflow Composition**: Combining multiple nodes into cohesive workflows
3. **Context Sharing**: Passing and modifying context between workflow steps
4. **Error Handling**: Strategies for handling and recovering from errors
5. **Concurrency Patterns**: Processing items in parallel with proper synchronization
6. **Event-Driven Workflows**: Handling asynchronous events in a structured way
7. **Type Safety**: Using Rust's type system for safe workflow execution

## Creating Your Own Examples

When creating your own examples:

1. Add your example file to the `examples/` directory
2. Add an entry to `Cargo.toml` under the `[[example]]` section
3. Ensure your example has proper documentation
4. Follow the established patterns for node and workflow creation

## References

For more information, refer to:

- [Flow Framework Documentation](../README.md)
- [Architectural Decision Records](../docs/adrs/)
- [API Reference](https://docs.rs/flowrs-core)
