#!/bin/bash
# Lint all code in the project

set -e

echo "🔍 Linting Rust code..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🐍 Linting Python code..."
if [ -d "persist-python" ]; then
    cd persist-python
    
    if command -v ruff &> /dev/null; then
        ruff check .
        echo "✓ Python ruff linting complete"
    else
        echo "⚠️  ruff not found, skipping Python linting"
        echo "   Install with: pip install ruff"
    fi
    
    if command -v mypy &> /dev/null; then
        mypy . --ignore-missing-imports
        echo "✓ Python type checking complete"
    else
        echo "⚠️  mypy not found, skipping type checking"
        echo "   Install with: pip install mypy"
    fi
    
    cd ..
fi

echo "✅ All linting complete!"
