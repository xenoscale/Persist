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

    /// S3 upload errors with context
    #[error("Failed to upload state to S3 (bucket: {bucket}, key: {key}): {source}")]
    S3UploadError {
        source: Box<dyn std::error::Error + Send + Sync>,
        bucket: String,
        key: String,
    },

    /// S3 download errors with context
    #[error("Failed to download state from S3 (bucket: {bucket}, key: {key}): {source}")]
    S3DownloadError {
        source: Box<dyn std::error::Error + Send + Sync>,
        bucket: String,
        key: String,
    },

    /// S3 object not found
    #[error("State not found in S3 (bucket: {bucket}, key: {key})")]
    S3NotFound { bucket: String, key: String },

    /// S3 access denied
    #[error("Access denied to S3 (bucket: {bucket}): check credentials and permissions")]
    S3AccessDenied { bucket: String },

    /// S3 configuration errors
    #[error("S3 configuration error: {0}")]
    S3Configuration(String),

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

    /// Create a new S3 upload error with context
    pub fn s3_upload_error<E: std::error::Error + Send + Sync + 'static>(
        source: E,
        bucket: String,
        key: String,
    ) -> Self {
        Self::S3UploadError {
            source: Box::new(source),
            bucket,
            key,
        }
    }

    /// Create a new S3 download error with context
    pub fn s3_download_error<E: std::error::Error + Send + Sync + 'static>(
        source: E,
        bucket: String,
        key: String,
    ) -> Self {
        Self::S3DownloadError {
            source: Box::new(source),
            bucket,
            key,
        }
    }

    /// Create a new S3 not found error
    pub fn s3_not_found(bucket: String, key: String) -> Self {
        Self::S3NotFound { bucket, key }
    }

    /// Create a new S3 access denied error
    pub fn s3_access_denied(bucket: String) -> Self {
        Self::S3AccessDenied { bucket }
    }

    /// Create a new S3 configuration error
    pub fn s3_configuration<S: Into<String>>(msg: S) -> Self {
        Self::S3Configuration(msg.into())
    }
}
