# ADR-0037: Fix for Cargo Workspaces Version Command Options

## Status

Proposed

## Date

2025-02-27

## Context

As part of our transition to using cargo-workspaces for managing versions and publishing (as documented in ADR-0033, ADR-0034, and ADR-0035), we encountered an issue with the `release_with_workspaces.sh` script. The script was using incompatible command-line options when calling `cargo workspaces version`.

Specifically, the script was using both `--no-git-commit` and `--allow-branch="*"` options together, which caused the following error:

```
error: The argument '--no-git-commit' cannot be used with '--allow-branch <pattern>'
USAGE:
    cargo workspaces version --no-git-commit <BUMP>
```

According to the cargo-workspaces documentation, these two options are mutually exclusive because:
- `--no-git-commit` tells cargo-workspaces not to commit the version changes to git
- `--allow-branch` specifies which branches to allow versioning from, which only makes sense in the context of git operations

Since our script is handling the git operations manually (adding, committing, and pushing changes), we need to use the `--no-git-commit` option and avoid using `--allow-branch`.

## Decision

We will modify the `release_with_workspaces.sh` script to remove the `--allow-branch="*"` option from the `cargo workspaces version` command, keeping only the `--no-git-commit` option.

This change ensures that:
1. The script can run successfully without errors
2. We maintain control over the git operations in our script
3. We follow the intended usage pattern for cargo-workspaces

## Implementation Plan

1. Update the `release_with_workspaces.sh` script to remove the `--allow-branch="*"` option from the `cargo workspaces version` command
2. Test the script to ensure it works correctly with various options (patch, minor, major, dry-run, skip-publish)
3. Update any documentation or workflows that reference this script to ensure they reflect the correct usage

## Consequences

### Positive

- The release process will work correctly without errors
- The script will handle git operations in a consistent manner
- We maintain a clean separation between cargo-workspaces version bumping and our custom git operations

### Negative

- None identified

## Related ADRs

- [ADR-0033: Implementing Single Package with Features](./0033-implementing-single-package-with-features.md)
- [ADR-0034: Script Consolidation for Release Process](./0034-script-consolidation-for-release-process.md)
- [ADR-0035: Combined Version Bump and Release Workflow](./0035-combined-version-bump-and-release-workflow.md)
- [ADR-0036: Cleanup of Deprecated Scripts and Workflows](./0036-cleanup-deprecated-scripts-and-workflows.md)
