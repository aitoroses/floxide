# ADR 0038: Core as Non-Optional Dependency

## Status

Accepted

## Context

The Floxide framework was originally designed with a feature-based architecture where even the core functionality could be optionally included via the `core` feature flag. This approach caused issues with the Cargo workspace and package management, specifically when using tools like `cargo-workspaces` for version bumping.

The error manifested as:

```
feature `core` includes `floxide-core`, but `floxide-core` is not an optional dependency
A non-optional dependency of the same name is defined; consider adding `optional = true` to its definition.
```

This occurred because we were trying to define a feature called "core" that included "floxide-core", but "floxide-core" was not marked as an optional dependency. This created a conflict in the Cargo.toml configuration.

## Decision

We have decided to make `floxide-core` a non-optional dependency that is always included in the framework. This means:

1. The `core` feature flag has been removed from the feature definitions
2. The `floxide-core` dependency is no longer marked as optional
3. All other feature modules (transform, event, timer, etc.) remain optional and can be enabled as needed
4. Examples that previously required the `core` feature now have no required features since core is always included

This change simplifies the dependency structure and aligns with the reality that the core functionality is fundamental to the framework and should always be included.

## Consequences

### Positive

- Resolves the issue with `cargo-workspaces` and version bumping
- Simplifies the mental model of the framework - core is always included, other modules are optional
- Reduces complexity in the Cargo.toml configuration
- Makes it clearer to users that core functionality is not optional

### Negative

- Slightly increases the minimum size of the framework since core can no longer be excluded
- Requires updates to documentation that referenced the `core` feature flag

### Neutral

- Examples that only use core functionality no longer need to specify required features
- The re-export of `floxide_core` in lib.rs is no longer conditionally compiled

## Implementation

The implementation involved:

1. Removing the `core` feature from the features section in Cargo.toml
2. Removing the dependency on `core` from other features
3. Moving `floxide-core` to be a standard (non-optional) dependency
4. Updating example configurations to remove the `core` feature requirement
5. Removing the `#[cfg(feature = "core")]` attribute from the core module re-export in lib.rs
6. Updating documentation to reflect that core is always included

## Related ADRs

- [ADR-0033: Implementing Single Package with Features](./0033-implementing-single-package-with-features.md)
- [ADR-0029: Feature-Based Crate Organization](./0029-feature-based-crate-organization.md)
