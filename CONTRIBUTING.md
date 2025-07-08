# Contributing to Persist

Thank you for your interest in contributing to Persist! This document provides guidelines and instructions for contributors.

## Project Vision

Persist is an enterprise-grade agent snapshot and restore system designed to:
- Provide reliable state persistence for AI agents
- Support multiple storage backends (local, S3, etc.)
- Maintain high performance with minimal overhead
- Follow hexagonal architecture principles for extensibility

## Getting Started

### Prerequisites

- **Rust**: Latest stable version (install via [rustup](https://rustup.rs/))
- **Python**: 3.8+ with pip
- **Git**: For version control
- **Docker**: Optional, for integration testing with LocalStack

### Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/xenoscale/Persist.git
   cd Persist
   ```

2. **Set up Rust toolchain**:
   ```bash
   rustup install stable
   rustup default stable
   rustup component add rustfmt clippy
   ```

3. **Set up Python environment** (optional, for Python SDK development):
   ```bash
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   pip install -r requirements-dev.txt
   ```

4. **Build the project**:
   ```bash
   cargo build --all
   ```

5. **Run tests**:
   ```bash
   # Rust tests
   cargo test --all
   
   # Python tests (if Python SDK development)
   cd persist-python
   maturin develop --release
   pytest
   ```

## Development Workflow

### 1. Code Style and Formatting

We enforce consistent code style through automated tools:

- **Rust**: `rustfmt` and `clippy`
- **Python**: `black` and `ruff`

Before submitting a PR, ensure your code passes:

```bash
# Format code
cargo fmt --all
black persist-python/

# Lint code
cargo clippy --all-targets --all-features -- -D warnings
ruff check persist-python/
```

### 2. Testing

All contributions must include appropriate tests:

- **Unit tests**: For individual functions and components
- **Integration tests**: For end-to-end functionality
- **Documentation tests**: Ensure code examples in docs work

Run the full test suite:
```bash
cargo test --all --all-features
```

### 3. Documentation

- All public APIs must have documentation comments
- Include usage examples where appropriate
- Update relevant documentation files for significant changes

Generate and review docs:
```bash
cargo doc --open --all-features
```

## Contributing Process

### 1. Issues and Feature Requests

- Check existing issues before creating new ones
- Use issue templates when available
- Provide clear reproduction steps for bugs
- Describe the use case and benefits for feature requests

### 2. Pull Requests

1. **Fork the repository** and create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the guidelines above

3. **Test your changes** thoroughly

4. **Update documentation** as needed

5. **Submit a pull request** with:
   - Clear description of changes
   - Reference to related issues
   - Test coverage information

### 3. Review Process

- All PRs require review from maintainers
- CI checks must pass (formatting, linting, tests)
- Address review feedback promptly
- Maintain a clean commit history

## Architecture Guidelines

### Rust Core

- Follow hexagonal architecture principles
- Use trait-based design for extensibility
- Implement proper error handling with `thiserror`
- Include comprehensive logging with `tracing`
- Write idiomatic Rust code

### Python SDK

- Provide Pythonic APIs that feel natural
- Map Rust errors to appropriate Python exceptions
- Include type hints and docstrings
- Follow PEP 8 style guidelines

### Storage Adapters

When adding new storage backends:

1. Implement the `StorageAdapter` trait
2. Add appropriate feature flags
3. Include comprehensive tests
4. Document configuration requirements
5. Add CLI support if applicable

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help newcomers and answer questions
- Maintain a professional tone in all interactions

## Getting Help

- **Issues**: For bugs and feature requests
- **Discussions**: For questions and general discussion
- **Documentation**: Check the `docs/` directory
- **Examples**: See `examples/` directory

## Recognition

All contributors will be acknowledged in the project. Significant contributions may be highlighted in release notes.

Thank you for contributing to Persist!
