# ADR-0037: Fix for Cargo Workspaces Version Command Options

## Status

Proposed

## Date

2025-02-27

## Context

As part of our transition to using cargo-workspaces for managing versions and publishing (as documented in ADR-0033, ADR-0034, and ADR-0035), we encountered several issues with the `release_with_workspaces.sh` script and its integration with GitHub Actions workflows:

1. The script was using incompatible command-line options when calling `cargo workspaces version`. Specifically, it was using both `--no-git-commit` and `--allow-branch="*"` options together, which caused the following error:

```
error: The argument '--no-git-commit' cannot be used with '--allow-branch <pattern>'
USAGE:
    cargo workspaces version --no-git-commit <BUMP>
```

2. The script was not using the `--yes` flag with `cargo workspaces` commands, causing the CI process to hang waiting for user input during the version bump process.

3. The script was always creating a git commit, which might not be desired in CI environments where git operations are handled separately.

## Decision

We will make the following changes to the `release_with_workspaces.sh` script:

1. Remove the `--allow-branch="*"` option from the `cargo workspaces version` command, keeping only the `--no-git-commit` option.

2. Add the `--yes` flag to all `cargo workspaces` commands to skip confirmation prompts, ensuring the script can run non-interactively in CI environments.

3. Add a new `--no-git-commit` option to the script to allow skipping the git commit step, making it more flexible for different environments.

4. Update the GitHub Actions workflow to use the new `--no-git-commit` option when calling the script.

These changes ensure that:
1. The script can run successfully without errors
2. The script can run non-interactively in CI environments
3. We have more flexibility in how git operations are handled
4. We follow the intended usage pattern for cargo-workspaces

## Implementation Plan

1. Update the `release_with_workspaces.sh` script to:
   - Remove the `--allow-branch="*"` option from the `cargo workspaces version` command
   - Add the `--yes` flag to all `cargo workspaces` commands
   - Add a new `--no-git-commit` option to the script
   - Make the git commit step conditional based on the new option

2. Update the GitHub Actions workflow to use the new `--no-git-commit` option when calling the script

3. Test the script to ensure it works correctly with various options (patch, minor, major, dry-run, skip-publish, no-git-commit)

## Consequences

### Positive

- The release process will work correctly without errors
- The script will run non-interactively in CI environments
- We have more flexibility in how git operations are handled
- The script will handle git operations in a consistent manner
- We maintain a clean separation between cargo-workspaces version bumping and our custom git operations

### Negative

- None identified

## Related ADRs

- [ADR-0033: Implementing Single Package with Features](./0033-implementing-single-package-with-features.md)
- [ADR-0034: Script Consolidation for Release Process](./0034-script-consolidation-for-release-process.md)
- [ADR-0035: Combined Version Bump and Release Workflow](./0035-combined-version-bump-and-release-workflow.md)
- [ADR-0036: Cleanup of Deprecated Scripts and Workflows](./0036-cleanup-deprecated-scripts-and-workflows.md)
