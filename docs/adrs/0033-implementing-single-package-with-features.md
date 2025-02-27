# ADR-0033: Simplified Publishing with Maintained Subcrate Structure

## Status

Accepted (Updated)

## Date

2025-02-27

## Context

In ADR-0029, we decided to transition to a feature-based crate organization to simplify the user experience. The current structure uses multiple subcrates:
- `floxide-core`: Core abstractions and functionality
- `floxide-transform`: Transform node implementations
- `floxide-event`: Event-driven workflow functionality
- `floxide-timer`: Time-based workflow functionality
- `floxide-longrunning`: Long-running process functionality
- `floxide-reactive`: Reactive workflow functionality

This modular approach provides clear separation of concerns during development. However, it creates complexity in the publishing process, requiring subcrates to be published in a specific order before the root crate.

We want to maintain the benefits of modular development with subcrates while simplifying the publishing process.

## Decision

We will implement a standard Rust workspace approach that:

1. **Maintains the existing subcrate structure** for development
2. **Publishes subcrates in the correct order** before the root crate
3. **Uses feature flags** in the root crate to conditionally include functionality

### Implementation Strategy

1. **Keep the existing workspace structure** with subcrates for development
2. **Update the root Cargo.toml** to include all subcrates as optional dependencies with feature flags
3. **Modify the release process** to publish subcrates in the correct order before the root crate
4. **Update the GitHub Actions workflow** to automate this publishing sequence

### Feature Structure

We will maintain the existing feature structure in the root crate:

```toml
[features]
default = ["core"]
core = ["floxide-core"]
transform = ["core", "floxide-transform"]
event = ["core", "floxide-event"]
timer = ["core", "floxide-timer"]
longrunning = ["core", "floxide-longrunning"]
reactive = ["core", "floxide-reactive"]
full = ["transform", "event", "timer", "longrunning", "reactive"]
```

## Consequences

### Advantages

1. **Standard Rust Approach**: Following established patterns in the Rust ecosystem
2. **Maintained Development Structure**: Keep the benefits of modular development
3. **Improved User Experience**: Users only need to include one dependency with appropriate features
4. **Simplified Dependency Management**: Clear feature-based organization
5. **Consistent Versioning**: Coordinated version numbers across all crates

### Disadvantages

1. **Publishing Order Dependency**: Subcrates must be published before the root crate
2. **Release Process Complexity**: Need to ensure correct publishing order
3. **Potential for Version Mismatches**: If subcrates are published with different versions

## Implementation Plan

1. **Update the root Cargo.toml** to include all subcrates as optional dependencies with feature flags
2. **Update the lib.rs file** to properly re-export the subcrates
3. **Use cargo-workspaces for version management and publishing** to ensure consistent versioning across all crates
4. **Create a release script** that leverages cargo-workspaces for the publishing process
5. **Update the GitHub Actions workflow** to use the new release process

## Alternatives Considered

### Manual Publishing of Individual Crates

- **Pros**:
  - Fine-grained control over each crate's publication
  - Ability to publish only specific crates
- **Cons**:
  - Complex ordering requirements
  - Error-prone manual process
  - Difficult to maintain version consistency

We rejected this approach in favor of using cargo-workspaces for a more automated and consistent process.

### Complex Publishing Scripts Approach

- **Pros**:
  - Single publishing step
  - No need to publish subcrates separately
- **Cons**:
  - Complex and error-prone scripts
  - Non-standard approach
  - Difficult to maintain

We rejected this approach in favor of a more standard Rust workspace approach.

### Complete Transition to Single Crate

- **Pros**:
  - Simplest structure overall
  - No distinction between development and published code
- **Cons**:
  - Loss of modular development benefits
  - Significant refactoring required
  - Potential for a more complex codebase

We rejected this approach to maintain the benefits of modular development.

## Related ADRs

- [ADR-0029: Feature-Based Crate Organization](0029-feature-based-crate-organization.md)
- [ADR-0030: Workspace Dependency Versioning](0030-workspace-dependency-versioning.md)
- [ADR-0014: Crate Publishing and CI/CD](0014-crate-publishing-and-cicd.md) 