# Project Rebranding: flowrs → floxide

## Overview

This document summarizes the rebranding process from "flowrs" to "floxide" that was completed on February 26, 2025.

## Rebranding Steps

1. **Name Selection**
   - Selected "floxide" as the new name, combining "flow" (workflow) and "oxide" (a reference to Rust)
   - Verified availability on crates.io

2. **Codebase Updates**
   - Updated all package names in Cargo.toml files
   - Renamed crate directories from `flowrs-*` to `floxide-*`
   - Updated all code references, including imports and error types
   - Updated documentation references

3. **Repository Changes**
   - Renamed repository directory from `flow-rs` to `floxide`
   - Updated Git remote URL to point to the new repository name

4. **Documentation**
   - Created ADR-0032 to document the rebranding decision
   - Updated README.md and other documentation files
   - Updated API documentation file names

5. **Cleanup**
   - Removed old directories and files
   - Updated references in the site directory

## Verification

The rebranding was verified through:
- Successful build with `cargo build`
- Successful test run with `cargo test`
- Grep searches to ensure no remaining references to the old name

## Next Steps

- Update GitHub repository name
- Update crates.io package names when publishing
- Notify users of the name change
- Update external documentation and links

## References

- [ADR-0032: Project Rebranding to Floxide](docs/adrs/0032-project-rebranding-to-floxide.md)
