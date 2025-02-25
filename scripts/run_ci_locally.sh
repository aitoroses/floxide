#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running CI checks locally...${NC}"

# Check if required tools are installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed. Please install Rust and Cargo.${NC}"
    exit 1
fi

# Install cargo-audit if not already installed
if ! command -v cargo-audit &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-audit...${NC}"
    cargo install cargo-audit
fi

# Set environment variables
export RUSTFLAGS="-D warnings"

# Step 1: Check formatting
echo -e "\n${YELLOW}Step 1/5: Checking code formatting...${NC}"
cargo fmt --all -- --check
echo -e "${GREEN}✓ Code formatting check passed${NC}"

# Step 2: Run clippy
echo -e "\n${YELLOW}Step 2/5: Running clippy...${NC}"
cargo clippy --all-targets --all-features -- -D warnings
echo -e "${GREEN}✓ Clippy check passed${NC}"

# Step 3: Run tests
echo -e "\n${YELLOW}Step 3/5: Running tests...${NC}"
cargo test --all-features
echo -e "${GREEN}✓ Tests passed${NC}"

# Step 4: Build documentation
echo -e "\n${YELLOW}Step 4/5: Building documentation...${NC}"
cargo doc --no-deps --all-features
echo -e "${GREEN}✓ Documentation built successfully${NC}"

# Step 5: Run security audit
echo -e "\n${YELLOW}Step 5/5: Running security audit...${NC}"
cargo audit
echo -e "${GREEN}✓ Security audit passed${NC}"

echo -e "\n${GREEN}All CI checks passed successfully!${NC}" 