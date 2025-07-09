#!/bin/bash
# Format all code in the project

set -e

echo "🎨 Formatting Rust code..."
cargo fmt --all

echo "🐍 Formatting Python code..."
if [ -d "persist-python" ]; then
    cd persist-python
    if command -v black &> /dev/null; then
        black .
        echo "✓ Python formatting complete"
    else
        echo "⚠️  black not found, skipping Python formatting"
        echo "   Install with: pip install black"
    fi
    cd ..
fi

echo "✨ All formatting complete!"
