# Contributing to Floxide

Thank you for considering contributing to Floxide! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How Can I Contribute?

### Reporting Bugs

- **Ensure the bug was not already reported** by searching on GitHub under [Issues](https://github.com/aitoroses/floxide/issues).
- If you're unable to find an open issue addressing the problem, [open a new one](https://github.com/aitoroses/floxide/issues/new). Be sure to include a **title and clear description**, as much relevant information as possible, and a **code sample** or an **executable test case** demonstrating the expected behavior that is not occurring.

### Suggesting Enhancements

- **Check if the enhancement has already been suggested** by searching on GitHub under [Issues](https://github.com/aitoroses/floxide/issues).
- If it hasn't, [create a new issue](https://github.com/aitoroses/floxide/issues/new) with a clear title and description of the suggested enhancement.

### Pull Requests

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Development Process

### Setting Up the Development Environment

1. Clone your fork of the repository
2. Install Rust (if not already installed) using [rustup](https://rustup.rs/)
3. Navigate to the project directory and run `cargo build` to build the project

### Coding Standards

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format your code
- Ensure your code passes `cargo clippy` without warnings
- Write tests for new features or bug fixes

### Commit Messages

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

### Documentation

- Update the README.md with details of changes to the interface, if applicable
- Update the documentation in the code using rustdoc comments
- For significant architectural decisions, create or update an ADR in the `docs/adrs` directory

## Architectural Decision Records (ADRs)

For significant architectural changes, we use ADRs to document the decision-making process. If your contribution involves an architectural decision:

1. Create a new ADR in the `docs/adrs` directory following the template
2. Reference the ADR in your pull request
3. Ensure the ADR is reviewed along with your code changes

## Testing

- Write tests for all new features and bug fixes
- Run the existing test suite with `cargo test` to ensure your changes don't break existing functionality
- For performance-critical code, consider adding benchmarks

## Releasing

The release process is automated through GitHub Actions. There are two main approaches:

### Automated Version Bump and Release Workflow

We provide a GitHub Actions workflow for automated version bumping and releasing:

1. Go to the "Actions" tab in the GitHub repository
2. Select the "Version Bump" workflow
3. Click "Run workflow"
4. Choose the version bump type (patch, minor, or major)
5. Configure additional options:
   - **Dry run**: Preview changes without committing (default: true)
   - **Force tag**: Force update existing tags if they exist (default: false)
   - **Skip tagging**: Skip tag creation entirely (default: false)
   - **Trigger release**: Ensure the release workflow is triggered after version bump (default: false)
6. Click "Run workflow"

This workflow will:
- Update version numbers across all crates in the workspace (all crates will have the same version)
- Commit the changes (if not a dry run)
- Create and push Git tags (if not skipped)
- Trigger the release workflow (if trigger_release is true)

#### Complete Release Process

For a complete release to crates.io, follow these steps:

1. Run the Version Bump workflow with:
   - Appropriate bump type (patch/minor/major)
   - Dry run: false
   - Force tag: true (if the tag already exists)
   - Trigger release: true

2. The workflow will:
   - Update all version numbers consistently across all crates
   - Commit and push changes
   - Create/update the version tag
   - Trigger the release workflow

3. The release workflow will then:
   - Verify the tag matches the Cargo.toml version
   - Run tests
   - Publish all crates to crates.io in dependency order
   - Create a GitHub release with release notes

### Manual Release Process

For manual releases:

1. Update the version numbers in Cargo.toml files (ensure all crates have the same version)
2. Create a pull request with these changes
3. Once merged, create a new tag with the version number (e.g., `v0.1.0`)
4. Push the tag to trigger the release workflow

## Questions?

If you have any questions or need help with the contribution process, feel free to [open an issue](https://github.com/aitoroses/floxide/issues/new) with your question.
