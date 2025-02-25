# Contributing to Flowrs

Thank you for your interest in contributing to the Flowrs framework! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

Please be respectful and considerate of others when contributing to the project. We aim to foster an inclusive and welcoming community.

## Getting Started

1. **Fork the Repository**: Start by forking the [Flowrs repository](https://github.com/aitoroses/flowrs) on GitHub.

2. **Clone Your Fork**: Clone your fork to your local machine:

   ```bash
   git clone https://github.com/your-username/flowrs.git
   cd flowrs
   ```

3. **Set Up Development Environment**: Make sure you have Rust and Cargo installed. We recommend using [rustup](https://rustup.rs/) for managing your Rust installation.

4. **Create a Branch**: Create a branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Workflow

### Architectural Decision Records (ADRs)

Before implementing any significant architectural changes, you must create or update an Architectural Decision Record (ADR). See the [ADR Process](architecture/adr-process.md) for details.

### Code Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `cargo fmt` to format your code.
- Use `cargo clippy` to check for common mistakes and improve your code.

### Testing

- Write tests for all new functionality.
- Ensure all tests pass before submitting a pull request.
- Run tests with `cargo test`.

### Documentation

- Document all public APIs with doc comments.
- Update relevant documentation when making changes.
- Build and check the documentation with `cargo doc`.

## Submitting Changes

1. **Commit Your Changes**: Make small, focused commits with clear commit messages:

   ```bash
   git commit -m "Add feature X"
   ```

2. **Push to Your Fork**:

   ```bash
   git push origin feature/your-feature-name
   ```

3. **Create a Pull Request**: Go to the [Flowrs repository](https://github.com/aitoroses/flowrs) and create a pull request from your branch.

4. **Code Review**: Wait for code review and address any feedback.

## Pull Request Guidelines

- Provide a clear description of the changes.
- Link to any relevant issues.
- Ensure all tests pass.
- Make sure your code follows the project's style guidelines.
- Include documentation updates if necessary.

## Reporting Issues

If you find a bug or have a feature request, please create an issue on the [GitHub issue tracker](https://github.com/aitoroses/flowrs/issues).

When reporting a bug, please include:

- A clear description of the issue
- Steps to reproduce
- Expected behavior
- Actual behavior
- Any relevant logs or error messages

## Community

Join our community to discuss the project, ask questions, and get help:

- GitHub Discussions
- Discord (coming soon)

## License

By contributing to Flowrs, you agree that your contributions will be licensed under the project's license.
