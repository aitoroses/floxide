# ADR 0030: Workspace Dependency Versioning for Publishing

## Status

Accepted

## Context

When publishing Rust crates to crates.io, all dependencies must have explicit version specifications, even if they are path dependencies within a workspace. This is a requirement of the crates.io publishing process.

In our workspace structure, we have a root `floxide` crate that depends on several subcrates (`floxide-core`, `floxide-transform`, etc.). These dependencies were initially specified only with `path` attributes, which works fine for local development but causes errors during the publishing process.

The error encountered was:
```
error: all dependencies must have a version specified when publishing.
dependency `floxide-core` does not specify a version
Note: The published dependency will use the version from crates.io,
the `path` specification will be removed from the dependency declaration.
```

Additionally, we need to ensure that when the workspace version changes, we don't have to manually update multiple version specifications throughout the codebase, which would be error-prone and create maintenance overhead.

We also encountered an issue during version bumping where some subcrates had hardcoded versions instead of using workspace inheritance, causing errors like:
```
error: failed to select a version for the requirement `floxide-event = "^1.0.2"`
candidate versions found which didn't match: 1.0.0
location searched: /home/runner/work/floxide/floxide/crates/floxide-event
required by package `floxide v1.0.3 (/home/runner/work/floxide/floxide)`
```

## Decision

We will implement a two-part solution to address these versioning issues:

1. **For the root crate's dependencies**: Maintain explicit version specifications for all workspace dependencies in the root `Cargo.toml` file, in addition to the path specifications. These versions will match the workspace version defined in `[workspace.package]`.

2. **For subcrates**: Ensure all subcrates use workspace inheritance for their versions by using `version.workspace = true` instead of hardcoded versions.

The dependency specifications in the root crate will follow this pattern:
```toml
floxide-core = { path = "./crates/floxide-core", version = "1.0.2", optional = true }
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
floxide-core = { path = "./crates/floxide-core", version = "1.0.2", optional = true }
floxide-transform = { path = "./crates/floxide-transform", version = "1.0.2", optional = true }
floxide-event = { path = "./crates/floxide-event", version = "1.0.2", optional = true }
floxide-timer = { path = "./crates/floxide-timer", version = "1.0.2", optional = true }
floxide-longrunning = { path = "./crates/floxide-longrunning", version = "1.0.2", optional = true }
floxide-reactive = { path = "./crates/floxide-reactive", version = "1.0.2", optional = true }
```

2. Ensuring all subcrates use workspace inheritance for their versions:

```toml
[package]
name = "floxide-event"
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
sed -i.bak -E "s/(floxide-[a-z]+[[:space:]]*=[[:space:]]*\{[[:space:]]*path[[:space:]]*=[[:space:]]*\"[^\"]+\",[[:space:]]*version[[:space:]]*=[[:space:]]*\")[0-9]+\.[0-9]+\.[0-9]+/\1$WORKSPACE_VERSION/g" Cargo.toml

# Remove backup file
rm Cargo.toml.bak

echo "Dependency versions updated successfully!"

# Verify the changes
echo "Verifying changes..."
grep -n "floxide-" Cargo.toml | grep "version"

echo "Done!"
```

4. Creating a script (`scripts/update_subcrate_versions.sh`) to ensure all subcrates use workspace inheritance:

```bash
#!/bin/bash
set -e

echo "Updating subcrates to use workspace inheritance for versions..."

# List of subcrates to check and update
SUBCRATES=(
  "floxide-core"
  "floxide-transform"
  "floxide-event"
  "floxide-timer"
  "floxide-longrunning"
  "floxide-reactive"
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
   - First publish `floxide-core`
   - Then publish `floxide-transform`
   - Then publish `floxide-event`
   - Then publish `floxide-timer`
   - Then publish `floxide-longrunning`
   - Then publish `floxide-reactive`
   - Finally publish the root `floxide` crate

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

## Additional Considerations

### Consistent Version Formats

When specifying versions for internal dependencies in the root crate, it's important to maintain a consistent format. We encountered issues with mixed version formats:

```toml
# Inconsistent version formats
floxide-core = { path = "./crates/floxide-core", version = "1.0.2", optional = true }  # No equals sign
floxide-event = { path = "./crates/floxide-event", version = "=1.0.3", optional = true }  # With equals sign
```

The `update_dependency_versions.sh` script has been enhanced to handle both formats:
- Version specifications with no equals sign: `version = "1.0.3"`
- Version specifications with equals sign: `version = "=1.0.3"`

Additionally, the script now handles cases where the `version` attribute might appear before the `path` attribute in the dependency specification.

### Publishing Order Importance

When publishing to crates.io, all dependencies must have explicit version specifications, even for local path dependencies. If a subcrate is published without all its dependencies having explicit versions, the publish will fail with an error like:

```
error: all dependencies must have a version specified when publishing.
dependency `floxide-core` does not specify a version
```

This reinforces the importance of:
1. Running the `update_dependency_versions.sh` script before publishing
2. Following the correct publishing order (subcrates first, then the root crate)
3. Ensuring all version numbers are consistent across the workspace 