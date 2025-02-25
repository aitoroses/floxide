# ADR-0028: Version Bump Workflow Improvements

## Status

Proposed

## Date

2024-02-25

## Context

The Flowrs framework uses GitHub Actions for CI/CD, including a workflow for version bumping that automates the process of updating version numbers across the workspace. This workflow uses `cargo-workspaces` to coordinate version changes and creates Git tags for releases.

We've encountered an issue where the version bump workflow fails with the error:

```
fatal: tag 'v1.0.0' already exists
Error: Process completed with exit code 128
```

This occurs because:

1. The workflow attempts to create a tag that already exists in the repository
2. The current implementation doesn't check for existing tags before attempting to create them
3. The workflow creates both individual package tags (e.g., `flowrs-core@1.0.0`) and a global version tag (e.g., `v1.0.0`)

Investigation shows that the repository already contains the following tags:
- `v1.0.0` (global version tag)
- Individual package tags like `flowrs-core@1.0.0`, `flowrs-event@1.0.0`, etc.

This prevents the workflow from completing successfully when attempting to bump versions to a value that has already been tagged.

Additionally, when the version bump workflow fails, it doesn't trigger the release workflow that publishes the crates to crates.io, breaking the automated release process.

## Decision

We will improve the version bump workflow to handle existing tags gracefully and ensure proper triggering of the release workflow by implementing the following changes:

1. **Check for Existing Tags**: Before attempting to create tags, check if they already exist.

2. **Force Option for Tags**: Add an option to force-update existing tags when necessary.

3. **Skip Tag Creation Option**: Add a workflow input parameter to optionally skip tag creation entirely.

4. **Trigger Release Option**: Add an option to ensure the release workflow is triggered after a successful version bump.

5. **Handle Individual Crate Tags**: When force-updating tags, also handle individual crate tags.

6. **Improved Error Handling**: Enhance error handling to provide clearer messages when tag-related issues occur.

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

And the tag creation step will be updated to:

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

Additionally, we'll update the `cargo workspaces version` command to include the `--no-git-tag` option when we want to manage tags manually:

```yaml
cargo workspaces version ${{ github.event.inputs.bump_type }} --exact --yes --no-git-tag
```

## Consequences

### Positive

1. **Improved Reliability**: The workflow will handle existing tags gracefully, reducing CI failures.
2. **More Flexibility**: Users will have more control over tag creation and updates.
3. **Better Error Messages**: Clearer feedback when tag-related issues occur.
4. **Safer Operations**: Prevents accidental tag overwrites without explicit permission.
5. **Complete Release Process**: Ensures the entire release pipeline from version bump to crates.io publishing works correctly.
6. **Comprehensive Tag Management**: Handles both global and individual crate tags properly.

### Negative

1. **Increased Complexity**: The workflow becomes slightly more complex with additional options.
2. **Potential for Confusion**: More options may require better documentation for users.

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