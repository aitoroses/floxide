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