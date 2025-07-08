/*!
Main snapshot engine that orchestrates the snapshot and restore operations.

This module contains the core business logic for creating and restoring snapshots,
orchestrating the metadata, compression, and storage components.
*/

use crate::{
    compression::CompressionAdapter, storage::StorageAdapter, PersistError, Result,
    SnapshotMetadata,
};
use serde_json;

/// Container for the complete snapshot data (metadata + agent state)
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SnapshotContainer {
    metadata: SnapshotMetadata,
    agent_state: serde_json::Value,
}

/// Main engine for snapshot and restore operations
///
/// This is the primary interface for the core functionality. It orchestrates
/// the metadata generation, compression, and storage operations while maintaining
/// the hexagonal architecture principles.
///
/// # Example
/// ```rust
/// use persist_core::{SnapshotEngine, SnapshotMetadata, LocalFileStorage, GzipCompressor};
///
/// let storage = LocalFileStorage::new();
/// let compressor = GzipCompressor::new();
/// let engine = SnapshotEngine::new(storage, compressor);
///
/// let metadata = SnapshotMetadata::new("agent_1", "session_1", 0);
/// let agent_json = r#"{"type": "langchain_agent", "state": "..."}"#;
///
/// // Save snapshot
/// engine.save_snapshot(agent_json, &metadata, "/path/to/snapshot.json.gz")?;
///
/// // Restore snapshot
/// let (metadata, agent_data) = engine.load_snapshot("/path/to/snapshot.json.gz")?;
/// ```
pub struct SnapshotEngine<S, C>
where
    S: StorageAdapter,
    C: CompressionAdapter,
{
    storage: S,
    compressor: C,
}

impl<S, C> SnapshotEngine<S, C>
where
    S: StorageAdapter,
    C: CompressionAdapter,
{
    /// Create a new snapshot engine with the specified storage and compression adapters
    ///
    /// # Arguments
    /// * `storage` - The storage adapter to use for saving/loading snapshots
    /// * `compressor` - The compression adapter to use for compressing/decompressing data
    pub fn new(storage: S, compressor: C) -> Self {
        Self {
            storage,
            compressor,
        }
    }

    /// Save an agent snapshot to storage
    ///
    /// This method:
    /// 1. Parses and validates the agent JSON data
    /// 2. Normalizes the JSON to ensure consistent formatting
    /// 3. Computes the content hash and updates metadata
    /// 4. Creates a snapshot container with metadata and agent state
    /// 5. Serializes the container to JSON
    /// 6. Compresses the JSON data
    /// 7. Saves the compressed data using the storage adapter
    ///
    /// # Arguments
    /// * `agent_json` - JSON string representation of the agent state
    /// * `metadata` - Snapshot metadata (will be updated with hash and size info)
    /// * `path` - Storage path where the snapshot should be saved
    ///
    /// # Returns
    /// Updated metadata with computed hash and compression info, or an error
    ///
    /// # Errors
    /// * `PersistError::Json` - If the agent JSON is invalid
    /// * `PersistError::Compression` - If compression fails
    /// * `PersistError::Storage` - If saving to storage fails
    pub fn save_snapshot(
        &self,
        agent_json: &str,
        metadata: &SnapshotMetadata,
        path: &str,
    ) -> Result<SnapshotMetadata> {
        // Parse and validate the agent JSON
        let agent_state: serde_json::Value =
            serde_json::from_str(agent_json).map_err(PersistError::Json)?;

        // Normalize the JSON to ensure consistent hash computation across save/load cycles
        let normalized_agent_json =
            serde_json::to_string(&agent_state).map_err(PersistError::Json)?;

        // Update metadata with content hash and size information (using normalized JSON)
        let agent_bytes = normalized_agent_json.as_bytes();
        let mut updated_metadata = metadata
            .clone()
            .with_content_hash(agent_bytes)
            .with_compression_algorithm(self.compressor.algorithm_name());

        // Validate metadata
        updated_metadata.validate()?;

        // Create the snapshot container
        let container = SnapshotContainer {
            metadata: updated_metadata.clone(),
            agent_state,
        };

        // Serialize the container to JSON
        let container_json = serde_json::to_string(&container).map_err(PersistError::Json)?;

        // Compress the JSON data
        let compressed_data = self.compressor.compress(container_json.as_bytes())?;

        // Update metadata with compressed size
        updated_metadata = updated_metadata.with_compressed_size(compressed_data.len());

        // Save to storage
        self.storage
            .save(&compressed_data, path)
            .map_err(|e| PersistError::Storage(format!("Failed to save snapshot: {e}")))?;

        Ok(updated_metadata)
    }

    /// Load an agent snapshot from storage
    ///
    /// This method:
    /// 1. Loads the compressed data from storage
    /// 2. Decompresses the data
    /// 3. Deserializes the JSON to extract metadata and agent state
    /// 4. Validates the metadata format compatibility
    /// 5. Verifies the integrity using the stored hash
    /// 6. Returns the metadata and agent JSON string
    ///
    /// # Arguments
    /// * `path` - Storage path where the snapshot is located
    ///
    /// # Returns
    /// Tuple of (metadata, agent_json_string) or an error
    ///
    /// # Errors
    /// * `PersistError::Storage` - If loading from storage fails
    /// * `PersistError::Compression` - If decompression fails
    /// * `PersistError::Json` - If JSON parsing fails
    /// * `PersistError::InvalidFormat` - If the snapshot format is incompatible
    /// * `PersistError::IntegrityCheckFailed` - If the content hash doesn't match
    pub fn load_snapshot(&self, path: &str) -> Result<(SnapshotMetadata, String)> {
        // Load compressed data from storage
        let compressed_data = self
            .storage
            .load(path)
            .map_err(|e| PersistError::Storage(format!("Failed to load snapshot: {e}")))?;

        // Decompress the data
        let decompressed_data = self.compressor.decompress(&compressed_data)?;

        // Parse the JSON container
        let container_json = String::from_utf8(decompressed_data)
            .map_err(|e| PersistError::invalid_format(format!("Invalid UTF-8 in snapshot: {e}")))?;

        let container: SnapshotContainer =
            serde_json::from_str(&container_json).map_err(PersistError::Json)?;

        // Check format compatibility
        if !container.metadata.is_compatible() {
            return Err(PersistError::invalid_format(format!(
                "Incompatible snapshot format version: {} (current: {})",
                container.metadata.format_version,
                crate::metadata::METADATA_FORMAT_VERSION
            )));
        }

        // Convert agent state back to JSON string (normalized format)
        let agent_json =
            serde_json::to_string(&container.agent_state).map_err(PersistError::Json)?;

        // Verify integrity
        container.metadata.verify_integrity(agent_json.as_bytes())?;

        Ok((container.metadata, agent_json))
    }

    /// Check if a snapshot exists at the specified path
    ///
    /// # Arguments
    /// * `path` - Storage path to check
    ///
    /// # Returns
    /// True if the snapshot exists, false otherwise
    pub fn snapshot_exists(&self, path: &str) -> bool {
        self.storage.exists(path)
    }

    /// Delete a snapshot from storage
    ///
    /// # Arguments
    /// * `path` - Storage path of the snapshot to delete
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn delete_snapshot(&self, path: &str) -> Result<()> {
        self.storage
            .delete(path)
            .map_err(|e| PersistError::Storage(format!("Failed to delete snapshot: {e}")))
    }

    /// Get metadata from a snapshot without loading the full agent data
    ///
    /// This is useful for inspecting snapshot information without the overhead
    /// of deserializing the complete agent state.
    ///
    /// # Arguments
    /// * `path` - Storage path of the snapshot
    ///
    /// # Returns
    /// The snapshot metadata or an error
    pub fn get_snapshot_metadata(&self, path: &str) -> Result<SnapshotMetadata> {
        let (metadata, _) = self.load_snapshot(path)?;
        Ok(metadata)
    }

    /// Verify the integrity of a snapshot without fully loading it
    ///
    /// This method loads the snapshot and verifies that:
    /// - The file can be decompressed successfully
    /// - The JSON format is valid
    /// - The content hash matches the stored hash
    /// - The format version is compatible
    ///
    /// # Arguments
    /// * `path` - Storage path of the snapshot to verify
    ///
    /// # Returns
    /// Result indicating if the snapshot is valid
    pub fn verify_snapshot(&self, path: &str) -> Result<()> {
        let _ = self.load_snapshot(path)?;
        Ok(())
    }
}

/// Convenience function to create a snapshot engine with default components
///
/// Creates an engine with:
/// - Local file storage (no base directory)
/// - Gzip compression with default level
///
/// # Example
/// ```rust
/// use persist_core::create_default_engine;
///
/// let engine = create_default_engine();
/// ```
pub fn create_default_engine(
) -> SnapshotEngine<crate::storage::local::LocalFileStorage, crate::compression::GzipCompressor> {
    SnapshotEngine::new(
        crate::storage::local::LocalFileStorage::new(),
        crate::compression::GzipCompressor::new(),
    )
}

/// Convenience function to create a snapshot engine with S3 storage
///
/// Creates an engine with:
/// - Amazon S3 storage for the specified bucket
/// - Gzip compression with default level
///
/// # Arguments
/// * `bucket` - The S3 bucket name to use for storage
///
/// # Returns
/// A snapshot engine configured for S3 storage or an error if S3 initialization fails
///
/// # Example
/// ```rust,no_run
/// use persist_core::create_s3_engine;
///
/// // Ensure AWS credentials are set in environment:
/// // export AWS_ACCESS_KEY_ID=your_access_key
/// // export AWS_SECRET_ACCESS_KEY=your_secret_key
/// // export AWS_REGION=us-west-2
///
/// let engine = create_s3_engine("my-snapshots-bucket".to_string())?;
/// # Ok::<(), persist_core::PersistError>(())
/// ```
pub fn create_s3_engine(
    bucket: String,
) -> Result<SnapshotEngine<crate::storage::S3StorageAdapter, crate::compression::GzipCompressor>> {
    let storage = crate::storage::S3StorageAdapter::new(bucket)?;
    Ok(SnapshotEngine::new(
        storage,
        crate::compression::GzipCompressor::new(),
    ))
}

/// Create a snapshot engine based on storage configuration
///
/// This function provides a unified interface for creating engines with different
/// storage backends based on configuration. It automatically selects the appropriate
/// storage adapter (Local or S3) based on the provided StorageConfig.
///
/// # Arguments
/// * `config` - Storage configuration specifying backend and parameters
///
/// # Returns
/// A boxed storage adapter that can be used with any snapshot engine
///
/// # Example
/// ```rust,no_run
/// use persist_core::{StorageConfig, create_engine_from_config};
///
/// // Local storage
/// let local_config = StorageConfig::default_local();
/// let engine = create_engine_from_config(local_config)?;
///
/// // S3 storage
/// let s3_config = StorageConfig::s3_with_bucket("my-bucket".to_string());
/// let engine = create_engine_from_config(s3_config)?;
/// # Ok::<(), persist_core::PersistError>(())
/// ```
pub fn create_engine_from_config(
    config: crate::config::StorageConfig,
) -> Result<Box<dyn SnapshotEngineInterface>> {
    use crate::config::StorageBackend;

    config.validate()?;

    match config.backend {
        StorageBackend::Local => {
            let storage = if let Some(base_path) = config.local_base_path {
                crate::storage::local::LocalFileStorage::with_base_dir(base_path)
            } else {
                crate::storage::local::LocalFileStorage::new()
            };
            let engine = SnapshotEngine::new(storage, crate::compression::GzipCompressor::new());
            Ok(Box::new(engine))
        }
        StorageBackend::S3 => {
            let bucket = config.s3_bucket.ok_or_else(|| {
                PersistError::validation("S3 bucket name is required for S3 backend")
            })?;
            let storage = crate::storage::S3StorageAdapter::new(bucket)?;
            let engine = SnapshotEngine::new(storage, crate::compression::GzipCompressor::new());
            Ok(Box::new(engine))
        }
    }
}

/// Trait for snapshot engine operations to enable dynamic dispatch
///
/// This trait allows using different storage and compression backends
/// through a common interface, enabling the create_engine_from_config function
/// to return engines with different concrete types.
pub trait SnapshotEngineInterface {
    fn save_snapshot(
        &self,
        agent_json: &str,
        metadata: &SnapshotMetadata,
        path: &str,
    ) -> Result<SnapshotMetadata>;
    fn load_snapshot(&self, path: &str) -> Result<(SnapshotMetadata, String)>;
    fn snapshot_exists(&self, path: &str) -> bool;
    fn delete_snapshot(&self, path: &str) -> Result<()>;
    fn get_snapshot_metadata(&self, path: &str) -> Result<SnapshotMetadata>;
    fn verify_snapshot(&self, path: &str) -> Result<()>;
}

impl<S, C> SnapshotEngineInterface for SnapshotEngine<S, C>
where
    S: StorageAdapter,
    C: CompressionAdapter,
{
    fn save_snapshot(
        &self,
        agent_json: &str,
        metadata: &SnapshotMetadata,
        path: &str,
    ) -> Result<SnapshotMetadata> {
        self.save_snapshot(agent_json, metadata, path)
    }

    fn load_snapshot(&self, path: &str) -> Result<(SnapshotMetadata, String)> {
        self.load_snapshot(path)
    }

    fn snapshot_exists(&self, path: &str) -> bool {
        self.snapshot_exists(path)
    }

    fn delete_snapshot(&self, path: &str) -> Result<()> {
        self.delete_snapshot(path)
    }

    fn get_snapshot_metadata(&self, path: &str) -> Result<SnapshotMetadata> {
        self.get_snapshot_metadata(path)
    }

    fn verify_snapshot(&self, path: &str) -> Result<()> {
        self.verify_snapshot(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compression::NoCompression, storage::MemoryStorage};

    fn create_test_engine() -> SnapshotEngine<MemoryStorage, NoCompression> {
        SnapshotEngine::new(MemoryStorage::new(), NoCompression::new())
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let engine = create_test_engine();

        let agent_json = r#"{"type": "test_agent", "memory": ["Hello", "World"], "tools": []}"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0)
            .with_description("Test snapshot");

        let path = "test_snapshot.json.gz";

        // Save snapshot
        let saved_metadata = engine.save_snapshot(agent_json, &metadata, path).unwrap();
        assert!(engine.snapshot_exists(path));
        assert_eq!(saved_metadata.agent_id, "test_agent");
        assert!(!saved_metadata.content_hash.is_empty());

        // Load snapshot
        let (loaded_metadata, loaded_agent_json) = engine.load_snapshot(path).unwrap();

        // Verify metadata matches
        assert_eq!(loaded_metadata.agent_id, saved_metadata.agent_id);
        assert_eq!(loaded_metadata.session_id, saved_metadata.session_id);
        assert_eq!(
            loaded_metadata.snapshot_index,
            saved_metadata.snapshot_index
        );
        assert_eq!(loaded_metadata.content_hash, saved_metadata.content_hash);

        // Verify agent data matches (JSON should be semantically equivalent)
        let original_value: serde_json::Value = serde_json::from_str(agent_json).unwrap();
        let loaded_value: serde_json::Value = serde_json::from_str(&loaded_agent_json).unwrap();
        assert_eq!(original_value, loaded_value);
    }

    #[test]
    fn test_snapshot_integrity_verification() {
        let engine = create_test_engine();

        let agent_json = r#"{"type": "test_agent"}"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0);
        let path = "test_snapshot.json.gz";

        // Save snapshot
        engine.save_snapshot(agent_json, &metadata, path).unwrap();

        // Verify snapshot
        assert!(engine.verify_snapshot(path).is_ok());

        // Load and verify integrity check works
        let (loaded_metadata, loaded_json) = engine.load_snapshot(path).unwrap();
        assert!(loaded_metadata
            .verify_integrity(loaded_json.as_bytes())
            .is_ok());
    }

    #[test]
    fn test_invalid_json() {
        let engine = create_test_engine();

        let invalid_json = r#"{"type": "test_agent", invalid json"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0);
        let path = "test_snapshot.json.gz";

        // Should fail to save invalid JSON
        let result = engine.save_snapshot(invalid_json, &metadata, path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PersistError::Json(_)));
    }

    #[test]
    fn test_snapshot_deletion() {
        let engine = create_test_engine();

        let agent_json = r#"{"type": "test_agent"}"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0);
        let path = "test_snapshot.json.gz";

        // Save snapshot
        engine.save_snapshot(agent_json, &metadata, path).unwrap();
        assert!(engine.snapshot_exists(path));

        // Delete snapshot
        engine.delete_snapshot(path).unwrap();
        assert!(!engine.snapshot_exists(path));
    }

    #[test]
    fn test_get_metadata_only() {
        let engine = create_test_engine();

        let agent_json = r#"{"type": "test_agent", "large_data": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"}"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 5)
            .with_description("Large snapshot");
        let path = "test_snapshot.json.gz";

        // Save snapshot
        let saved_metadata = engine.save_snapshot(agent_json, &metadata, path).unwrap();

        // Get metadata only
        let retrieved_metadata = engine.get_snapshot_metadata(path).unwrap();
        assert_eq!(retrieved_metadata.agent_id, saved_metadata.agent_id);
        assert_eq!(retrieved_metadata.snapshot_index, 5);
        assert_eq!(
            retrieved_metadata.description,
            Some("Large snapshot".to_string())
        );
    }

    #[test]
    fn test_with_real_compression() {
        use crate::compression::GzipCompressor;

        let engine = SnapshotEngine::new(MemoryStorage::new(), GzipCompressor::new());

        let agent_json = r#"{"type": "test_agent", "data": "repetitive data repetitive data repetitive data repetitive data repetitive data repetitive data repetitive data repetitive data"}"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0);
        let path = "compressed_snapshot.json.gz";

        // Save and load with compression
        let saved_metadata = engine.save_snapshot(agent_json, &metadata, path).unwrap();
        let (_loaded_metadata, loaded_json) = engine.load_snapshot(path).unwrap();

        // Verify compression worked (compressed size should be set)
        assert!(saved_metadata.compressed_size.is_some());
        assert_eq!(saved_metadata.compression_algorithm, "gzip");

        // Verify data integrity
        let original_value: serde_json::Value = serde_json::from_str(agent_json).unwrap();
        let loaded_value: serde_json::Value = serde_json::from_str(&loaded_json).unwrap();
        assert_eq!(original_value, loaded_value);
    }
}
