# ADR 0032: Project Rebranding to Floxide

## Status

Accepted

## Context

The project was initially named "floxide" which served as a descriptive name combining "flow" (for workflow) and "rs" (for Rust). However, as the project evolved, we identified the need for a more distinctive and engaging name that would:

1. Be more memorable and unique in the Rust ecosystem
2. Better reflect the project's identity as a Rust-based workflow framework
3. Be available as a crate name on crates.io
4. Provide a stronger brand identity

## Decision

We have decided to rebrand the project from "floxide" to "floxide". The name "floxide" is a portmanteau of:

- "flow" - representing the core workflow functionality
- "oxide" - a play on Rust (as iron oxide) and a common suffix for Rust projects

This rebranding affects:

1. The main crate name (from `floxide` to `floxide`)
2. All subcrate names (from `floxide-*` to `floxide-*`)
3. Error types (from `FloxideError` to `FloxideError`)
4. Result types (from `FloxideResult` to `FloxideResult`)
5. Repository URLs and documentation references
6. All mentions in documentation and code comments

## Consequences

### Positive

- The new name is more distinctive and memorable
- "floxide" has a scientific feel that subtly references Rust's systems programming nature
- The name is available on crates.io
- The portmanteau is elegant and flows naturally when spoken
- The new name maintains the connection to workflow functionality while adding Rust-specific identity

### Negative

- Existing users will need to update their dependencies
- Documentation and references to the old name will need to be updated
- Some search engine results and external links may become outdated

### Neutral

- The core functionality and API design remain unchanged
- The rebranding is purely cosmetic and does not affect the framework's architecture

## Implementation

The rebranding was implemented through a systematic approach:

1. Renamed all crate directories from `floxide-*` to `floxide-*`
2. Updated all Cargo.toml files to reference the new crate names
3. Used search and replace to update all code references
4. Updated documentation and examples
5. Ensured backward compatibility notes are added to the README

## References

- [Crates.io naming conventions](https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field)
- [Previous ADR on crate organization (ADR-0002)](./0002-project-structure-and-crate-organization.md)
- [Previous ADR on crate publishing (ADR-0014)](./0014-crate-publishing-and-cicd.md)
