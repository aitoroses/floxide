#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Floxide Version Management Script${NC}"
echo -e "${BLUE}=================================${NC}"

# Get the current workspace version from Cargo.toml
WORKSPACE_VERSION=$(grep -m 1 'version = ' Cargo.toml | cut -d '"' -f 2)

echo -e "${YELLOW}Current workspace version: ${GREEN}$WORKSPACE_VERSION${NC}"
echo ""

# Part 1: Update subcrates to use workspace inheritance
echo -e "${YELLOW}Part 1: Updating subcrates to use workspace inheritance...${NC}"

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
  echo -e "  Checking ${BLUE}$CARGO_FILE${NC}..."
  
  # Check if the subcrate is using workspace inheritance for version
  if grep -q "version.workspace = true" "$CARGO_FILE"; then
    echo -e "    ${GREEN}✓ Already using workspace inheritance for version${NC}"
  else
    echo -e "    ${YELLOW}⚠️ Not using workspace inheritance. Updating...${NC}"
    
    # Get the current version line
    VERSION_LINE=$(grep -m 1 "^version = " "$CARGO_FILE" || echo "")
    
    if [ -n "$VERSION_LINE" ]; then
      # Replace the version line with workspace inheritance
      sed -i.bak "s/^version = .*/version.workspace = true/" "$CARGO_FILE"
      rm -f "$CARGO_FILE.bak"
      echo -e "    ${GREEN}✓ Updated to use workspace inheritance${NC}"
    else
      echo -e "    ${YELLOW}⚠️ No version line found, skipping${NC}"
    fi
  fi
  
  # Check for other workspace inheritance opportunities
  for field in "edition" "authors" "license" "repository" "description"; do
    if grep -q "^$field = " "$CARGO_FILE" && ! grep -q "$field.workspace = true" "$CARGO_FILE"; then
      echo -e "    ${YELLOW}⚠️ $field could use workspace inheritance. Consider updating.${NC}"
    fi
  done
  
  echo ""
done

# Part 2: Update dependency versions in the root Cargo.toml
echo -e "${YELLOW}Part 2: Updating dependency versions in root Cargo.toml...${NC}"

# Update all internal dependency versions in the root Cargo.toml
# First, handle the format: version = "x.y.z"
sed -i.bak -E "s/(floxide-[a-z]+[[:space:]]*=[[:space:]]*\{[[:space:]]*path[[:space:]]*=[[:space:]]*\"[^\"]+\",[[:space:]]*version[[:space:]]*=[[:space:]]*\")[0-9]+\.[0-9]+\.[0-9]+/\1$WORKSPACE_VERSION/g" Cargo.toml

# Then, handle the format: version = "=x.y.z"
sed -i.bak -E "s/(floxide-[a-z]+[[:space:]]*=[[:space:]]*\{[[:space:]]*path[[:space:]]*=[[:space:]]*\"[^\"]+\",[[:space:]]*version[[:space:]]*=[[:space:]]*\"=)[0-9]+\.[0-9]+\.[0-9]+/\1$WORKSPACE_VERSION/g" Cargo.toml

# Also handle cases where version might come before path
sed -i.bak -E "s/(floxide-[a-z]+[[:space:]]*=[[:space:]]*\{[[:space:]]*version[[:space:]]*=[[:space:]]*\")[0-9]+\.[0-9]+\.[0-9]+/\1$WORKSPACE_VERSION/g" Cargo.toml
sed -i.bak -E "s/(floxide-[a-z]+[[:space:]]*=[[:space:]]*\{[[:space:]]*version[[:space:]]*=[[:space:]]*\"=)[0-9]+\.[0-9]+\.[0-9]+/\1$WORKSPACE_VERSION/g" Cargo.toml

# Remove backup file
rm -f Cargo.toml.bak

echo -e "${GREEN}✓ Dependency versions updated successfully!${NC}"

# Verify the changes
echo -e "${YELLOW}Verifying changes...${NC}"
grep -n "floxide-" Cargo.toml | grep "version"

echo -e "\n${GREEN}All version updates completed successfully!${NC}"
