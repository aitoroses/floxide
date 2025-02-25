# ADR-0014: Crate Publishing and CI/CD Setup

## Status

Accepted

## Date

2024-02-25

## Context

As the Flowrs framework matures, we need to establish a professional publishing workflow to make the crates available on crates.io. This includes setting up automated CI/CD pipelines through GitHub Actions to ensure quality, run tests, and automate the release process.

We need to determine:

1. How to structure our GitHub repository
2. What CI/CD workflows to implement
3. How to manage versioning across the workspace
4. What quality checks to enforce
5. How to automate the release process

## Decision

We will implement a comprehensive CI/CD pipeline using GitHub Actions with the following components:

### Repository Structure

We will create a public GitHub repository with the following structure:

- Main branch protected and requiring PR reviews
- Development through feature branches and PRs
- Releases tagged with semantic versioning

### CI/CD Workflows

We will implement the following GitHub Actions workflows:

1. **CI Pipeline**:

   - Triggered on pull requests and pushes to main
   - Runs tests, linting, and formatting checks
   - Builds the project on multiple platforms (Linux, macOS, Windows)
   - Generates and publishes test coverage reports

2. **Release Pipeline**:

   - Triggered on release tags (e.g., v0.1.0)
   - Builds the project
   - Publishes crates to crates.io
   - Generates release notes

3. **Documentation Pipeline**:
   - Builds and publishes documentation to GitHub Pages
   - Updates on releases and optionally on main branch changes

### Versioning Strategy

We will use:

- Semantic versioning (MAJOR.MINOR.PATCH)
- Workspace-level version management through the workspace manifest
- Automated version bumping through conventional commits

### Quality Checks

The CI pipeline will enforce:

- All tests passing
- Code formatting with `rustfmt`
- Linting with `clippy`
- Documentation coverage
- No unsafe code without explicit approval
- Dependency auditing with `cargo audit`

### Release Automation

The release process will be automated:

1. Create a release PR that bumps versions
2. Upon approval and merge, tag the release
3. GitHub Actions will publish to crates.io
4. Release notes will be generated from commit history

### Crates.io Publishing

For publishing to crates.io, we will:

- Reserve the crate names early
- Ensure all metadata is complete and professional
- Include appropriate keywords and categories
- Provide comprehensive documentation
- Use a dedicated API token stored as a GitHub secret

## Consequences

### Positive

1. **Professional Appearance**: A well-maintained CI/CD pipeline demonstrates project quality
2. **Quality Assurance**: Automated checks ensure consistent code quality
3. **Reduced Manual Work**: Automation reduces release overhead
4. **Consistency**: Enforced standards across the codebase
5. **Reliability**: Predictable and repeatable release process

### Negative

1. **Maintenance Overhead**: CI/CD pipelines require maintenance
2. **Complexity**: More moving parts in the development process
3. **Learning Curve**: Contributors need to understand the workflow
4. **Potential Delays**: CI checks may slow down the development process

## Alternatives Considered

### Manual Publishing

- **Pros**:
  - Simpler setup initially
  - More direct control over the process
- **Cons**:
  - Error-prone
  - Time-consuming
  - Less consistent
  - Requires developer access to crates.io tokens

We rejected this approach as it doesn't scale well and introduces unnecessary manual steps and potential for errors.

### Third-Party CI Services

- **Pros**:
  - Potentially more features
  - Separation from GitHub
- **Cons**:
  - Additional integration points
  - More accounts/services to manage
  - Potential costs

We chose GitHub Actions for its tight integration with our repository and sufficient feature set for our needs.

### Single Crate Publishing vs. Workspace

- **Pros** of individual crate publishing:
  - More granular version control
  - Independent release cycles
- **Cons**:
  - More complex coordination
  - Potential version mismatches
  - Multiple publishing steps

We chose to coordinate versions at the workspace level for simplicity and consistency, while still allowing for independent versioning when necessary.
