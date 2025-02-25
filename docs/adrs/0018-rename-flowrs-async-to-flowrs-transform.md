# ADR-0018: Rename flowrs-async Crate to flowrs-transform

## Status

Implemented

## Date

2024-02-25

## Context

In ADR-0016, we decided to rename the `AsyncNode` trait to `TransformNode` to better reflect its purpose, as the trait is primarily focused on data transformation rather than providing unique asynchronous capabilities beyond what's already available in other node types.

While we've successfully renamed the trait and related types within the codebase, the containing crate is still named `flowrs-async`, which creates a terminology inconsistency with the renamed components it contains.

Package and module names should accurately reflect their primary purpose and the abstractions they provide. The current mismatch between the crate name (flowrs-async) and its primary component (TransformNode) can lead to confusion for users of the framework.

## Decision

We will rename the `flowrs-async` crate to `flowrs-transform` to maintain consistency with the renamed `TransformNode` trait and related types. This change will involve:

1. Creating a new crate named `flowrs-transform` with the same content as `flowrs-async`
2. Updating all imports in the codebase from `flowrs_async` to `flowrs_transform`
3. Updating the Cargo.toml files to reflect these changes
4. Removing the `flowrs-async` crate entirely to avoid confusion

## Consequences

### Advantages

1. **Consistent Terminology**: The crate name will now match the primary abstraction it provides
2. **Better Clarity**: Developers will have a clearer understanding of the crate's purpose
3. **Forward Compatible**: The rename aligns with our ongoing effort to clarify the framework's abstractions

### Disadvantages

1. **Migration Effort**: Existing code will need to update imports
2. **Documentation Updates**: Documentation references will need to be updated

### Migration Path

1. Create the new `flowrs-transform` crate with identical functionality
2. Update all internal framework code to use the new imports
3. Document the change clearly in the changelog
4. Remove the old crate entirely to avoid confusion and maintenance overhead

## Alternatives Considered

### 1. Keep the Existing Crate Name

We could maintain the existing `flowrs-async` crate name despite the internal renaming of types. This would require less immediate change but would perpetuate the inconsistency between the crate name and its primary abstractions.

### 2. Create a New Crate but Maintain Both

We could create the new `flowrs-transform` crate while keeping `flowrs-async` indefinitely as a thin wrapper that re-exports everything from `flowrs-transform`. This would avoid breaking changes but would add maintenance overhead and potential confusion.

## Related ADRs

- [ADR-0016: TransformNode Renaming and Async Extension Patterns](0016-transform-node-and-async-extensions.md)
- [ADR-0002: Project Structure and Crate Organization](0002-project-structure-and-crate-organization.md)
