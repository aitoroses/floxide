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

## Questions?

If you have any questions or need help with the contribution process, feel free to [open an issue](https://github.com/aitoroses/floxide/issues/new) with your question.
