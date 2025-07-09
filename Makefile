# Persist Project - Build and Test Automation
# This Makefile provides convenient targets for building and testing the entire project

.PHONY: all build test clean install help format lint quick dev release docker docs
.DEFAULT_GOAL := help

# Configuration
CARGO_FLAGS ?= --all-features
PYTHON_DIR := persist-python
CLI_DIR := persist-cli

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m # No Color

define print_target
	@echo "$(CYAN)→ $(1)$(NC)"
endef

## Main targets

all: format lint build test  ## Run complete build and test pipeline (equivalent to ./build-and-test.sh)
	@echo "$(GREEN)✅ All targets completed successfully!$(NC)"

build: build-rust build-cli build-python  ## Build all components
	$(call print_target,"Building all components")

test: test-rust test-python test-docs  ## Run all tests
	$(call print_target,"Running all tests")

quick: format-check lint-quick build-quick test-quick  ## Quick build and test for development
	$(call print_target,"Quick development cycle completed")

dev: quick  ## Alias for quick - fast development cycle

release: format lint build-release test  ## Build release version with full testing
	$(call print_target,"Release build completed")

## Build targets

build-rust:  ## Build Rust components
	$(call print_target,"Building Rust components")
	cargo build $(CARGO_FLAGS)

build-cli:  ## Build CLI tool
	$(call print_target,"Building CLI tool")
	cargo build -p persist-cli $(CARGO_FLAGS)

build-python:  ## Build Python extension
	$(call print_target,"Building Python extension")
	@if [ -d "$(PYTHON_DIR)" ] && command -v maturin >/dev/null 2>&1; then \
		cd $(PYTHON_DIR) && maturin develop --release && cd ..; \
	else \
		echo "$(YELLOW)⚠️  Skipping Python build (missing maturin or persist-python)$(NC)"; \
	fi

build-release:  ## Build release version
	$(call print_target,"Building release version")
	cargo build --release $(CARGO_FLAGS)
	@if [ -d "$(CLI_DIR)" ]; then \
		cargo build -p persist-cli --release $(CARGO_FLAGS); \
	fi

build-quick:  ## Quick build (debug only)
	$(call print_target,"Quick build (debug)")
	cargo build

## Test targets

test-rust:  ## Run Rust tests
	$(call print_target,"Running Rust tests")
	cargo test $(CARGO_FLAGS)

test-python:  ## Run Python tests
	$(call print_target,"Running Python tests")
	@if [ -d "$(PYTHON_DIR)" ] && command -v pytest >/dev/null 2>&1; then \
		cd $(PYTHON_DIR) && pytest && cd ..; \
	else \
		echo "$(YELLOW)⚠️  Skipping Python tests (missing pytest or persist-python)$(NC)"; \
	fi

test-docs:  ## Run documentation tests
	$(call print_target,"Running documentation tests")
	cargo test --doc

test-quick:  ## Quick test run (unit tests only)
	$(call print_target,"Running quick tests")
	cargo test --lib

test-integration:  ## Run integration tests
	$(call print_target,"Running integration tests")
	./scripts/test.sh --integration

## Code quality targets

format:  ## Format all code
	$(call print_target,"Formatting code")
	./scripts/format.sh

format-check:  ## Check code formatting
	$(call print_target,"Checking code formatting")
	cargo fmt --all -- --check

lint:  ## Run linting (clippy)
	$(call print_target,"Running linting")
	./scripts/lint.sh

lint-quick:  ## Quick lint check
	$(call print_target,"Quick linting")
	cargo clippy --all-targets -- -D warnings

## Utility targets

clean:  ## Clean build artifacts
	$(call print_target,"Cleaning build artifacts")
	cargo clean
	@if [ -d "$(PYTHON_DIR)/target" ]; then rm -rf $(PYTHON_DIR)/target; fi
	@if [ -d "$(PYTHON_DIR)/build" ]; then rm -rf $(PYTHON_DIR)/build; fi
	@if [ -d "$(PYTHON_DIR)/*.egg-info" ]; then rm -rf $(PYTHON_DIR)/*.egg-info; fi

install:  ## Install the project locally
	$(call print_target,"Installing project")
	cargo install --path .
	@if [ -d "$(PYTHON_DIR)" ] && command -v maturin >/dev/null 2>&1; then \
		cd $(PYTHON_DIR) && maturin develop --release && cd ..; \
	fi

docs:  ## Generate documentation
	$(call print_target,"Generating documentation")
	cargo doc --all-features --no-deps --open

## Advanced targets

coverage:  ## Generate test coverage report
	$(call print_target,"Generating coverage report")
	@if command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo tarpaulin --all-features --workspace --timeout 120 --out Html; \
		echo "$(GREEN)Coverage report generated: tarpaulin-report.html$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  Install cargo-tarpaulin for coverage: cargo install cargo-tarpaulin$(NC)"; \
	fi

benchmark:  ## Run performance benchmarks
	$(call print_target,"Running benchmarks")
	cargo bench

flamegraph:  ## Generate performance flamegraph
	$(call print_target,"Generating flamegraph")
	@if command -v cargo-flamegraph >/dev/null 2>&1; then \
		cargo flamegraph --bench benchmark_name; \
	else \
		echo "$(YELLOW)⚠️  Install cargo-flamegraph: cargo install flamegraph$(NC)"; \
	fi

## Docker targets (if Dockerfile exists)

docker-build:  ## Build Docker image
	$(call print_target,"Building Docker image")
	@if [ -f "Dockerfile" ]; then \
		docker build -t persist:latest .; \
	else \
		echo "$(YELLOW)⚠️  Dockerfile not found$(NC)"; \
	fi

docker-test:  ## Test in Docker container
	$(call print_target,"Testing in Docker")
	@if [ -f "Dockerfile" ]; then \
		docker run --rm persist:latest make test; \
	else \
		echo "$(YELLOW)⚠️  Dockerfile not found$(NC)"; \
	fi

## Script delegation targets

automated-build:  ## Run the automated build script
	$(call print_target,"Running automated build script")
	bash build-and-test.sh

automated-build-quick:  ## Run automated build in quick mode
	$(call print_target,"Running automated build (quick mode)")
	bash build-and-test.sh --quick

automated-build-verbose:  ## Run automated build with verbose output
	$(call print_target,"Running automated build (verbose)")
	bash build-and-test.sh --verbose

## Help target

help:  ## Show this help message
	@echo "$(CYAN)Persist Project - Build and Test Automation$(NC)"
	@echo ""
	@echo "$(GREEN)Usage:$(NC)"
	@echo "  make [target]"
	@echo ""
	@echo "$(GREEN)Main Targets:$(NC)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(CYAN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -E "(all|build|test|quick|dev|release):"
	@echo ""
	@echo "$(GREEN)Development Targets:$(NC)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(CYAN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -E "(build-|test-|format|lint|clean|install):"
	@echo ""
	@echo "$(GREEN)Advanced Targets:$(NC)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(CYAN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -E "(coverage|benchmark|flamegraph|docker-|automated-):"
	@echo ""
	@echo "$(GREEN)Examples:$(NC)"
	@echo "  make              # Show this help"
	@echo "  make all          # Complete build and test pipeline"
	@echo "  make quick        # Fast development cycle"
	@echo "  make build test   # Build then test"
	@echo "  make clean build  # Clean build"
	@echo ""
	@echo "$(GREEN)Alternative:$(NC)"
	@echo "  ./build-and-test.sh           # Full automation script"
	@echo "  ./build-and-test.sh --quick   # Quick mode"
	@echo "  ./build-and-test.sh --help    # Script help"

# Advanced make features
.SILENT: help

# Ensure bash is used for shell commands
SHELL := /bin/bash

# Check if we're in the right directory
ifeq (,$(wildcard Cargo.toml))
    $(error This Makefile must be run from the root of the Persist repository)
endif
