# ADR-0026: Documentation Deployment Strategy

## Status

Accepted

## Date

2024-02-25

## Context

As the Flowrs framework matures, we need a reliable and automated way to deploy our documentation to GitHub Pages. This includes both API documentation generated from Rustdoc and conceptual documentation created with MkDocs. We need to determine the best approach for deploying this documentation, considering:

1. Automation through GitHub Actions
2. Permissions and security considerations
3. Consistency and reliability of the deployment process
4. Support for both Rustdoc and MkDocs documentation

## Decision

We will implement a documentation deployment strategy using GitHub Actions with the following components:

### Documentation Types

We will maintain two types of documentation:

1. **API Documentation**: Generated from Rustdoc comments in the codebase
2. **Conceptual Documentation**: Written in Markdown and built with MkDocs

### Deployment Approach

We will use GitHub Actions to automate the deployment process:

1. **Direct GitHub Pages Deployment**: We will use the `peaceiris/actions-gh-pages@v3` action to directly deploy to the gh-pages branch, which is more reliable and has fewer permission issues than the newer GitHub Pages deployment approach.

2. **Permissions Configuration**: We will set the following permissions for the GitHub Actions workflow:
   ```yaml
   permissions:
     contents: write  # Allows pushing to the gh-pages branch
   ```

3. **Force Orphan**: We will use the `force_orphan: true` option to ensure a clean gh-pages branch with only the latest documentation.

4. **GitHub Pages Source**: We will configure GitHub Pages to use the gh-pages branch as the source for deployment.

### Workflow Triggers

The documentation deployment workflow will be triggered on:

1. Pushes to the main branch
2. Manual triggers through the workflow_dispatch event

### Concurrency Control

We will implement concurrency control to ensure only one deployment runs at a time:

```yaml
concurrency:
  group: "pages"
  cancel-in-progress: true
```

## Consequences

### Positive

1. **Automated Deployment**: Documentation is automatically updated when changes are pushed to the main branch.
2. **Consistent Process**: The deployment process is consistent and reliable.
3. **Proper Permissions**: The workflow uses the minimum necessary permissions for deployment.
4. **Modern Approach**: We use the latest GitHub Actions features for Pages deployment.
5. **Manual Option**: Documentation can be manually deployed when needed.

### Negative

1. **Setup Complexity**: Initial setup requires configuring GitHub Pages in repository settings.
2. **Maintenance Overhead**: The workflow may need updates as GitHub Actions evolves.
3. **Potential Conflicts**: Multiple documentation types may require careful coordination.

## Alternatives Considered

### GitHub Actions Pages Deployment API

- **Pros**:
  - Modern approach with GitHub's latest features
  - Potentially more secure
  - Cleaner separation of concerns
- **Cons**:
  - Requires specific permissions that may be restricted in some organizations
  - More complex setup with environment configurations
  - Less reliable in some repository configurations

We initially tried this approach but encountered permission issues with the GitHub Pages API. The error "Resource not accessible by integration" indicated that our GitHub Actions workflow didn't have sufficient permissions to create or configure a GitHub Pages site, despite having the correct permission settings in the workflow file.

### External Documentation Hosting

- **Pros**:
  - Independence from GitHub infrastructure
  - Potentially more customization options
- **Cons**:
  - Additional service to maintain
  - Separate authentication and deployment process
  - Disconnected from the repository

We chose GitHub Pages for its tight integration with our GitHub repository and sufficient feature set for our documentation needs. 