#!/bin/bash
set -e

echo "Renaming project from floxide to floxide..."

# Step 1: Rename directories
echo "Renaming directories..."
for dir in crates/floxide-*; do
  if [ -d "$dir" ]; then
    new_dir=$(echo "$dir" | sed 's/floxide/floxide/')
    echo "  $dir -> $new_dir"
    mkdir -p "$new_dir"
    cp -r "$dir"/* "$new_dir"/
  fi
done

# Step 2: Update file contents with sed
echo "Updating file contents..."
find . -type f -not -path "*/target/*" -not -path "*/.git/*" -not -path "*/\.*" -exec grep -l "floxide" {} \; | while read file; do
  echo "  Processing $file"
  sed -i '' 's/floxide/floxide/g' "$file"
  sed -i '' 's/Floxide/Floxide/g' "$file"
  sed -i '' 's/FLOXIDE/FLOXIDE/g' "$file"
done

# Step 3: Update specific error type references
echo "Updating error type references..."
find . -type f -not -path "*/target/*" -not -path "*/.git/*" -not -path "*/\.*" -name "*.rs" -exec grep -l "FloxideError" {} \; | while read file; do
  echo "  Processing $file for FloxideError"
  sed -i '' 's/FloxideError/FloxideError/g' "$file"
  sed -i '' 's/FloxideResult/FloxideResult/g' "$file"
done

echo "Rebranding complete! Please review changes and run cargo build to verify."
