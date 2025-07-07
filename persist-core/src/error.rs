/*!
Error types for the Persist core engine.
*/

use thiserror::Error;

/// Result type used throughout the Persist core.
pub type Result<T> = std::result::Result<T, PersistError>;

/// Errors that can occur during snapshot operations.
#[derive(Error, Debug)]
pub enum PersistError {
    /// I/O errors during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Compression/decompression errors
    #[error("Compression error: {0}")]
    Compression(String),

    /// Integrity check failures
    #[error("Integrity check failed: expected hash {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    /// Invalid snapshot format
    #[error("Invalid snapshot format: {0}")]
    InvalidFormat(String),

    /// Missing required metadata fields
    #[error("Missing required metadata field: {0}")]
    MissingMetadata(String),

    /// Storage adapter errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),
}

impl PersistError {
    /// Create a new compression error
    pub fn compression<S: Into<String>>(msg: S) -> Self {
        Self::Compression(msg.into())
    }

    /// Create a new storage error
    pub fn storage<S: Into<String>>(msg: S) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a new validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a new invalid format error
    pub fn invalid_format<S: Into<String>>(msg: S) -> Self {
        Self::InvalidFormat(msg.into())
    }
}
