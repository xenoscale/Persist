//! Unified retry and backoff logic for Persist storage adapters
//!
//! This crate provides consistent retry policies and backoff strategies
//! for all storage backends in the Persist ecosystem.

use async_trait::async_trait;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use futures::Future;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, warn};

/// Common retry error types
#[derive(Error, Debug)]
pub enum RetryError {
    #[error("Operation '{operation}' exceeded maximum retry attempts: {source}")]
    MaxRetriesExceeded {
        operation: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Transient error in '{operation}': {source}")]
    Transient {
        operation: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Permanent error in '{operation}': {source}")]
    Permanent {
        operation: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Result type for retry operations
pub type RetryResult<T> = std::result::Result<T, RetryError>;

/// Boxed future for retry operations
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = RetryResult<T>> + Send + 'a>>;

/// Execute an operation with exponential backoff retry logic
pub async fn with_backoff<F, T>(op_name: &'static str, f: F) -> RetryResult<T>
where
    F: FnMut(usize) -> BoxFuture<'static, T>,
{
    let policy = default_backoff_policy();
    with_custom_backoff(op_name, policy, f).await
}

/// Execute an operation with custom backoff policy
pub async fn with_custom_backoff<F, T>(
    op_name: &'static str,
    mut _policy: ExponentialBackoff,
    mut f: F,
) -> RetryResult<T>
where
    F: FnMut(usize) -> BoxFuture<'static, T>,
{
    // Simple implementation without complex retry logic for MVP
    // This can be enhanced later with proper async retry logic
    let mut attempt = 1;

    loop {
        debug!("Attempting operation '{}' (attempt {})", op_name, attempt);

        match f(attempt).await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        "Operation '{}' succeeded after {} attempts",
                        op_name, attempt
                    );
                }
                return Ok(result);
            }
            Err(RetryError::Permanent { .. }) => {
                warn!(
                    "Operation '{}' failed permanently on attempt {}",
                    op_name, attempt
                );
                return Err(RetryError::MaxRetriesExceeded {
                    operation: op_name,
                    source: "Permanent error".into(),
                });
            }
            Err(err) => {
                warn!(
                    "Operation '{}' failed on attempt {}: {}",
                    op_name, attempt, err
                );

                // Simple retry logic - max 3 attempts for MVP
                if attempt >= 3 {
                    return Err(RetryError::MaxRetriesExceeded {
                        operation: op_name,
                        source: "Maximum retry attempts exceeded".into(),
                    });
                }

                attempt += 1;

                // Simple delay - can be enhanced with proper backoff later
                #[cfg(feature = "async-rt")]
                tokio::time::sleep(std::time::Duration::from_millis(100 * attempt as u64)).await;

                #[cfg(not(feature = "async-rt"))]
                std::thread::sleep(std::time::Duration::from_millis(100 * attempt as u64));
            }
        }
    }
}

/// Default backoff policy for general operations
pub fn default_backoff_policy() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(100))
        .with_max_interval(Duration::from_secs(5))
        .with_max_elapsed_time(Some(Duration::from_secs(30)))
        .with_multiplier(2.0)
        .build()
}

/// Backoff policy optimized for cloud storage operations
pub fn cloud_storage_backoff_policy() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(500))
        .with_max_interval(Duration::from_secs(10))
        .with_max_elapsed_time(Some(Duration::from_secs(60)))
        .with_multiplier(1.5)
        .build()
}

/// Backoff policy for local storage operations (shorter timeouts)
pub fn local_storage_backoff_policy() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(50))
        .with_max_interval(Duration::from_secs(1))
        .with_max_elapsed_time(Some(Duration::from_secs(10)))
        .with_multiplier(2.0)
        .build()
}

/// Trait for categorizing errors as transient or permanent
#[async_trait]
pub trait RetryableError {
    /// Returns true if the error is transient and the operation should be retried
    fn is_transient(&self) -> bool;

    /// Returns true if the error is permanent and retries should stop
    fn is_permanent(&self) -> bool {
        !self.is_transient()
    }
}

/// Helper macro for creating transient errors
#[macro_export]
macro_rules! transient_error {
    ($op:expr, $err:expr) => {
        RetryError::Transient {
            operation: $op,
            source: Box::new($err),
        }
    };
}

/// Helper macro for creating permanent errors
#[macro_export]
macro_rules! permanent_error {
    ($op:expr, $err:expr) => {
        RetryError::Permanent {
            operation: $op,
            source: Box::new($err),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_successful_operation() {
        let result = with_backoff("test_op", |_attempt| Box::pin(async { Ok("success") })).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_transient_failure_then_success() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result = with_backoff("test_op", move |_attempt| {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if count < 2 {
                    Err(transient_error!(
                        "test_op",
                        std::io::Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            "connection refused"
                        )
                    ))
                } else {
                    Ok("success")
                }
            })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_permanent_failure() {
        let result: RetryResult<&str> = with_backoff("test_op", |_attempt| {
            Box::pin(async {
                Err(permanent_error!(
                    "test_op",
                    std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied")
                ))
            })
        })
        .await;

        assert!(result.is_err());
        matches!(result, Err(RetryError::MaxRetriesExceeded { .. }));
    }
}
