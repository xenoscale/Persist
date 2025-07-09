/*!
Comprehensive tests for the snapshot engine including integration tests and performance scenarios.
*/

#[cfg(test)]
mod tests {
    use crate::snapshot::{SnapshotEngine, SnapshotEngineInterface, create_default_engine};
    use crate::metadata::SnapshotMetadata;
    use crate::storage::LocalFileStorage;
    use crate::compression::{GzipCompressor, NoCompression};
    use crate::error::PersistError;
    use tempfile::TempDir;
    use std::fs;
    use std::thread;
    use std::sync::Arc;
    use rayon::prelude::*;

    #[test]
    fn test_snapshot_engine_creation() {
        let storage = LocalFileStorage::new();
        let compressor = GzipCompressor::new();
        let engine = SnapshotEngine::new(storage, compressor);
        
        // Engine should be created successfully
        assert_eq!(std::mem::size_of_val(&engine), std::mem::size_of::<SnapshotEngine<LocalFileStorage, GzipCompressor>>());
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let compressor = GzipCompressor::new();
        let engine = SnapshotEngine::new(storage, compressor);
        
        let agent_data = r#"{"type": "test_agent", "memory": ["hello", "world"], "state": {"active": true}}"#;
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0);
        let file_path = temp_dir.path().join("test_snapshot.json.gz");
        
        // Save snapshot
        engine.save_snapshot(agent_data, &metadata, file_path.to_str().unwrap()).unwrap();
        
        // Verify file exists and is not empty
        assert!(file_path.exists());
        assert!(fs::metadata(&file_path).unwrap().len() > 0);
        
        // Load snapshot
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        // Verify data integrity
        assert_eq!(loaded_data, agent_data);
        assert_eq!(loaded_metadata.agent_id(), metadata.agent_id());
        assert_eq!(loaded_metadata.session_id(), metadata.session_id());
        assert_eq!(loaded_metadata.snapshot_index(), metadata.snapshot_index());
    }

    #[test]
    fn test_snapshot_with_different_compression() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test with Gzip compression
        let gzip_engine = SnapshotEngine::new(LocalFileStorage::new(), GzipCompressor::new());
        
        // Test with no compression
        let no_comp_engine = SnapshotEngine::new(LocalFileStorage::new(), NoCompression::new());
        
        let agent_data = "test data that could be compressed".repeat(100);
        let metadata = SnapshotMetadata::new("agent", "session", 0);
        
        let gzip_path = temp_dir.path().join("gzip_snapshot.json.gz");
        let no_comp_path = temp_dir.path().join("no_comp_snapshot.json");
        
        // Save with both engines
        gzip_engine.save_snapshot(&agent_data, &metadata, gzip_path.to_str().unwrap()).unwrap();
        no_comp_engine.save_snapshot(&agent_data, &metadata, no_comp_path.to_str().unwrap()).unwrap();
        
        // Load with both engines
        let (_, gzip_data) = gzip_engine.load_snapshot(gzip_path.to_str().unwrap()).unwrap();
        let (_, no_comp_data) = no_comp_engine.load_snapshot(no_comp_path.to_str().unwrap()).unwrap();
        
        // Data should be identical
        assert_eq!(gzip_data, agent_data);
        assert_eq!(no_comp_data, agent_data);
        assert_eq!(gzip_data, no_comp_data);
        
        // File sizes should be different (gzip should be smaller for repetitive data)
        let gzip_size = fs::metadata(&gzip_path).unwrap().len();
        let no_comp_size = fs::metadata(&no_comp_path).unwrap().len();
        assert!(gzip_size < no_comp_size);
    }

    #[test]
    fn test_snapshot_integrity_verification() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        let agent_data = r#"{"type": "agent", "data": "important_state"}"#;
        let metadata = SnapshotMetadata::new("agent", "session", 0);
        let file_path = temp_dir.path().join("integrity_test.json.gz");
        
        // Save snapshot
        engine.save_snapshot(agent_data, &metadata, file_path.to_str().unwrap()).unwrap();
        
        // Manually corrupt the file
        let mut file_content = fs::read(&file_path).unwrap();
        if !file_content.is_empty() {
            file_content[file_content.len() / 2] ^= 0xFF; // Flip some bits
            fs::write(&file_path, &file_content).unwrap();
        }
        
        // Loading should detect corruption
        let result = engine.load_snapshot(file_path.to_str().unwrap());
        assert!(result.is_err());
        
        match result.unwrap_err() {
            PersistError::IntegrityCheckFailed { .. } => {}, // Expected
            PersistError::Json(_) => {}, // Also acceptable (corrupted JSON)
            PersistError::Compression(_) => {}, // Also acceptable (corrupted compression)
            other => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn test_snapshot_large_data() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        // Create large JSON data (1MB+)
        let large_object = serde_json::json!({
            "type": "large_agent",
            "memory": vec!["large_data"; 10000],
            "history": (0..1000).map(|i| format!("interaction_{}", i)).collect::<Vec<_>>(),
            "state": {
                "embeddings": vec![0.5; 1000],
                "weights": vec![vec![0.1; 100]; 100]
            }
        });
        
        let agent_data = serde_json::to_string(&large_object).unwrap();
        let metadata = SnapshotMetadata::new("large_agent", "session", 0);
        let file_path = temp_dir.path().join("large_snapshot.json.gz");
        
        // Save and load large snapshot
        engine.save_snapshot(&agent_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, agent_data);
        assert_eq!(loaded_metadata.agent_id(), "large_agent");
        
        // Verify compression was effective
        let file_size = fs::metadata(&file_path).unwrap().len();
        let original_size = agent_data.len() as u64;
        assert!(file_size < original_size); // Should be compressed
    }

    #[test]
    fn test_snapshot_concurrent_operations() {
        let temp_dir = TempDir::new().unwrap();
        let engine = Arc::new(create_default_engine());
        let mut handles = vec![];
        
        // Perform concurrent snapshot operations
        for i in 0..10 {
            let engine_clone = Arc::clone(&engine);
            let temp_path = temp_dir.path().to_path_buf();
            
            let handle = thread::spawn(move || {
                let agent_data = format!(r#"{{"agent_id": {}, "data": "concurrent_test"}}"#, i);
                let metadata = SnapshotMetadata::new(&format!("agent_{}", i), "concurrent_session", i as u64);
                let file_path = temp_path.join(format!("concurrent_{}.json.gz", i));
                
                // Save snapshot
                engine_clone.save_snapshot(&agent_data, &metadata, file_path.to_str().unwrap()).unwrap();
                
                // Load snapshot
                let (loaded_metadata, loaded_data) = engine_clone.load_snapshot(file_path.to_str().unwrap()).unwrap();
                
                assert_eq!(loaded_data, agent_data);
                assert_eq!(loaded_metadata.agent_id(), format!("agent_{}", i));
                assert_eq!(loaded_metadata.snapshot_index(), i as u64);
            });
            
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all files were created
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        assert_eq!(entries.count(), 10);
    }

    #[test]
    fn test_snapshot_parallel_processing() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        // Create multiple snapshots using parallel processing
        let agents: Vec<_> = (0..20).map(|i| {
            (
                format!(r#"{{"agent_id": {}, "type": "parallel_agent"}}"#, i),
                SnapshotMetadata::new(&format!("parallel_agent_{}", i), "parallel_session", i as u64),
                temp_dir.path().join(format!("parallel_{}.json.gz", i))
            )
        }).collect();
        
        // Save snapshots in parallel using rayon
        agents.par_iter().for_each(|(data, metadata, path)| {
            engine.save_snapshot(data, metadata, path.to_str().unwrap()).unwrap();
        });
        
        // Load snapshots in parallel
        let results: Vec<_> = agents.par_iter().map(|(original_data, original_metadata, path)| {
            let (loaded_metadata, loaded_data) = engine.load_snapshot(path.to_str().unwrap()).unwrap();
            (loaded_data, loaded_metadata, original_data.clone(), original_metadata.clone())
        }).collect();
        
        // Verify all results
        for (loaded_data, loaded_metadata, original_data, original_metadata) in results {
            assert_eq!(loaded_data, original_data);
            assert_eq!(loaded_metadata.agent_id(), original_metadata.agent_id());
            assert_eq!(loaded_metadata.snapshot_index(), original_metadata.snapshot_index());
        }
    }

    #[test]
    fn test_snapshot_empty_data() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        let empty_data = "";
        let metadata = SnapshotMetadata::new("empty_agent", "session", 0);
        let file_path = temp_dir.path().join("empty_snapshot.json.gz");
        
        engine.save_snapshot(empty_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, empty_data);
        assert_eq!(loaded_metadata.agent_id(), "empty_agent");
    }

    #[test]
    fn test_snapshot_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        let special_data = r#"{"text": "Hello 🌍! Special chars: àáâãäåæçèéêë", "unicode": "🚀🎯🔥⭐"}"#;
        let metadata = SnapshotMetadata::new("unicode_agent", "unicode_session", 0);
        let file_path = temp_dir.path().join("unicode_snapshot.json.gz");
        
        engine.save_snapshot(special_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, special_data);
        assert_eq!(loaded_metadata.agent_id(), "unicode_agent");
    }

    #[test]
    fn test_snapshot_multiple_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        let agent_id = "multi_session_agent";
        let sessions = ["session_1", "session_2", "session_3"];
        
        // Create multiple snapshots for different sessions
        for (session_idx, session) in sessions.iter().enumerate() {
            for snapshot_idx in 0..3 {
                let data = format!(r#"{{"session": "{}", "snapshot": {}, "data": "test"}}"#, session, snapshot_idx);
                let metadata = SnapshotMetadata::new(agent_id, session, snapshot_idx as u64);
                let file_path = temp_dir.path().join(format!("{}_{}_{}snapshot.json.gz", agent_id, session, snapshot_idx));
                
                engine.save_snapshot(&data, &metadata, file_path.to_str().unwrap()).unwrap();
                
                // Verify immediate load
                let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
                assert_eq!(loaded_data, data);
                assert_eq!(loaded_metadata.agent_id(), agent_id);
                assert_eq!(loaded_metadata.session_id(), *session);
                assert_eq!(loaded_metadata.snapshot_index(), snapshot_idx as u64);
            }
        }
        
        // Verify all snapshots exist
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        assert_eq!(entries.count(), 9); // 3 sessions × 3 snapshots each
    }

    #[test]
    fn test_snapshot_error_conditions() {
        let engine = create_default_engine();
        
        // Test save to invalid path
        let invalid_data = "test data";
        let metadata = SnapshotMetadata::new("agent", "session", 0);
        let result = engine.save_snapshot(invalid_data, &metadata, "/invalid/path/file.json.gz");
        assert!(result.is_err());
        
        // Test load from nonexistent file
        let result = engine.load_snapshot("/nonexistent/file.json.gz");
        assert!(result.is_err());
        
        // Test load from empty path
        let result = engine.load_snapshot("");
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_metadata_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        let agent_data = r#"{"data": "consistency_test"}"#;
        let original_metadata = SnapshotMetadata::new("consistency_agent", "consistency_session", 42);
        let file_path = temp_dir.path().join("consistency_test.json.gz");
        
        // Save snapshot
        engine.save_snapshot(agent_data, &original_metadata, file_path.to_str().unwrap()).unwrap();
        
        // Load and verify metadata is preserved
        let (loaded_metadata, _) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_metadata.agent_id(), original_metadata.agent_id());
        assert_eq!(loaded_metadata.session_id(), original_metadata.session_id());
        assert_eq!(loaded_metadata.snapshot_index(), original_metadata.snapshot_index());
        assert_eq!(loaded_metadata.format_version(), original_metadata.format_version());
        
        // Timestamp should be preserved within reasonable bounds
        let time_diff = if loaded_metadata.timestamp() > original_metadata.timestamp() {
            loaded_metadata.timestamp() - original_metadata.timestamp()
        } else {
            original_metadata.timestamp() - loaded_metadata.timestamp()
        };
        assert!(time_diff <= 1); // Allow 1 second difference
    }

    #[test]
    fn test_snapshot_performance_baseline() {
        let temp_dir = TempDir::new().unwrap();
        let engine = create_default_engine();
        
        // Create medium-sized data for performance testing
        let test_data = serde_json::to_string(&serde_json::json!({
            "type": "performance_test",
            "data": vec!["test_item"; 1000],
            "metadata": std::collections::HashMap::from([
                ("key1", "value1"),
                ("key2", "value2"),
                ("key3", "value3"),
            ])
        })).unwrap();
        
        let metadata = SnapshotMetadata::new("perf_agent", "perf_session", 0);
        let file_path = temp_dir.path().join("perf_test.json.gz");
        
        let start = std::time::Instant::now();
        
        // Perform save operation
        engine.save_snapshot(&test_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let save_duration = start.elapsed();
        
        let start = std::time::Instant::now();
        
        // Perform load operation
        let (_, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        let load_duration = start.elapsed();
        
        // Verify correctness
        assert_eq!(loaded_data, test_data);
        
        // Basic performance expectations (these are baselines, not strict requirements)
        assert!(save_duration.as_millis() < 1000, "Save took too long: {:?}", save_duration);
        assert!(load_duration.as_millis() < 1000, "Load took too long: {:?}", load_duration);
        
        println!("Performance baseline - Save: {:?}, Load: {:?}", save_duration, load_duration);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::config::{StorageConfig, StorageBackend};
    use crate::snapshot::{create_engine_from_config, create_default_engine};

    #[test]
    fn test_engine_from_config_local() {
        let config = StorageConfig {
            backend: StorageBackend::Local,
            bucket_name: None,
            region: None,
            access_key_id: None,
            secret_access_key: None,
            endpoint_url: None,
        };
        
        let engine = create_engine_from_config(config).unwrap();
        
        // Test basic functionality
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config_test.json.gz");
        let test_data = r#"{"test": "config_integration"}"#;
        let metadata = SnapshotMetadata::new("config_agent", "config_session", 0);
        
        engine.save_snapshot(test_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, test_data);
        assert_eq!(loaded_metadata.agent_id(), "config_agent");
    }

    #[test]
    fn test_default_engine_functionality() {
        let engine = create_default_engine();
        
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("default_test.json.gz");
        let test_data = r#"{"agent": "default_test"}"#;
        let metadata = SnapshotMetadata::new("default_agent", "default_session", 1);
        
        engine.save_snapshot(test_data, &metadata, file_path.to_str().unwrap()).unwrap();
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, test_data);
        assert_eq!(loaded_metadata.snapshot_index(), 1);
    }

    #[test]
    fn test_cross_engine_compatibility() {
        // Test that snapshots created by one engine can be read by another
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("cross_engine_test.json.gz");
        
        let test_data = r#"{"cross": "engine", "compatibility": true}"#;
        let metadata = SnapshotMetadata::new("cross_agent", "cross_session", 0);
        
        // Save with one engine
        let engine1 = create_default_engine();
        engine1.save_snapshot(test_data, &metadata, file_path.to_str().unwrap()).unwrap();
        
        // Load with another engine
        let engine2 = create_default_engine();
        let (loaded_metadata, loaded_data) = engine2.load_snapshot(file_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, test_data);
        assert_eq!(loaded_metadata.agent_id(), "cross_agent");
    }
}
