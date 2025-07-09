/*!
Comprehensive tests for metadata functionality including edge cases and error conditions.
*/

#[cfg(test)]
mod tests {
    use crate::metadata::SnapshotMetadata;

    #[test]
    fn test_metadata_creation() {
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 42);

        assert_eq!(metadata.agent_id, "test_agent");
        assert_eq!(metadata.session_id, "test_session");
        assert_eq!(metadata.snapshot_index, 42);
        assert_eq!(metadata.format_version, 1);

        // Timestamp should be close to current time
        let current_time = chrono::Utc::now();
        let time_diff = (current_time - metadata.timestamp).num_seconds().abs();
        assert!(time_diff <= 5); // Allow 5 seconds difference
    }

    #[test]
    fn test_metadata_with_hash() {
        let mut metadata = SnapshotMetadata::new("agent", "session", 0);
        metadata.content_hash = "abcd1234efgh5678".to_string();

        assert_eq!(metadata.content_hash, "abcd1234efgh5678");
    }

    #[test]
    fn test_metadata_serialization_roundtrip() {
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 123);

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: SnapshotMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.agent_id, deserialized.agent_id);
        assert_eq!(metadata.session_id, deserialized.session_id);
        assert_eq!(metadata.snapshot_index, deserialized.snapshot_index);
        assert_eq!(metadata.timestamp, deserialized.timestamp);
        assert_eq!(metadata.format_version, deserialized.format_version);
    }

    #[test]
    fn test_metadata_validation() {
        // Test valid metadata
        let metadata = SnapshotMetadata::with_all_fields(
            "valid_agent",
            "valid_session",
            0,
            "sha256hash",
            "gzip",
            1024,
        );
        assert!(metadata.validate().is_ok());

        // Test empty agent_id
        let metadata = SnapshotMetadata::with_all_fields("", "session", 0, "hash", "gzip", 1024);
        assert!(metadata.validate().is_err());

        // Test empty session_id
        let metadata = SnapshotMetadata::with_all_fields("agent", "", 0, "hash", "gzip", 1024);
        assert!(metadata.validate().is_err());
    }

    #[test]
    fn test_metadata_edge_cases() {
        // Test very long IDs
        let long_id = "a".repeat(1000);
        let metadata = SnapshotMetadata::new(&long_id, &long_id, u64::MAX);
        assert_eq!(metadata.agent_id, long_id);
        assert_eq!(metadata.session_id, long_id);
        assert_eq!(metadata.snapshot_index, u64::MAX);
    }

    #[test]
    fn test_metadata_special_characters() {
        let special_agent = "agent-with-special!@#$%^&*()_+{}|:<>?[]\\;'\",./ chars";
        let special_session = "session_with_unicode_🚀_🎯_🔥";

        let metadata = SnapshotMetadata::new(special_agent, special_session, 1);

        // Should serialize and deserialize correctly
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: SnapshotMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.agent_id, deserialized.agent_id);
        assert_eq!(metadata.session_id, deserialized.session_id);
    }

    #[test]
    fn test_metadata_hash_validation() {
        let mut metadata = SnapshotMetadata::new("agent", "session", 0);

        // Valid SHA-256 hash (64 hex characters)
        let valid_hash = "a".repeat(64);
        metadata.content_hash = valid_hash;
        assert!(metadata.validate().is_ok());

        // Invalid hash - empty hash should fail validation
        metadata.content_hash = String::new();
        assert!(metadata.validate().is_err());

        // Set a valid hash for further testing
        metadata.content_hash = "abcdef1234567890".to_string();
        assert!(metadata.validate().is_ok());
    }

    #[test]
    fn test_metadata_builder_pattern() {
        let metadata = SnapshotMetadata::new("agent", "session", 0);

        // Test that metadata fields can be modified
        let mut metadata_copy = metadata.clone();
        metadata_copy.content_hash = "new_hash".to_string();
        assert_ne!(metadata.content_hash, metadata_copy.content_hash);
    }

    #[test]
    fn test_metadata_comparison() {
        let metadata1 = SnapshotMetadata::new("agent", "session", 0);
        let metadata2 = SnapshotMetadata::new("agent", "session", 0);
        let metadata3 = SnapshotMetadata::new("agent", "session", 1);

        // Same metadata (except timestamp) should be equal in key fields
        assert_eq!(metadata1.agent_id, metadata2.agent_id);
        assert_eq!(metadata1.session_id, metadata2.session_id);
        assert_eq!(metadata1.snapshot_index, metadata2.snapshot_index);

        // Different index should be different
        assert_ne!(metadata1.snapshot_index, metadata3.snapshot_index);
    }

    #[test]
    fn test_metadata_format_version_compatibility() {
        let metadata = SnapshotMetadata::new("agent", "session", 0);

        // Current format version should be 1
        assert_eq!(metadata.format_version, 1);

        // Test compatibility check
        assert!(metadata.is_compatible());
    }

    #[test]
    fn test_metadata_default_values() {
        let metadata = SnapshotMetadata::new("agent", "session", 0);

        // Hash should be empty by default
        assert_eq!(metadata.content_hash, "");

        // Timestamp should be set
        assert!(
            metadata.timestamp > chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap()
        );
    }

    #[test]
    fn test_metadata_json_structure() {
        let mut metadata = SnapshotMetadata::new("test_agent", "test_session", 5);
        metadata.content_hash = "test_hash".to_string();

        let json = serde_json::to_string_pretty(&metadata).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("agent_id"));
        assert!(json.contains("session_id"));
        assert!(json.contains("snapshot_index"));
        assert!(json.contains("timestamp"));
        assert!(json.contains("content_hash"));
        assert!(json.contains("format_version"));
        assert!(json.contains("test_agent"));
        assert!(json.contains("test_session"));
        assert!(json.contains("test_hash"));
    }

    #[test]
    fn test_metadata_concurrent_creation() {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;
        use std::thread;

        let counter = Arc::new(AtomicU64::new(0));
        let mut handles = vec![];

        // Create metadata in multiple threads
        for i in 0..10 {
            let counter_clone = Arc::clone(&counter);
            let handle = thread::spawn(move || {
                let metadata =
                    SnapshotMetadata::new(format!("agent_{i}"), format!("session_{i}"), i as u64);
                counter_clone.fetch_add(1, Ordering::SeqCst);
                metadata
            });
            handles.push(handle);
        }

        let mut metadatas = vec![];
        for handle in handles {
            metadatas.push(handle.join().unwrap());
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
        assert_eq!(metadatas.len(), 10);

        // Verify all metadatas are unique
        for (i, metadata) in metadatas.iter().enumerate() {
            assert_eq!(metadata.agent_id, format!("agent_{i}"));
            assert_eq!(metadata.session_id, format!("session_{i}"));
            assert_eq!(metadata.snapshot_index, i as u64);
        }
    }
}
