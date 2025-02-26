# Project Structure

This document describes the overall structure of the Floxide framework codebase.

## Workspace Organization

The Floxide framework is organized as a Cargo workspace with multiple crates, each with a specific responsibility:

```
floxide/
├── Cargo.toml              # Workspace definition
├── floxide-core/            # Core abstractions and interfaces
├── floxide-transform/       # Functional transformation capabilities
├── floxide-event/           # Event-driven workflow support
├── floxide-timer/           # Timer and scheduling functionality
├── floxide-reactive/        # Reactive programming patterns
├── floxide-longrunning/     # Long-running task support
├── floxide-async/           # (Deprecated) Alias for floxide-transform
└── docs/                   # Documentation
    ├── adrs/              # Architectural Decision Records
    ├── api/               # API Documentation
    ├── architecture/      # Architecture Documentation
    └── examples/          # Example Code
```

## Crate Responsibilities

### floxide-core

The `floxide-core` crate provides the fundamental abstractions and interfaces for the framework:

- Node trait and lifecycle methods (prep, exec, post)
- Context and state management
- Workflow orchestration
- Action types and transitions
- Error handling with custom error types

### floxide-transform

The `floxide-transform` crate (formerly `floxide-async`) provides functional transformation capabilities:

- Explicit input/output type transformations
- Custom error types per transformation
- Three-phase transformation lifecycle
- Functional composition patterns

### floxide-event

The `floxide-event` crate provides event-driven workflow support:

- Event emission and handling
- Event-based routing
- Event context management
- Pub/sub patterns

### floxide-timer

The `floxide-timer` crate provides timer and scheduling functionality:

- Multiple schedule types (Once, Periodic, Cron)
- Timer workflow composition
- Timeout handling
- Scheduled task orchestration

### floxide-reactive

The `floxide-reactive` crate provides reactive programming patterns:

- Stream-based processing
- Backpressure handling
- Change detection
- Reactive node composition

### floxide-longrunning

The `floxide-longrunning` crate provides support for long-running tasks:

- Background processing
- Progress tracking
- Cancellation support
- Resource cleanup

### floxide-async (Deprecated)

This crate is deprecated and exists only for backward compatibility. It re-exports everything from `floxide-transform`. Users should migrate to `floxide-transform`.

## Documentation Structure

### API Documentation (`/docs/api/`)

Detailed API documentation for each crate:
- `floxide-core.md` - Core abstractions and interfaces
- `floxide-reactive.md` - Reactive programming patterns
- `floxide-timer.md` - Timer and scheduling
- `floxide-transform.md` - Transformation patterns

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

- All crates depend on `floxide-core`
- `floxide-async` depends on `floxide-transform`
- Dependencies are kept minimal to allow users to include only what they need
- Each crate can be used independently (except `floxide-async`)

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
