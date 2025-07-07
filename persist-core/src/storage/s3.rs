/*!
Amazon S3 storage adapter implementation.

This module provides S3 cloud storage support for snapshots using the official AWS SDK.
*/

use aws_config::SdkConfig;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::error::ProvideErrorMetadata;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};

use crate::{PersistError, Result};
use super::StorageAdapter;

/// Amazon S3 storage adapter
///
/// This implementation stores snapshots as objects in Amazon S3.
/// It uses the official AWS SDK and supports standard AWS credential providers.
///
/// # Authentication
/// The adapter uses the standard AWS credential provider chain:
/// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_SESSION_TOKEN)
/// 2. AWS credentials file (~/.aws/credentials)
/// 3. IAM roles for EC2 instances
/// 4. ECS task roles
///
/// # Example
/// ```rust,no_run
/// use persist_core::storage::S3StorageAdapter;
/// 
/// // Set environment variables:
/// // export AWS_ACCESS_KEY_ID=your_access_key
/// // export AWS_SECRET_ACCESS_KEY=your_secret_key
/// // export AWS_REGION=us-west-2
/// 
/// let adapter = S3StorageAdapter::new("my-snapshots-bucket".to_string())?;
/// let data = b"compressed snapshot data";
/// adapter.save(data, "agent1/session1/snapshot.json.gz")?;
/// # Ok::<(), persist_core::PersistError>(())
/// ```
#[derive(Debug)]
pub struct S3StorageAdapter {
    client: S3Client,
    bucket: String,
    runtime: Arc<Runtime>,
}

impl S3StorageAdapter {
    /// Create a new S3 storage adapter for the specified bucket
    ///
    /// # Arguments
    /// * `bucket` - The S3 bucket name to use for storage
    ///
    /// # Returns
    /// A new S3StorageAdapter instance or an error if initialization fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - AWS credentials are not available
    /// - The Tokio runtime cannot be created
    /// - AWS configuration cannot be loaded
    pub fn new(bucket: String) -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PersistError::storage(format!(
                "Failed to create async runtime for S3 client: {}", e
            )))?;

        // Load AWS configuration from environment
        let sdk_config = runtime.block_on(async {
            aws_config::defaults(aws_config::BehaviorVersion::latest()).load().await
        });

        // Validate that we have credentials
        if sdk_config.credentials_provider().is_none() {
            return Err(PersistError::storage(
                "AWS credentials not found. Please set AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, and AWS_REGION environment variables".to_string()
            ));
        }

        let client = S3Client::new(&sdk_config);
        
        info!(bucket = %bucket, "Initialized S3 storage adapter");
        
        Ok(S3StorageAdapter {
            client,
            bucket,
            runtime: Arc::new(runtime),
        })
    }

    /// Create a new S3 storage adapter with explicit AWS configuration
    ///
    /// # Arguments
    /// * `bucket` - The S3 bucket name to use for storage
    /// * `config` - The AWS SDK configuration
    ///
    /// # Returns
    /// A new S3StorageAdapter instance or an error if initialization fails
    pub fn with_config(bucket: String, config: SdkConfig) -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PersistError::storage(format!(
                "Failed to create async runtime for S3 client: {}", e
            )))?;

        let client = S3Client::new(&config);
        
        info!(bucket = %bucket, "Initialized S3 storage adapter with custom config");
        
        Ok(S3StorageAdapter {
            client,
            bucket,
            runtime: Arc::new(runtime),
        })
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Perform S3 save operation with retry logic
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
                        "S3 save attempt failed, retrying..."
                    );
                    // Simple backoff - could be enhanced with exponential backoff
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Perform a single S3 save operation
    fn save_once(&self, data: &[u8], key: &str) -> Result<()> {
        debug!(
            bucket = %self.bucket,
            key = %key,
            size = data.len(),
            "Starting S3 put_object operation"
        );

        let result = self.runtime.block_on(async {
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(ByteStream::from(data.to_vec()))
                .send()
                .await
        });

        match result {
            Ok(_) => {
                debug!(
                    bucket = %self.bucket,
                    key = %key,
                    size = data.len(),
                    "Successfully saved snapshot to S3"
                );
                Ok(())
            }
            Err(e) => {
                let mapped_error = map_s3_error("put_object", e, key);
                error!(
                    bucket = %self.bucket,
                    key = %key,
                    error = ?mapped_error,
                    "Failed to save snapshot to S3"
                );
                Err(mapped_error)
            }
        }
    }

    /// Perform S3 load operation with retry logic
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
                        "S3 load attempt failed, retrying..."
                    );
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Perform a single S3 load operation
    fn load_once(&self, key: &str) -> Result<Vec<u8>> {
        debug!(
            bucket = %self.bucket,
            key = %key,
            "Starting S3 get_object operation"
        );

        let result = self.runtime.block_on(async {
            self.client
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
        });

        match result {
            Ok(output) => {
                // Collect the response body stream into bytes
                let bytes_result = self.runtime.block_on(async {
                    output.body.collect().await
                });

                match bytes_result {
                    Ok(data) => {
                        let bytes = data.into_bytes().to_vec();
                        debug!(
                            bucket = %self.bucket,
                            key = %key,
                            size = bytes.len(),
                            "Successfully loaded snapshot from S3"
                        );
                        Ok(bytes)
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to read S3 object stream: {}", e);
                        error!(bucket = %self.bucket, key = %key, error = %error_msg);
                        Err(PersistError::storage(error_msg))
                    }
                }
            }
            Err(e) => {
                let mapped_error = map_s3_error("get_object", e, key);
                error!(
                    bucket = %self.bucket,
                    key = %key,
                    error = ?mapped_error,
                    "Failed to load snapshot from S3"
                );
                Err(mapped_error)
            }
        }
    }
}

impl StorageAdapter for S3StorageAdapter {
    fn save(&self, data: &[u8], path: &str) -> Result<()> {
        info!(
            bucket = %self.bucket,
            key = %path,
            size = data.len(),
            "Saving snapshot to S3"
        );
        self.save_with_retry(data, path)
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        info!(
            bucket = %self.bucket,
            key = %path,
            "Loading snapshot from S3"
        );
        self.load_with_retry(path)
    }

    fn exists(&self, path: &str) -> bool {
        debug!(
            bucket = %self.bucket,
            key = %path,
            "Checking if S3 object exists"
        );

        let result = self.runtime.block_on(async {
            self.client
                .head_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
        });

        let exists = result.is_ok();
        debug!(
            bucket = %self.bucket,
            key = %path,
            exists = exists,
            "S3 object existence check completed"
        );
        exists
    }

    fn delete(&self, path: &str) -> Result<()> {
        info!(
            bucket = %self.bucket,
            key = %path,
            "Deleting snapshot from S3"
        );

        let result = self.runtime.block_on(async {
            self.client
                .delete_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
        });

        match result {
            Ok(_) => {
                debug!(
                    bucket = %self.bucket,
                    key = %path,
                    "Successfully deleted snapshot from S3"
                );
                Ok(())
            }
            Err(e) => {
                let mapped_error = map_s3_error("delete_object", e, path);
                error!(
                    bucket = %self.bucket,
                    key = %path,
                    error = ?mapped_error,
                    "Failed to delete snapshot from S3"
                );
                Err(mapped_error)
            }
        }
    }
}

/// Map AWS SDK errors to PersistError with appropriate context
fn map_s3_error<E: ProvideErrorMetadata + std::fmt::Debug>(op: &str, error: aws_sdk_s3::error::SdkError<E>, key: &str) -> PersistError {
    use aws_sdk_s3::error::SdkError;

    match &error {
        SdkError::DispatchFailure(dispatch_err) => {
            let msg = format!("S3 {} request failed to dispatch: {:?}", op, dispatch_err);
            PersistError::storage(msg)
        }
        SdkError::TimeoutError(_) => {
            let msg = format!("S3 {} request timed out (key: {})", op, key);
            PersistError::storage(msg)
        }
        SdkError::ResponseError(response_err) => {
            let msg = format!("S3 {} response error: {:?}", op, response_err);
            PersistError::storage(msg)
        }
        SdkError::ServiceError(service_err) => {
            if let Some(code) = service_err.err().code() {
                match code {
                    "NoSuchBucket" => {
                        PersistError::storage(format!("S3 bucket not found"))
                    }
                    "NoSuchKey" => {
                        PersistError::storage(format!("S3 object '{}' not found", key))
                    }
                    "AccessDenied" | "Forbidden" => {
                        PersistError::storage("Access denied to S3 (check credentials and permissions)".to_string())
                    }
                    "InvalidBucketName" => {
                        PersistError::storage("Invalid S3 bucket name".to_string())
                    }
                    _ => {
                        let msg = format!(
                            "S3 service error ({}): {}", 
                            code, 
                            service_err.err().message().unwrap_or("Unknown error")
                        );
                        PersistError::storage(msg)
                    }
                }
            } else {
                PersistError::storage(format!("S3 {} service error: {:?}", op, service_err))
            }
        }
        _ => {
            PersistError::storage(format!("S3 {} error: {}", op, error))
        }
    }
}

/// Check if an error is transient and should be retried
fn is_transient_error(error: &PersistError) -> bool {
    match error {
        PersistError::Storage(msg) => {
            // Retry on network/timeout issues
            msg.contains("timed out") 
                || msg.contains("dispatch") 
                || msg.contains("InternalError")
                || msg.contains("503")
                || msg.contains("502")
                || msg.contains("500")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;

    mock! {
        S3Client {
            async fn put_object(&self, bucket: &str, key: &str, data: &[u8]) -> Result<(), String>;
            async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, String>;
            async fn head_object(&self, bucket: &str, key: &str) -> Result<(), String>;
            async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), String>;
        }
    }

    #[test]
    fn test_s3_adapter_creation() {
        // This test requires AWS credentials to be set up
        // In a real test environment, you would mock the AWS config loading
        // For now, we'll test the error case when credentials are missing
        
        // Clear AWS environment variables for this test
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        std::env::remove_var("AWS_REGION");
        
        let result = S3StorageAdapter::new("test-bucket".to_string());
        
        // Should fail due to missing credentials
        assert!(result.is_err());
        if let Err(PersistError::Storage(msg)) = result {
            assert!(msg.contains("AWS credentials not found"));
        } else {
            panic!("Expected storage error for missing credentials");
        }
    }

    #[test]
    fn test_error_mapping() {
        use aws_sdk_s3::error::SdkError;
        use aws_sdk_s3::operation::get_object::{GetObjectError, GetObjectOutput};
        
        // Test timeout error mapping
        let timeout_error = SdkError::TimeoutError(Box::new(aws_smithy_runtime_api::client::timeout::TimeoutError::new()));
        let mapped = map_s3_error("get_object", timeout_error, "test-key");
        
        if let PersistError::Storage(msg) = mapped {
            assert!(msg.contains("timed out"));
            assert!(msg.contains("test-key"));
        } else {
            panic!("Expected storage error for timeout");
        }
    }

    #[test]
    fn test_is_transient_error() {
        let timeout_error = PersistError::storage("S3 get_object request timed out (key: test)");
        assert!(is_transient_error(&timeout_error));
        
        let dispatch_error = PersistError::storage("S3 put_object request failed to dispatch");
        assert!(is_transient_error(&dispatch_error));
        
        let auth_error = PersistError::storage("Access denied to S3");
        assert!(!is_transient_error(&auth_error));
        
        let other_error = PersistError::validation("Invalid input");
        assert!(!is_transient_error(&other_error));
    }
}

// Include tests module
#[cfg(test)]
mod s3_tests;
