# Persist Architecture

This document provides a comprehensive overview of the Persist system architecture, design principles, and component interactions.

## Overview

Persist is designed as an enterprise-grade agent snapshot and restore system with a focus on:
- **Reliability**: Robust error handling and data integrity
- **Performance**: Efficient compression and async operations
- **Extensibility**: Pluggable architecture for storage and compression
- **Multi-language Support**: Rust core with Python bindings

## High-Level Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Python SDK    │    │       CLI       │    │  Future SDKs    │
│   (PyO3 FFI)    │    │   (Rust bin)    │    │  (Go, Node.js)  │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────┴──────────────┐
                    │      Persist Core          │
                    │     (Rust Library)         │
                    └─────────────┬──────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
    ┌─────┴─────┐           ┌─────┴─────┐           ┌─────┴─────┐
    │  Storage  │           │Compression│           │Observability│
    │ Adapters  │           │ Adapters  │           │   Layer   │
    └───────────┘           └───────────┘           └───────────┘
          │                       │                       │
    ┌─────┴─────┐           ┌─────┴─────┐           ┌─────┴─────┐
    │Local│ S3  │           │Gzip│Future│           │Logs│Metrics│
    │Disk │Cloud│           │    │ Algs │           │    │ & Traces│
    └─────┴─────┘           └─────┴─────┘           └─────┴─────┘
```

## Design Principles

### 1. Hexagonal Architecture (Ports & Adapters)

The core domain logic is isolated from infrastructure concerns:

- **Ports**: Abstract interfaces (traits) defining contracts
- **Adapters**: Concrete implementations of ports for specific technologies
- **Domain**: Core business logic independent of external systems

This separation enables:
- Easy testing with mock implementations
- Technology agnostic core logic
- Simple addition of new storage backends or compression algorithms

### 2. Language Separation

- **Rust Core**: Performance-critical operations, memory safety, type safety
- **Language Bindings**: Ergonomic APIs for different languages
- **FFI Boundaries**: Clean interfaces between languages with proper error mapping

## Core Components

### Snapshot Engine

The central orchestrator that coordinates all snapshot operations:

```rust
pub struct SnapshotEngine<S, C> 
where
    S: StorageAdapter,
    C: CompressionAdapter,
{
    storage: S,
    compressor: C,
}
```

**Responsibilities:**
- Orchestrate save/load operations
- Coordinate storage and compression adapters
- Manage metadata and integrity verification
- Handle error propagation and recovery

### Storage Layer

Abstracted through the `StorageAdapter` trait:

```rust
pub trait StorageAdapter {
    fn save(&self, data: &[u8], path: &str) -> Result<()>;
    fn load(&self, path: &str) -> Result<Vec<u8>>;
    fn exists(&self, path: &str) -> bool;
    fn delete(&self, path: &str) -> Result<()>;
}
```

**Current Implementations:**
- **LocalFileStorage**: Direct filesystem operations
- **S3StorageAdapter**: AWS S3 with retry logic and error handling
- **MemoryStorage**: In-memory storage for testing

**Future Extensions:**
- Azure Blob Storage
- Google Cloud Storage
- Database storage (PostgreSQL, MongoDB)

### Compression Layer

Abstracted through the `CompressionAdapter` trait:

```rust
pub trait CompressionAdapter {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
}
```

**Current Implementation:**
- **GzipCompressor**: Standard gzip compression

**Future Extensions:**
- Zstandard (zstd) for better compression ratios
- LZ4 for faster compression/decompression
- Brotli for web-optimized compression

### Metadata Management

Rich metadata with integrity verification:

```rust
pub struct SnapshotMetadata {
    agent_id: String,
    session_id: String,
    snapshot_index: u64,
    timestamp: i64,
    content_hash: String,
    format_version: u8,
    description: Option<String>,
}
```

**Features:**
- SHA-256 integrity verification
- Version compatibility tracking
- Hierarchical organization (agent → session → snapshots)
- Extensible metadata fields

### Error Handling

Comprehensive error handling with `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum PersistError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },
    
    // ... other variants
}
```

**Benefits:**
- Rich error context and chaining
- Type-safe error handling
- Easy mapping to language-specific exceptions

## Data Flow

### Save Operation

1. **Input Validation**: Verify agent data and metadata
2. **Serialization**: Convert agent state to JSON
3. **Hashing**: Calculate SHA-256 hash for integrity
4. **Metadata Creation**: Package metadata with integrity hash
5. **Compression**: Apply compression algorithm
6. **Storage**: Write to configured storage backend
7. **Verification**: Optional integrity check

### Load Operation

1. **Storage Retrieval**: Read data from storage backend
2. **Decompression**: Decompress snapshot data
3. **Parsing**: Extract metadata and agent state
4. **Integrity Check**: Verify SHA-256 hash matches
5. **Deserialization**: Convert JSON back to agent state
6. **Return**: Provide metadata and agent state to caller

## Performance Characteristics

### Compression

| Algorithm | Ratio | Speed | Use Case |
|-----------|-------|-------|----------|
| Gzip | Good | Moderate | General purpose (current) |
| Zstd | Better | Fast | Future: High throughput |
| LZ4 | Lower | Very Fast | Future: Real-time scenarios |

### Storage Backends

| Backend | Latency | Throughput | Durability | Scalability |
|---------|---------|------------|------------|-------------|
| Local Disk | Very Low | High | Medium | Limited |
| AWS S3 | Low-Medium | Very High | Very High | Unlimited |
| Memory | Minimal | Very High | None | Limited |

### Async Operations

- **Non-blocking I/O**: All storage operations use async/await
- **Concurrent Operations**: Multiple snapshots can be processed simultaneously
- **Backpressure Handling**: Built-in retry logic with exponential backoff

## Observability

### Logging

Structured logging with `tracing`:
- **Spans**: Track operation lifecycles
- **Events**: Record important occurrences
- **Context**: Carry request IDs and metadata
- **Levels**: Configurable verbosity (error, warn, info, debug, trace)

### Metrics

Prometheus metrics for monitoring:
- **Operation Counters**: Success/failure rates
- **Latency Histograms**: Performance tracking
- **Size Distributions**: Snapshot size analytics
- **Error Rates**: Error categorization and trends

### Distributed Tracing

OpenTelemetry integration (future):
- **Request Tracing**: End-to-end operation tracking
- **Service Maps**: Understand system interactions
- **Performance Analysis**: Identify bottlenecks

## Security Considerations

### Data Integrity

- **SHA-256 Hashing**: Cryptographic verification of data integrity
- **Format Versioning**: Protect against format tampering
- **Error Detection**: Immediate detection of corruption

### Access Control

- **Storage-Level Security**: Leverage backend security (S3 IAM, filesystem permissions)
- **Encryption at Rest**: Backend-provided encryption
- **Encryption in Transit**: TLS for S3, no network for local storage

### Privacy

- **No Credential Storage**: Rely on environment/external credential providers
- **Minimal Metadata**: Only essential information is stored
- **Audit Trails**: Comprehensive logging for security analysis

## Extension Points

### Adding New Storage Backends

1. Implement `StorageAdapter` trait
2. Add feature flag for optional inclusion
3. Implement configuration support
4. Add comprehensive test suite
5. Update CLI support
6. Document configuration and usage

### Adding New Compression Algorithms

1. Implement `CompressionAdapter` trait
2. Add algorithm-specific dependencies
3. Benchmark against existing algorithms
4. Add configuration options
5. Update documentation

### Adding New Language Bindings

1. Choose appropriate FFI technology (PyO3, cbindgen, etc.)
2. Design idiomatic API for target language
3. Implement proper error mapping
4. Create comprehensive test suite
5. Package for language-specific distribution

## Future Architecture Enhancements

### Microservices Architecture

For large-scale deployments:
- **Snapshot Service**: Core snapshot operations
- **Metadata Service**: Metadata management and search
- **Storage Service**: Abstract storage operations
- **Authentication Service**: Centralized access control

### Caching Layer

For improved performance:
- **Redis Integration**: Fast metadata lookups
- **CDN Support**: Geographically distributed access
- **Local Caching**: Reduce repeated storage access

### Stream Processing

For real-time scenarios:
- **Event Streaming**: Real-time snapshot notifications
- **Batch Processing**: Bulk operations on snapshots
- **Analytics Pipeline**: Derive insights from usage patterns

## Conclusion

The Persist architecture is designed for:
- **Flexibility**: Easy extension and customization
- **Performance**: Efficient operations at scale
- **Reliability**: Robust error handling and data integrity
- **Maintainability**: Clean separation of concerns

This foundation supports current requirements while providing a clear path for future enhancements and integrations.
