# Development Guide

This guide provides detailed instructions for setting up a development environment and working with the Persist codebase.

## Prerequisites

### Required Software

- **Rust**: Latest stable version
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source ~/.cargo/env
  rustup component add rustfmt clippy
  ```

- **Python**: 3.8 or higher
  ```bash
  # Verify version
  python --version  # Should be 3.8+
  ```

- **Git**: For version control
- **maturin**: For Python package building
  ```bash
  pip install maturin
  ```

### Optional Software

- **Docker**: For integration testing with LocalStack (S3 emulation)
- **aws-cli**: For S3 testing with real AWS accounts

## Repository Structure

```
Persist/
├── persist-core/          # Rust core library
│   ├── src/              # Source code
│   ├── benches/          # Benchmarks
│   └── examples/         # Usage examples
├── persist-python/       # Python SDK (PyO3 bindings)
│   ├── src/              # PyO3 Rust code
│   └── tests/            # Python tests
├── persist-cli/          # Command-line interface
├── docs/                 # Documentation
├── scripts/              # Development scripts
└── tests/                # Integration tests
```

## Development Setup

### 1. Clone and Basic Setup

```bash
git clone https://github.com/xenoscale/Persist.git
cd Persist

# Verify Rust installation
cargo --version
rustc --version

# Build all crates
cargo build --all
```

### 2. Python Development Setup

```bash
# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install development dependencies
pip install -r requirements-dev.txt

# Build and install Python package in development mode
cd persist-python
maturin develop --release
cd ..
```

### 3. Environment Configuration

Create a `.env` file in the project root (use `.env.example` as template):

```bash
# Copy example environment file
cp .env.example .env

# Edit with your settings
editor .env
```

### 4. Feature Flags

The project uses feature flags to enable optional functionality:

- `s3`: AWS S3 storage support (default: enabled)
- `metrics`: Prometheus metrics (default: enabled)
- `cli`: CLI-specific features

Build with specific features:
```bash
# Build without S3 support
cargo build --no-default-features

# Build with all features
cargo build --all-features

# Build specific features
cargo build --features "s3,metrics"
```

## Development Workflow

### Building

```bash
# Build all crates
cargo build --all

# Build specific crate
cargo build -p persist-core

# Release build
cargo build --all --release
```

### Testing

#### Rust Tests

```bash
# Run all tests
cargo test --all

# Run tests for specific crate
cargo test -p persist-core

# Run tests with features
cargo test --all-features

# Run specific test
cargo test test_name

# Run integration tests only
cargo test --test integration_tests
```

#### Python Tests

```bash
cd persist-python

# Install in development mode
maturin develop

# Run Python tests
pytest

# Run with coverage
pytest --cov=persist

# Run specific test file
pytest tests/test_python_sdk.py
```

#### Integration Tests

For S3 integration tests, you can use LocalStack:

```bash
# Start LocalStack (if using Docker)
docker-compose up -d localstack

# Set environment for local S3
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1

# Run S3 integration tests
cargo test --test s3_integration
```

### Code Quality

#### Formatting

```bash
# Format Rust code
cargo fmt --all

# Format Python code
black persist-python/
```

#### Linting

```bash
# Rust linting
cargo clippy --all-targets --all-features -- -D warnings

# Python linting
ruff check persist-python/

# Type checking (Python)
mypy persist-python/
```

### Documentation

#### Generate Documentation

```bash
# Generate Rust docs
cargo doc --open --all-features

# Generate docs without opening
cargo doc --no-deps --all-features
```

#### Documentation Tests

```bash
# Test documentation examples
cargo test --doc
```

## Working with Features

### Adding a New Storage Backend

1. **Implement the trait**:
   ```rust
   // In persist-core/src/storage/my_backend.rs
   use crate::{Result, StorageAdapter};
   
   pub struct MyBackend {
       // fields
   }
   
   impl StorageAdapter for MyBackend {
       fn save(&self, data: &[u8], path: &str) -> Result<()> {
           // implementation
       }
       
       // other methods...
   }
   ```

2. **Add feature flag** (if optional):
   ```toml
   # In persist-core/Cargo.toml
   [features]
   my_backend = ["dep:my-backend-crate"]
   ```

3. **Add to module**:
   ```rust
   // In persist-core/src/storage/mod.rs
   #[cfg(feature = "my_backend")]
   pub mod my_backend;
   
   #[cfg(feature = "my_backend")]
   pub use my_backend::MyBackend;
   ```

4. **Write tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_my_backend() {
           // test implementation
       }
   }
   ```

### Adding CLI Commands

1. **Add to CLI enum**:
   ```rust
   // In persist-cli/src/main.rs
   #[derive(Subcommand)]
   enum Commands {
       // existing commands...
       MyCommand {
           #[arg(short, long)]
           option: String,
       },
   }
   ```

2. **Implement handler**:
   ```rust
   async fn handle_my_command(option: &str) -> Result<(), anyhow::Error> {
       // implementation
   }
   ```

3. **Add to match statement**:
   ```rust
   match cli.command {
       // existing cases...
       Commands::MyCommand { option } => handle_my_command(&option).await?,
   }
   ```

## Debugging

### Enabling Logs

```bash
# Enable debug logs
export RUST_LOG=debug
cargo run

# Enable specific module logs
export RUST_LOG=persist_core::storage=debug
```

### Using the CLI for Debugging

```bash
# Build CLI
cargo build -p persist-cli

# List snapshots
./target/debug/persist list --verbose

# Show snapshot details
./target/debug/persist show snapshot_id

# Verify snapshot integrity
./target/debug/persist verify snapshot_id
```

### Performance Profiling

```bash
# Run benchmarks
cargo bench

# Profile with perf (Linux)
cargo build --release
perf record --call-graph=dwarf ./target/release/your_binary
perf report
```

## Troubleshooting

### Common Issues

1. **Build errors with AWS SDK**:
   ```bash
   # Ensure you have the latest Rust version
   rustup update stable
   ```

2. **Python import errors**:
   ```bash
   # Rebuild Python extension
   cd persist-python
   maturin develop --release
   ```

3. **S3 connection timeouts**:
   ```bash
   # Check network connectivity and AWS credentials
   aws s3 ls  # Test with AWS CLI
   ```

### Getting Help

- Check existing issues on GitHub
- Read the documentation in `docs/`
- Run tests to ensure setup is correct
- Enable verbose logging for debugging

## Performance Considerations

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench snapshot_benchmarks
```

### Memory Usage

```bash
# Profile memory usage with dhat
cargo run --features dhat-heap --release
```

### Optimization Tips

- Use `--release` builds for performance testing
- Enable link-time optimization for production builds
- Profile with appropriate tools for your platform
- Consider feature flags to reduce binary size

## Release Process

### Version Updates

1. Update versions in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create git tag
4. Build and test release artifacts

### Publishing

```bash
# Publish Rust crate
cargo publish -p persist-core

# Build Python wheels
cd persist-python
maturin build --release
```

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [PyO3 Guide](https://pyo3.rs/)
- [AWS SDK for Rust](https://aws.amazon.com/sdk-for-rust/)
- [Project Architecture](docs/Architecture.md)
