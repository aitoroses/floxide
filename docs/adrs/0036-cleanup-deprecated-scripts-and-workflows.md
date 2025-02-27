# ADR-0036: Cleanup of Deprecated Scripts and Workflows

## Status

Proposed

## Date

2025-02-27

## Context

As part of our ongoing efforts to streamline the release process, we have:

1. Created a consolidated script (`update_versions.sh`) that combines the functionality of `update_dependency_versions.sh` and `update_subcrate_versions.sh` (ADR-0034)
2. Enhanced `release_with_workspaces.sh` to handle both version bumping and publishing
3. Created a combined workflow (`combined-release.yml`) that handles the entire release process (ADR-0035)

These improvements have left several scripts and workflows deprecated:

**Scripts:**
- `update_dependency_versions.sh`
- `update_subcrate_versions.sh`
- `test_version_bump.sh`
- `publish.sh`
- `publish_local.sh`

**Workflows:**
- `version-bump.yml`
- `release.yml`

While we've added deprecation notices to these files, they still exist in the codebase and could cause confusion for contributors.

## Decision

We will remove all deprecated scripts and workflows from the codebase to reduce clutter and prevent confusion. This will ensure that contributors only see and use the current, recommended tools for the release process.

## Implementation Plan

1. Remove the following deprecated scripts:
   - `update_dependency_versions.sh`
   - `update_subcrate_versions.sh`
   - `test_version_bump.sh`
   - `publish.sh`
   - `publish_local.sh`

2. Remove the following deprecated workflows:
   - `version-bump.yml`
   - `release.yml`

3. Update the README.md to reflect these changes and provide clear guidance on the current release process.

4. Update any references to these scripts or workflows in other documentation or code.

5. Fix compatibility issues with cargo-workspaces:
   - Remove the conflicting `--allow-branch` parameter when used with `--no-git-commit` in the publish command.

## Consequences

### Advantages

1. **Cleaner Codebase**: Removing deprecated files reduces clutter and makes the codebase easier to navigate.
2. **Clearer Guidance**: Contributors will only see the current, recommended tools for the release process.
3. **Reduced Maintenance Burden**: Fewer files to maintain and update.
4. **Prevents Accidental Use**: Eliminates the possibility of accidentally using deprecated scripts or workflows.

### Disadvantages

1. **Breaking Change**: Contributors who were using the deprecated scripts or workflows will need to adapt to the new ones.
2. **Historical Context**: Some historical context about how the release process evolved may be lost.

## Alternatives Considered

### Keep Deprecated Files with Notices

- **Pros**: Preserves historical context and provides a transition period for contributors.
- **Cons**: Continues to clutter the codebase and may cause confusion.

### Move Deprecated Files to an Archive Directory

- **Pros**: Preserves historical context while reducing clutter in the main directories.
- **Cons**: Still maintains files that are no longer used, and the archive directory itself could become cluttered over time.

## Related ADRs

- [ADR-0033: Simplified Publishing with Maintained Subcrate Structure](0033-implementing-single-package-with-features.md)
- [ADR-0034: Script Consolidation for Release Process](0034-script-consolidation-for-release-process.md)
- [ADR-0035: Combined Version Bump and Release Workflow](0035-combined-version-bump-and-release-workflow.md)
