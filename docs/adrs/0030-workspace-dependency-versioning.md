# ADR 0030: Workspace Dependency Versioning for Publishing

## Status

Accepted

## Context

When publishing Rust crates to crates.io, all dependencies must have explicit version specifications, even if they are path dependencies within a workspace. This is a requirement of the crates.io publishing process.

In our workspace structure, we have a root `flowrs` crate that depends on several subcrates (`flowrs-core`, `flowrs-transform`, etc.). These dependencies were initially specified only with `path` attributes, which works fine for local development but causes errors during the publishing process.

The error encountered was:
```
error: all dependencies must have a version specified when publishing.
dependency `flowrs-core` does not specify a version
Note: The published dependency will use the version from crates.io,
the `path` specification will be removed from the dependency declaration.
```

Additionally, we need to ensure that when the workspace version changes, we don't have to manually update multiple version specifications throughout the codebase, which would be error-prone and create maintenance overhead.

We also encountered an issue during version bumping where some subcrates had hardcoded versions instead of using workspace inheritance, causing errors like:
```
error: failed to select a version for the requirement `flowrs-event = "^1.0.2"`
candidate versions found which didn't match: 1.0.0
location searched: /home/runner/work/flowrs/flowrs/crates/flowrs-event
required by package `flowrs v1.0.3 (/home/runner/work/flowrs/flowrs)`
```

## Decision

We will implement a two-part solution to address these versioning issues:

1. **For the root crate's dependencies**: Maintain explicit version specifications for all workspace dependencies in the root `Cargo.toml` file, in addition to the path specifications. These versions will match the workspace version defined in `[workspace.package]`.

2. **For subcrates**: Ensure all subcrates use workspace inheritance for their versions by using `version.workspace = true` instead of hardcoded versions.

The dependency specifications in the root crate will follow this pattern:
```toml
flowrs-core = { path = "./crates/flowrs-core", version = "1.0.2", optional = true }
```

To reduce maintenance burden, we will create two scripts:
- `scripts/update_dependency_versions.sh`: Automatically updates the version specifications in the root Cargo.toml
- `scripts/update_subcrate_versions.sh`: Ensures all subcrates use workspace inheritance for their versions

Additionally, we will maintain a specific publishing order in our release workflow, where subcrates are published first, followed by the main crate. This ensures that all dependencies are available on crates.io when the main crate is published.

## Consequences

### Positive

- Enables smooth publishing to crates.io
- Maintains correct version relationships between published crates
- Preserves local development workflow using path dependencies
- Ensures that users installing the crate from crates.io get the correct dependency versions
- The update scripts reduce maintenance burden
- The defined publishing order ensures all dependencies are available when needed
- Consistent versioning across all crates in the workspace

### Negative

- Requires running the update scripts when the workspace version changes
- Introduces potential for version mismatch if the scripts are not run
- Adds slight complexity to the Cargo.toml file
- Requires a specific publishing order that must be maintained in the CI/CD pipeline

### Neutral

- The published crate on crates.io will only use the version specification, as the path specification is removed during publishing

## Implementation

The implementation involves:

1. Updating the root `Cargo.toml` file to include version specifications for all internal dependencies:

```toml
flowrs-core = { path = "./crates/flowrs-core", version = "1.0.2", optional = true }
flowrs-transform = { path = "./crates/flowrs-transform", version = "1.0.2", optional = true }
flowrs-event = { path = "./crates/flowrs-event", version = "1.0.2", optional = true }
flowrs-timer = { path = "./crates/flowrs-timer", version = "1.0.2", optional = true }
flowrs-longrunning = { path = "./crates/flowrs-longrunning", version = "1.0.2", optional = true }
flowrs-reactive = { path = "./crates/flowrs-reactive", version = "1.0.2", optional = true }
```

2. Ensuring all subcrates use workspace inheritance for their versions:

```toml
[package]
name = "flowrs-event"
version.workspace = true
edition.workspace = true
# ...
```

3. Creating a script (`scripts/update_dependency_versions.sh`) to automatically update the root crate's dependency versions:

```bash
#!/bin/bash
set -e

# Get the current workspace version from Cargo.toml
WORKSPACE_VERSION=$(grep -m 1 'version = ' Cargo.toml | cut -d '"' -f 2)

echo "Updating dependency versions to match workspace version: $WORKSPACE_VERSION"

# Update all internal dependency versions in the root Cargo.toml
sed -i.bak -E "s/(flowrs-[a-z]+[[:space:]]*=[[:space:]]*\{[[:space:]]*path[[:space:]]*=[[:space:]]*\"[^\"]+\",[[:space:]]*version[[:space:]]*=[[:space:]]*\")[0-9]+\.[0-9]+\.[0-9]+/\1$WORKSPACE_VERSION/g" Cargo.toml

# Remove backup file
rm Cargo.toml.bak

echo "Dependency versions updated successfully!"

# Verify the changes
echo "Verifying changes..."
grep -n "flowrs-" Cargo.toml | grep "version"

echo "Done!"
```

4. Creating a script (`scripts/update_subcrate_versions.sh`) to ensure all subcrates use workspace inheritance:

```bash
#!/bin/bash
set -e

echo "Updating subcrates to use workspace inheritance for versions..."

# List of subcrates to check and update
SUBCRATES=(
  "flowrs-core"
  "flowrs-transform"
  "flowrs-event"
  "flowrs-timer"
  "flowrs-longrunning"
  "flowrs-reactive"
)

for subcrate in "${SUBCRATES[@]}"; do
  CARGO_FILE="crates/$subcrate/Cargo.toml"
  echo "Checking $CARGO_FILE..."
  
  # Check if the subcrate is using workspace inheritance for version
  if grep -q "version.workspace = true" "$CARGO_FILE"; then
    echo "  ✅ $subcrate is already using workspace inheritance for version"
  else
    echo "  ⚠️ $subcrate is not using workspace inheritance for version. Updating..."
    
    # Get the current version line
    VERSION_LINE=$(grep -m 1 "^version = " "$CARGO_FILE" || echo "")
    
    if [ -n "$VERSION_LINE" ]; then
      # Replace the version line with workspace inheritance
      sed -i.bak "s/^version = .*/version.workspace = true/" "$CARGO_FILE"
      rm "$CARGO_FILE.bak"
      echo "  ✅ Updated $subcrate to use workspace inheritance for version"
    else
      echo "  ❌ Could not find version line in $subcrate"
    fi
  fi
done

echo "Done updating subcrates!"
```

5. Maintaining a specific publishing order in our release workflow:
   - First publish `flowrs-core`
   - Then publish `flowrs-transform`
   - Then publish `flowrs-event`
   - Then publish `flowrs-timer`
   - Then publish `flowrs-longrunning`
   - Then publish `flowrs-reactive`
   - Finally publish the root `flowrs` crate

6. Integrating both update scripts into our release process to ensure versions are always in sync.

### Future Improvements

For future consideration:

1. Adding a CI check to ensure that all version specifications match the workspace version
2. Investigating if Cargo workspace inheritance can be leveraged for this use case in future Rust/Cargo versions
3. Exploring other approaches to simplify dependency management in workspaces
4. Automating the publishing process further to reduce manual steps
5. Adding pre-commit hooks to run the version update scripts automatically

## Related ADRs

- [ADR 0014: Crate Publishing and CI/CD](0014-crate-publishing-and-cicd.md)
- [ADR 0029: Feature-Based Crate Organization](0029-feature-based-crate-organization.md) 