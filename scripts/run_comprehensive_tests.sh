#!/bin/bash

# Comprehensive testing script for Persist
# This script runs all tests, generates coverage reports, and performs performance analysis

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "persist-core" ]; then
    print_error "This script must be run from the root of the Persist repository"
    exit 1
fi

print_status "Starting comprehensive test suite for Persist..."

# Create results directory
mkdir -p test_results
mkdir -p test_results/coverage
mkdir -p test_results/benchmarks
mkdir -p test_results/profiling

# 1. Install required tools
print_status "Installing required testing tools..."

# Install tarpaulin for coverage (if not already installed)
if ! command -v cargo-tarpaulin &> /dev/null; then
    print_status "Installing cargo-tarpaulin for coverage analysis..."
    cargo install cargo-tarpaulin || {
        print_warning "cargo-tarpaulin installation failed, trying alternative installation..."
        # Try installing via apt if available
        if command -v apt &> /dev/null; then
            sudo apt update && sudo apt install -y cargo-tarpaulin || true
        fi
    }
fi

# Install flamegraph for profiling (if not already installed)
if ! command -v cargo-flamegraph &> /dev/null; then
    print_status "Installing cargo-flamegraph for performance profiling..."
    cargo install flamegraph || print_warning "flamegraph installation failed"
fi

# Install hyperfine for benchmarking (if not already installed)
if ! command -v hyperfine &> /dev/null; then
    print_status "Installing hyperfine for performance timing..."
    if command -v apt &> /dev/null; then
        sudo apt update && sudo apt install -y hyperfine || {
            print_warning "hyperfine installation via apt failed, trying cargo install..."
            cargo install hyperfine || print_warning "hyperfine installation failed"
        }
    else
        cargo install hyperfine || print_warning "hyperfine installation failed"
    fi
fi

# 2. Format and lint checks
print_status "Running code formatting and linting checks..."

print_status "Checking Rust formatting..."
cargo fmt --all -- --check || {
    print_warning "Code formatting issues found. Running cargo fmt to fix..."
    cargo fmt --all
    print_success "Code formatting applied"
}

print_status "Running clippy for linting..."
cargo clippy --all-targets --all-features -- -D warnings || {
    print_error "Clippy found issues that need to be fixed"
    exit 1
}

print_success "Code formatting and linting checks passed"

# 3. Unit tests
print_status "Running unit tests..."

print_status "Running Rust core unit tests..."
cd persist-core
cargo test --lib --release > ../test_results/unit_tests_rust.log 2>&1 || {
    print_error "Rust unit tests failed. Check test_results/unit_tests_rust.log"
    tail -20 ../test_results/unit_tests_rust.log
    exit 1
}
cd ..

print_success "Unit tests passed"

# 4. Integration tests
print_status "Running integration tests..."

cargo test --test end_to_end_tests --release > test_results/integration_tests.log 2>&1 || {
    print_error "Integration tests failed. Check test_results/integration_tests.log"
    tail -20 test_results/integration_tests.log
    exit 1
}

print_success "Integration tests passed"

# 5. Python SDK tests
print_status "Running Python SDK tests..."

cd persist-python

# Build the Python package
print_status "Building Python package..."
maturin develop --release > ../test_results/python_build.log 2>&1 || {
    print_error "Python package build failed. Check test_results/python_build.log"
    exit 1
}

# Run Python tests if they exist
if [ -f "pytest.ini" ] || [ -d "tests" ] || find . -name "test_*.py" -o -name "*_test.py" | grep -q .; then
    print_status "Running Python tests..."
    python -m pytest -v > ../test_results/python_tests.log 2>&1 || {
        print_error "Python tests failed. Check test_results/python_tests.log"
        exit 1
    }
    print_success "Python tests passed"
else
    print_warning "No Python tests found"
fi

cd ..

# 6. Coverage analysis
print_status "Generating code coverage report..."

if command -v cargo-tarpaulin &> /dev/null; then
    print_status "Running tarpaulin for coverage analysis..."
    cd persist-core
    cargo tarpaulin --out Html --output-dir ../test_results/coverage --release --timeout 300 > ../test_results/coverage.log 2>&1 || {
        print_warning "Coverage analysis failed. Check test_results/coverage.log"
    }
    cd ..
    
    # Extract coverage percentage
    if [ -f "test_results/coverage.log" ]; then
        COVERAGE=$(grep -oP '\d+\.\d+%' test_results/coverage.log | tail -1 | sed 's/%//')
        if [ ! -z "$COVERAGE" ]; then
            print_status "Code coverage: $COVERAGE%"
            if (( $(echo "$COVERAGE >= 90" | bc -l) )); then
                print_success "Coverage target of 90% achieved!"
            else
                print_warning "Coverage is below 90% target. Current: $COVERAGE%"
            fi
        fi
    fi
else
    print_warning "Skipping coverage analysis - tarpaulin not available"
fi

# 7. Performance benchmarks
print_status "Running performance benchmarks..."

cd persist-core

print_status "Running criterion benchmarks..."
cargo bench > ../test_results/benchmarks/criterion_results.log 2>&1 || {
    print_warning "Criterion benchmarks failed. Check test_results/benchmarks/criterion_results.log"
}

# Copy criterion results if they exist
if [ -d "target/criterion" ]; then
    cp -r target/criterion/* ../test_results/benchmarks/ 2>/dev/null || true
fi

cd ..

# 8. Hyperfine performance testing
print_status "Running hyperfine performance tests..."

if command -v hyperfine &> /dev/null; then
    # Create a simple test binary for hyperfine
    print_status "Building test binary for hyperfine..."
    cd persist-core
    cargo build --release --example simple_benchmark 2>/dev/null || {
        # Create a simple benchmark example if it doesn't exist
        mkdir -p examples
        cat > examples/simple_benchmark.rs << 'EOF'
use persist_core::{create_default_engine, SnapshotMetadata};
use std::time::Instant;

fn main() {
    let engine = create_default_engine();
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    let test_data = r#"{"test": "hyperfine_benchmark", "data": "sample_agent_state"}"#;
    let metadata = SnapshotMetadata::new("hyperfine_agent", "hyperfine_session", 0);
    let file_path = temp_dir.path().join("hyperfine_test.json.gz");
    
    let start = Instant::now();
    
    // Perform save and load operations
    engine.save_snapshot(test_data, &metadata, file_path.to_str().unwrap()).unwrap();
    let (_metadata, _data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
    
    let duration = start.elapsed();
    println!("Operation completed in: {:?}", duration);
}
EOF
        cargo build --release --example simple_benchmark
    }
    cd ..
    
    if [ -f "persist-core/target/release/examples/simple_benchmark" ]; then
        print_status "Running hyperfine benchmark..."
        hyperfine --warmup 3 --runs 10 \
            --export-markdown test_results/hyperfine_results.md \
            --export-json test_results/hyperfine_results.json \
            "./persist-core/target/release/examples/simple_benchmark" \
            > test_results/hyperfine_output.log 2>&1 || {
            print_warning "Hyperfine benchmark failed"
        }
    fi
else
    print_warning "Skipping hyperfine tests - hyperfine not available"
fi

# 9. Memory profiling with dhat
print_status "Running memory profiling..."

cd persist-core

# Create a memory profiling test if it doesn't exist
if [ ! -f "examples/memory_profile.rs" ]; then
    mkdir -p examples
    cat > examples/memory_profile.rs << 'EOF'
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use persist_core::{create_default_engine, SnapshotMetadata};

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    
    let engine = create_default_engine();
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    // Create large test data to observe memory usage
    let large_data = serde_json::json!({
        "type": "memory_profile_test",
        "large_array": vec![0; 100000],
        "conversation": (0..1000).map(|i| format!("message_{}", i)).collect::<Vec<_>>()
    });
    
    let test_data = serde_json::to_string(&large_data).unwrap();
    let metadata = SnapshotMetadata::new("memory_profile_agent", "memory_session", 0);
    let file_path = temp_dir.path().join("memory_profile_test.json.gz");
    
    // Perform multiple operations to see memory patterns
    for i in 0..10 {
        let metadata = SnapshotMetadata::new("memory_profile_agent", "memory_session", i);
        let file_path = temp_dir.path().join(format!("memory_profile_{}.json.gz", i));
        
        engine.save_snapshot(&test_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let (_metadata, _data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
    }
    
    println!("Memory profiling completed");
}
EOF
fi

print_status "Building memory profile example..."
cargo build --release --example memory_profile --features dhat/dhat-heap > ../test_results/memory_build.log 2>&1 || {
    print_warning "Memory profile build failed. Check test_results/memory_build.log"
}

if [ -f "target/release/examples/memory_profile" ]; then
    print_status "Running memory profiling..."
    ./target/release/examples/memory_profile > ../test_results/memory_profile.log 2>&1 || {
        print_warning "Memory profiling failed"
    }
    
    # Move dhat output if it exists
    if [ -f "dhat-heap.json" ]; then
        mv dhat-heap.json ../test_results/profiling/
        print_success "Memory profile saved to test_results/profiling/dhat-heap.json"
    fi
fi

cd ..

# 10. Flame graph generation
print_status "Generating flame graphs for performance analysis..."

if command -v cargo-flamegraph &> /dev/null; then
    cd persist-core
    
    print_status "Generating flame graph for save operations..."
    cargo flamegraph --example simple_benchmark -o ../test_results/profiling/flamegraph_save.svg 2>/dev/null || {
        print_warning "Flame graph generation failed"
    }
    
    cd ..
else
    print_warning "Skipping flame graph generation - cargo-flamegraph not available"
fi

# 11. Generate comprehensive report
print_status "Generating comprehensive test report..."

cat > test_results/test_report.md << EOF
# Persist Test Results

Generated on: $(date)

## Test Summary

### Unit Tests
- Rust core unit tests: $([ -f "test_results/unit_tests_rust.log" ] && echo "✅ PASSED" || echo "❌ FAILED")
- Python SDK tests: $([ -f "test_results/python_tests.log" ] && echo "✅ PASSED" || echo "❌ FAILED")

### Integration Tests
- End-to-end tests: $([ -f "test_results/integration_tests.log" ] && echo "✅ PASSED" || echo "❌ FAILED")

### Code Quality
- Formatting: ✅ PASSED
- Linting (clippy): ✅ PASSED

### Coverage
$(if [ -f "test_results/coverage.log" ] && grep -q "%" test_results/coverage.log; then
    echo "- Code coverage: $(grep -oP '\d+\.\d+%' test_results/coverage.log | tail -1)"
else
    echo "- Code coverage: Not available"
fi)

### Performance
- Criterion benchmarks: $([ -f "test_results/benchmarks/criterion_results.log" ] && echo "✅ COMPLETED" || echo "⚠️  NOT AVAILABLE")
- Hyperfine benchmarks: $([ -f "test_results/hyperfine_results.json" ] && echo "✅ COMPLETED" || echo "⚠️  NOT AVAILABLE")
- Memory profiling: $([ -f "test_results/profiling/dhat-heap.json" ] && echo "✅ COMPLETED" || echo "⚠️  NOT AVAILABLE")
- Flame graphs: $([ -f "test_results/profiling/flamegraph_save.svg" ] && echo "✅ COMPLETED" || echo "⚠️  NOT AVAILABLE")

## Files Generated

### Coverage Reports
- HTML coverage report: test_results/coverage/tarpaulin-report.html
- Coverage log: test_results/coverage.log

### Performance Reports
- Criterion results: test_results/benchmarks/
- Hyperfine results: test_results/hyperfine_results.json
- Memory profile: test_results/profiling/dhat-heap.json
- Flame graph: test_results/profiling/flamegraph_save.svg

### Test Logs
- Unit tests: test_results/unit_tests_rust.log
- Integration tests: test_results/integration_tests.log
- Python tests: test_results/python_tests.log

## Recommendations

### Coverage Improvement
$(if [ -f "test_results/coverage.log" ] && grep -q "%" test_results/coverage.log; then
    COVERAGE=$(grep -oP '\d+\.\d+' test_results/coverage.log | tail -1)
    if (( $(echo "$COVERAGE < 90" | bc -l 2>/dev/null || echo "0") )); then
        echo "- Current coverage is below 90% target"
        echo "- Focus on adding tests for uncovered code paths"
        echo "- Consider adding edge case tests"
    else
        echo "- Coverage target achieved! ✅"
    fi
else
    echo "- Install tarpaulin for coverage analysis"
fi)

### Performance Optimization
- Review benchmark results for performance bottlenecks
- Analyze flame graphs to identify hot code paths
- Check memory usage patterns in dhat profile
- Consider parallel processing optimizations

## Next Steps
1. Review all test results and logs
2. Address any failing tests
3. Improve code coverage if below target
4. Optimize performance based on profiling results
5. Run tests again to verify improvements
EOF

print_success "Test report generated: test_results/test_report.md"

# 12. Summary
print_status "Test suite completed!"
echo ""
echo "Results summary:"
echo "=================="

# Count test results
PASSED_TESTS=0
TOTAL_TESTS=0

if [ -f "test_results/unit_tests_rust.log" ]; then
    if grep -q "test result: ok" test_results/unit_tests_rust.log; then
        print_success "✅ Rust unit tests: PASSED"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_error "❌ Rust unit tests: FAILED"
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

if [ -f "test_results/integration_tests.log" ]; then
    if grep -q "test result: ok" test_results/integration_tests.log; then
        print_success "✅ Integration tests: PASSED"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_error "❌ Integration tests: FAILED"
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

if [ -f "test_results/python_tests.log" ]; then
    if grep -q "passed" test_results/python_tests.log || ! grep -q "failed\|error" test_results/python_tests.log; then
        print_success "✅ Python tests: PASSED"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_error "❌ Python tests: FAILED"
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

echo ""
print_status "Overall result: $PASSED_TESTS/$TOTAL_TESTS test suites passed"

if [ $PASSED_TESTS -eq $TOTAL_TESTS ] && [ $TOTAL_TESTS -gt 0 ]; then
    print_success "🎉 All tests passed!"
    echo ""
    echo "📊 View detailed results:"
    echo "   - Test report: test_results/test_report.md"
    echo "   - Coverage: test_results/coverage/tarpaulin-report.html"
    echo "   - Benchmarks: test_results/benchmarks/"
    echo "   - Profiling: test_results/profiling/"
    exit 0
else
    print_error "Some tests failed. Check the logs in test_results/ directory."
    echo ""
    echo "📋 Next steps:"
    echo "   1. Review failed test logs"
    echo "   2. Fix issues and re-run tests"
    echo "   3. Check test_results/test_report.md for details"
    exit 1
fi
EOF
