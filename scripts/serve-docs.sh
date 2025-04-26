#!/bin/bash

# Check if mkdocs is installed
if ! command -v mkdocs &> /dev/null; then
    echo "MkDocs is not installed. Installing..."
    pip install mkdocs
    pip install mkdocs-terminal
    pip install pymdown-extensions
fi

# Serve the documentation
echo "Starting MkDocs server..."
mkdocs serve 