# ADR-0008: Event-Driven Node Extensions

## Status

Accepted

## Date

2025-02-27

## Context

The floxide framework is being extended with event-driven capabilities through the `floxide-event` crate. This crate provides functionality for creating nodes that respond to external events and can be integrated into standard workflows. However, several issues have been identified in the current implementation:

1. The `EventProcessor` struct has unused type parameters (`Context` and `Action`)
2. The `FloxideError` enum lacks methods needed by the event system (`node_not_found` and `timeout`)
3. The `NodeOutcome::RouteToAction` variant is being used incorrectly with two arguments when it expects one
4. Type inference issues in several places where trait bounds cannot be properly resolved
5. Missing trait implementations, specifically `Default` for the `Action` type parameter
6. Incorrect variant names for `DefaultAction` in the `EventActionExt` trait implementation

These issues need to be addressed to ensure the event-driven node extensions work correctly within the framework.

## Decision

We will make the following changes to address the issues:

1. **Fix `EventProcessor` type parameters**: Update the `EventProcessor` struct to either use the unused type parameters or remove them, using `PhantomData` where necessary.

2. **Extend `FloxideError` with required methods**: Add methods to the `FloxideError` type in the core crate for `node_not_found` and `timeout` functionality.

3. **Fix `NodeOutcome::RouteToAction` usage**: Update the event crate to correctly use this variant with a single argument instead of two.

4. **Address type inference issues**: Add proper type annotations and function signatures to improve type inference in the event-driven components.

5. **Add required trait bounds**: Ensure that all generic type parameters have the necessary trait bounds, including `Default` where required.

6. **Correct EventActionExt implementation**: Fix the implementation to use the proper function syntax instead of non-existent enum variants.

7. **Clean up unused imports**: Remove unused imports in both the transform and event crates.

## Implementation Details

The implementation involved the following key changes:

1. Added `node_not_found`, `timeout`, and `is_timeout` methods to `FloxideError` in the core crate.

2. Fixed `EventProcessor` by adding `PhantomData` for unused type parameters and adding proper trait bounds.

3. Updated the `EventDrivenNode` trait to change `wait_for_event` from taking an immutable reference to a mutable reference, solving a borrowing issue with the `mpsc::Receiver`.

4. Replaced all incorrect uses of `NodeOutcome::RouteToAction` with the correct single-argument syntax.

5. Added the `#[async_trait]` macro to all implementations of async traits.

6. Used fully-qualified path syntax to address type inference issues in the `wait_for_event` and `id` methods.

7. Wrapped the `ChannelEventSource` in a `tokio::sync::Mutex` to allow safe mutable access from multiple places.

8. Fixed the `EventActionExt` trait implementation for `DefaultAction` to create proper custom actions.

9. Removed unused imports in both crates.

## Consequences

### Positive

- The event-driven extensions now compile and work correctly
- The code is more type-safe and follows Rust best practices
- Better error handling with proper error types for event-driven workflows
- Cleaner code with unused imports removed
- Improved consistency between the core, transform, and event APIs

### Negative

- Small API changes may require updates to existing code that uses these features:
  - `EventDrivenNode::wait_for_event` now requires a mutable reference
  - The `Action` type parameter now requires the `Default` trait

### Neutral

- The architecture of the event-driven node system remains fundamentally the same
- Core framework abstractions are unchanged

## Future Work

- Consider adding more comprehensive documentation for the event-driven capabilities
- Explore adding more event source implementations (e.g., WebSocket, HTTP, etc.)
- Consider adding a more ergonomic API for creating event-driven workflows
