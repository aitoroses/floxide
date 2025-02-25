# ADR-0023: Examples Directory Restructuring

## Status

Proposed

## Date

2025-02-25

## Context

Currently, the examples in the flowrs framework are organized as a separate crate in the workspace defined in the `examples/` directory with its own `Cargo.toml`. This approach has several drawbacks:

1. It adds unnecessary complexity to the project structure
2. It requires maintaining a separate crate with its own dependencies
3. It doesn't align with the standard Rust convention for examples
4. The examples are still being defined in the root `Cargo.toml` file using the `[[example]]` format, creating redundancy

The standard Rust convention is to have a simple `examples/` directory with standalone `.rs` files that are referenced in the root `Cargo.toml` file, without a separate crate structure.

## Decision

We will restructure the examples directory to follow the standard Rust convention:

1. Remove the separate crate structure (delete `examples/Cargo.toml` and `examples/src/`)
2. Keep the `.rs` files in the `examples/` directory
3. Keep the examples configuration in the root `Cargo.toml` file
4. Ensure all examples can still be run with `cargo run --example example_name`

## Consequences

### Advantages

1. **Simplifies Project Structure**: Removes unnecessary complexity from the project
2. **Follows Rust Conventions**: Better aligns with standard Rust practices for organizing examples
3. **Reduces Maintenance Overhead**: No need to maintain a separate crate with its own dependencies
4. **Clearer Organization**: Makes the purpose of the examples directory more obvious to new contributors

### Disadvantages

1. **Migration Effort**: Requires removing the crate structure while ensuring examples still work
2. **Potential Build Changes**: May require adjusting how examples are built and run

## Implementation Details

The implementation will involve:

1. Removing `examples/Cargo.toml`
2. Removing `examples/src/` directory
3. Keeping all existing `.rs` example files in the `examples/` directory
4. Ensuring all examples use dependencies from the root `Cargo.toml`
5. Testing that all examples can be run using `cargo run --example example_name`

## Related ADRs

- [ADR-0002: Project Structure and Crate Organization](0002-project-structure-and-crate-organization.md)
- [ADR-0019: Examples Structure Standardization](0019-examples-structure-standardization.md)
