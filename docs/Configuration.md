# Configuration Guide

This guide covers all configuration options available in Persist, including environment variables, command-line arguments, and programmatic configuration.

## Quick Start

### Environment Setup

1. **Copy the environment template**:
   ```bash
   cp .env.example .env
   ```

2. **Configure for your environment**:
   ```bash
   # For local development
   PERSIST_DEFAULT_STORAGE=disk
   PERSIST_DEFAULT_PATH=./snapshots
   
   # For production with S3
   PERSIST_DEFAULT_STORAGE=s3
   AWS_S3_BUCKET=my-production-snapshots
   AWS_REGION=us-west-2
   ```

3. **Set AWS credentials** (for S3 usage):
   ```bash
   export AWS_ACCESS_KEY_ID=your_key
   export AWS_SECRET_ACCESS_KEY=your_secret
   ```

## Configuration Methods

Configuration can be provided through multiple methods, in order of precedence:

1. **Programmatic Configuration** (highest priority)
2. **Command-line Arguments**
3. **Environment Variables**
4. **Configuration Files**
5. **Default Values** (lowest priority)

## Environment Variables

### Core Configuration

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `PERSIST_DEFAULT_STORAGE` | Default storage backend | `disk` | `s3` |
| `PERSIST_DEFAULT_PATH` | Default storage path/bucket | `./snapshots` | `my-bucket` |
| `PERSIST_VERIFY_INTEGRITY` | Enable integrity verification | `true` | `false` |
| `PERSIST_MAX_FILE_SIZE` | Maximum snapshot size (bytes) | `104857600` | `52428800` |

### AWS S3 Configuration

| Variable | Description | Required | Example |
|----------|-------------|----------|---------|
| `AWS_ACCESS_KEY_ID` | AWS access key | Yes (for S3) | `AKIAIOSFODNN7EXAMPLE` |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key | Yes (for S3) | `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY` |
| `AWS_REGION` | AWS region | Yes (for S3) | `us-west-2` |
| `AWS_S3_BUCKET` | S3 bucket name | Yes (for S3) | `my-snapshots` |
| `AWS_ENDPOINT_URL` | Custom S3 endpoint | No | `http://localhost:4566` |

### Performance Tuning

| Variable | Description | Default | Range |
|----------|-------------|---------|-------|
| `PERSIST_S3_MAX_RETRIES` | S3 retry attempts | `3` | `1-10` |
| `PERSIST_S3_TIMEOUT` | S3 operation timeout (seconds) | `30` | `5-300` |
| `PERSIST_COMPRESSION_LEVEL` | Compression level | `6` | `1-9` |

### Observability

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `RUST_LOG` | Rust logging level | `info` | `debug` |
| `PERSIST_METRICS_ENABLED` | Enable Prometheus metrics | `true` | `false` |
| `JAEGER_ENDPOINT` | Jaeger tracing endpoint | None | `http://localhost:14268` |

## Programmatic Configuration

### Rust API

```rust
use persist_core::{StorageConfig, StorageBackend, SnapshotEngine};

// Local storage configuration
let config = StorageConfig::new(
    StorageBackend::Local,
    "./my_snapshots".to_string()
);

// S3 storage configuration
let s3_config = StorageConfig::new(
    StorageBackend::S3,
    "my-bucket".to_string()
);

// Create engine with configuration
let engine = create_engine_from_config(config)?;
```

### Python API

```python
import persist

# Configure for local storage
persist.configure(
    storage="disk",
    path="./snapshots"
)

# Configure for S3
persist.configure(
    storage="s3",
    bucket="my-snapshots",
    region="us-west-2"
)

# Use with context manager
with persist.configured(storage="s3", bucket="temp-bucket"):
    persist.save(agent, "temp_snapshot")
```

## Storage Backend Configuration

### Local Filesystem

**Configuration:**
```bash
PERSIST_DEFAULT_STORAGE=disk
PERSIST_DEFAULT_PATH=/path/to/snapshots
```

**Features:**
- Fast access for development and testing
- No network dependencies
- Simple backup and versioning

**Best Practices:**
- Use absolute paths for production
- Ensure directory permissions are correct
- Consider disk space monitoring
- Implement regular backups

**Example Directory Structure:**
```
snapshots/
├── agent_1/
│   ├── session_1_snapshot_0.json.gz
│   ├── session_1_snapshot_1.json.gz
│   └── session_2_snapshot_0.json.gz
└── agent_2/
    └── session_1_snapshot_0.json.gz
```

### AWS S3

**Configuration:**
```bash
PERSIST_DEFAULT_STORAGE=s3
AWS_S3_BUCKET=my-persist-snapshots
AWS_REGION=us-west-2
AWS_ACCESS_KEY_ID=your_key
AWS_SECRET_ACCESS_KEY=your_secret
```

**Advanced S3 Configuration:**
```bash
# Custom endpoint (for LocalStack or other S3-compatible services)
AWS_ENDPOINT_URL=http://localhost:4566

# Performance tuning
PERSIST_S3_MAX_RETRIES=5
PERSIST_S3_TIMEOUT=60

# Server-side encryption
AWS_S3_SSE=AES256
```

**S3 Best Practices:**
- Use IAM roles instead of access keys when possible
- Enable server-side encryption
- Configure lifecycle policies for cost optimization
- Use bucket versioning for data protection
- Monitor costs and usage

**IAM Policy Example:**
```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject",
                "s3:ListBucket"
            ],
            "Resource": [
                "arn:aws:s3:::my-persist-snapshots",
                "arn:aws:s3:::my-persist-snapshots/*"
            ]
        }
    ]
}
```

## CLI Configuration

### Global Options

```bash
persist --storage s3 --path my-bucket list
persist --verbose --storage disk --path ./snapshots show snapshot_id
```

### Configuration File

Create `~/.persist/config.toml`:
```toml
[default]
storage = "s3"
bucket = "my-default-bucket"
region = "us-west-2"

[development]
storage = "disk"
path = "./dev_snapshots"

[testing]
storage = "disk"
path = "/tmp/test_snapshots"
```

Use with:
```bash
persist --profile development list
persist --profile testing verify snapshot_id
```

## Development and Testing

### LocalStack (S3 Emulation)

For local S3 testing without AWS costs:

```bash
# Start LocalStack
docker run -d --name localstack -p 4566:4566 localstack/localstack

# Configure for LocalStack
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1
export AWS_S3_BUCKET=test-bucket

# Create test bucket
aws --endpoint-url=http://localhost:4566 s3 mb s3://test-bucket

# Run tests
cargo test --features s3
```

### Test Configuration

```bash
# Test-specific environment variables
PERSIST_TEST_BUCKET=persist-test-$(date +%s)
PERSIST_COMPRESSION_LEVEL=1  # Faster compression for tests
PERSIST_S3_MAX_RETRIES=1     # Fail fast in tests
```

## Production Configuration

### High Availability Setup

```bash
# Multiple region configuration
AWS_REGION=us-west-2
AWS_S3_BUCKET=persist-primary
AWS_S3_BACKUP_BUCKET=persist-backup-us-east-1

# Performance optimization
PERSIST_S3_MAX_RETRIES=5
PERSIST_S3_TIMEOUT=120
PERSIST_COMPRESSION_LEVEL=9  # Maximum compression for storage efficiency
```

### Security Configuration

```bash
# Enable all security features
PERSIST_VERIFY_INTEGRITY=true
AWS_S3_SSE=aws:kms
AWS_S3_SSE_KMS_KEY_ID=arn:aws:kms:us-west-2:123456789012:key/12345678-1234-1234-1234-123456789012

# Audit logging
RUST_LOG=persist_core=info,aws_sdk_s3=warn
PERSIST_AUDIT_LOG_PATH=/var/log/persist-audit.log
```

### Monitoring Configuration

```bash
# Prometheus metrics
PERSIST_METRICS_ENABLED=true
PERSIST_METRICS_PORT=9090

# Distributed tracing
JAEGER_ENDPOINT=http://jaeger:14268/api/traces
JAEGER_SERVICE_NAME=persist-core

# Custom metrics tags
PERSIST_ENVIRONMENT=production
PERSIST_DATACENTER=us-west-2a
```

## Configuration Validation

### Validation Commands

```bash
# Validate configuration
persist config validate

# Test storage connectivity
persist config test-storage

# Show current configuration
persist config show
```

### Common Configuration Issues

1. **AWS Credentials Not Found**:
   ```
   Error: Unable to load AWS credentials
   Solution: Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY
   ```

2. **S3 Bucket Access Denied**:
   ```
   Error: Access denied to S3 bucket
   Solution: Check bucket permissions and IAM policies
   ```

3. **Local Directory Not Writable**:
   ```
   Error: Permission denied writing to ./snapshots
   Solution: Check directory permissions or use absolute path
   ```

## Environment-Specific Examples

### Development Environment

```bash
# .env.development
PERSIST_DEFAULT_STORAGE=disk
PERSIST_DEFAULT_PATH=./dev_snapshots
RUST_LOG=debug
PERSIST_COMPRESSION_LEVEL=1
PERSIST_VERIFY_INTEGRITY=false  # Faster development iteration
```

### Staging Environment

```bash
# .env.staging
PERSIST_DEFAULT_STORAGE=s3
AWS_S3_BUCKET=persist-staging
AWS_REGION=us-west-2
RUST_LOG=info
PERSIST_COMPRESSION_LEVEL=6
PERSIST_VERIFY_INTEGRITY=true
PERSIST_METRICS_ENABLED=true
```

### Production Environment

```bash
# .env.production
PERSIST_DEFAULT_STORAGE=s3
AWS_S3_BUCKET=persist-production
AWS_REGION=us-west-2
RUST_LOG=warn
PERSIST_COMPRESSION_LEVEL=9
PERSIST_VERIFY_INTEGRITY=true
PERSIST_METRICS_ENABLED=true
PERSIST_S3_MAX_RETRIES=5
PERSIST_S3_TIMEOUT=120
```

## Troubleshooting

### Debug Configuration Issues

1. **Enable debug logging**:
   ```bash
   RUST_LOG=debug persist config show
   ```

2. **Validate AWS credentials**:
   ```bash
   aws sts get-caller-identity
   ```

3. **Test S3 connectivity**:
   ```bash
   aws s3 ls s3://your-bucket
   ```

4. **Check file permissions**:
   ```bash
   ls -la ./snapshots
   ```

### Configuration Precedence Debugging

Use the CLI to see effective configuration:
```bash
persist config show --explain
```

This will show:
- Final effective values
- Source of each configuration value
- Any environment variables that were ignored
- Validation warnings or errors

## Best Practices

1. **Use environment-specific configuration files**
2. **Never commit secrets to version control**
3. **Use IAM roles instead of access keys when possible**
4. **Monitor configuration drift in production**
5. **Validate configuration in CI/CD pipelines**
6. **Document custom configuration for your team**
7. **Use the principle of least privilege for AWS permissions**
8. **Regularly rotate credentials**
9. **Monitor costs for cloud storage usage**
10. **Test disaster recovery procedures**
