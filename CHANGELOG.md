# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Google Cloud Storage Hardening**: Production-ready GCS backend with comprehensive features
  - Streaming uploads and downloads for large snapshots
  - Bucket validation on initialization with fail-fast error handling
  - Exponential backoff with jitter for retry logic
  - Advanced error classification and mapping (404→NotFound, 403→Forbidden, 5xx→Transient)
  - Configurable prefix support for snapshot organization
  - Transfer size metrics and enhanced observability
  - KMS encryption support via `PERSIST_GCS_KMS_KEY` environment variable
  - PII scrubbing in logs (unless RUST_LOG=debug)
  - CLI integration with environment variable support
- Production readiness improvements
- Feature flags for optional components (S3, metrics, CLI)
- Command-line interface (CLI) for snapshot inspection and management
- Comprehensive repository documentation (CONTRIBUTING.md, DEVELOPMENT.md)
- Environment configuration template (.env.example)
- Enhanced error handling with detailed context
- Type hints and improved Python SDK ergonomics
- Pre-commit hooks configuration
- Utility scripts for development workflow
- Enhanced CI/CD workflows with comprehensive testing

### Changed
- **BREAKING**: Removed "metrics" from default features - users need to explicitly enable `metrics` feature for Prometheus support
- Improved error mapping between Rust and Python
- Enhanced observability with structured logging and metrics
- Standardized project structure and naming conventions
- Updated dependencies for security and performance

### Fixed
- Resolved protobuf security vulnerability (RUSTSEC-2024-0437)
- Improved async safety and performance
- Enhanced test coverage and reliability

## [0.1.0] - 2025-07-08

### Added
- Initial MVP release
- Core snapshot and restore functionality
- Support for local filesystem storage
- AWS S3 storage backend
- Python SDK with PyO3 bindings
- LangChain integration
- Compression with gzip
- Metadata management with integrity verification
- Basic observability with tracing and Prometheus metrics
- GitHub Actions CI/CD pipelines
- Basic testing infrastructure

### Core Features
- Hexagonal architecture with pluggable storage adapters
- Rich metadata with SHA-256 integrity verification
- Efficient compression and decompression
- Thread-safe operations with async support
- Comprehensive error handling

### Storage Backends
- Local filesystem storage
- AWS S3 storage with retry logic and error handling
- Memory storage for testing

### Python Integration
- PyO3-based Python bindings
- LangChain agent serialization support
- Python exception mapping
- Type-safe interfaces

### Observability
- Structured logging with tracing
- Prometheus metrics for monitoring
- Performance benchmarking
- Debug logging for troubleshooting

[Unreleased]: https://github.com/xenoscale/Persist/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/xenoscale/Persist/releases/tag/v0.1.0
