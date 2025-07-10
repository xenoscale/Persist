/*!
Storage adapters for snapshot persistence.

This module defines the storage abstraction (port) and concrete implementations (adapters)
following hexagonal architecture principles. The core domain logic is independent of
storage details, making it easy to add new storage backends.
*/

pub mod gcs;
pub mod local;
pub mod s3;

use crate::Result;

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

// Re-export types for convenience
pub use gcs::GCSStorageAdapter;
pub use local::LocalFileStorage;
pub use s3::S3StorageAdapter;

/// Memory-based storage adapter for testing
///
/// This implementation stores snapshots in memory using a HashMap.
/// Useful for unit testing without touching the filesystem.
#[cfg(test)]
pub struct MemoryStorage {
    data: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

#[cfg(test)]
impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
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
            .ok_or_else(|| crate::PersistError::storage(format!("Snapshot not found: {path}")))
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
