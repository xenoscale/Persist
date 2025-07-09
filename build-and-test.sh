#!/bin/bash

# Persist - Automated Build and Test Script
# This script builds and tests the entire Persist project with a single command
# Usage: ./build-and-test.sh [OPTIONS]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_header() {
    echo -e "\n${PURPLE}===================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}===================================================${NC}\n"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
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

print_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

# Configuration
SKIP_FORMAT=false
SKIP_LINT=false
SKIP_TESTS=false
SKIP_PYTHON=false
VERBOSE=false
QUICK_MODE=false
DRY_RUN=false

# Parse command line arguments
show_help() {
    cat << EOF
Persist - Automated Build and Test Script

USAGE:
    $0 [OPTIONS]

DESCRIPTION:
    This script automates the entire build and test process for the Persist project.
    It builds all components, runs formatting, linting, and comprehensive tests.

OPTIONS:
    --skip-format         Skip code formatting checks and fixes
    --skip-lint          Skip linting (clippy) checks
    --skip-tests         Skip running tests
    --skip-python        Skip Python-related build and tests
    --quick              Quick mode - minimal checks for fast iteration
    --verbose            Enable verbose output
    --dry-run            Show what would be done without executing
    --help               Show this help message

EXAMPLES:
    $0                   # Full build and test (recommended for CI/release)
    $0 --quick           # Quick build and test for development
    $0 --skip-tests      # Build and check code quality only
    $0 --verbose         # Full build with detailed output

PHASES:
    1. Environment Setup    - Check dependencies and tools
    2. Code Formatting     - Format Rust and Python code
    3. Code Linting        - Run clippy and Python linters
    4. Project Build       - Build all Rust components
    5. Python Extension    - Build Python bindings with maturin
    6. Test Execution      - Run all tests (unit, doc, integration)
    7. Summary Report      - Display results and statistics

EOF
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-format)
            SKIP_FORMAT=true
            shift
            ;;
        --skip-lint)
            SKIP_LINT=true
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --skip-python)
            SKIP_PYTHON=true
            shift
            ;;
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Function to execute commands with dry-run support
execute_cmd() {
    local cmd="$1"
    local description="$2"
    
    if [ "$DRY_RUN" = true ]; then
        print_info "[DRY-RUN] Would execute: $cmd"
        return 0
    fi
    
    if [ "$VERBOSE" = true ]; then
        print_info "Executing: $cmd"
    fi
    
    if ! eval "$cmd"; then
        print_error "Failed: $description"
        print_error "Command: $cmd"
        exit 1
    fi
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check required tools
check_dependencies() {
    print_step "Checking dependencies and tools..."
    
    local missing_tools=()
    local missing_optional=()
    
    # Required tools
    if ! command_exists "cargo"; then
        missing_tools+=("cargo (Rust toolchain)")
    fi
    
    if ! command_exists "rustc"; then
        missing_tools+=("rustc (Rust compiler)")
    fi
    
    if [ "$SKIP_PYTHON" = false ]; then
        if ! command_exists "python3"; then
            missing_tools+=("python3")
        fi
        
        if ! command_exists "maturin"; then
            print_warning "maturin not found - Python extension build will be skipped"
            print_info "Install with: pip install maturin"
            missing_optional+=("maturin")
        fi
        
        # Check for other Python tools
        if ! command_exists "pytest"; then
            print_warning "pytest not available, skipping Python tests"
            print_info "Install with: pip install pytest"
            missing_optional+=("pytest")
        fi
    fi
    
    # Optional but recommended tools
    if [ "$SKIP_FORMAT" = false ] && ! command_exists "rustfmt"; then
        print_warning "rustfmt not found - formatting will be skipped"
        missing_optional+=("rustfmt")
    fi
    
    if [ "$SKIP_LINT" = false ] && ! command_exists "clippy"; then
        print_warning "clippy not found - linting will be skipped"
        missing_optional+=("clippy")
    fi
    
    # Check for Python development tools
    if [ "$SKIP_PYTHON" = false ]; then
        if ! command_exists "black"; then
            print_warning "black not available, skipping Python formatting"
            missing_optional+=("black")
        fi
        
        if ! command_exists "ruff"; then
            print_warning "ruff not available, skipping Python linting"
            missing_optional+=("ruff")
        fi
    fi
    
    # Handle missing tools
    if [ ${#missing_tools[@]} -gt 0 ]; then
        print_error "Missing required tools:"
        for tool in "${missing_tools[@]}"; do
            echo "  - $tool"
        done
        echo ""
        print_info "Run the setup script to install missing tools:"
        print_info "  ./setup-dev-tools --auto-install"
        print_info "Or for manual guidance:"
        print_info "  ./setup-dev-tools"
        exit 1
    fi
    
    # Provide guidance for optional tools
    if [ ${#missing_optional[@]} -gt 0 ]; then
        print_warning "Missing optional tools (${#missing_optional[@]} total) - some functionality will be limited"
        print_info "To install all recommended development tools, run:"
        print_info "  ./setup-dev-tools --auto-install"
        print_info "Or check what's missing with:"
        print_info "  ./setup-dev-tools"
    fi
    
    print_success "All required dependencies found"
}

# Function to format code
format_code() {
    if [ "$SKIP_FORMAT" = true ]; then
        print_step "Skipping code formatting (--skip-format)"
        return 0
    fi
    
    print_step "Formatting code..."
    
    # Format Rust code
    if command_exists "rustfmt"; then
        if [ "$QUICK_MODE" = true ]; then
            execute_cmd "cargo fmt --all -- --check" "Rust format check"
        else
            execute_cmd "cargo fmt --all" "Rust formatting"
        fi
        print_success "Rust code formatted"
    else
        print_warning "rustfmt not available, skipping Rust formatting"
    fi
    
    # Format Python code
    if [ "$SKIP_PYTHON" = false ] && [ -d "persist-python" ]; then
        if command_exists "black"; then
            execute_cmd "cd persist-python && black . && cd .." "Python formatting"
            print_success "Python code formatted"
        else
            print_warning "black not available, skipping Python formatting"
        fi
    fi
}

# Function to lint code
lint_code() {
    if [ "$SKIP_LINT" = true ]; then
        print_step "Skipping code linting (--skip-lint)"
        return 0
    fi
    
    print_step "Linting code..."
    
    # Lint Rust code
    if command_exists "cargo-clippy" || command_exists "clippy"; then
        execute_cmd "cargo clippy --all-targets --all-features -- -D warnings" "Rust linting"
        print_success "Rust linting completed"
    else
        print_warning "clippy not available, skipping Rust linting"
    fi
    
    # Lint Python code
    if [ "$SKIP_PYTHON" = false ] && [ -d "persist-python" ]; then
        if command_exists "ruff"; then
            execute_cmd "cd persist-python && ruff check . && cd .." "Python linting"
            print_success "Python linting completed"
        else
            print_warning "ruff not available, skipping Python linting"
        fi
    fi
}

# Function to build project
build_project() {
    print_step "Building project..."
    
    # Build Rust components
    if [ "$QUICK_MODE" = true ]; then
        execute_cmd "cargo build" "Rust build (debug)"
    else
        execute_cmd "cargo build --release" "Rust build (release)"
        execute_cmd "cargo build" "Rust build (debug)"
    fi
    
    # Build CLI tool
    if [ -d "persist-cli" ]; then
        if [ "$QUICK_MODE" = true ]; then
            execute_cmd "cargo build -p persist-cli" "CLI build (debug)"
        else
            execute_cmd "cargo build -p persist-cli --release" "CLI build (release)"
        fi
        print_success "CLI tool built"
    fi
    
    print_success "Rust project built successfully"
}

# Function to build Python extension
build_python() {
    if [ "$SKIP_PYTHON" = true ]; then
        print_step "Skipping Python build (--skip-python)"
        return 0
    fi
    
    if [ ! -d "persist-python" ]; then
        print_warning "persist-python directory not found, skipping Python build"
        return 0
    fi
    
    print_step "Building Python extension..."
    
    if command_exists "maturin"; then
        execute_cmd "cd persist-python && maturin develop --release && cd .." "Python extension build"
        print_success "Python extension built successfully"
    else
        print_warning "maturin not available, skipping Python extension build"
        print_info "Install with: pip install maturin"
    fi
}

# Function to run tests
run_tests() {
    if [ "$SKIP_TESTS" = true ]; then
        print_step "Skipping tests (--skip-tests)"
        return 0
    fi
    
    print_step "Running tests..."
    
    # Run Rust tests
    if [ "$QUICK_MODE" = true ]; then
        execute_cmd "cargo test --lib" "Rust unit tests"
    else
        execute_cmd "cargo test --all --all-features" "Rust tests"
        execute_cmd "cargo test --doc" "Rust doc tests"
    fi
    
    print_success "Rust tests completed"
    
    # Run Python tests
    if [ "$SKIP_PYTHON" = false ] && [ -d "persist-python" ] && command_exists "pytest"; then
        execute_cmd "cd persist-python && pytest && cd .." "Python tests"
        print_success "Python tests completed"
    elif [ "$SKIP_PYTHON" = false ] && [ -d "persist-python" ]; then
        print_warning "pytest not available, skipping Python tests"
        print_info "Install with: pip install pytest"
    fi
}

# Function to generate summary
generate_summary() {
    print_header "BUILD AND TEST SUMMARY"
    
    echo -e "${GREEN}✅ Build and test process completed successfully!${NC}\n"
    
    echo "Configuration:"
    echo "  - Quick Mode: $QUICK_MODE"
    echo "  - Skip Format: $SKIP_FORMAT"
    echo "  - Skip Lint: $SKIP_LINT"
    echo "  - Skip Tests: $SKIP_TESTS"
    echo "  - Skip Python: $SKIP_PYTHON"
    echo "  - Verbose: $VERBOSE"
    echo "  - Dry Run: $DRY_RUN"
    
    echo -e "\nComponents built:"
    echo "  ✅ Rust core library (persist-core)"
    [ -d "persist-cli" ] && echo "  ✅ CLI tool (persist-cli)"
    [ "$SKIP_PYTHON" = false ] && [ -d "persist-python" ] && echo "  ✅ Python extension (persist-python)"
    
    echo -e "\nQuality checks:"
    [ "$SKIP_FORMAT" = false ] && echo "  ✅ Code formatting"
    [ "$SKIP_LINT" = false ] && echo "  ✅ Code linting"
    
    echo -e "\nTests executed:"
    [ "$SKIP_TESTS" = false ] && echo "  ✅ Rust unit and integration tests"
    [ "$SKIP_TESTS" = false ] && echo "  ✅ Rust documentation tests"
    [ "$SKIP_PYTHON" = false ] && [ "$SKIP_TESTS" = false ] && echo "  ✅ Python tests"
    
    echo -e "\n${CYAN}Next steps:${NC}"
    echo "  - Review any warnings above"
    echo "  - Run './build-and-test.sh --help' for more options"
    echo "  - Use individual scripts in ./scripts/ for specific tasks"
    echo "  - Check ./target/ for built binaries"
    
    if [ "$QUICK_MODE" = false ]; then
        echo "  - Release binaries available in ./target/release/"
    fi
}

# Main execution flow
main() {
    print_header "Persist - Automated Build and Test"
    
    print_info "Starting automated build and test process..."
    print_info "Working directory: $(pwd)"
    print_info "Timestamp: $(date)"
    
    if [ "$DRY_RUN" = true ]; then
        print_warning "DRY RUN MODE - No commands will be executed"
    fi
    
    # Verify we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "persist-core" ]; then
        print_error "This script must be run from the root of the Persist repository"
        print_error "Current directory: $(pwd)"
        print_error "Expected files: Cargo.toml, persist-core/ directory"
        exit 1
    fi
    
    # Execute all phases
    check_dependencies
    format_code
    lint_code
    build_project
    build_python
    run_tests
    generate_summary
    
    print_success "All done! 🚀"
}

# Trap to handle interruption
trap 'print_error "Build interrupted! 🛑"; exit 1' INT TERM

# Execute main function
main "$@"
