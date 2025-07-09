#!/bin/bash
# Run all tests in the project

set -e

# Parse command line arguments
INCLUDE_INTEGRATION=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --integration)
            INCLUDE_INTEGRATION=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--integration] [--verbose]"
            echo "  --integration  Include integration tests (requires setup)"
            echo "  --verbose      Enable verbose output"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "🧪 Running Rust tests..."
if [ "$VERBOSE" = true ]; then
    RUST_LOG=debug cargo test --all --all-features -- --nocapture
else
    cargo test --all --all-features
fi

if [ "$INCLUDE_INTEGRATION" = true ]; then
    echo "🔗 Running integration tests..."
    if [ "$VERBOSE" = true ]; then
        RUST_LOG=debug cargo test --test integration_tests -- --nocapture
    else
        cargo test --test integration_tests
    fi
fi

echo "🐍 Running Python tests..."
if [ -d "persist-python" ]; then
    cd persist-python
    
    # Build the Python extension first
    echo "📦 Building Python extension..."
    if command -v maturin &> /dev/null; then
        maturin develop --release
    else
        echo "⚠️  maturin not found, cannot build Python extension"
        echo "   Install with: pip install maturin"
        cd ..
        exit 1
    fi
    
    # Run Python tests
    if command -v pytest &> /dev/null; then
        if [ "$VERBOSE" = true ]; then
            pytest -v
        else
            pytest
        fi
        echo "✓ Python tests complete"
    else
        echo "⚠️  pytest not found, skipping Python tests"
        echo "   Install with: pip install pytest"
    fi
    
    cd ..
fi

echo "📚 Running documentation tests..."
cargo test --doc

echo "✅ All tests complete!"
