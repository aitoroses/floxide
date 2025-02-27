# ADR-0035: Combined Version Bump and Release Workflow

## Status

Proposed

## Date

2025-02-27

## Context

Currently, our release process is split across two GitHub Actions workflows:

1. **version-bump.yml**: Handles version bumping, committing changes, and creating tags
2. **release.yml**: Handles publishing to crates.io and creating GitHub releases

This separation requires coordination between the workflows, where version-bump can optionally trigger release. This adds complexity and potential points of failure in the release process.

## Decision

We will combine the version-bump and release workflows into a single unified workflow that handles the entire release process from version bumping to publishing. This new workflow will:

1. Accept the same inputs as the current version-bump workflow
2. Perform version bumping, committing, and tagging
3. Optionally publish to crates.io and create a GitHub release based on user input
4. Provide clear feedback throughout the process

## Implementation Plan

1. Create a new `release.yml` workflow that combines the functionality of both existing workflows
2. Add a new input parameter to control whether to publish after bumping the version
3. Ensure the workflow can be run in dry-run mode for testing
4. Deprecate the old workflows with notices pointing to the new workflow
5. Update documentation to reflect the new release process

## Consequences

### Advantages

1. **Simplified Process**: One workflow to handle the entire release process
2. **Reduced Complexity**: No need for workflow triggering between separate workflows
3. **Better Atomicity**: The entire release process becomes a single atomic operation
4. **Improved Visibility**: Easier to track the entire release process in a single workflow run
5. **Reduced Maintenance**: Only one workflow file to maintain

### Disadvantages

1. **Migration Period**: Users will need to adapt to the new workflow
2. **Potentially Longer Workflow Runs**: The combined workflow will take longer to run than each individual workflow
3. **Less Granular Control**: Users lose the ability to run just the version bump or just the release process separately (though this can be mitigated with appropriate input parameters)

## Alternatives Considered

### Keep Workflows Separate

- **Pros**: More granular control, shorter individual workflow runs
- **Cons**: More complex process, potential for coordination issues

### Use External Release Management Tools

- **Pros**: Could provide more advanced features
- **Cons**: Adds external dependencies, increases complexity for contributors

## Related ADRs

- [ADR-0033: Simplified Publishing with Maintained Subcrate Structure](0033-implementing-single-package-with-features.md)
- [ADR-0034: Script Consolidation for Release Process](0034-script-consolidation-for-release-process.md)
