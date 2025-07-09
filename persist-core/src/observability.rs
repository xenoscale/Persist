/*!
Observability infrastructure for the Persist system.

This module provides comprehensive observability features including:
- Structured logging and tracing setup
- Prometheus metrics instrumentation
- Trace exporters (Jaeger, console)
*/

#[cfg(feature = "metrics")]
use prometheus::{Counter, Encoder, Histogram, Registry, TextEncoder};
#[cfg(feature = "metrics")]
use std::sync::OnceLock;
#[cfg(feature = "metrics")]
use std::time::Instant;
use tracing::subscriber::set_global_default;
// use tracing_opentelemetry::OpenTelemetryLayer; // Temporarily disabled
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry as TracingRegistry};

use crate::{PersistError, Result};

/// Global metrics instance
#[cfg(feature = "metrics")]
static METRICS: OnceLock<PersistMetrics> = OnceLock::new();

/// Metrics collection for Persist operations
#[cfg(feature = "metrics")]
#[derive(Debug)]
pub struct PersistMetrics {
    // S3 operation metrics
    pub s3_requests_total: Counter,
    pub s3_errors_total: Counter,
    pub s3_latency_seconds: Histogram,
    pub s3_retries_total: Counter,

    // GCS operation metrics
    pub gcs_requests_total: Counter,
    pub gcs_errors_total: Counter,
    pub gcs_latency_seconds: Histogram,
    pub gcs_retries_total: Counter,

    // State size metrics
    pub state_size_bytes: Histogram,

    // Prometheus registry for scraping
    registry: Registry,
}

#[cfg(feature = "metrics")]
impl PersistMetrics {
    /// Initialize new metrics instance
    fn new() -> Result<Self> {
        // Create Prometheus registry
        let registry = Registry::new();

        // Initialize metrics
        let s3_requests_total = Counter::new(
            "persist_s3_requests_total",
            "Total S3 requests made by Persist",
        )
        .map_err(|e| {
            PersistError::storage(format!("Failed to create s3_requests_total metric: {e}"))
        })?;

        let s3_errors_total = Counter::new(
            "persist_s3_errors_total",
            "Total S3 request errors in Persist",
        )
        .map_err(|e| {
            PersistError::storage(format!("Failed to create s3_errors_total metric: {e}"))
        })?;

        let s3_latency_seconds = Histogram::with_opts(prometheus::HistogramOpts::new(
            "persist_s3_latency_seconds",
            "Duration of S3 operations in seconds",
        ))
        .map_err(|e| {
            PersistError::storage(format!("Failed to create s3_latency_seconds metric: {e}"))
        })?;

        let s3_retries_total = Counter::new(
            "persist_s3_retries_total",
            "Total S3 retry attempts in Persist",
        )
        .map_err(|e| {
            PersistError::storage(format!("Failed to create s3_retries_total metric: {e}"))
        })?;

        // Initialize GCS metrics
        let gcs_requests_total = Counter::new(
            "persist_gcs_requests_total",
            "Total GCS requests made by Persist",
        )
        .map_err(|e| {
            PersistError::storage(format!("Failed to create gcs_requests_total metric: {e}"))
        })?;

        let gcs_errors_total = Counter::new(
            "persist_gcs_errors_total",
            "Total GCS request errors in Persist",
        )
        .map_err(|e| {
            PersistError::storage(format!("Failed to create gcs_errors_total metric: {e}"))
        })?;

        let gcs_latency_seconds = Histogram::with_opts(prometheus::HistogramOpts::new(
            "persist_gcs_latency_seconds",
            "Duration of GCS operations in seconds",
        ))
        .map_err(|e| {
            PersistError::storage(format!("Failed to create gcs_latency_seconds metric: {e}"))
        })?;

        let gcs_retries_total = Counter::new(
            "persist_gcs_retries_total",
            "Total GCS retry attempts in Persist",
        )
        .map_err(|e| {
            PersistError::storage(format!("Failed to create gcs_retries_total metric: {e}"))
        })?;

        let state_size_bytes = Histogram::with_opts(prometheus::HistogramOpts::new(
            "persist_state_size_bytes",
            "Size of agent state in bytes",
        ))
        .map_err(|e| {
            PersistError::storage(format!("Failed to create state_size_bytes metric: {e}"))
        })?;

        // Register metrics with the registry
        registry
            .register(Box::new(s3_requests_total.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register s3_requests_total: {e}"))
            })?;

        registry
            .register(Box::new(s3_errors_total.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register s3_errors_total: {e}"))
            })?;

        registry
            .register(Box::new(s3_latency_seconds.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register s3_latency_seconds: {e}"))
            })?;

        registry
            .register(Box::new(s3_retries_total.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register s3_retries_total: {e}"))
            })?;

        registry
            .register(Box::new(state_size_bytes.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register state_size_bytes: {e}"))
            })?;

        // Register GCS metrics
        registry
            .register(Box::new(gcs_requests_total.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register gcs_requests_total: {e}"))
            })?;

        registry
            .register(Box::new(gcs_errors_total.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register gcs_errors_total: {e}"))
            })?;

        registry
            .register(Box::new(gcs_latency_seconds.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register gcs_latency_seconds: {e}"))
            })?;

        registry
            .register(Box::new(gcs_retries_total.clone()))
            .map_err(|e| {
                PersistError::storage(format!("Failed to register gcs_retries_total: {e}"))
            })?;

        Ok(Self {
            s3_requests_total,
            s3_errors_total,
            s3_latency_seconds,
            s3_retries_total,
            gcs_requests_total,
            gcs_errors_total,
            gcs_latency_seconds,
            gcs_retries_total,
            state_size_bytes,
            registry,
        })
    }

    /// Get or initialize global metrics instance
    pub fn global() -> &'static PersistMetrics {
        METRICS.get_or_init(|| Self::new().expect("Failed to initialize Persist metrics"))
    }

    /// Record an S3 request
    pub fn record_s3_request(&self, _operation: &str) {
        self.s3_requests_total.inc();
    }

    /// Record an S3 error
    pub fn record_s3_error(&self, _operation: &str) {
        self.s3_errors_total.inc();
    }

    /// Record S3 operation latency
    pub fn record_s3_latency(&self, _operation: &str, duration: std::time::Duration) {
        self.s3_latency_seconds.observe(duration.as_secs_f64());
    }

    /// Record an S3 retry
    pub fn record_s3_retry(&self, _operation: &str) {
        self.s3_retries_total.inc();
    }

    /// Record a GCS request
    pub fn record_gcs_request(&self, _operation: &str) {
        self.gcs_requests_total.inc();
    }

    /// Record a GCS error
    pub fn record_gcs_error(&self, _operation: &str) {
        self.gcs_errors_total.inc();
    }

    /// Record GCS operation latency
    pub fn record_gcs_latency(&self, _operation: &str, duration: std::time::Duration) {
        self.gcs_latency_seconds.observe(duration.as_secs_f64());
    }

    /// Record a GCS retry
    pub fn record_gcs_retry(&self, _operation: &str) {
        self.gcs_retries_total.inc();
    }

    /// Record state size
    pub fn record_state_size(&self, size_bytes: usize) {
        self.state_size_bytes.observe(size_bytes as f64);
    }

    /// Gather metrics in Prometheus format
    pub fn gather_metrics(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();

        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|e| PersistError::storage(format!("Failed to encode metrics: {e}")))?;

        String::from_utf8(buffer)
            .map_err(|e| PersistError::storage(format!("Failed to convert metrics to string: {e}")))
    }
}

/// Metrics timer helper for measuring operation durations
#[cfg(feature = "metrics")]
pub struct MetricsTimer {
    start: Instant,
    operation: String,
}

#[cfg(feature = "metrics")]
impl MetricsTimer {
    /// Start a new timer for the given operation
    pub fn new(operation: impl Into<String>) -> Self {
        let operation = operation.into();
        PersistMetrics::global().record_s3_request(&operation);

        Self {
            start: Instant::now(),
            operation,
        }
    }

    /// Start a new timer for S3 operations
    pub fn start_s3_operation(operation: impl Into<String>) -> Self {
        let operation = operation.into();
        PersistMetrics::global().record_s3_request(&operation);

        Self {
            start: Instant::now(),
            operation,
        }
    }

    /// Start a new timer for GCS operations
    pub fn start_gcs_operation(operation: impl Into<String>) -> Self {
        let operation = operation.into();
        PersistMetrics::global().record_gcs_request(&operation);

        Self {
            start: Instant::now(),
            operation,
        }
    }

    /// Complete the timer, recording success latency
    pub fn finish(self) {
        let duration = self.start.elapsed();
        PersistMetrics::global().record_s3_latency(&self.operation, duration);
    }

    /// Complete the timer with an error, recording both latency and error
    pub fn finish_with_error(self) {
        let duration = self.start.elapsed();
        PersistMetrics::global().record_s3_latency(&self.operation, duration);
        PersistMetrics::global().record_s3_error(&self.operation);
    }

    /// Complete the timer for GCS operation, recording success latency
    pub fn finish_gcs(self) {
        let duration = self.start.elapsed();
        PersistMetrics::global().record_gcs_latency(&self.operation, duration);
    }

    /// Complete the GCS timer with an error, recording both latency and error
    pub fn finish_gcs_with_error(self) {
        let duration = self.start.elapsed();
        PersistMetrics::global().record_gcs_latency(&self.operation, duration);
        PersistMetrics::global().record_gcs_error(&self.operation);
    }

    /// Record a retry for this operation
    pub fn record_retry(&self) {
        PersistMetrics::global().record_s3_retry(&self.operation);
    }

    /// Record a GCS retry for this operation
    pub fn record_gcs_retry(&self) {
        PersistMetrics::global().record_gcs_retry(&self.operation);
    }
}

/// Initialize the global observability system
///
/// This function sets up:
/// - Structured logging with JSON output
/// - OpenTelemetry tracing
/// - Metrics collection
/// - Optional Jaeger trace export
///
/// # Arguments
/// * `enable_jaeger` - Whether to enable Jaeger tracing export
/// * `jaeger_endpoint` - Optional Jaeger endpoint (defaults to localhost:14268)
///
/// # Returns
/// Result indicating success or failure of initialization
pub fn init_observability(enable_jaeger: bool, _jaeger_endpoint: Option<String>) -> Result<()> {
    // Initialize metrics (this sets up the global meter provider)
    #[cfg(feature = "metrics")]
    PersistMetrics::global();

    // Build the tracing subscriber with JSON formatting
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(false)
        .with_current_span(false);

    // For now, we'll focus on console tracing and metrics
    // OpenTelemetry Jaeger integration can be added later when version compatibility is resolved
    if enable_jaeger {
        tracing::warn!(
            "Jaeger tracing is temporarily disabled due to version compatibility issues"
        );
    }

    // Initialize tracing subscriber with console output
    let subscriber = TracingRegistry::default()
        .with(EnvFilter::from_default_env().add_directive("persist=info".parse().unwrap()))
        .with(fmt_layer);

    set_global_default(subscriber).map_err(|e| {
        PersistError::storage(format!("Failed to set global tracing subscriber: {e}"))
    })?;

    tracing::info!("Persist observability system initialized");
    Ok(())
}

/// Initialize observability with default settings
pub fn init_default_observability() -> Result<()> {
    init_observability(false, None)
}

#[cfg(all(test, feature = "metrics"))]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        let metrics = PersistMetrics::global();

        // Test that we can record metrics without panicking
        metrics.record_s3_request("put_object");
        metrics.record_s3_error("get_object");
        metrics.record_s3_latency("put_object", std::time::Duration::from_millis(100));
        metrics.record_s3_retry("put_object");
        metrics.record_state_size(1024);
    }

    #[test]
    fn test_metrics_timer() {
        let timer = MetricsTimer::new("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(1));
        timer.finish();

        // Test error case
        let timer = MetricsTimer::new("test_error");
        timer.finish_with_error();
    }

    #[test]
    fn test_metrics_gathering() {
        let metrics = PersistMetrics::global();

        // Record some test metrics
        metrics.record_s3_request("test");
        metrics.record_s3_error("test");

        // Gather metrics - should not panic
        let result = metrics.gather_metrics();
        assert!(result.is_ok());

        let metrics_text = result.unwrap();
        assert!(metrics_text.contains("persist_s3_requests_total"));
    }
}
