# ADR 0039: Crates.io Publishing Strategy

## Status

Accepted

## Date

2025-02-27

## Context

After successfully publishing the Floxide framework to crates.io, we need to document our publishing strategy and ensure it is properly integrated into our CI/CD pipeline. This ADR builds upon and consolidates the learnings from our initial publishing experience and supersedes parts of previous ADRs related to publishing.

The key challenges we faced during the initial publishing process were:

1. **Dependency Order**: Ensuring crates are published in the correct dependency order
2. **Version Consistency**: Maintaining consistent version numbers across all crates
3. **Internal Dependency References**: Ensuring internal dependencies reference the correct versions
4. **Automation**: Streamlining the process to avoid manual steps

We discovered that using `cargo-workspaces` with the `--exact` flag provides an effective solution for managing these challenges.

## Decision

We will adopt a standardized publishing strategy using `cargo-workspaces` as our primary tool for version management and publishing. This approach will be integrated into our CI/CD pipeline.

### 1. Tool Selection

We will use:
- **cargo-workspaces**: For version bumping and publishing in the correct dependency order
- **GitHub Actions**: For automating the release process

### 2. Configuration Files

We will maintain two key configuration files:

#### cargo-workspaces.toml

```toml
[workspace]
# Whether to inherit the version from the workspace
inherit_version = true

# Whether to link all dependencies to their workspace versions
link_workspace_deps = true

# Whether to update the versions of workspace dependencies when versioning
update_workspace_deps = true

# Custom publish order
publish_order = [
  "floxide-core",
  "floxide-transform",
  "floxide-event",
  "floxide-timer",
  "floxide-longrunning",
  "floxide-reactive",
  "floxide"
]
```

#### release.toml (for cargo-release, as a backup option)

```toml
# Don't push changes automatically
push = false

# Don't publish to crates.io automatically
publish = false

# Don't create a tag automatically
tag = false

# Don't create a GitHub release automatically
release = false

# Update dependencies with exact version
dependent-version = "fix"

# Use conventional commits for changelog generation
pre-release-commit-message = "chore(release): {{version}}"
```

### 3. Version Management Process

The standard process for bumping versions will be:

```bash
# Bump versions with exact dependency references
cargo workspaces version [patch|minor|major] --exact
```

This command will:
1. Bump all crate versions according to semver
2. Update all internal dependency references with exact version pins (e.g., `=1.0.11`)
3. Create a commit with the version changes
4. Create git tags for the release

### 4. Publishing Process

The standard process for publishing will be:

```bash
# Publish all crates in the correct dependency order
cargo workspaces publish --from-git
```

This command will:
1. Publish crates in the order specified in `cargo-workspaces.toml`
2. Use the versions from the git tags

### 5. CI/CD Integration

We will update our GitHub Actions workflow to use this approach:

```yaml
# Version bump step
- name: Bump version
  run: |
    cargo workspaces version ${{ inputs.release_type }} --exact --yes

# Publishing step
- name: Publish to crates.io
  run: |
    cargo workspaces publish --from-git --yes
```

## Consequences

### Positive

1. **Automated Dependency Management**: The `--exact` flag ensures all internal dependencies use exact version pins
2. **Correct Publishing Order**: The `publish_order` in `cargo-workspaces.toml` ensures crates are published in the right order
3. **Simplified Process**: The entire release process is reduced to two commands
4. **Consistency**: All crates maintain the same version number

### Negative

1. **Tool Dependency**: We now rely on `cargo-workspaces` for our release process
2. **Exact Version Pins**: Using exact version pins (`=1.0.11`) may be more restrictive than semver ranges

### Neutral

1. **Version Synchronization**: All crates share the same version number, which may not reflect the actual changes in each crate

## Superseded ADRs

This ADR supersedes or modifies parts of:

- **ADR-0014**: Updates the publishing strategy section
- **ADR-0030**: Replaces the manual scripts with `cargo-workspaces`
- **ADR-0035**: Updates the release workflow to use `cargo-workspaces`

## References

- [cargo-workspaces documentation](https://github.com/pksunkara/cargo-workspaces)
- Initial publishing experience (February 2025)
