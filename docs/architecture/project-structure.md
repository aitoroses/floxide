# Project Structure

This document describes the overall structure of the Flowrs framework codebase.

## Workspace Organization

The Flowrs framework is organized as a Cargo workspace with multiple crates, each with a specific responsibility:

```
flowrs/
├── Cargo.toml              # Workspace definition
├── flowrs-core/            # Core abstractions and interfaces
├── flowrs-transform/       # Functional transformation capabilities
├── flowrs-event/           # Event-driven workflow support
├── flowrs-timer/           # Timer and scheduling functionality
├── flowrs-reactive/        # Reactive programming patterns
├── flowrs-longrunning/     # Long-running task support
├── flowrs-async/           # (Deprecated) Alias for flowrs-transform
└── docs/                   # Documentation
    ├── adrs/              # Architectural Decision Records
    ├── api/               # API Documentation
    ├── architecture/      # Architecture Documentation
    └── examples/          # Example Code
```

## Crate Responsibilities

### flowrs-core

The `flowrs-core` crate provides the fundamental abstractions and interfaces for the framework:

- Node trait and lifecycle methods (prep, exec, post)
- Context and state management
- Workflow orchestration
- Action types and transitions
- Error handling with custom error types

### flowrs-transform

The `flowrs-transform` crate (formerly `flowrs-async`) provides functional transformation capabilities:

- Explicit input/output type transformations
- Custom error types per transformation
- Three-phase transformation lifecycle
- Functional composition patterns

### flowrs-event

The `flowrs-event` crate provides event-driven workflow support:

- Event emission and handling
- Event-based routing
- Event context management
- Pub/sub patterns

### flowrs-timer

The `flowrs-timer` crate provides timer and scheduling functionality:

- Multiple schedule types (Once, Periodic, Cron)
- Timer workflow composition
- Timeout handling
- Scheduled task orchestration

### flowrs-reactive

The `flowrs-reactive` crate provides reactive programming patterns:

- Stream-based processing
- Backpressure handling
- Change detection
- Reactive node composition

### flowrs-longrunning

The `flowrs-longrunning` crate provides support for long-running tasks:

- Background processing
- Progress tracking
- Cancellation support
- Resource cleanup

### flowrs-async (Deprecated)

This crate is deprecated and exists only for backward compatibility. It re-exports everything from `flowrs-transform`. Users should migrate to `flowrs-transform`.

## Documentation Structure

### API Documentation (`/docs/api/`)

Detailed API documentation for each crate:
- `flowrs-core.md` - Core abstractions and interfaces
- `flowrs-reactive.md` - Reactive programming patterns
- `flowrs-timer.md` - Timer and scheduling
- `flowrs-transform.md` - Transformation patterns

### Architecture Documentation (`/docs/architecture/`)

High-level architectural documentation:
- Design decisions and patterns
- Component interactions
- Implementation guidelines
- Performance considerations

### Examples (`/docs/examples/`)

Comprehensive examples demonstrating framework usage:
- `basic-workflow.md` - Core workflow concepts
- `reactive-node.md` - Reactive programming patterns
- `timer-node.md` - Timer and scheduling
- `transform-node.md` - Data transformation
- `event-driven-workflow.md` - Event handling
- `longrunning-node.md` - Long-running tasks

### ADRs (`/docs/adrs/`)

Architectural Decision Records documenting all significant decisions:
- Numbered sequentially (e.g., ADR-0001)
- Each ADR covers one architectural decision
- Includes context, consequences, and alternatives
- Marks superseded decisions

## Dependencies Between Crates

The crates have the following dependency relationships:

- All crates depend on `flowrs-core`
- `flowrs-async` depends on `flowrs-transform`
- Dependencies are kept minimal to allow users to include only what they need
- Each crate can be used independently (except `flowrs-async`)

## Testing Structure

Each crate follows a consistent testing structure:

- Unit tests in `src/tests/` modules
- Integration tests in `tests/` directory
- Example code in `/docs/examples/`
- Property-based tests where appropriate

## Future Development

Planned future additions:
- Batch processing capabilities
- Enhanced monitoring and metrics
- Additional node types and patterns
- Extended workflow composition features
