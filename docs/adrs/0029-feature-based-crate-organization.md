# ADR-0029: Feature-Based Crate Organization

## Status

Accepted

## Date

2024-02-26

## Context

Currently, the Floxide framework is organized as a workspace with multiple subcrates, each providing specific functionality:

- `floxide-core`: Core abstractions and functionality
- `floxide-transform`: Transform node implementations
- `floxide-event`: Event-driven workflow functionality
- `floxide-timer`: Time-based workflow functionality
- `floxide-longrunning`: Long-running process functionality
- `floxide-reactive`: Reactive workflow functionality

While this modular approach provides flexibility, it also introduces complexity for users who need to manage multiple dependencies. We want to maintain the ability to selectively include functionality while simplifying the user experience.

## Decision

We will reorganize the crate structure to use feature flags instead of separate crates for publishing. This approach will:

1. Publish a single crate named `floxide` to crates.io
2. Use feature flags to enable/disable specific functionality
3. Maintain the workspace structure for development
4. Conditionally re-export modules based on enabled features

### Feature Structure

The following features will be defined:

- `core` (default): Core abstractions and functionality
- `transform`: Transform node implementations
- `event`: Event-driven workflow functionality
- `timer`: Time-based workflow functionality
- `longrunning`: Long-running process functionality
- `reactive`: Reactive workflow functionality
- `full`: Enables all features

### Implementation Approach

1. Keep the root crate name as `floxide` in Cargo.toml
2. Add feature definitions that conditionally include subcrates
3. Modify src/lib.rs to conditionally re-export modules based on enabled features
4. Update examples to use the appropriate features
5. Update the GitHub Actions workflow to publish the single crate

## Consequences

### Positive

1. **Simplified User Experience**: Users can include a single dependency with desired features
2. **Reduced Dependency Management**: No need to manage version compatibility between subcrates
3. **Flexible Inclusion**: Users can include only the functionality they need
4. **Smaller Binaries**: Applications only include the code they use
5. **Consistent Branding**: Maintains the "floxide" name across all components

### Negative

1. **Migration Effort**: Existing users will need to update their dependencies
2. **More Complex Conditional Compilation**: The codebase will use more `#[cfg(feature = "...")]` directives
3. **Potential for Unused Code**: Users might include more functionality than needed if they don't carefully select features

## Alternatives Considered

### Keep Current Structure with Multiple Crates

- **Pros**:
  - Minimal changes required
  - Clear separation of concerns
- **Cons**:
  - Users still need to manage multiple dependencies
  - More complex dependency graph

### Single Monolithic Crate

- **Pros**:
  - Simplest user experience
  - No conditional compilation needed
- **Cons**:
  - No way to exclude unused functionality
  - Larger binary sizes
  - Less modular development

## Implementation Plan

1. Implement feature flags in the root crate
2. Update documentation to reflect the new structure
3. Create a migration guide for existing users
4. Update CI/CD pipeline for the new publishing approach 