/*!
Google Cloud Storage (GCS) storage adapter implementation.

This module provides GCS cloud storage support for snapshots using the official Google Cloud Storage client.
*/

use google_cloud_storage::client::{Client as GcsClient, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};

use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};

use super::StorageAdapter;
#[cfg(feature = "metrics")]
use crate::observability::MetricsTimer;
use crate::{PersistError, Result};

/// Google Cloud Storage adapter
///
/// This implementation stores snapshots as objects in Google Cloud Storage.
/// It uses the official Google Cloud Storage client and supports standard GCP credential providers.
///
/// # Authentication
/// The adapter uses the standard GCP credential provider chain:
/// 1. GOOGLE_APPLICATION_CREDENTIALS environment variable pointing to service account JSON
/// 2. Default application credentials (ADC) from environment
/// 3. GCE/GKE metadata service credentials
///
/// # Example
/// ```rust,no_run
/// use persist_core::{storage::GCSStorageAdapter, StorageAdapter};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Set environment variables:
/// // export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
///
/// let adapter = GCSStorageAdapter::new("my-snapshots-bucket".to_string(), None)?;
/// let data = b"compressed snapshot data";
/// adapter.save(data, "agent1/session1/snapshot.json.gz")?;
/// # Ok(())
/// # }
/// ```
pub struct GCSStorageAdapter {
    client: GcsClient,
    bucket: String,
    runtime: Arc<Runtime>,
}

impl GCSStorageAdapter {
    /// Create a new GCS storage adapter for the specified bucket
    ///
    /// # Arguments
    /// * `bucket` - The GCS bucket name to use for storage
    /// * `credentials_path` - Optional path to service account JSON file
    ///
    /// # Returns
    /// A new GCSStorageAdapter instance or an error if initialization fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - GCP credentials are not available
    /// - The Tokio runtime cannot be created
    /// - GCP configuration cannot be loaded
    pub fn new(bucket: String, credentials_path: Option<String>) -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| {
            PersistError::storage(format!(
                "Failed to create async runtime for GCS client: {e}"
            ))
        })?;

        // Load GCP configuration with authentication
        let config = runtime
            .block_on(async {
                if let Some(path) = credentials_path {
                    // Set the GOOGLE_APPLICATION_CREDENTIALS environment variable temporarily
                    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &path);
                }
                // Use default ADC (Application Default Credentials) from env or metadata
                ClientConfig::default().with_auth().await
            })
            .map_err(|e| PersistError::storage(format!("GCS client initialization failed: {e}")))?;

        // Create the GCS client
        let client = GcsClient::new(config);

        info!(bucket = %bucket, "Initialized GCS storage adapter");

        Ok(GCSStorageAdapter {
            client,
            bucket,
            runtime: Arc::new(runtime),
        })
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Perform GCS save operation with retry logic
    fn save_with_retry(&self, data: &[u8], key: &str) -> Result<()> {
        let max_attempts = 3;
        let mut attempts = 0;

        loop {
            attempts += 1;
            match self.save_once(data, key) {
                Ok(()) => return Ok(()),
                Err(e) if attempts < max_attempts && is_transient_error(&e) => {
                    warn!(
                        attempt = attempts,
                        max_attempts = max_attempts,
                        bucket = %self.bucket,
                        key = %key,
                        error = %e,
                        "GCS save attempt failed, retrying..."
                    );
                    // Record retry metric
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global()
                        .record_gcs_retry("upload_object");

                    // Simple backoff - could be enhanced with exponential backoff
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Perform a single GCS save operation
    #[tracing::instrument(level = "debug", skip(self, data), fields(bucket = %self.bucket, key = %key, size = data.len()))]
    fn save_once(&self, data: &[u8], key: &str) -> Result<()> {
        #[cfg(feature = "metrics")]
        let timer = MetricsTimer::new("upload_object");

        debug!(
            bucket = %self.bucket,
            key = %key,
            size = data.len(),
            "Starting GCS upload_object operation"
        );

        // Convert to owned values to avoid lifetime issues in async block
        let key_owned = key.to_string();
        let data_owned = data.to_vec();
        let bucket = self.bucket.clone();
        let client = self.client.clone();

        let result = self.runtime.block_on(async move {
            let upload_type = UploadType::Simple(Media::new(key_owned.clone()));
            let req = UploadObjectRequest {
                bucket,
                ..Default::default()
            };

            client.upload_object(&req, data_owned, &upload_type).await
        });

        match result {
            Ok(_) => {
                debug!(
                    bucket = %self.bucket,
                    key = %key,
                    size = data.len(),
                    "Successfully saved snapshot to GCS"
                );
                #[cfg(feature = "metrics")]
                timer.finish();
                Ok(())
            }
            Err(e) => {
                let mapped_error = map_gcs_error("upload_object", &e, key, &self.bucket);
                error!(
                    bucket = %self.bucket,
                    key = %key,
                    error = ?mapped_error,
                    "Failed to save snapshot to GCS"
                );
                #[cfg(feature = "metrics")]
                timer.finish_with_error();
                Err(mapped_error)
            }
        }
    }

    /// Perform GCS load operation with retry logic
    fn load_with_retry(&self, key: &str) -> Result<Vec<u8>> {
        let max_attempts = 3;
        let mut attempts = 0;

        loop {
            attempts += 1;
            match self.load_once(key) {
                Ok(data) => return Ok(data),
                Err(e) if attempts < max_attempts && is_transient_error(&e) => {
                    warn!(
                        attempt = attempts,
                        max_attempts = max_attempts,
                        bucket = %self.bucket,
                        key = %key,
                        error = %e,
                        "GCS load attempt failed, retrying..."
                    );
                    // Record retry metric
                    #[cfg(feature = "metrics")]
                    crate::observability::PersistMetrics::global()
                        .record_gcs_retry("download_object");

                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Perform a single GCS load operation
    #[tracing::instrument(level = "debug", skip(self), fields(bucket = %self.bucket, key = %key))]
    fn load_once(&self, key: &str) -> Result<Vec<u8>> {
        #[cfg(feature = "metrics")]
        let timer = MetricsTimer::new("download_object");

        debug!(
            bucket = %self.bucket,
            key = %key,
            "Starting GCS download_object operation"
        );

        // Convert to owned values to avoid lifetime issues in async block
        let bucket = self.bucket.clone();
        let object = key.to_string();
        let client = self.client.clone();

        let result = self.runtime.block_on(async move {
            let req = GetObjectRequest {
                bucket,
                object,
                ..Default::default()
            };

            client.download_object(&req, &Range::default()).await
        });

        match result {
            Ok(data) => {
                debug!(
                    bucket = %self.bucket,
                    key = %key,
                    size = data.len(),
                    "Successfully loaded snapshot from GCS"
                );
                #[cfg(feature = "metrics")]
                timer.finish();
                Ok(data)
            }
            Err(e) => {
                let mapped_error = map_gcs_error("download_object", &e, key, &self.bucket);
                error!(
                    bucket = %self.bucket,
                    key = %key,
                    error = ?mapped_error,
                    "Failed to load snapshot from GCS"
                );
                #[cfg(feature = "metrics")]
                timer.finish_with_error();
                Err(mapped_error)
            }
        }
    }
}

impl StorageAdapter for GCSStorageAdapter {
    #[tracing::instrument(level = "info", skip(self, data), fields(bucket = %self.bucket, key = %path, size = data.len()))]
    fn save(&self, data: &[u8], path: &str) -> Result<()> {
        info!(
            bucket = %self.bucket,
            key = %path,
            size = data.len(),
            "Saving snapshot to GCS"
        );

        // Record state size metric
        #[cfg(feature = "metrics")]
        crate::observability::PersistMetrics::global().record_state_size(data.len());

        self.save_with_retry(data, path)
    }

    #[tracing::instrument(level = "info", skip(self), fields(bucket = %self.bucket, key = %path))]
    fn load(&self, path: &str) -> Result<Vec<u8>> {
        info!(
            bucket = %self.bucket,
            key = %path,
            "Loading snapshot from GCS"
        );
        self.load_with_retry(path)
    }

    fn exists(&self, path: &str) -> bool {
        debug!(
            bucket = %self.bucket,
            key = %path,
            "Checking if GCS object exists"
        );

        let result = self.runtime.block_on(async {
            let req = GetObjectRequest {
                bucket: self.bucket.clone(),
                object: path.to_string(),
                ..Default::default()
            };

            // Use a HEAD-style request by trying to get object metadata
            self.client.get_object(&req).await
        });

        let exists = result.is_ok();
        debug!(
            bucket = %self.bucket,
            key = %path,
            exists = exists,
            "GCS object existence check completed"
        );
        exists
    }

    fn delete(&self, path: &str) -> Result<()> {
        info!(
            bucket = %self.bucket,
            key = %path,
            "Deleting snapshot from GCS"
        );

        let result = self.runtime.block_on(async {
            use google_cloud_storage::http::objects::delete::DeleteObjectRequest;

            let req = DeleteObjectRequest {
                bucket: self.bucket.clone(),
                object: path.to_string(),
                ..Default::default()
            };

            self.client.delete_object(&req).await
        });

        match result {
            Ok(_) => {
                debug!(
                    bucket = %self.bucket,
                    key = %path,
                    "Successfully deleted snapshot from GCS"
                );
                Ok(())
            }
            Err(e) => {
                let mapped_error = map_gcs_error("delete_object", &e, path, &self.bucket);
                error!(
                    bucket = %self.bucket,
                    key = %path,
                    error = ?mapped_error,
                    "Failed to delete snapshot from GCS"
                );
                Err(mapped_error)
            }
        }
    }
}

/// Map GCS errors to PersistError with appropriate context
fn map_gcs_error(
    op: &str,
    error: &google_cloud_storage::http::Error,
    key: &str,
    bucket: &str,
) -> PersistError {
    let error_msg = format!("GCS {op} error for gs://{bucket}/{key}: {error}");

    // For now, just return a generic error message
    // TODO: Improve error handling based on actual error types
    PersistError::storage(error_msg)
}

/// Check if an error is transient and should be retried
fn is_transient_error(error: &PersistError) -> bool {
    match error {
        PersistError::Storage(msg) => {
            // Retry on network/timeout issues and server errors
            msg.contains("timed out")
                || msg.contains("connection")
                || msg.contains("network")
                || msg.contains("503")
                || msg.contains("502")
                || msg.contains("500")
                || msg.contains("timeout")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcs_adapter_creation() {
        // This test is environment-dependent and may pass or fail based on GCP credentials
        // In CI environments, credentials might be available, making this test unreliable

        let result = GCSStorageAdapter::new("test-bucket".to_string(), None);

        // Accept both success and failure cases since this depends on environment
        match result {
            Ok(_adapter) => {
                // GCS adapter created successfully (credentials available)
                println!("GCS adapter creation succeeded - credentials available");
            }
            Err(PersistError::Storage(msg)) => {
                // Expected error case when credentials are missing
                assert!(
                    msg.contains("GCS client initialization failed")
                        || msg.contains("Failed to create")
                );
            }
            Err(e) => {
                panic!("Unexpected error type: {e:?}");
            }
        }
    }

    #[test]
    fn test_is_transient_error() {
        let timeout_error = PersistError::storage("GCS download_object timed out (key: test)");
        assert!(is_transient_error(&timeout_error));

        let network_error = PersistError::storage("GCS upload failed: connection error");
        assert!(is_transient_error(&network_error));

        let server_error = PersistError::storage("GCS server error (503): Service unavailable");
        assert!(is_transient_error(&server_error));

        let auth_error = PersistError::storage("GCS access denied for bucket");
        assert!(!is_transient_error(&auth_error));

        let other_error = PersistError::validation("Invalid input");
        assert!(!is_transient_error(&other_error));
    }

    #[test]
    fn test_bucket_getter() {
        // Test with a mock bucket name since we can't test actual GCS without credentials
        let bucket_name = "test-bucket-name";

        // This will likely fail without credentials, but we just want to test
        // that if creation succeeds, the bucket name is correct
        if let Ok(adapter) = GCSStorageAdapter::new(bucket_name.to_string(), None) {
            assert_eq!(adapter.bucket(), bucket_name);
        }
    }
}
