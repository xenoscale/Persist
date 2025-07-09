# Script Organization and Usage Guide

This document describes the organization and purpose of all scripts in the Persist repository.

## Root Level Scripts

### Development and Build Scripts
- **`build-and-test.sh`** - Main automated build and test script
  - Comprehensive build, test, and quality checks
  - Multiple modes: `--quick`, `--skip-tests`, `--skip-python`, etc.
  - Primary entry point for CI/CD and development workflows

- **`quick_test.sh`** - Fast basic functionality tests  
  - Lightweight testing without heavy dependencies
  - Good for quick validation during development
  - Tests compilation, basic unit tests, and examples

- **`validate-build-script.sh`** - Build script validation tool
  - Tests that build-and-test.sh works correctly under different scenarios
  - Validates behavior when dependencies are missing
  - Used for regression testing of the build system

### Setup and Configuration Scripts
- **`setup-dev-environment.sh`** - Development environment setup (Unix/Linux/macOS)
- **`setup-dev-environment.ps1`** - Development environment setup (Windows PowerShell)  
- **`setup-dev-tools`** - Development tools installer and validator

### Utility Scripts
- **`quick_ci_test.py`** - CI testing utilities

## Scripts Directory (`./scripts/`)

### Code Quality Scripts
- **`format.sh`** - Code formatting for Rust and Python
  - Uses `cargo fmt` for Rust
  - Uses `black` for Python (if available)

- **`lint.sh`** - Code linting and static analysis
  - Uses `cargo clippy` for Rust
  - Uses `ruff` and `mypy` for Python (if available)

### Testing Scripts  
- **`test.sh`** - Comprehensive test runner
  - Runs all Rust tests (unit, integration, doc)
  - Builds and tests Python extensions
  - Supports `--integration` and `--verbose` flags

- **`run_comprehensive_tests.sh`** - Advanced testing with coverage and benchmarks
  - Generates coverage reports using tarpaulin
  - Runs performance benchmarks
  - Creates detailed test reports

### Analysis Scripts
- **`performance_analysis.py`** - Performance analysis and benchmarking tools

## Script Usage Patterns

### For Daily Development
```bash
# Quick development cycle
./quick_test.sh                    # Fast basic tests
./build-and-test.sh --quick         # Quick build and test

# Code quality
./scripts/format.sh                 # Format code
./scripts/lint.sh                   # Lint code
```

### For Comprehensive Testing
```bash
# Full build and test
./build-and-test.sh                 # Complete pipeline

# Advanced testing
./scripts/run_comprehensive_tests.sh # Coverage + benchmarks
./scripts/test.sh --integration     # Include integration tests
```

### For CI/CD
```bash
# Recommended CI command
./build-and-test.sh --skip-format   # Skip formatting in CI
```

### For Setup
```bash
# First time setup
./setup-dev-environment.sh          # Unix/Linux/macOS
./setup-dev-environment.ps1         # Windows

# Install development tools
./setup-dev-tools                   # Check and install tools
```

## Script Permissions

All scripts have been configured with proper execute permissions:
- Shell scripts (`.sh`): `755` (rwxr-xr-x)
- Python scripts with shebang: `755` (rwxr-xr-x)
- Setup tools: `755` (rwxr-xr-x)

## Maintenance Notes

### Script Dependencies
- Most scripts automatically check for required tools
- Missing optional tools result in warnings, not failures
- Setup scripts help install missing dependencies

### Cross-Platform Compatibility
- Unix/Linux/macOS: Use `.sh` scripts
- Windows: Use `.ps1` scripts or WSL with `.sh` scripts
- All scripts designed to be robust across different environments

### Adding New Scripts
When adding new scripts:
1. Place in appropriate directory (`./` for primary tools, `./scripts/` for utilities)
2. Set execute permissions: `chmod +x script-name.sh`
3. Add proper shebang line: `#!/bin/bash` or `#!/usr/bin/env python3`
4. Update this documentation
5. Follow naming convention: `kebab-case.sh` or `snake_case.py`
