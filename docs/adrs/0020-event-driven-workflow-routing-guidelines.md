# ADR-0020: Event-Driven Workflow Routing Guidelines

## Status

Proposed

## Date

2023-05-29

## Context

In implementing event-driven workflows with the Flow Framework, we've encountered challenges with proper routing between event sources and processing nodes. Specifically, the `event_driven_workflow.rs` example exhibits an error during execution: "TemperatureClassifier is not an event source" when the workflow routing attempts to use a processor node as an event source.

The current event-driven workflow pattern allows for connections between nodes using the `set_route` method, but doesn't clearly distinguish between nodes that can generate events (event sources) and nodes that can only process them (processors). This can lead to execution errors when the workflow engine attempts to wait for events from a node that cannot generate them.

Key issues observed:

1. Processing nodes incorrectly being treated as event sources
2. Confusion about the correct routing patterns for different node types
3. Unclear execution flow for event-driven workflows with multiple node types

## Decision

We will establish clear routing guidelines for event-driven workflows to ensure proper distinction between event sources and processor nodes:

1. **Source-Processor Routing Pattern**:

   - Event sources (implementing `wait_for_event`) should be the only nodes that generate new events
   - Processor nodes should only receive events for processing and return actions
   - After a processor node completes, the workflow should always return to an event source

2. **Routing Rules**:

   - All action routes from processor nodes must point to valid event sources
   - When a processor returns a non-terminating action, execution must be routed to an event source
   - Processors should never route to other processors without going through an event source first

3. **Workflow Validation**:

   - Add validation in `EventDrivenWorkflow` to ensure routes from processors lead to valid event sources
   - Provide clear error messages when invalid routing is detected during workflow construction
   - Add debugging facilities to trace the execution flow of event-driven workflows

4. **Documentation Updates**:
   - Clearly document the distinction between event sources and processors in API docs
   - Provide examples of correct routing patterns in the examples

## Consequences

### Positive

- Clearer understanding of the roles of different node types in event-driven workflows
- Reduced likelihood of runtime errors due to invalid routing
- More predictable workflow execution paths
- Better diagnostic information for debugging workflows

### Negative

- Additional validation logic adds complexity to the workflow engine
- May require refactoring of existing workflows to comply with the new guidelines
- Stricter routing rules might feel constraining for some use cases

### Neutral

- This formalization makes explicit what was previously implicit in the design

## Implementation

The implementation will involve:

1. Adding validation checks in the `set_route` method of `EventDrivenWorkflow` to verify that routes from processors point to event sources
2. Enhancing error messages to provide more context about routing issues
3. Updating the event-driven workflow example to clearly demonstrate correct routing patterns
4. Adding documentation comments that explain the routing rules and patterns

## Related ADRs

- [ADR-0009: Event-Driven Workflow Pattern](./0009-event-driven-workflow-pattern.md)
- [ADR-0017: Event-Driven Node Pattern](./0017-event-driven-node-pattern.md)
