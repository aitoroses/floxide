# ADR-0034: Script Consolidation for Release Process

## Status

Proposed

## Date

2025-02-27

## Context

As part of our transition to using cargo-workspaces for managing versions and publishing (as documented in ADR-0033), we need to evaluate our existing scripts and determine which ones are still necessary and which can be consolidated or removed.

Currently, we have several scripts related to the release process:

1. `publish.sh` - Original script for publishing crates in order
2. `publish_local.sh` - Script for testing publishing locally
3. `release_with_workspaces.sh` - New script using cargo-workspaces for releases
4. `test_version_bump.sh` - Script for testing version bumps
5. `update_dependency_versions.sh` - Script for updating dependency versions in the root crate
6. `update_subcrate_versions.sh` - Script for updating subcrate versions to use workspace inheritance
7. `run_ci_locally.sh` - Script for running CI checks locally
8. `serve-docs.sh` - Script for serving documentation locally

These scripts serve different purposes in our development and release workflows, but there may be redundancy or opportunities for consolidation.

## Decision

After evaluating the scripts and their usage in our workflows, we will:

1. **Keep and maintain the following scripts**:
   - `release_with_workspaces.sh` - Our primary script for version management and publishing
   - `run_ci_locally.sh` - Useful for developers to run CI checks locally
   - `serve-docs.sh` - Useful for previewing documentation locally

2. **Consolidate the following scripts**:
   - `update_dependency_versions.sh` and `update_subcrate_versions.sh` - These can be combined into a single `update_versions.sh` script that handles both tasks
   - `test_version_bump.sh` - This functionality can be incorporated into `release_with_workspaces.sh` with the `--dry-run` flag

3. **Deprecate the following scripts**:
   - `publish.sh` - Replaced by `release_with_workspaces.sh`
   - `publish_local.sh` - Replaced by `release_with_workspaces.sh` with the `--dry-run` flag

## Implementation Plan

1. Create a new `update_versions.sh` script that combines the functionality of `update_dependency_versions.sh` and `update_subcrate_versions.sh`
2. Enhance `release_with_workspaces.sh` to include the functionality of `test_version_bump.sh`
3. Update the GitHub Actions workflows to use the consolidated scripts
4. Add deprecation notices to the scripts that will be removed
5. Document the changes in the README.md file

## Consequences

### Advantages

1. **Simplified Maintenance**: Fewer scripts to maintain and update
2. **Clearer Developer Experience**: Developers have fewer scripts to learn and remember
3. **Consistent Versioning**: Using cargo-workspaces ensures consistent versioning across all crates
4. **Streamlined Release Process**: The release process is more straightforward and less error-prone

### Disadvantages

1. **Migration Period**: There will be a period where developers need to adapt to the new scripts
2. **Potential for Confusion**: Until the deprecated scripts are removed, there may be confusion about which scripts to use

## Alternatives Considered

### Keep All Scripts Separate

- **Pros**: Each script has a clear, focused purpose
- **Cons**: More scripts to maintain, potential for divergence in functionality

### Automate Everything in CI/CD

- **Pros**: Less reliance on scripts, more automation
- **Cons**: Less flexibility for developers, harder to debug issues locally

## Related ADRs

- [ADR-0033: Simplified Publishing with Maintained Subcrate Structure](0033-implementing-single-package-with-features.md)
