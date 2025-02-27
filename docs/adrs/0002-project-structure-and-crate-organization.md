# ADR-0002: Project Structure and Crate Organization

## Status

Accepted

## Date

2025-02-27

## Context

As we begin development of the floxide framework, we need to determine how to structure the codebase. The way we organize our project will impact testability, maintainability, and the ability to evolve the codebase over time. It will also influence how consumers of our library will interact with it.

Rust provides specific patterns and organization concepts such as crates, workspaces, and modules that we need to consider for optimal organization.

## Decision

We will organize the floxide framework as a Cargo workspace with multiple crates to provide modularity and separation of concerns.

### Workspace Structure

The project will be structured as follows:

```
floxide/
├── Cargo.toml         # Workspace manifest
├── crates/
│   ├── floxide-core/   # Core traits and structures
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── floxide-transform/  # Transform node implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── floxide-derive/ # Optional proc macros for code generation
│   │   ├── Cargo.toml
│   │   └── src/
│   └── floxide-test/   # Test utilities and fixtures
│       ├── Cargo.toml
│       └── src/
├── examples/          # Example implementations
│   ├── Cargo.toml
│   └── examples/      # Standard example files
├── benches/           # Performance benchmarks
│   ├── Cargo.toml
│   └── src/
└── docs/              # Documentation
    └── adrs/          # Architectural Decision Records
```

### Core Crates

1. **floxide-core**:

   - Contains the fundamental traits and structures for the framework
   - Includes `BaseNode`, `Flow`, and `BatchFlow` implementations
   - Provides the directed graph structure and core execution model
   - Has minimal dependencies

2. **floxide-transform**:

   - Implements transformation node patterns
   - Provides the `TransformNode` trait for data transformation
   - Depends on Tokio for the async runtime
   - Provides utilities for creating transformation workflows

3. **floxide-derive** (optional):

   - Provides procedural macros for code generation
   - Simplifies common patterns through macros
   - Makes the API more ergonomic

4. **floxide-test**:
   - Contains testing utilities and fixtures
   - Provides mock implementations of framework components
   - Simplifies writing tests for consumers of the framework

### Module Organization

Within each crate, we will follow these module organization principles:

1. **Public API**:

   - Exposed through `lib.rs` with clear documentation
   - Use the re-export pattern to provide a clean public API
   - Versioned according to semver

2. **Internal Implementation**:

   - Organized in modules with a clear responsibility
   - Private modules prefixed with underscore if not part of the public API
   - Clear separation between public interfaces and internal details

3. **Tests**:
   - Unit tests in the same file as the code they test using `#[cfg(test)]`
   - Integration tests in a separate `tests/` directory

## Consequences

### Positive

1. **Modularity**: Separate crates allow for focused concerns and clear dependencies
2. **Versioning**: Each crate can evolve at its own pace
3. **Dependency Management**: Consumers only need to depend on the crates they use
4. **Testability**: Easier to write focused tests for each component
5. **Compilation Times**: Smaller compilation units can improve development experience

### Negative

1. **Complexity**: Multi-crate projects are more complex to manage
2. **Potential API Fragmentation**: Need to ensure consistent patterns across crates
3. **Version Synchronization**: Need to manage versions across interdependent crates
4. **Documentation**: More effort to provide cohesive documentation across crates

## Alternatives Considered

### Single Crate Approach

- **Pros**:

  - Simpler to manage
  - All code in one place
  - Easier to maintain API consistency
  - Simpler dependency management for consumers

- **Cons**:
  - Less flexibility for evolution
  - Longer compile times as the project grows
  - All consumers must take all features, even if not needed
  - Could lead to monolithic design

We chose the multi-crate approach to provide greater flexibility and maintainability, especially as the project grows. This aligns with Rust ecosystem practices seen in mature libraries like Tokio, Serde, and others.

### Feature-Based Single Crate

- **Pros**:

  - Maintains single crate simplicity
  - Allows optional features through Cargo features
  - Provides some flexibility without multi-crate complexity

- **Cons**:
  - Still results in longer compile times for the main crate
  - Feature combinations can lead to complexity
  - Less clear separation of concerns

While feature-based configuration will still be used within individual crates, the multi-crate approach provides clearer boundaries and better separation of concerns.
