#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default values
VERSION=""
DRY_RUN=false
SKIP_PUBLISH=false
HELP=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help)
      HELP=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --skip-publish)
      SKIP_PUBLISH=true
      shift
      ;;
    *)
      if [ -z "$VERSION" ]; then
        VERSION=$1
      else
        echo -e "${RED}Error: Unexpected argument: $1${NC}"
        HELP=true
      fi
      shift
      ;;
  esac
done

# Show help
if [ "$HELP" = true ] || [ -z "$VERSION" ]; then
  echo -e "${BLUE}Floxide Release Script${NC}"
  echo -e "${BLUE}======================${NC}"
  echo ""
  echo "Usage: $0 <version> [options]"
  echo ""
  echo "Arguments:"
  echo "  version             Version bump type (patch, minor, major) or specific version"
  echo ""
  echo "Options:"
  echo "  --dry-run           Show what would happen without making changes"
  echo "  --skip-publish      Skip the publishing step (only bump version)"
  echo "  -h, --help          Show this help message"
  echo ""
  echo "Examples:"
  echo "  $0 patch            # Bump patch version and publish"
  echo "  $0 minor --dry-run  # Show what would happen with a minor version bump"
  echo "  $0 1.2.3            # Set version to 1.2.3 and publish"
  echo "  $0 patch --skip-publish  # Only bump version, don't publish"
  exit 0
fi

# Check if a token is provided (only required for actual publishing)
if [ "$DRY_RUN" = false ] && [ "$SKIP_PUBLISH" = false ] && [ -z "$CRATES_IO_TOKEN" ]; then
  echo -e "${RED}Error: CRATES_IO_TOKEN environment variable is not set.${NC}"
  echo "Please set it with: export CRATES_IO_TOKEN=your_token"
  exit 1
fi

echo -e "${BLUE}Floxide Release Process${NC}"
echo -e "${BLUE}======================${NC}"

if [ "$DRY_RUN" = true ]; then
  echo -e "${YELLOW}Running in dry-run mode. No changes will be made.${NC}"
fi

if [ "$SKIP_PUBLISH" = true ]; then
  echo -e "${YELLOW}Publishing will be skipped.${NC}"
fi

echo -e "${YELLOW}Version: ${GREEN}$VERSION${NC}"
echo ""

# Step 1: Version bump
echo -e "${YELLOW}Step 1: Bumping version ($VERSION)...${NC}"
if [ "$DRY_RUN" = true ]; then
  # For dry run, just show what would happen
  echo -e "${BLUE}Current versions:${NC}"
  cargo workspaces list -a
  echo ""
  echo -e "${BLUE}Would bump to: ${GREEN}$VERSION${NC}"
else
  # For actual run, perform the version bump
  echo -e "${BLUE}Bumping version...${NC}"
  cargo workspaces version $VERSION --no-git-commit
  
  # Commit the version changes
  echo -e "${BLUE}Committing version changes...${NC}"
  git add .
  git commit -m "chore: bump version to $(cargo workspaces list -a | grep 'floxide ' | awk '{print $2}')"
  echo -e "${GREEN}✓ Version bumped and committed${NC}"
fi

# Step 2: Publish all crates
if [ "$DRY_RUN" = true ]; then
  echo -e "\n${YELLOW}Step 2: Publishing (dry run)${NC}"
  echo -e "${BLUE}Would publish crates in dependency order:${NC}"
  cargo workspaces list
elif [ "$SKIP_PUBLISH" = true ]; then
  echo -e "\n${YELLOW}Step 2: Publishing (skipped)${NC}"
  echo -e "${BLUE}Skipping publish as requested${NC}"
else
  echo -e "\n${YELLOW}Step 2: Publishing crates...${NC}"
  echo -e "${BLUE}Publishing in dependency order...${NC}"
  cargo workspaces publish --from-git --no-git-commit --token $CRATES_IO_TOKEN
  echo -e "${GREEN}✓ All crates published successfully${NC}"
fi

echo -e "\n${GREEN}Release process completed successfully!${NC}"
echo ""
echo -e "${BLUE}Note: cargo-workspaces handles the publishing order automatically,${NC}"
echo -e "${BLUE}ensuring that dependencies are published before the crates that depend on them.${NC}"
