/*!
# Persist Core Engine

Enterprise-grade agent snapshot and restore system core library.

This crate provides the foundational functionality for snapshotting and restoring
AI agent states with support for:

- Hexagonal architecture with pluggable storage and compression adapters
- Rich metadata with integrity verification
- Efficient compression and decompression
- Local filesystem storage (extensible to cloud storage)

## Architecture

The core follows hexagonal architecture principles:
- Domain logic is isolated from infrastructure concerns
- Storage and compression are implemented as adapters
- Easy to extend with new storage backends or compression algorithms

## Usage

```rust,no_run
use persist_core::{SnapshotEngine, SnapshotMetadata, LocalFileStorage, GzipCompressor};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let storage = LocalFileStorage::new();
let compressor = GzipCompressor::new();
let engine = SnapshotEngine::new(storage, compressor);

let metadata = SnapshotMetadata::new("agent_1", "session_1", 0);
let agent_data = r#"{"type": "agent", "state": "..."}"#;

// Save snapshot
engine.save_snapshot(agent_data, &metadata, "/path/to/snapshot.json.gz")?;

// Restore snapshot
let (restored_metadata, restored_data) = engine.load_snapshot("/path/to/snapshot.json.gz")?;
# Ok(())
# }
```
*/

pub mod compression;
pub mod config;
pub mod error;
pub mod metadata;
#[cfg(test)]
mod metadata_tests;
pub mod observability;
pub mod snapshot;
pub mod storage;

pub use compression::{CompressionAdapter, GzipCompressor};
pub use config::{StorageBackend, StorageConfig};
pub use error::{PersistError, Result};
pub use metadata::SnapshotMetadata;

#[cfg(feature = "metrics")]
pub use observability::{
    init_default_observability, init_observability, MetricsTimer, PersistMetrics,
};

pub use snapshot::{
    create_default_engine, create_engine_from_config, SnapshotEngine, SnapshotEngineInterface,
};

#[cfg(feature = "s3")]
pub use snapshot::create_s3_engine;

#[cfg(feature = "gcs")]
pub use snapshot::create_gcs_engine;

pub use storage::{LocalFileStorage, StorageAdapter};

#[cfg(feature = "s3")]
pub use storage::S3StorageAdapter;

#[cfg(feature = "gcs")]
pub use storage::GCSStorageAdapter;
