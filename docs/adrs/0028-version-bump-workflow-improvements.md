# ADR-0028: Version Bump Workflow Improvements

## Status

Proposed

## Date

2025-02-27

## Context

The Floxide framework uses GitHub Actions for CI/CD, including a workflow for version bumping that automates the process of updating version numbers across the workspace. This workflow uses `cargo-workspaces` to coordinate version changes and creates Git tags for releases.

We've encountered several issues with the version bump workflow:

1. **Tag Conflict Error**: The workflow fails with the error:

```
fatal: tag 'v1.0.0' already exists
Error: Process completed with exit code 128
```

This occurs because:
- The workflow attempts to create a tag that already exists in the repository
- The current implementation doesn't check for existing tags before attempting to create them
- The workflow creates both individual package tags (e.g., `floxide-core@1.0.0`) and a global version tag (e.g., `v1.0.0`)

2. **Dependency Version Mismatch**: When trying to update only some crates in the workspace, we encounter errors like:

```
error: failed to select a version for the requirement `floxide-core = "=1.0.0"`
candidate versions found which didn't match: 1.0.1
```

This happens because:
- Some crates are updated while others remain at the previous version
- Crates have exact version dependencies on each other (using `=` in version requirements)
- This creates inconsistent dependency requirements that cannot be satisfied

3. **Explicit Version Requirements Not Updated**: The main `floxide` crate at the root has explicit version requirements for its dependencies:

```toml
floxide-core = { path = "crates/floxide-core", version = "1.0.0", optional = true }
```

When the workspace version is updated, these explicit version requirements are not automatically updated by `cargo-workspaces`, causing dependency resolution errors.

Additionally, when the version bump workflow fails, it doesn't trigger the release workflow that publishes the crates to crates.io, breaking the automated release process.

## Decision

We will improve the version bump workflow to handle existing tags gracefully, ensure consistent versioning across all crates, update explicit version requirements, and ensure proper triggering of the release workflow by implementing the following changes:

1. **Check for Existing Tags**: Before attempting to create tags, check if they already exist.

2. **Force Option for Tags**: Add an option to force-update existing tags when necessary.

3. **Skip Tag Creation Option**: Add a workflow input parameter to optionally skip tag creation entirely.

4. **Trigger Release Option**: Add an option to ensure the release workflow is triggered after a successful version bump.

5. **Handle Individual Crate Tags**: When force-updating tags, also handle individual crate tags.

6. **Update All Crates Together**: Use the `--all` flag with `cargo workspaces` to ensure all crates in the workspace are updated to the same version, maintaining consistent dependencies.

7. **Update Explicit Version Requirements**: Add a step to update the explicit version requirements in the main Cargo.toml file.

8. **Improved Error Handling**: Enhance error handling to provide clearer messages when tag-related issues occur.

The implementation will involve updating the `.github/workflows/version-bump.yml` file to include these improvements.

### Updated Workflow

The updated workflow will include:

```yaml
# In the workflow_dispatch inputs section
inputs:
  bump_type:
    description: "Version bump type"
    required: true
    type: choice
    options:
      - patch
      - minor
      - major
  dry_run:
    description: "Dry run (no commits)"
    required: true
    type: boolean
    default: true
  force_tag:
    description: "Force update existing tags"
    required: false
    type: boolean
    default: false
  skip_tagging:
    description: "Skip tag creation entirely"
    required: false
    type: boolean
    default: false
  trigger_release:
    description: "Trigger release workflow after version bump"
    required: false
    type: boolean
    default: false
```

The version bump command will be updated to:

```yaml
# Use --no-git-tag to handle tagging manually and --all to update all packages
cargo workspaces version ${{ github.event.inputs.bump_type }} --exact --yes --no-git-tag --all
```

And we'll add a step to update explicit version requirements:

```yaml
# Also update the explicit version requirements in the main Cargo.toml
# This is needed because cargo-workspaces doesn't update these
sed -i "s/floxide-core = { path = \"crates\/floxide-core\", version = \"[0-9.]*\"/floxide-core = { path = \"crates\/floxide-core\", version = \"$NEW_VERSION\"/g" Cargo.toml
sed -i "s/floxide-transform = { path = \"crates\/floxide-transform\", version = \"[0-9.]*\"/floxide-transform = { path = \"crates\/floxide-transform\", version = \"$NEW_VERSION\"/g" Cargo.toml
# ... (similar lines for other crates)

# Commit the changes to the explicit version requirements
git add Cargo.toml
git commit --amend --no-edit
git push --force-with-lease origin HEAD
```

The tag creation step will be updated to:

```yaml
# Create and push tag if not skipped
if [ "${{ github.event.inputs.skip_tagging }}" != "true" ]; then
  TAG_NAME="v${NEW_VERSION}"
  
  # Check if tag exists
  if git show-ref --tags "$TAG_NAME" --quiet; then
    if [ "${{ github.event.inputs.force_tag }}" = "true" ]; then
      echo "Tag $TAG_NAME exists, force updating"
      git tag -d "$TAG_NAME"
      git push origin ":refs/tags/$TAG_NAME" || true
      
      # Also delete individual crate tags if they exist
      for CRATE_TAG in $(git tag | grep "@${NEW_VERSION}$"); do
        echo "Deleting individual crate tag: $CRATE_TAG"
        git tag -d "$CRATE_TAG"
        git push origin ":refs/tags/$CRATE_TAG" || true
      done
      
      # Create new tag
      git tag -a "$TAG_NAME" -m "Release $TAG_NAME"
      git push origin "$TAG_NAME"
      
      echo "Tags updated successfully"
    else
      echo "Tag $TAG_NAME already exists. Use force_tag option to update it."
      exit 0
    fi
  else
    git tag -a "$TAG_NAME" -m "Release $TAG_NAME"
    git push origin "$TAG_NAME"
  fi
  
  # If trigger_release is true, wait a moment for tag to be processed
  # and then check if the tag was successfully pushed
  if [ "${{ github.event.inputs.trigger_release }}" = "true" ]; then
    echo "Waiting for tag to be processed before triggering release workflow..."
    sleep 10
    
    # Check if the tag was successfully pushed
    if git ls-remote --tags origin | grep -q "$TAG_NAME"; then
      echo "Tag $TAG_NAME successfully pushed, release workflow should be triggered automatically."
    else
      echo "Warning: Tag $TAG_NAME not found on remote. Release workflow may not trigger."
    fi
  fi
else
  echo "Skipping tag creation as requested"
fi
```

## Consequences

### Positive

1. **Improved Reliability**: The workflow will handle existing tags gracefully, reducing CI failures.
2. **More Flexibility**: Users will have more control over tag creation and updates.
3. **Better Error Messages**: Clearer feedback when tag-related issues occur.
4. **Safer Operations**: Prevents accidental tag overwrites without explicit permission.
5. **Complete Release Process**: Ensures the entire release pipeline from version bump to crates.io publishing works correctly.
6. **Comprehensive Tag Management**: Handles both global and individual crate tags properly.
7. **Consistent Versioning**: Ensures all crates in the workspace are updated together, preventing dependency version mismatches.
8. **Dependency Coherence**: Updates explicit version requirements in the main Cargo.toml to match the new workspace version.

### Negative

1. **Increased Complexity**: The workflow becomes slightly more complex with additional options.
2. **Potential for Confusion**: More options may require better documentation for users.
3. **Less Granular Control**: Using `--all` means individual crates cannot be versioned independently, which may be limiting in some scenarios.
4. **Brittle Version Updating**: The sed commands for updating explicit version requirements are somewhat brittle and may need maintenance if the Cargo.toml structure changes.

## Alternatives Considered

### Manually Deleting Tags Before Running the Workflow

- **Pros**:
  - Simple approach
  - No workflow changes needed
- **Cons**:
  - Manual intervention required
  - Error-prone
  - Not scalable

We rejected this approach as it doesn't provide a sustainable, automated solution.

### Using Different Tag Naming Conventions

- **Pros**:
  - Could avoid conflicts by using unique names
  - Maintains history differently
- **Cons**:
  - Breaks existing conventions
  - May confuse users
  - Requires migration

We chose to maintain the existing tag naming convention for consistency and compatibility.

### Disabling Tagging in cargo-workspaces

- **Pros**:
  - Simpler workflow
  - More direct control
- **Cons**:
  - Loses some automation benefits
  - Requires manual tag management

We chose a hybrid approach that leverages cargo-workspaces for version management but gives us more control over tag creation.

### Separate Workflows for Version Bump and Release

- **Pros**:
  - Clearer separation of concerns
  - Potentially simpler workflows
- **Cons**:
  - More manual steps
  - Potential for inconsistency between version and release

We chose to enhance the existing workflow structure to maintain the automated pipeline while adding more control options.

### Independent Versioning of Crates

- **Pros**:
  - More granular control over individual crate versions
  - Allows for different release cycles per crate
- **Cons**:
  - Requires more complex dependency management
  - Prone to version mismatch errors
  - More difficult to maintain

We chose to use the `--all` flag to ensure consistent versioning across all crates, which simplifies dependency management and reduces the chance of errors.

### Using Cargo Workspace Inheritance Without Explicit Versions

- **Pros**:
  - Simpler dependency declarations
  - Automatic version synchronization
- **Cons**:
  - Less explicit about version requirements
  - May not work well with external consumers of the crates

We chose to maintain explicit version requirements in the main Cargo.toml for clarity and compatibility with external consumers, while adding automation to keep these versions in sync. 