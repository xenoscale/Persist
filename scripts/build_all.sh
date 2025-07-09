#!/bin/bash

# Build All - Simple wrapper for complete build and test automation
# This script provides a simple entry point for building and testing everything

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to check if we're in the right directory
check_directory() {
    if [ ! -f "../Cargo.toml" ] || [ ! -d "../persist-core" ]; then
        print_warning "Not in scripts directory of Persist repository"
        print_info "Trying to find repository root..."
        
        # Try to find repository root
        if [ -f "Cargo.toml" ] && [ -d "persist-core" ]; then
            print_info "Found repository root in current directory"
            return 0
        elif [ -f "../Cargo.toml" ] && [ -d "../persist-core" ]; then
            print_info "Found repository root in parent directory"
            cd ..
            return 0
        else
            echo "Error: Could not find Persist repository root"
            echo "This script should be run from the repository root or scripts/ directory"
            exit 1
        fi
    else
        # We're in scripts directory, go to root
        cd ..
    fi
}

# Function to show available options
show_options() {
    echo "Build All - Complete Build and Test Automation"
    echo ""
    echo "Available automation options:"
    echo ""
    echo "1. Complete Automation Script:"
    echo "   ./build-and-test.sh               # Full build and test"
    echo "   ./build-and-test.sh --quick       # Quick development mode"
    echo "   ./build-and-test.sh --help        # Show all options"
    echo ""
    echo "2. Makefile Targets:"
    echo "   make all                          # Complete pipeline"
    echo "   make quick                        # Quick development cycle"
    echo "   make build test                   # Build then test"
    echo "   make help                         # Show all targets"
    echo ""
    echo "3. Individual Scripts:"
    echo "   ./scripts/format.sh               # Format code"
    echo "   ./scripts/lint.sh                 # Lint code"
    echo "   ./scripts/test.sh                 # Run tests"
    echo ""
    echo "Running default: Complete automation script..."
}

# Main function
main() {
    check_directory
    
    print_info "Starting automated build and test process..."
    
    # Show options if --help is requested
    if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
        show_options
        exit 0
    fi
    
    # Check if build-and-test.sh exists
    if [ ! -f "build-and-test.sh" ]; then
        echo "Error: build-and-test.sh not found in repository root"
        echo "Please ensure you're running this from the correct directory"
        exit 1
    fi
    
    # Make script executable if possible
    chmod +x build-and-test.sh 2>/dev/null || true
    
    # Run the main automation script
    print_info "Executing: ./build-and-test.sh $*"
    ./build-and-test.sh "$@"
    
    print_success "Build and test automation completed!"
    print_info "Check output above for any warnings or errors"
}

# Show available options first if no arguments
if [ $# -eq 0 ]; then
    show_options
    echo ""
    echo "Proceeding with default automation..."
    echo ""
fi

# Execute main function
main "$@"
