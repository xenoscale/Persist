/*!
Snapshot metadata management and schema definition.
*/

use crate::{PersistError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Current metadata format version for compatibility tracking
pub const METADATA_FORMAT_VERSION: u8 = 1;

/// Comprehensive metadata for each snapshot providing traceability and integrity verification
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SnapshotMetadata {
    /// Unique identifier for the agent (e.g., agent name, UUID)
    pub agent_id: String,

    /// Identifier for the session or context (multiple sessions per agent)
    pub session_id: String,

    /// Sequence number of this snapshot within the session (0, 1, 2, ...)
    pub snapshot_index: u64,

    /// ISO 8601 timestamp when the snapshot was created
    pub timestamp: DateTime<Utc>,

    /// SHA-256 hash of the agent state payload for integrity verification
    pub content_hash: String,

    /// Format version for compatibility (current: 1)
    pub format_version: u8,

    /// Unique identifier for this specific snapshot
    pub snapshot_id: String,

    /// Optional human-readable description
    pub description: Option<String>,

    /// Size of the uncompressed agent data in bytes
    pub uncompressed_size: usize,

    /// Size of the compressed snapshot file in bytes
    pub compressed_size: Option<usize>,

    /// Compression algorithm used
    pub compression_algorithm: String,
}

impl SnapshotMetadata {
    /// Create new snapshot metadata with required fields
    ///
    /// # Arguments
    /// * `agent_id` - Unique identifier for the agent
    /// * `session_id` - Session identifier (use "default" if not using sessions)
    /// * `snapshot_index` - Sequence number for this snapshot
    ///
    /// # Example
    /// ```rust
    /// use persist_core::SnapshotMetadata;
    ///
    /// let metadata = SnapshotMetadata::new("agent_1", "session_1", 0);
    /// assert_eq!(metadata.agent_id, "agent_1");
    /// assert_eq!(metadata.snapshot_index, 0);
    /// ```
    pub fn new<S1, S2>(agent_id: S1, session_id: S2, snapshot_index: u64) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            snapshot_index,
            timestamp: Utc::now(),
            content_hash: String::new(), // Will be set when computing hash
            format_version: METADATA_FORMAT_VERSION,
            snapshot_id: Uuid::new_v4().to_string(),
            description: None,
            uncompressed_size: 0,  // Will be set when processing data
            compressed_size: None, // Will be set after compression
            compression_algorithm: "gzip".to_string(), // Default compression
        }
    }

    /// Create metadata with all fields specified (useful for testing or custom scenarios)
    pub fn with_all_fields<S1, S2, S3, S4>(
        agent_id: S1,
        session_id: S2,
        snapshot_index: u64,
        content_hash: S3,
        compression_algorithm: S4,
        uncompressed_size: usize,
    ) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
        S4: Into<String>,
    {
        Self {
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            snapshot_index,
            timestamp: Utc::now(),
            content_hash: content_hash.into(),
            format_version: METADATA_FORMAT_VERSION,
            snapshot_id: Uuid::new_v4().to_string(),
            description: None,
            uncompressed_size,
            compressed_size: None,
            compression_algorithm: compression_algorithm.into(),
        }
    }

    /// Set optional description for the snapshot
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the content hash from agent data
    ///
    /// # Arguments
    /// * `agent_data` - The serialized agent data as bytes
    ///
    /// # Returns
    /// Updated metadata with computed hash and uncompressed size
    pub fn with_content_hash(mut self, agent_data: &[u8]) -> Self {
        self.content_hash = Self::compute_hash(agent_data);
        self.uncompressed_size = agent_data.len();
        self
    }

    /// Set the compressed size after compression
    pub fn with_compressed_size(mut self, compressed_size: usize) -> Self {
        self.compressed_size = Some(compressed_size);
        self
    }

    /// Set the compression algorithm
    pub fn with_compression_algorithm<S: Into<String>>(mut self, algorithm: S) -> Self {
        self.compression_algorithm = algorithm.into();
        self
    }

    /// Compute SHA-256 hash of the provided data
    ///
    /// # Arguments
    /// * `data` - The data to hash
    ///
    /// # Returns
    /// Hexadecimal string representation of the SHA-256 hash
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Verify the integrity of agent data against the stored hash
    ///
    /// # Arguments
    /// * `agent_data` - The agent data to verify
    ///
    /// # Returns
    /// Ok(()) if the hash matches, Err(PersistError::IntegrityCheckFailed) otherwise
    pub fn verify_integrity(&self, agent_data: &[u8]) -> Result<()> {
        let computed_hash = Self::compute_hash(agent_data);
        if computed_hash == self.content_hash {
            Ok(())
        } else {
            Err(PersistError::IntegrityCheckFailed {
                expected: self.content_hash.clone(),
                actual: computed_hash,
            })
        }
    }

    /// Validate that all required fields are properly set
    pub fn validate(&self) -> Result<()> {
        if self.agent_id.is_empty() {
            return Err(PersistError::validation("agent_id cannot be empty"));
        }
        if self.session_id.is_empty() {
            return Err(PersistError::validation("session_id cannot be empty"));
        }
        if self.content_hash.is_empty() {
            return Err(PersistError::validation("content_hash cannot be empty"));
        }
        if self.snapshot_id.is_empty() {
            return Err(PersistError::validation("snapshot_id cannot be empty"));
        }
        Ok(())
    }

    /// Check if this metadata is compatible with the current format version
    pub fn is_compatible(&self) -> bool {
        self.format_version <= METADATA_FORMAT_VERSION
    }

    /// Generate a suggested filename for this snapshot
    ///
    /// Format: {agent_id}_{session_id}_{snapshot_index}_{timestamp}.json.gz
    pub fn suggested_filename(&self) -> String {
        let timestamp = self.timestamp.format("%Y%m%d_%H%M%S");
        format!(
            "{}_{}_{}_{}.json.gz",
            self.agent_id, self.session_id, self.snapshot_index, timestamp
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let metadata = SnapshotMetadata::new("agent_1", "session_1", 0);
        assert_eq!(metadata.agent_id, "agent_1");
        assert_eq!(metadata.session_id, "session_1");
        assert_eq!(metadata.snapshot_index, 0);
        assert_eq!(metadata.format_version, METADATA_FORMAT_VERSION);
        assert!(!metadata.snapshot_id.is_empty());
    }

    #[test]
    fn test_content_hash() {
        let data = b"test data";
        let hash = SnapshotMetadata::compute_hash(data);

        // SHA-256 of "test data" should be consistent
        assert_eq!(
            hash,
            "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9"
        );
    }

    #[test]
    fn test_integrity_verification() {
        let data = b"test data";
        let metadata = SnapshotMetadata::new("agent", "session", 0).with_content_hash(data);

        // Should pass with same data
        assert!(metadata.verify_integrity(data).is_ok());

        // Should fail with different data
        let different_data = b"different data";
        assert!(metadata.verify_integrity(different_data).is_err());
    }

    #[test]
    fn test_validation() {
        let mut metadata = SnapshotMetadata::new("agent", "session", 0);
        metadata.content_hash = "dummy_hash".to_string();

        // Should pass validation
        assert!(metadata.validate().is_ok());

        // Should fail with empty agent_id
        metadata.agent_id = String::new();
        assert!(metadata.validate().is_err());
    }

    #[test]
    fn test_suggested_filename() {
        let metadata = SnapshotMetadata::new("test_agent", "main_session", 5);
        let filename = metadata.suggested_filename();

        assert!(filename.contains("test_agent"));
        assert!(filename.contains("main_session"));
        assert!(filename.contains("5"));
        assert!(filename.ends_with(".json.gz"));
    }
}
