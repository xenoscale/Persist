/*!
Storage adapters for snapshot persistence.

This module defines the storage abstraction (port) and concrete implementations (adapters)
following hexagonal architecture principles. The core domain logic is independent of
storage details, making it easy to add new storage backends.
*/

#[cfg(feature = "gcs")]
pub mod gcs;
pub mod local;
#[cfg(feature = "s3")]
pub mod s3;

use crate::Result;
use async_trait::async_trait;
use futures::io::AsyncRead;

#[cfg(feature = "async-rt")]
use once_cell::sync::Lazy;
#[cfg(feature = "async-rt")]
use std::sync::Arc;
#[cfg(feature = "async-rt")]
use tokio::runtime::Runtime;

#[cfg(feature = "async-rt")]
static GLOBAL_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get().max(4))
        .enable_all()
        .build()
        .expect("Failed to create global async runtime")
});

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

/// Async storage abstraction for save and load operations
///
/// This trait defines an async interface for storage operations, enabling
/// better performance for I/O-bound operations and non-blocking behavior.
#[async_trait]
pub trait AsyncStorageAdapter: Send + Sync {
    /// Save snapshot data asynchronously from a reader
    ///
    /// # Arguments
    /// * `reader` - Async reader containing the data to save
    /// * `path` - The storage location (interpretation depends on implementation)
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn save(&self, reader: impl AsyncRead + Send + 'static, path: &str) -> Result<()>;

    /// Load snapshot data asynchronously
    ///
    /// # Arguments
    /// * `path` - The storage location to load from
    ///
    /// # Returns
    /// An async reader providing the loaded data or an error
    async fn load(&self, path: &str) -> Result<Box<dyn AsyncRead + Send + Unpin>>;

    /// Check if a snapshot exists at the specified location
    ///
    /// # Arguments
    /// * `path` - The storage location to check
    ///
    /// # Returns
    /// True if the snapshot exists, false otherwise
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Delete a snapshot from the specified location
    ///
    /// # Arguments
    /// * `path` - The storage location to delete
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn delete(&self, path: &str) -> Result<()>;
}

/// Blocking wrapper for async storage adapters
///
/// This wrapper allows async storage implementations to be used in sync contexts
/// by using a global runtime to block on async operations.
#[cfg(feature = "async-rt")]
pub struct BlockingStorage<A: AsyncStorageAdapter> {
    inner: Arc<A>,
}

#[cfg(feature = "async-rt")]
impl<A: AsyncStorageAdapter> BlockingStorage<A> {
    pub fn new(adapter: A) -> Self {
        Self {
            inner: Arc::new(adapter),
        }
    }
}

#[cfg(feature = "async-rt")]
impl<A: AsyncStorageAdapter> StorageAdapter for BlockingStorage<A> {
    fn save(&self, data: &[u8], path: &str) -> Result<()> {
        let data_owned = data.to_vec();
        let reader = futures::io::Cursor::new(data_owned);
        GLOBAL_RT.block_on(self.inner.save(reader, path))
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        use futures::io::AsyncReadExt;

        GLOBAL_RT.block_on(async {
            let mut reader = self.inner.load(path).await?;
            let mut data = Vec::new();
            reader
                .read_to_end(&mut data)
                .await
                .map_err(|e| crate::PersistError::storage(format!("Failed to read data: {e}")))?;
            Ok(data)
        })
    }

    fn exists(&self, path: &str) -> bool {
        GLOBAL_RT.block_on(self.inner.exists(path)).unwrap_or(false)
    }

    fn delete(&self, path: &str) -> Result<()> {
        GLOBAL_RT.block_on(self.inner.delete(path))
    }
}

// Re-export types for convenience
#[cfg(feature = "gcs")]
pub use gcs::GCSStorageAdapter;
pub use local::LocalFileStorage;
#[cfg(feature = "s3")]
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
