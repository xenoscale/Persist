/*!
Storage adapters for snapshot persistence.

This module defines the storage abstraction (port) and concrete implementations (adapters)
following hexagonal architecture principles. The core domain logic is independent of
storage details, making it easy to add new storage backends.
*/

use std::fs;
use std::path::{Path, PathBuf};
use crate::{PersistError, Result};

/// Storage abstraction for saving and loading snapshot data
///
/// This trait defines the interface that all storage implementations must provide.
/// It abstracts away the specifics of where and how data is stored, allowing
/// the core engine to work with any storage backend.
pub trait StorageAdapter {
    /// Save snapshot data to the specified location
    ///
    /// # Arguments
    /// * `data` - The compressed snapshot data to save
    /// * `path` - The storage location (interpretation depends on implementation)
    ///
    /// # Returns
    /// Result indicating success or failure
    fn save(&self, data: &[u8], path: &str) -> Result<()>;

    /// Load snapshot data from the specified location
    ///
    /// # Arguments
    /// * `path` - The storage location to load from
    ///
    /// # Returns
    /// The loaded data bytes or an error
    fn load(&self, path: &str) -> Result<Vec<u8>>;

    /// Check if a snapshot exists at the specified location
    ///
    /// # Arguments
    /// * `path` - The storage location to check
    ///
    /// # Returns
    /// True if the snapshot exists, false otherwise
    fn exists(&self, path: &str) -> bool;

    /// Delete a snapshot from the specified location
    ///
    /// # Arguments
    /// * `path` - The storage location to delete
    ///
    /// # Returns
    /// Result indicating success or failure
    fn delete(&self, path: &str) -> Result<()>;
}

/// Local filesystem storage adapter
///
/// This implementation stores snapshots as files on the local filesystem.
/// It automatically creates parent directories if they don't exist.
///
/// # Example
/// ```rust
/// use persist_core::LocalFileStorage;
/// 
/// let storage = LocalFileStorage::new();
/// // Will create any missing directories
/// let data = b"compressed snapshot data";
/// storage.save(data, "/path/to/snapshots/agent1.json.gz")?;
/// ```
#[derive(Debug, Clone)]
pub struct LocalFileStorage {
    /// Optional base directory for all snapshots
    base_dir: Option<PathBuf>,
}

impl LocalFileStorage {
    /// Create a new local file storage adapter without a base directory
    ///
    /// Paths provided to save/load will be used as-is.
    pub fn new() -> Self {
        Self { base_dir: None }
    }

    /// Create a new local file storage adapter with a base directory
    ///
    /// All paths will be resolved relative to the base directory.
    ///
    /// # Arguments
    /// * `base_dir` - The base directory for all snapshot files
    ///
    /// # Example
    /// ```rust
    /// use persist_core::LocalFileStorage;
    /// 
    /// let storage = LocalFileStorage::with_base_dir("/var/persist/snapshots");
    /// // save("data", "agent1.json.gz") will save to "/var/persist/snapshots/agent1.json.gz"
    /// ```
    pub fn with_base_dir<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: Some(base_dir.as_ref().to_path_buf()),
        }
    }

    /// Resolve the full path for a given storage path
    fn resolve_path(&self, path: &str) -> PathBuf {
        match &self.base_dir {
            Some(base) => base.join(path),
            None => PathBuf::from(path),
        }
    }

    /// Ensure the parent directory exists, creating it if necessary
    fn ensure_parent_dir(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| PersistError::storage(format!(
                        "Failed to create directory {}: {}", 
                        parent.display(), 
                        e
                    )))?;
            }
        }
        Ok(())
    }
}

impl Default for LocalFileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageAdapter for LocalFileStorage {
    fn save(&self, data: &[u8], path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        
        // Ensure parent directory exists
        self.ensure_parent_dir(&full_path)?;
        
        // Write the data
        fs::write(&full_path, data)
            .map_err(|e| PersistError::storage(format!(
                "Failed to write snapshot to {}: {}", 
                full_path.display(), 
                e
            )))?;
        
        Ok(())
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path);
        
        fs::read(&full_path)
            .map_err(|e| PersistError::storage(format!(
                "Failed to read snapshot from {}: {}", 
                full_path.display(), 
                e
            )))
    }

    fn exists(&self, path: &str) -> bool {
        let full_path = self.resolve_path(path);
        full_path.exists()
    }

    fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        
        if full_path.exists() {
            fs::remove_file(&full_path)
                .map_err(|e| PersistError::storage(format!(
                    "Failed to delete snapshot {}: {}", 
                    full_path.display(), 
                    e
                )))?;
        }
        
        Ok(())
    }
}

/// Memory-based storage adapter for testing
///
/// This implementation stores snapshots in memory using a HashMap.
/// Useful for unit testing without touching the filesystem.
#[cfg(test)]
pub struct MemoryStorage {
    data: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

#[cfg(test)]
impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[cfg(test)]
impl StorageAdapter for MemoryStorage {
    fn save(&self, data: &[u8], path: &str) -> Result<()> {
        let mut storage = self.data.lock().unwrap();
        storage.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        let storage = self.data.lock().unwrap();
        storage
            .get(path)
            .cloned()
            .ok_or_else(|| PersistError::storage(format!("Snapshot not found: {}", path)))
    }

    fn exists(&self, path: &str) -> bool {
        let storage = self.data.lock().unwrap();
        storage.contains_key(path)
    }

    fn delete(&self, path: &str) -> Result<()> {
        let mut storage = self.data.lock().unwrap();
        storage.remove(path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_local_file_storage_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::with_base_dir(temp_dir.path());
        
        let test_data = b"test snapshot data";
        let path = "test_snapshot.json.gz";
        
        // Test save
        assert!(storage.save(test_data, path).is_ok());
        
        // Test exists
        assert!(storage.exists(path));
        
        // Test load
        let loaded_data = storage.load(path).unwrap();
        assert_eq!(loaded_data, test_data);
        
        // Test delete
        assert!(storage.delete(path).is_ok());
        assert!(!storage.exists(path));
    }

    #[test]
    fn test_local_file_storage_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::with_base_dir(temp_dir.path());
        
        let test_data = b"test snapshot data";
        let path = "agents/agent1/sessions/session1/snapshot.json.gz";
        
        // Should create nested directories automatically
        assert!(storage.save(test_data, path).is_ok());
        assert!(storage.exists(path));
        
        let loaded_data = storage.load(path).unwrap();
        assert_eq!(loaded_data, test_data);
    }

    #[test]
    fn test_memory_storage() {
        let storage = MemoryStorage::new();
        
        let test_data = b"test snapshot data";
        let path = "test_snapshot";
        
        // Test save
        assert!(storage.save(test_data, path).is_ok());
        
        // Test exists
        assert!(storage.exists(path));
        
        // Test load
        let loaded_data = storage.load(path).unwrap();
        assert_eq!(loaded_data, test_data);
        
        // Test delete
        assert!(storage.delete(path).is_ok());
        assert!(!storage.exists(path));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::with_base_dir(temp_dir.path());
        
        let result = storage.load("nonexistent.json.gz");
        assert!(result.is_err());
    }
}
