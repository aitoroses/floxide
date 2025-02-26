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