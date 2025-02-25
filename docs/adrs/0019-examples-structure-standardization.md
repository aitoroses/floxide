# ADR-0019: Examples Structure Standardization

## Status

Implemented

## Date

2024-02-25

## Context

Previously, the examples in the flowrs framework were organized as binary applications in the `examples/src/bin/` directory. This structure required users to run examples using the `cargo run --bin example_name` command, which diverges from the standard Rust convention for example code.

Additionally, example filenames had a redundant `_example` suffix (e.g., `transform_node_example.rs`), which doesn't align with conventional Rust naming practices for examples.

## Decision

We will reorganize our examples to follow the standard Rust convention for examples:

1. Move examples from `examples/src/bin/` to `examples/examples/`
2. Remove the redundant `_example` suffix from the filenames
3. Update the `examples/Cargo.toml` to properly define examples using the `[[example]]` format
4. Enable examples to be run using the standard `cargo run --example example_name` command

## Consequences

### Advantages

1. **Follows Rust Conventions**: Aligns with the standard Rust practice for organizing and running examples
2. **Improved Discoverability**: Makes examples easier to discover and run for new users
3. **Cleaner Naming**: Removes redundancy in filename suffixes
4. **Better Documentation**: Examples serve more clearly as documentation rather than just binaries

### Disadvantages

1. **Migration Effort**: Requires updating all examples and references to them
2. **Potential Backward Compatibility**: Users familiar with the old structure will need to adjust

## Implementation Details

The implementation involves:

1. Creating an `examples/examples/` directory
2. Moving example files from `examples/src/bin/` to `examples/examples/` with renamed files:
   - `transform_node_example.rs` → `transform_node.rs`
   - `lifecycle_node_example.rs` → `lifecycle_node.rs`
   - `order_processing.rs` remains as is (no suffix to remove)
3. Updating `examples/Cargo.toml` to define examples with their paths:

   ```toml
   [[example]]
   name = "transform_node"
   path = "examples/transform_node.rs"

   [[example]]
   name = "lifecycle_node"
   path = "examples/lifecycle_node.rs"

   [[example]]
   name = "order_processing"
   path = "examples/order_processing.rs"
   ```

4. Removing the unused `examples/src/bin/` directory

## Related ADRs

- [ADR-0002: Project Structure and Crate Organization](0002-project-structure-and-crate-organization.md)
