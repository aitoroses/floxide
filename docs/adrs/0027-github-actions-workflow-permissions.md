# ADR-0027: GitHub Actions Workflow Permissions

## Status

Accepted

## Date

2025-02-27

## Context

Our project uses GitHub Actions for various automation tasks, including CI/CD, documentation deployment, and version bumping. Each of these workflows requires specific permissions to perform their tasks. In particular, workflows that need to push changes to the repository (such as version bumping and documentation deployment) require write permissions to the repository contents.

We encountered an issue with our version bump workflow where the GitHub Actions bot was denied permission to push changes to the repository:

```
error: unable to push to remote, out = , err = remote: Permission to aitoroses/floxide.git denied to github-actions[bot].
fatal: unable to access 'https://github.com/aitoroses/floxide/': The requested URL returned error: 403
```

This indicates that the workflow does not have the necessary permissions to push changes to the repository.

## Decision

We will explicitly set the required permissions for each GitHub Actions workflow to ensure they have the necessary access to perform their tasks. Specifically:

1. For workflows that need to push changes to the repository (version-bump.yml, docs.yml), we will set:
   ```yaml
   permissions:
     contents: write  # Allows pushing to the repository
   ```

2. For workflows that only need to read from the repository (ci.yml), we will set:
   ```yaml
   permissions:
     contents: read
   ```

3. For the release workflow that needs to publish to crates.io, we will continue to use the CRATES_IO_TOKEN secret.

We will update all existing workflows to include these explicit permission declarations to prevent permission-related issues in the future.

## Consequences

### Positive

1. **Clear Permission Model**: Explicitly declaring permissions makes it clear what access each workflow requires.
2. **Reduced Errors**: Proper permission settings will prevent permission-denied errors during workflow execution.
3. **Security Best Practice**: Following the principle of least privilege by only granting the permissions each workflow needs.
4. **Consistency**: All workflows will follow the same pattern for permission declaration.

### Negative

1. **Maintenance Overhead**: Additional configuration to maintain in each workflow file.
2. **Potential for Over-permissioning**: If not carefully managed, workflows might be granted more permissions than necessary.

## Implementation Plan

1. Update the version-bump.yml workflow to include the `contents: write` permission.
2. Review and update all other workflows to include appropriate permission declarations.
3. Test the workflows to ensure they function correctly with the new permission settings.

## Alternatives Considered

### Using Personal Access Tokens (PATs)

- **Pros**:
  - More flexible permissions
  - Can work across repositories
- **Cons**:
  - Security risk if tokens are compromised
  - Additional secret management
  - More complex setup

We chose to use the built-in GITHUB_TOKEN with explicit permissions as it provides sufficient access for our needs while being more secure and easier to manage.

### Repository-Level Permission Settings

- **Pros**:
  - Centralized permission management
  - Applies to all workflows
- **Cons**:
  - Less granular control
  - May grant more permissions than necessary to some workflows

We chose workflow-specific permissions for more granular control and to follow the principle of least privilege. 