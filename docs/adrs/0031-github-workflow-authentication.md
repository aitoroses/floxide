# ADR 0031: GitHub Workflow Authentication for Release Process

## Date
2023-11-15

## Status
Accepted

## Context
The Flow Framework release process requires authentication for two primary operations:
1. Publishing packages to crates.io
2. Creating GitHub releases and interacting with the repository

These operations require different types of authentication tokens with specific permissions. Without proper authentication, the automated release process would fail, requiring manual intervention.

## Decision
We have decided to implement a two-token authentication strategy for our GitHub Actions workflow:

1. **CRATES_IO_TOKEN**: Used for authenticating with crates.io to publish packages
   - This token is specific to crates.io and has permissions to publish packages under the account that generated it
   - It is used with the `--token` flag in all `cargo publish` commands

2. **GITHUB_TOKEN**: Used for GitHub repository operations that don't need to trigger other workflows
   - This is automatically provided by GitHub Actions
   - It has permissions defined in the workflow file
   - It's used for creating GitHub releases

3. **WORKFLOW_PAT** (only when needed): Used for GitHub repository operations that need to trigger other workflows
   - This is a GitHub Personal Access Token (PAT) with appropriate repository permissions
   - It's used for checkout operations when we need to push changes that should trigger other workflows
   - The default `GITHUB_TOKEN` cannot trigger workflow runs when pushing commits/tags

Both tokens are stored as GitHub repository secrets and referenced in the workflow file using the `${{ secrets.TOKEN_NAME }}` syntax. The `GITHUB_TOKEN` is automatically available in all workflows.

## Implementation
The tokens are used in the following ways in our `.github/workflows/release.yml` file:

1. **CRATES_IO_TOKEN**:
   - Used in all `cargo publish` commands for each subcrate and the root crate
   - Example: `cargo publish --token ${{ secrets.CRATES_IO_TOKEN }}`

2. **WORKFLOW_PAT**:
   - Used in the checkout step when we need to push changes that should trigger workflows: 
     ```yaml
     - uses: actions/checkout@v3
       with:
         ref: ${{ steps.determine_ref.outputs.TAG_REF }}
         fetch-depth: 0
         token: ${{ secrets.WORKFLOW_PAT }}
     ```

3. **GITHUB_TOKEN**:
   - Used in the GitHub Release creation step:
     ```yaml
     - name: Create GitHub Release
       uses: softprops/action-gh-release@v1
       with:
         tag_name: ${{ steps.determine_ref.outputs.TAG_REF }}
         name: Release ${{ steps.determine_ref.outputs.TAG_REF }}
         draft: false
         prerelease: false
         generate_release_notes: true
       env:
         GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
     ```

## Token Setup Instructions
1. **CRATES_IO_TOKEN**:
   - Log in to crates.io
   - Go to Account Settings
   - Generate a new API token
   - Add it as a repository secret in GitHub

2. **WORKFLOW_PAT** (only needed for operations that must trigger other workflows):
   - Go to GitHub Settings > Developer settings > Personal access tokens
   - Generate a new token with the following permissions:
     - `repo` (Full control of private repositories)
     - `workflow` (Update GitHub Action workflows)
   - Add it as a repository secret in GitHub

3. **GITHUB_TOKEN**:
   - No setup required - automatically provided by GitHub Actions
   - Permissions can be configured in the workflow file

## When to Use Each Token

1. **GITHUB_TOKEN**:
   - Use for most GitHub operations (creating releases, commenting on issues, etc.)
   - Has limited permissions by default for security
   - Cannot trigger new workflow runs when pushing commits/tags

2. **WORKFLOW_PAT**:
   - Use only when you need to push commits/tags that should trigger other workflows
   - Has broader permissions, so use with caution
   - Requires manual creation and rotation

3. **CRATES_IO_TOKEN**:
   - Use only for publishing to crates.io
   - Specific to the crates.io registry

## Consequences
### Positive
- Automated release process with proper authentication
- Separation of concerns between crates.io publishing and GitHub operations
- Secure handling of tokens through GitHub secrets
- Using `GITHUB_TOKEN` where possible for better security

### Negative
- Tokens need to be rotated periodically for security
- Multiple tokens to manage
- PATs are tied to individual GitHub accounts, which may cause issues if the token creator leaves the project
- Need to understand the limitations of `GITHUB_TOKEN` vs. `WORKFLOW_PAT`

## Future Considerations
- Consider using GitHub's OIDC provider for more secure, short-lived tokens
- Implement token rotation reminders or automation
- Document token permissions and access in a central location
- Consider using GitHub Apps instead of PATs for better security and management
- Evaluate if we can restructure workflows to minimize the need for `WORKFLOW_PAT` 