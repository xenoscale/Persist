/*!
Google Cloud Storage (GCS) adapter implementation.

This module provides GCS cloud storage support for snapshots using the official
Google Cloud Storage client library.
*/

#[cfg(feature = "gcs")]
use google_cloud_storage::client::{Client as GcsClient, ClientConfig};
#[cfg(feature = "gcs")]
use std::sync::Arc;
#[cfg(feature = "gcs")]
use std::path::PathBuf;
#[cfg(feature = "gcs")]
use tokio::runtime::Runtime;
#[cfg(feature = "gcs")]
use tracing::{debug, error, info, warn};

#[cfg(feature = "gcs")]
use super::StorageAdapter;
#[cfg(feature = "gcs")]
#[cfg(feature = "metrics")]
use crate::observability::MetricsTimer;
#[cfg(feature = "gcs")]
use crate::{PersistError, Result};

/// Google Cloud Storage adapter
///
/// This implementation stores snapshots as objects in Google Cloud Storage.
/// It uses the official Google Cloud Storage client library and supports
/// standard GCP credential providers.
///
/// # Authentication
/// The adapter uses the standard GCP credential provider chain:
/// 1. GOOGLE_APPLICATION_CREDENTIALS environment variable pointing to service account JSON
/// 2. Service account attached to the compute instance (GCE, GKE, Cloud Run, etc.)
/// 3. gcloud user credentials (when running locally with gcloud auth)
///
/// # Example
/// ```rust,no_run
/// use persist_core::{storage::GCSStorageAdapter, StorageAdapter};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Set environment variable:
/// // export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
///
/// let adapter = GCSStorageAdapter::new("my-snapshots-bucket".to_string(), None)?;
/// let data = b"compressed snapshot data";
/// adapter.save(data, "agent1/session1/snapshot.json.gz")?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "gcs")]
pub struct GCSStorageAdapter {
    client: GcsClient,
    bucket: String,
    prefix: Option<String>,
    runtime: Arc<Runtime>,
}

#[cfg(feature = "gcs")]
impl GCSStorageAdapter {
    /// Create a new GCS storage adapter for the specified bucket
    ///
    /// # Arguments
    /// * `bucket` - The GCS bucket name to use for storage
    /// * `prefix` - Optional prefix for organizing snapshots within the bucket
    /// * `creds_json` - Optional path to service account JSON file
    ///
    /// # Returns
    /// A new GCSStorageAdapter instance or an error if initialization fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - The bucket does not exist or is not accessible
    /// - GCP credentials are not available or invalid
    /// - The Tokio runtime cannot be created
    /// - GCS configuration cannot be loaded
    pub fn new(bucket: impl Into<String>, prefix: Option<String>, creds_json: Option<PathBuf>) -> Result<Self> {
        let bucket = bucket.into();
        let runtime = Runtime::new().map_err(|e| {
            PersistError::storage(format!(
                "Failed to create async runtime for GCS client: {e}"
            ))
        })?;

        // Load GCS client configuration with authentication
        let config = runtime
            .block_on(async {
                if let Some(path) = creds_json {
                    // If credentials path is provided, set it as environment variable
                    // This allows the client to discover it automatically
                    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &path);
                }

                // Use default authentication flow which will check:
                // 1. GOOGLE_APPLICATION_CREDENTIALS env var
                // 2. Metadata server for attached service accounts
                // 3. Other default credential sources
                ClientConfig::default().with_auth().await
            })
            .map_err(|e| PersistError::storage(format!("GCS authentication failed: {e}")))?;

        let client = GcsClient::new(config);

        // Fail fast: validate bucket exists and is accessible
        runtime
            .block_on(async {
                use google_cloud_storage::http::buckets::get::GetBucketRequest;
                let req = GetBucketRequest {
                    bucket: bucket.clone(),
                    ..Default::default()
                };
                client.get_bucket(&req).await
            })
            .map_err(|e| {
                PersistError::storage(format!(
                    "Failed to access GCS bucket '{}': {}. Ensure the bucket exists and you have proper permissions.",
                    bucket, e
                ))
            })?;

        info!(bucket = %bucket, prefix = ?prefix, "Initialized GCS storage adapter with bucket validation");

        Ok(GCSStorageAdapter {
            client,
            bucket,
            prefix,
            runtime: Arc::new(runtime),
        })
    }

    /// Helper method to build the full GCS object path with prefix support
    fn build_object_path(&self, key: &str) -> String {
        match &self.prefix {
            Some(prefix) => {
                if prefix.ends_with('/') {
                    format!("{}{}", prefix, key)
                } else {
                    format!("{}/{}", prefix, key)
                }
            }
            None => key.to_string(),
        }
    }
}

#[cfg(feature = "gcs")]
impl StorageAdapter for GCSStorageAdapter {
    /// Save snapshot data to GCS
    ///
    /// Uploads the data as an object to the configured GCS bucket.
    /// Includes retry logic for transient failures.
    fn save(&self, data: &[u8], path: &str) -> Result<()> {
        #[cfg(feature = "metrics")]
        let _timer = MetricsTimer::start_gcs_operation("save");

        let key = self.build_object_path(path);
        info!(bucket=%self.bucket, key=%key, size=%data.len(), "Saving snapshot to GCS");

        let mut attempts = 0;
        let max_attempts = 3;
        let key_str = key.to_string();

        loop {
            attempts += 1;

            let bucket = self.bucket.clone();
            let key_for_async = key_str.clone();
            let data_owned = data.to_vec();
            let client = self.client.clone();

            let result = self.runtime.block_on(async move {
                use google_cloud_storage::http::objects::upload::{
                    Media, UploadObjectRequest, UploadType,
                };

                let req = UploadObjectRequest {
                    bucket: bucket.clone(),
                    ..Default::default()
                };

                let upload_type = UploadType::Simple(Media::new(key_for_async.clone()));
                client.upload_object(&req, data_owned, &upload_type).await
            });

            match result {
                Ok(_) => {
                    debug!(
                        "Successfully saved snapshot to gs://{}/{}",
                        self.bucket, key
                    );
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global().record_gcs_request("save");
                    return Ok(());
                }
                Err(e) if attempts < max_attempts && is_retryable_error(&e) => {
                    warn!(
                        bucket=%self.bucket,
                        key=%key,
                        attempt=%attempts,
                        error=?e,
                        "GCS save failed, retrying..."
                    );
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global().record_gcs_retry("save");

                    // Exponential backoff
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    continue;
                }
                Err(e) => {
                    let err = map_gcs_error("upload_object", &e, &key);
                    error!(bucket=%self.bucket, key=%key, error=?err, "Failed to save snapshot to GCS");
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global().record_gcs_error("save");
                    return Err(err);
                }
            }
        }
    }

    /// Load snapshot data from GCS
    ///
    /// Downloads the object data from the configured GCS bucket.
    /// Includes retry logic for transient failures.
    fn load(&self, path: &str) -> Result<Vec<u8>> {
        #[cfg(feature = "metrics")]
        let _timer = MetricsTimer::start_gcs_operation("load");

        let key = self.build_object_path(path);
        info!(bucket=%self.bucket, key=%key, "Loading snapshot from GCS");

        let mut attempts = 0;
        let max_attempts = 3;
        let key_str = key.to_string();

        loop {
            attempts += 1;

            let bucket = self.bucket.clone();
            let key_for_async = key_str.clone();
            let client = self.client.clone();

            let result = self.runtime.block_on(async move {
                use google_cloud_storage::http::objects::get::GetObjectRequest;

                let req = GetObjectRequest {
                    bucket: bucket.clone(),
                    object: key_for_async.clone(),
                    ..Default::default()
                };

                client.download_object(&req, &Default::default()).await
            });

            match result {
                Ok(data) => {
                    debug!(
                        "Downloaded {} bytes from gs://{}/{}",
                        data.len(),
                        self.bucket,
                        key
                    );
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global().record_gcs_request("load");
                    return Ok(data);
                }
                Err(e) if attempts < max_attempts && is_retryable_error(&e) => {
                    warn!(
                        bucket=%self.bucket,
                        key=%key,
                        attempt=%attempts,
                        error=?e,
                        "GCS load failed, retrying..."
                    );
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global().record_gcs_retry("load");

                    // Exponential backoff
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    continue;
                }
                Err(e) => {
                    let err = map_gcs_error("download_object", &e, &key);
                    error!(bucket=%self.bucket, key=%key, error=?err, "Failed to load snapshot from GCS");
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global().record_gcs_error("load");
                    return Err(err);
                }
            }
        }
    }

    /// Check if a snapshot exists at the specified GCS location
    fn exists(&self, path: &str) -> bool {
        let key = self.build_object_path(path);
        let bucket = self.bucket.clone();
        let key_str = key.to_string();
        let client = self.client.clone();

        let result = self.runtime.block_on(async move {
            use google_cloud_storage::http::objects::get::GetObjectRequest;

            let req = GetObjectRequest {
                bucket: bucket.clone(),
                object: key_str.clone(),
                ..Default::default()
            };

            // Use head request equivalent to check existence
            client.get_object(&req).await
        });

        result.is_ok()
    }

    /// Delete a snapshot from GCS
    fn delete(&self, path: &str) -> Result<()> {
        #[cfg(feature = "metrics")]
        let _timer = MetricsTimer::start_gcs_operation("delete");

        let key = self.build_object_path(path);
        info!(bucket=%self.bucket, key=%key, "Deleting snapshot from GCS");

        let bucket = self.bucket.clone();
        let key_str = key.to_string();
        let client = self.client.clone();

        let result = self.runtime.block_on(async move {
            use google_cloud_storage::http::objects::delete::DeleteObjectRequest;

            let req = DeleteObjectRequest {
                bucket: bucket.clone(),
                object: key_str.clone(),
                ..Default::default()
            };

            client.delete_object(&req).await
        });

        match result {
            Ok(_) => {
                debug!(
                    "Successfully deleted snapshot from gs://{}/{}",
                    self.bucket, key
                );
                #[cfg(feature = "metrics")]
                crate::observability::PersistMetrics::global().record_gcs_request("delete");
                Ok(())
            }
            Err(e) => {
                let err = map_gcs_error("delete_object", &e, &key);
                error!(bucket=%self.bucket, key=%key, error=?err, "Failed to delete snapshot from GCS");
                #[cfg(feature = "metrics")]
                crate::observability::PersistMetrics::global().record_gcs_error("delete");
                Err(err)
            }
        }
    }

    // Note: Streaming upload/download methods will be added in a future update
    // when the async trait architecture is properly implemented
}

/// Check if a GCS error is retryable
#[cfg(feature = "gcs")]
fn is_retryable_error(error: &google_cloud_storage::http::Error) -> bool {
    // For now, implement basic retry logic for common transient errors
    // This could be expanded to check specific error codes
    match error {
        // Network-related errors are usually retryable
        _ if error.to_string().contains("timeout") => true,
        _ if error.to_string().contains("connection") => true,
        _ if error.to_string().contains("network") => true,
        // Server errors (5xx) are retryable, client errors (4xx) are not
        _ if error.to_string().contains("500") => true,
        _ if error.to_string().contains("502") => true,
        _ if error.to_string().contains("503") => true,
        _ if error.to_string().contains("504") => true,
        _ => false,
    }
}

/// Map GCS errors to PersistError with comprehensive error classification
#[cfg(feature = "gcs")]
fn map_gcs_error(
    operation: &str,
    error: &google_cloud_storage::http::Error,
    key: &str,
) -> PersistError {
    let error_str = error.to_string();

    // Map specific HTTP status codes to appropriate PersistError types
    if error_str.contains("404") || error_str.contains("not found") {
        // Object not found - use storage error with clear message
        PersistError::storage(format!("GCS object not found: {key}"))
    } else if error_str.contains("403") || error_str.contains("401") {
        // Permission/authentication errors - use storage error with clear message
        PersistError::storage(format!("GCS permission denied for object '{key}': Ensure you have proper IAM permissions. Error: {error}"))
    } else if error_str.contains("409") {
        // Conflict - object already exists in some cases
        PersistError::storage(format!("GCS conflict for object '{key}': {error}"))
    } else if error_str.contains("412") {
        // Precondition failed
        PersistError::storage(format!("GCS precondition failed for object '{key}': {error}"))
    } else if error_str.contains("429") {
        // Rate limited - mark as transient but use storage error for now
        PersistError::storage(format!("GCS rate limit exceeded for object '{key}' (transient error): {error}"))
    } else if error_str.contains("499") 
        || error_str.contains("500") 
        || error_str.contains("502") 
        || error_str.contains("503") 
        || error_str.contains("504") {
        // Server errors - mark as transient but use storage error for now  
        PersistError::storage(format!("GCS server error for object '{key}' (transient error): {error}"))
    } else if error_str.contains("timeout") 
        || error_str.contains("connection") 
        || error_str.contains("network") {
        // Network-related errors - mark as transient but use storage error for now
        PersistError::storage(format!("GCS network error for object '{key}' (transient error): {error}"))
    } else {
        // Generic storage error for anything else
        PersistError::storage(format!("GCS {operation} error for object '{key}': {error}"))
    }
}

// When GCS feature is disabled, provide a stub implementation
#[cfg(not(feature = "gcs"))]
pub struct GCSStorageAdapter;

#[cfg(not(feature = "gcs"))]
impl GCSStorageAdapter {
    pub fn new(_bucket: impl Into<String>, _prefix: Option<String>, _creds_json: Option<std::path::PathBuf>) -> Result<Self> {
        Err(PersistError::storage(
            "GCS support not enabled. Recompile with --features gcs".to_string(),
        ))
    }
}
