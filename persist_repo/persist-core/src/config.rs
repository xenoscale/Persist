//! Configuration module for storage backend selection and settings
//!
//! This module provides configuration structures and enums for selecting
//! between different storage backends (Local filesystem, S3, etc.) and
//! configuring their parameters.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Enumeration of supported storage backends
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBackend {
    /// Local filesystem storage
    Local,
    /// Amazon S3 cloud storage
    S3,
    /// Google Cloud Storage
    GCS,
}

/// Configuration structure for storage backend settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// The storage backend to use
    pub backend: StorageBackend,
    /// S3 bucket name (required for S3 backend)
    pub s3_bucket: Option<String>,
    /// AWS region for S3 operations (optional, defaults to environment)
    pub s3_region: Option<String>,
    /// Base path for local storage (optional, defaults to current directory)
    pub local_base_path: Option<PathBuf>,
    /// GCS bucket name (required for GCS backend)
    pub gcs_bucket: Option<String>,
    /// Path to GCP service account JSON file (optional, uses ADC if not provided)
    pub gcs_credentials_path: Option<PathBuf>,
}

impl StorageConfig {
    /// Create a default configuration for local filesystem storage
    pub fn default_local() -> Self {
        StorageConfig {
            backend: StorageBackend::Local,
            s3_bucket: None,
            s3_region: None,
            local_base_path: None,
            gcs_bucket: None,
            gcs_credentials_path: None,
        }
    }

    /// Create a default configuration for S3 storage with fallback bucket
    pub fn default_s3() -> Self {
        StorageConfig {
            backend: StorageBackend::S3,
            s3_bucket: Some("persist-default-bucket".to_string()),
            s3_region: None, // Will use AWS environment default
            local_base_path: None,
            gcs_bucket: None,
            gcs_credentials_path: None,
        }
    }

    /// Create an S3 configuration with specified bucket
    pub fn s3_with_bucket(bucket: String) -> Self {
        StorageConfig {
            backend: StorageBackend::S3,
            s3_bucket: Some(bucket),
            s3_region: None,
            local_base_path: None,
            gcs_bucket: None,
            gcs_credentials_path: None,
        }
    }

    /// Create an S3 configuration with bucket and region
    pub fn s3_with_bucket_and_region(bucket: String, region: String) -> Self {
        StorageConfig {
            backend: StorageBackend::S3,
            s3_bucket: Some(bucket),
            s3_region: Some(region),
            local_base_path: None,
            gcs_bucket: None,
            gcs_credentials_path: None,
        }
    }

    /// Create a default configuration for GCS storage with fallback bucket
    pub fn default_gcs() -> Self {
        StorageConfig {
            backend: StorageBackend::GCS,
            s3_bucket: None,
            s3_region: None,
            local_base_path: None,
            gcs_bucket: Some("persist-default-gcs-bucket".to_string()),
            gcs_credentials_path: None,
        }
    }

    /// Create a GCS configuration with specified bucket
    pub fn gcs_with_bucket(bucket: String) -> Self {
        StorageConfig {
            backend: StorageBackend::GCS,
            s3_bucket: None,
            s3_region: None,
            local_base_path: None,
            gcs_bucket: Some(bucket),
            gcs_credentials_path: None,
        }
    }

    /// Create a GCS configuration with bucket and credentials path
    pub fn gcs_with_bucket_and_credentials(bucket: String, credentials_path: PathBuf) -> Self {
        StorageConfig {
            backend: StorageBackend::GCS,
            s3_bucket: None,
            s3_region: None,
            local_base_path: None,
            gcs_bucket: Some(bucket),
            gcs_credentials_path: Some(credentials_path),
        }
    }

    /// Parse a storage URI and create appropriate configuration
    ///
    /// Supports formats:
    /// - `s3://bucket-name/path` for S3 storage
    /// - `gs://bucket-name/path` for GCS storage
    /// - `/local/path` or `./relative/path` for local storage
    ///
    /// Returns the config and the extracted key/path component
    pub fn from_uri(uri: &str) -> Result<(StorageConfig, String), crate::PersistError> {
        if let Some(s3_part) = uri.strip_prefix("s3://") {
            let parts: Vec<&str> = s3_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(crate::PersistError::validation(
                    "Invalid S3 URI: missing bucket name",
                ));
            }

            let bucket = parts[0].to_string();
            let key = parts.get(1).unwrap_or(&"").to_string();

            let config = StorageConfig::s3_with_bucket(bucket);
            Ok((config, key))
        } else if let Some(gcs_part) = uri.strip_prefix("gs://") {
            let parts: Vec<&str> = gcs_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(crate::PersistError::validation(
                    "Invalid GCS URI: missing bucket name",
                ));
            }

            let bucket = parts[0].to_string();
            let key = parts.get(1).unwrap_or(&"").to_string();

            let config = StorageConfig::gcs_with_bucket(bucket);
            Ok((config, key))
        } else {
            // Treat as local path
            let config = StorageConfig::default_local();
            Ok((config, uri.to_string()))
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::Result<()> {
        match self.backend {
            StorageBackend::S3 => {
                if self.s3_bucket.is_none() || self.s3_bucket.as_ref().unwrap().is_empty() {
                    return Err(crate::PersistError::validation(
                        "S3 backend requires a valid bucket name",
                    ));
                }
            }
            StorageBackend::Local => {
                // Local storage validation can be added here if needed
            }
            StorageBackend::GCS => {
                if self.gcs_bucket.is_none() || self.gcs_bucket.as_ref().unwrap().is_empty() {
                    return Err(crate::PersistError::validation(
                        "GCS backend requires a valid bucket name",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::default_local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_local_config() {
        let config = StorageConfig::default_local();
        assert_eq!(config.backend, StorageBackend::Local);
        assert!(config.s3_bucket.is_none());
        assert!(config.local_base_path.is_none());
    }

    #[test]
    fn test_default_s3_config() {
        let config = StorageConfig::default_s3();
        assert_eq!(config.backend, StorageBackend::S3);
        assert_eq!(config.s3_bucket, Some("persist-default-bucket".to_string()));
    }

    #[test]
    fn test_s3_with_bucket() {
        let config = StorageConfig::s3_with_bucket("my-bucket".to_string());
        assert_eq!(config.backend, StorageBackend::S3);
        assert_eq!(config.s3_bucket, Some("my-bucket".to_string()));
    }

    #[test]
    fn test_from_uri_s3() {
        let (config, key) = StorageConfig::from_uri("s3://test-bucket/path/to/object").unwrap();
        assert_eq!(config.backend, StorageBackend::S3);
        assert_eq!(config.s3_bucket, Some("test-bucket".to_string()));
        assert_eq!(key, "path/to/object");
    }

    #[test]
    fn test_from_uri_s3_bucket_only() {
        let (config, key) = StorageConfig::from_uri("s3://test-bucket").unwrap();
        assert_eq!(config.backend, StorageBackend::S3);
        assert_eq!(config.s3_bucket, Some("test-bucket".to_string()));
        assert_eq!(key, "");
    }

    #[test]
    fn test_from_uri_local() {
        let (config, path) = StorageConfig::from_uri("/local/path/file.json").unwrap();
        assert_eq!(config.backend, StorageBackend::Local);
        assert_eq!(path, "/local/path/file.json");
    }

    #[test]
    fn test_from_uri_invalid_s3() {
        let result = StorageConfig::from_uri("s3://");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing bucket name"));
    }

    #[test]
    fn test_validate_s3_config() {
        let mut config = StorageConfig::default_s3();
        assert!(config.validate().is_ok());

        config.s3_bucket = None;
        assert!(config.validate().is_err());

        config.s3_bucket = Some("".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_local_config() {
        let config = StorageConfig::default_local();
        assert!(config.validate().is_ok());
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_default_gcs_config() {
        let config = StorageConfig::default_gcs();
        assert_eq!(config.backend, StorageBackend::GCS);
        assert_eq!(
            config.gcs_bucket,
            Some("persist-default-gcs-bucket".to_string())
        );
        assert!(config.gcs_credentials_path.is_none());
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_gcs_with_bucket() {
        let config = StorageConfig::gcs_with_bucket("my-gcs-bucket".to_string());
        assert_eq!(config.backend, StorageBackend::GCS);
        assert_eq!(config.gcs_bucket, Some("my-gcs-bucket".to_string()));
        assert!(config.gcs_credentials_path.is_none());
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_gcs_with_bucket_and_credentials() {
        let creds_path = PathBuf::from("/path/to/service-account.json");
        let config = StorageConfig::gcs_with_bucket_and_credentials(
            "my-gcs-bucket".to_string(),
            creds_path.clone(),
        );
        assert_eq!(config.backend, StorageBackend::GCS);
        assert_eq!(config.gcs_bucket, Some("my-gcs-bucket".to_string()));
        assert_eq!(config.gcs_credentials_path, Some(creds_path));
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_from_uri_gcs() {
        let (config, key) = StorageConfig::from_uri("gs://test-bucket/path/to/object").unwrap();
        assert_eq!(config.backend, StorageBackend::GCS);
        assert_eq!(config.gcs_bucket, Some("test-bucket".to_string()));
        assert_eq!(key, "path/to/object");
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_from_uri_gcs_bucket_only() {
        let (config, key) = StorageConfig::from_uri("gs://test-bucket").unwrap();
        assert_eq!(config.backend, StorageBackend::GCS);
        assert_eq!(config.gcs_bucket, Some("test-bucket".to_string()));
        assert_eq!(key, "");
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_from_uri_invalid_gcs() {
        let result = StorageConfig::from_uri("gs://");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing bucket name"));
    }

    #[cfg(feature = "gcs")]
    #[test]
    fn test_validate_gcs_config() {
        let mut config = StorageConfig::default_gcs();
        assert!(config.validate().is_ok());

        config.gcs_bucket = None;
        assert!(config.validate().is_err());

        config.gcs_bucket = Some("".to_string());
        assert!(config.validate().is_err());
    }

    #[cfg(not(feature = "gcs"))]
    #[test]
    fn test_from_uri_gcs_disabled() {
        let result = StorageConfig::from_uri("gs://test-bucket/path");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("GCS support not enabled"));
    }
}
