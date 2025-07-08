# Persist Observability Guide

This guide covers the enhanced observability features in Persist, including structured logging, distributed tracing, and comprehensive metrics. These features provide deep visibility into Persist's operations, enabling effective monitoring, debugging, and performance optimization.

## Table of Contents

- [Overview](#overview)
- [Structured Logging](#structured-logging)
- [Distributed Tracing](#distributed-tracing)
- [Metrics and Monitoring](#metrics-and-monitoring)
- [Configuration](#configuration)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)

## Overview

Persist's observability features are built on industry-standard tools and protocols:

- **Structured Logging**: Using the `tracing` crate for structured, contextual logging
- **Distributed Tracing**: OpenTelemetry-compatible tracing with span instrumentation
- **Metrics**: Prometheus-compatible metrics for monitoring performance and errors
- **Error Handling**: Enhanced error types with detailed context

These features work together to provide comprehensive visibility into Persist's operations, from high-level API calls down to individual S3 operations.

## Structured Logging

### Overview

Persist uses structured logging throughout the codebase, providing rich context and filtering capabilities. All logs include relevant metadata and are formatted in JSON for easy parsing by log aggregation tools.

### Log Levels

- **ERROR**: Critical errors that prevent operations from completing
- **WARN**: Recoverable issues or unusual conditions
- **INFO**: High-level operational events (saves, loads, etc.)
- **DEBUG**: Detailed diagnostic information
- **TRACE**: Very verbose debugging information

### Key Logged Events

#### Successful Operations
```
INFO persist_core::storage::s3: S3 operation completed successfully
  bucket="my-bucket" 
  key="agent/snapshot_001.json.gz" 
  operation="put_object" 
  duration_ms=150
```

#### Error Conditions
```
ERROR persist_core::storage::s3: S3 upload failed
  bucket="my-bucket" 
  key="agent/snapshot_001.json.gz" 
  error="NoSuchBucket: The specified bucket does not exist"
  retry_count=2
```

#### Retry Operations
```
WARN persist_core::storage::s3: Retrying S3 operation due to transient error
  bucket="my-bucket" 
  key="agent/snapshot_001.json.gz" 
  operation="put_object" 
  retry_attempt=1 
  max_retries=3
```

### Configuration

Control logging output through environment variables:

```bash
# Set log level for Persist components
export RUST_LOG="persist_core=info,persist_python=info"

# Enable debug logging for troubleshooting
export RUST_LOG="persist_core=debug"

# JSON structured output (recommended for production)
export PERSIST_LOG_FORMAT="json"

# Human-readable output (good for development)
export PERSIST_LOG_FORMAT="pretty"
```

### Python Integration

Enhanced error handling ensures that Rust errors are properly propagated to Python with meaningful exception types:

```python
import persist

try:
    # This will raise a specific exception type
    persist.restore("/nonexistent/snapshot.json.gz")
except FileNotFoundError as e:
    print(f"Snapshot not found: {e}")
except PermissionError as e:
    print(f"Access denied: {e}")
except persist.PersistError as e:
    print(f"General Persist error: {e}")
```

## Distributed Tracing

### Overview

Persist implements distributed tracing using OpenTelemetry, providing detailed visibility into the execution flow of operations. Traces show the complete lifecycle of operations, including timing information and error details.

### Trace Structure

Each major operation creates a trace span hierarchy:

```
save_snapshot (200ms)
├── serialize_agent_state (5ms)
├── compress_data (15ms)
└── s3_upload (180ms)
    ├── put_object_request (170ms)
    └── verify_upload (10ms)
```

### Key Instrumented Operations

#### High-Level Operations
- `save_snapshot`: Complete snapshot save operation
- `load_snapshot`: Complete snapshot load operation
- `verify_snapshot`: Snapshot integrity verification

#### Storage Operations
- `s3_upload`: S3 upload with retries
- `s3_download`: S3 download with retries
- `local_file_write`: Local filesystem write
- `local_file_read`: Local filesystem read

#### Internal Operations
- `compress_data`: Data compression
- `decompress_data`: Data decompression
- `calculate_hash`: Integrity hash calculation

### Trace Attributes

Spans include relevant attributes for filtering and analysis:

- `bucket`: S3 bucket name (for S3 operations)
- `key`: Object key or file path
- `size_bytes`: Data size being processed
- `operation_type`: Type of operation (save, load, verify, etc.)
- `agent_id`: Agent identifier from metadata
- `session_id`: Session identifier from metadata
- `retry_count`: Number of retry attempts

### Configuration

#### Jaeger Export

To export traces to Jaeger:

```rust
use persist_core::init_observability_with_jaeger;

// Initialize with Jaeger exporter
init_observability_with_jaeger("http://localhost:14268/api/traces")
    .expect("Failed to initialize observability");
```

#### Console Output

For development, traces can be output to console:

```bash
export RUST_LOG="persist_core=trace"
```

## Metrics and Monitoring

### Overview

Persist exposes comprehensive metrics in Prometheus format, enabling monitoring of performance, error rates, and system health.

### Available Metrics

#### Request Metrics
- `persist_s3_requests_total{operation}`: Total number of S3 requests by operation type
- `persist_s3_errors_total{operation}`: Total number of S3 errors by operation type
- `persist_s3_retries_total{operation}`: Total number of retry attempts

#### Performance Metrics
- `persist_s3_latency_seconds{operation}`: Histogram of S3 operation latencies
- `persist_state_size_bytes`: Histogram of agent state sizes

#### Error Rate Metrics
- `persist_error_rate`: Derived metric (errors/total requests)

### Metrics Collection

#### Prometheus Endpoint

Access metrics via HTTP endpoint:

```bash
# Default endpoint (if using built-in server)
curl http://localhost:9090/metrics

# Programmatic access
let metrics = PersistMetrics::global().gather_metrics().unwrap();
println!("{}", metrics);
```

#### Example Metrics Output

```prometheus
# HELP persist_s3_requests_total Total number of S3 requests
# TYPE persist_s3_requests_total counter
persist_s3_requests_total{operation="put_object"} 1234
persist_s3_requests_total{operation="get_object"} 5678

# HELP persist_s3_latency_seconds S3 operation latencies
# TYPE persist_s3_latency_seconds histogram
persist_s3_latency_seconds_bucket{operation="put_object",le="0.1"} 100
persist_s3_latency_seconds_bucket{operation="put_object",le="0.5"} 800
persist_s3_latency_seconds_bucket{operation="put_object",le="1.0"} 1200
persist_s3_latency_seconds_bucket{operation="put_object",le="+Inf"} 1234
persist_s3_latency_seconds_sum{operation="put_object"} 456.78
persist_s3_latency_seconds_count{operation="put_object"} 1234
```

### Monitoring Alerts

Recommended alerting rules:

```yaml
# High error rate
- alert: PersistHighErrorRate
  expr: rate(persist_s3_errors_total[5m]) / rate(persist_s3_requests_total[5m]) > 0.05
  for: 2m
  labels:
    severity: warning
  annotations:
    summary: High error rate in Persist S3 operations

# High latency
- alert: PersistHighLatency
  expr: histogram_quantile(0.95, rate(persist_s3_latency_seconds_bucket[5m])) > 1.0
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: High latency in Persist S3 operations
```

## Configuration

### Environment Variables

```bash
# Logging configuration
export RUST_LOG="persist_core=info"
export PERSIST_LOG_FORMAT="json"  # or "pretty"

# Tracing configuration
export PERSIST_TRACING_ENABLED="true"
export PERSIST_JAEGER_ENDPOINT="http://localhost:14268/api/traces"

# Metrics configuration
export PERSIST_METRICS_ENABLED="true"
export PERSIST_METRICS_PORT="9090"
```

### Programmatic Configuration

```rust
use persist_core::{init_default_observability, init_observability_with_jaeger};

// Default configuration (console logging + basic metrics)
init_default_observability()?;

// With Jaeger tracing
init_observability_with_jaeger("http://localhost:14268/api/traces")?;
```

## Best Practices

### Development

1. **Use appropriate log levels**: DEBUG/TRACE for detailed diagnostics, INFO for key events
2. **Include context**: Always include relevant identifiers (agent_id, session_id, etc.)
3. **Avoid logging sensitive data**: Never log credentials, personal data, or large payloads
4. **Test error paths**: Ensure error logging works correctly in failure scenarios

### Production

1. **Set appropriate log levels**: INFO or WARN to avoid log flooding
2. **Use structured logging**: JSON format for machine parsing
3. **Monitor key metrics**: Error rates, latencies, and throughput
4. **Set up alerting**: Alert on high error rates or unusual latency patterns
5. **Retain traces**: Keep traces for debugging production issues

### Performance

1. **Sampling**: Use trace sampling in high-throughput scenarios
2. **Async export**: Use asynchronous trace/metric export to avoid blocking
3. **Buffer limits**: Configure appropriate buffer sizes for trace/metric exporters

## Troubleshooting

### Common Issues

#### No Metrics Available
```bash
# Check if observability is initialized
curl http://localhost:9090/metrics

# Enable debug logging
export RUST_LOG="persist_core=debug"
```

#### Missing Traces
```bash
# Verify Jaeger endpoint
export PERSIST_JAEGER_ENDPOINT="http://localhost:14268/api/traces"

# Check Jaeger connectivity
curl http://localhost:16686/api/traces
```

#### High Overhead
```bash
# Reduce log level
export RUST_LOG="persist_core=warn"

# Enable trace sampling
export PERSIST_TRACE_SAMPLE_RATE="0.1"  # 10% sampling
```

### Debug Information

Access debug information programmatically:

```rust
use persist_core::PersistMetrics;

let metrics = PersistMetrics::global();
let debug_info = metrics.gather_debug_info();
println!("Metrics debug info: {:#?}", debug_info);
```

## Examples

### Basic Setup

```rust
use persist_core::{create_engine_from_config, init_default_observability, StorageConfig};

// Initialize observability
init_default_observability()?;

// Create engine and perform operations
let config = StorageConfig::s3_with_bucket("my-bucket".to_string());
let engine = create_engine_from_config(config)?;

// Operations are automatically instrumented
let result = engine.save_snapshot(&agent_json, &metadata, "snapshot.json.gz")?;
```

### Monitoring Dashboard

Example Grafana dashboard queries:

```promql
# Request rate by operation
rate(persist_s3_requests_total[5m])

# Error rate
rate(persist_s3_errors_total[5m]) / rate(persist_s3_requests_total[5m])

# 95th percentile latency
histogram_quantile(0.95, rate(persist_s3_latency_seconds_bucket[5m]))

# State size distribution
histogram_quantile(0.50, rate(persist_state_size_bytes_bucket[5m]))
```

### Error Investigation

When investigating errors, use logs and traces together:

1. **Find error in logs**: Search for ERROR level logs with relevant context
2. **Get trace ID**: Extract trace ID from error log entry
3. **View full trace**: Look up trace in Jaeger to see complete operation flow
4. **Check metrics**: Verify if error is part of a pattern or isolated incident

---

This observability system provides comprehensive visibility into Persist's operations, enabling effective monitoring, debugging, and performance optimization in production environments.
