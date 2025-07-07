/*!
Local filesystem storage adapter implementation.
*/

use std::fs;
use std::path::{Path, PathBuf};
use crate::{PersistError, Result};
use super::StorageAdapter;

/// Local filesystem storage adapter
///
/// This implementation stores snapshots as files on the local filesystem.
/// It automatically creates parent directories if they don't exist.
///
/// # Example
/// ```rust
/// use persist_core::storage::LocalFileStorage;
/// 
/// let storage = LocalFileStorage::new();
/// // Will create any missing directories
/// let data = b"compressed snapshot data";
/// storage.save(data, "/path/to/snapshots/agent1.json.gz")?;
/// # Ok::<(), persist_core::PersistError>(())
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
    /// use persist_core::storage::LocalFileStorage;
    /// 
    /// let storage = LocalFileStorage::with_base_dir("/var/persist/snapshots");
    /// // save("data", "agent1.json.gz") will save to "/var/persist/snapshots/agent1.json.gz"
    /// # let storage = LocalFileStorage::with_base_dir("/tmp"); // Use tmp for test
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
    fn test_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFileStorage::with_base_dir(temp_dir.path());
        
        let result = storage.load("nonexistent.json.gz");
        assert!(result.is_err());
    }
}
