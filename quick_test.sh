#!/bin/bash

# Quick test script to verify basic functionality without heavy dependencies
set -e

echo "Running quick functionality tests..."

cd /workspace/persist_repo

# Test 1: Check compilation
echo "Testing compilation..."
source ~/.cargo/env
cargo check --release --quiet && echo "✅ Compilation: PASSED" || echo "❌ Compilation: FAILED"

# Test 2: Run a simple unit test (without AWS dependencies)
echo "Testing basic unit tests..."
cd persist-core
cargo test --lib --release compression::tests::test_gzip_compression_roundtrip --quiet && echo "✅ Basic tests: PASSED" || echo "❌ Basic tests: FAILED"

# Test 3: Test the example
echo "Building simple benchmark example..."
cargo build --example simple_benchmark --release --quiet && echo "✅ Example build: PASSED" || echo "❌ Example build: FAILED"

# Test 4: Run the example if it built successfully
if [ -f "target/release/examples/simple_benchmark" ]; then
    echo "Running benchmark example..."
    timeout 30s ./target/release/examples/simple_benchmark && echo "✅ Example run: PASSED" || echo "❌ Example run: FAILED"
fi

cd ..

# Test 5: Python SDK build test
echo "Testing Python SDK build..."
cd persist-python
if command -v maturin &> /dev/null; then
    maturin build --release --quiet && echo "✅ Python build: PASSED" || echo "❌ Python build: FAILED"
else
    echo "⚠️  Maturin not available - skipping Python build test"
fi

cd ..

echo ""
echo "Quick test summary completed!"
echo "For comprehensive testing, run: bash scripts/run_comprehensive_tests.sh"
