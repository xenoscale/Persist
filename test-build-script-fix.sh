#!/bin/bash

# Test script to validate the build-and-test.sh fix for Python test module import issue
# This script tests different scenarios to ensure the fix works correctly

set -e

echo "=== Testing build-and-test.sh Python Test Fix ==="
echo ""

# Test 1: Simulate maturin not available
echo "Test 1: Testing behavior when maturin is not available"
echo "-------------------------------------------------------"

# Create a temporary PATH without maturin 
export OLD_PATH="$PATH"
export PATH="/bin:/usr/bin:/usr/local/bin"

# Remove maturin from PATH if it exists
if command -v maturin >/dev/null 2>&1; then
    echo "Maturin found in PATH, removing it for test"
    MATURIN_PATH=$(which maturin)
    MATURIN_DIR=$(dirname "$MATURIN_PATH")
    export PATH=$(echo "$PATH" | sed "s|:$MATURIN_DIR||g" | sed "s|$MATURIN_DIR:||g" | sed "s|$MATURIN_DIR||g")
fi

# Verify maturin is not available
if ! command -v maturin >/dev/null 2>&1; then
    echo "✅ Maturin correctly not available"
else
    echo "❌ Failed to remove maturin from PATH"
    exit 1
fi

# Run build script with maturin unavailable
echo "Running build-and-test.sh --quick --skip-lint --skip-format..."
if source $HOME/.cargo/env 2>/dev/null && ./build-and-test.sh --quick --skip-lint --skip-format 2>&1 | grep -q "Python extension not built, skipping Python tests"; then
    echo "✅ Test 1 PASSED: Script correctly skips Python tests when maturin unavailable"
else
    echo "❌ Test 1 FAILED: Script did not properly handle missing maturin"
    exit 1
fi

echo ""

# Test 2: Verify the script doesn't crash with import errors
echo "Test 2: Ensuring no Python import errors occur"
echo "-----------------------------------------------"

# Check that the script completes successfully
if source $HOME/.cargo/env 2>/dev/null && ./build-and-test.sh --quick --skip-lint --skip-format >/dev/null 2>&1; then
    echo "✅ Test 2 PASSED: Script completes without crashing"
else
    echo "❌ Test 2 FAILED: Script crashed or returned error"
    exit 1
fi

echo ""

# Restore PATH
export PATH="$OLD_PATH"

# Test 3: Check the correct messages are shown
echo "Test 3: Verifying correct user messages"
echo "---------------------------------------"

# Run the script and capture output
if source $HOME/.cargo/env 2>/dev/null; then
    OUTPUT=$(./build-and-test.sh --quick --skip-lint --skip-format 2>&1)
    
    # Check for expected messages
    if echo "$OUTPUT" | grep -q "Python extension not built, skipping Python tests"; then
        echo "✅ Correct warning message shown"
    else
        echo "❌ Missing expected warning message"
        exit 1
    fi
    
    if echo "$OUTPUT" | grep -q "The persist module is required for Python tests"; then
        echo "✅ Correct informational message shown"
    else
        echo "❌ Missing expected informational message"
        exit 1
    fi
    
    if echo "$OUTPUT" | grep -q "⚠️  Python extension (persist-python) - skipped (maturin not available)"; then
        echo "✅ Correct summary status shown"
    else
        echo "❌ Missing expected summary status"
        exit 1
    fi
    
    if echo "$OUTPUT" | grep -q "⚠️  Python tests - skipped (extension not built)"; then
        echo "✅ Correct test status shown"
    else
        echo "❌ Missing expected test status"
        exit 1
    fi
fi

echo ""
echo "=== All Tests PASSED! ==="
echo ""
echo "Summary of fix:"
echo "- Script now checks if Python extension was built before running Python tests"
echo "- Clear warning messages are shown when maturin is not available"
echo "- Script completes successfully instead of crashing with import errors"
echo "- Summary section accurately reflects the actual build status"
echo ""
echo "The original issue has been resolved:"
echo "- No more 'ModuleNotFoundError: No module named persist' errors"
echo "- Users get clear guidance on what tools they need to install"
echo "- Build process is more robust and user-friendly"
