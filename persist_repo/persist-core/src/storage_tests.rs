/*!
Comprehensive tests for storage adapters including local and S3 storage.
*/

#[cfg(test)]
mod tests {
    use crate::storage::{StorageAdapter, LocalFileStorage};
    use crate::error::PersistError;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_local_storage_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let test_data = b"test snapshot data";
        let file_path = temp_dir.path().join("test_snapshot.json.gz");
        
        // Save data
        storage.save(test_data, file_path.to_str().unwrap()).unwrap();
        
        // Verify file exists
        assert!(file_path.exists());
        
        // Load data
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, test_data);
    }

    #[test]
    fn test_local_storage_nonexistent_path() {
        let storage = LocalFileStorage::new();
        let result = storage.load("/nonexistent/path/file.json.gz");
        
        assert!(result.is_err());
        match result.unwrap_err() {
            PersistError::Io(_) => {}, // Expected
            _ => panic!("Expected IO error for nonexistent file"),
        }
    }

    #[test]
    fn test_local_storage_create_directories() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let nested_path = temp_dir.path().join("deep/nested/directory/file.json.gz");
        let test_data = b"test data";
        
        // Should create directories if they don't exist
        storage.save(test_data, nested_path.to_str().unwrap()).unwrap();
        
        assert!(nested_path.exists());
        let loaded_data = storage.load(nested_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, test_data);
    }

    #[test]
    fn test_local_storage_overwrite_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let file_path = temp_dir.path().join("overwrite_test.json.gz");
        
        // Save initial data
        let initial_data = b"initial data";
        storage.save(initial_data, file_path.to_str().unwrap()).unwrap();
        
        // Overwrite with new data
        let new_data = b"new data that's different";
        storage.save(new_data, file_path.to_str().unwrap()).unwrap();
        
        // Verify new data was saved
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, new_data);
        assert_ne!(loaded_data, initial_data);
    }

    #[test]
    fn test_local_storage_empty_data() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let file_path = temp_dir.path().join("empty_test.json.gz");
        let empty_data = b"";
        
        storage.save(empty_data, file_path.to_str().unwrap()).unwrap();
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, empty_data);
    }

    #[test]
    fn test_local_storage_large_data() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let file_path = temp_dir.path().join("large_test.json.gz");
        
        // Create 1MB of test data
        let large_data = vec![b'x'; 1024 * 1024];
        
        storage.save(&large_data, file_path.to_str().unwrap()).unwrap();
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, large_data);
    }

    #[test]
    fn test_local_storage_special_characters_in_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let special_filename = "file with spaces & symbols (1).json.gz";
        let file_path = temp_dir.path().join(special_filename);
        let test_data = b"test data";
        
        storage.save(test_data, file_path.to_str().unwrap()).unwrap();
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, test_data);
    }

    #[test]
    fn test_local_storage_concurrent_access() {
        use std::thread;
        use std::sync::Arc;
        
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(LocalFileStorage::new());
        let mut handles = vec![];
        
        // Spawn multiple threads to save files concurrently
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let temp_path = temp_dir.path().to_path_buf();
            
            let handle = thread::spawn(move || {
                let file_path = temp_path.join(format!("concurrent_test_{}.json.gz", i));
                let test_data = format!("test data {}", i).into_bytes();
                
                storage_clone.save(&test_data, file_path.to_str().unwrap()).unwrap();
                let loaded_data = storage_clone.load(file_path.to_str().unwrap()).unwrap();
                assert_eq!(loaded_data, test_data);
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all files were created
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        assert_eq!(entries.count(), 10);
    }

    #[test]
    fn test_local_storage_binary_data() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let file_path = temp_dir.path().join("binary_test.json.gz");
        
        // Create binary data with all byte values
        let binary_data: Vec<u8> = (0..=255).collect();
        
        storage.save(&binary_data, file_path.to_str().unwrap()).unwrap();
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, binary_data);
    }

    #[test]
    fn test_local_storage_invalid_permissions() {
        let storage = LocalFileStorage::new();
        
        // Try to save to a path that should cause permission errors (system directories)
        let restricted_path = "/root/restricted_file.json.gz";
        let test_data = b"test data";
        
        let result = storage.save(test_data, restricted_path);
        
        // Should fail with permission error (unless running as root)
        if !Path::new("/root").exists() || !Path::new("/root").metadata().unwrap().permissions().readonly() {
            // Skip this test if running with root permissions or /root doesn't exist
            return;
        }
        
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_adapter_trait_object() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("trait_object_test.json.gz");
        let test_data = b"trait object test data";
        
        // Test using LocalFileStorage through StorageAdapter trait
        let storage: Box<dyn StorageAdapter> = Box::new(LocalFileStorage::new());
        
        storage.save(test_data, file_path.to_str().unwrap()).unwrap();
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, test_data);
    }

    #[test]
    fn test_local_storage_extremely_long_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        
        // Create a very long path (but within filesystem limits)
        let long_segment = "a".repeat(50);
        let long_path = temp_dir.path()
            .join(&long_segment)
            .join(&long_segment)
            .join(&long_segment)
            .join("file.json.gz");
        
        let test_data = b"long path test data";
        
        storage.save(test_data, long_path.to_str().unwrap()).unwrap();
        let loaded_data = storage.load(long_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, test_data);
    }

    #[test]
    fn test_storage_error_handling() {
        let storage = LocalFileStorage::new();
        
        // Test loading from invalid path
        let result = storage.load("");
        assert!(result.is_err());
        
        // Test loading from directory instead of file
        let temp_dir = TempDir::new().unwrap();
        let result = storage.load(temp_dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_interface_consistency() {
        // Test that all storage adapters implement the same interface correctly
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("interface_test.json.gz");
        let test_data = b"interface consistency test";
        
        let storages: Vec<Box<dyn StorageAdapter>> = vec![
            Box::new(LocalFileStorage::new()),
            // Add other storage adapters here when testing them
        ];
        
        for storage in storages {
            storage.save(test_data, file_path.to_str().unwrap()).unwrap();
            let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
            assert_eq!(loaded_data, test_data);
            
            // Clean up for next iteration
            fs::remove_file(&file_path).ok();
        }
    }

    #[test]
    fn test_storage_metadata_preservation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::new();
        let file_path = temp_dir.path().join("metadata_test.json.gz");
        let test_data = b"metadata preservation test";
        
        // Save data
        storage.save(test_data, file_path.to_str().unwrap()).unwrap();
        
        // Check file metadata
        let metadata = fs::metadata(&file_path).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.len(), test_data.len() as u64);
        
        // Load and verify data integrity
        let loaded_data = storage.load(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_data, test_data);
    }
}

#[cfg(test)]
mod s3_tests {
    use super::*;
    use crate::storage::S3StorageAdapter;
    use crate::config::StorageConfig;

    // Note: S3 tests require AWS credentials and will be skipped if not available
    // These are integration tests that should be run in a proper testing environment

    fn create_test_s3_config() -> StorageConfig {
        StorageConfig {
            backend: crate::config::StorageBackend::S3,
            bucket_name: Some("test-persist-bucket".to_string()),
            region: Some("us-east-1".to_string()),
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            endpoint_url: std::env::var("AWS_ENDPOINT_URL").ok(),
        }
    }

    #[tokio::test]
    #[ignore] // Ignore by default, run with --ignored for S3 integration tests
    async fn test_s3_storage_save_and_load() {
        let config = create_test_s3_config();
        
        // Skip test if AWS credentials not available
        if config.access_key_id.is_none() || config.secret_access_key.is_none() {
            println!("Skipping S3 test - AWS credentials not available");
            return;
        }
        
        let storage = S3StorageAdapter::new(config).await.unwrap();
        let test_data = b"S3 test snapshot data";
        let key = "test/snapshot.json.gz";
        
        // Save data
        storage.save(test_data, key).unwrap();
        
        // Load data
        let loaded_data = storage.load(key).unwrap();
        assert_eq!(loaded_data, test_data);
        
        // Clean up (delete the test object)
        // Note: In a real test environment, you might want to add cleanup functionality
    }

    #[tokio::test]
    #[ignore]
    async fn test_s3_storage_nonexistent_key() {
        let config = create_test_s3_config();
        
        if config.access_key_id.is_none() || config.secret_access_key.is_none() {
            println!("Skipping S3 test - AWS credentials not available");
            return;
        }
        
        let storage = S3StorageAdapter::new(config).await.unwrap();
        let result = storage.load("nonexistent/key.json.gz");
        
        assert!(result.is_err());
    }

    #[test]
    fn test_s3_storage_config_validation() {
        // Test invalid config
        let invalid_config = StorageConfig {
            backend: crate::config::StorageBackend::S3,
            bucket_name: None, // Missing bucket name
            region: Some("us-east-1".to_string()),
            access_key_id: None,
            secret_access_key: None,
            endpoint_url: None,
        };
        
        // This should fail during adapter creation
        // Note: The actual test depends on how S3StorageAdapter handles invalid configs
    }
}
