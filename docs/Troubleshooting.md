# Troubleshooting Guide

This guide helps you diagnose and resolve common issues with Persist.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Configuration Problems](#configuration-problems)
- [Storage Backend Issues](#storage-backend-issues)
- [Performance Problems](#performance-problems)
- [Error Messages](#error-messages)
- [Development Issues](#development-issues)
- [FAQ](#frequently-asked-questions)

## Installation Issues

### Python Package Installation

**Problem**: `pip install persist` fails with compilation errors

**Solutions**:
1. **Update Rust toolchain**:
   ```bash
   rustup update stable
   rustup default stable
   ```

2. **Install build dependencies**:
   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install build-essential pkg-config libssl-dev
   
   # macOS
   xcode-select --install
   
   # Windows
   # Install Microsoft C++ Build Tools
   ```

3. **Use pre-built wheels** (if available):
   ```bash
   pip install --only-binary=persist persist
   ```

4. **Build from source**:
   ```bash
   git clone https://github.com/xenoscale/Persist.git
   cd Persist/persist-python
   maturin develop --release
   ```

**Problem**: Import error after installation

**Solutions**:
1. **Check Python version compatibility**:
   ```bash
   python --version  # Should be 3.8+
   ```

2. **Verify installation**:
   ```python
   import persist
   print(persist.__version__)
   ```

3. **Check for conflicting packages**:
   ```bash
   pip list | grep persist
   pip uninstall persist  # Remove conflicts
   pip install persist
   ```

### Rust Crate Compilation

**Problem**: `cargo build` fails with dependency errors

**Solutions**:
1. **Update dependencies**:
   ```bash
   cargo update
   cargo clean
   cargo build
   ```

2. **Check feature flags**:
   ```bash
   # Build with specific features
   cargo build --features "s3,metrics"
   ```

3. **Resolve version conflicts**:
   ```toml
   # In Cargo.toml, pin problematic dependencies
   [dependencies]
   tokio = "=1.35.0"  # Pin specific version
   ```

## Configuration Problems

### Environment Variables Not Recognized

**Problem**: Persist ignores environment variables

**Diagnostic**:
```bash
# Check if variables are set
env | grep -E "(AWS_|PERSIST_|RUST_)"

# Test with explicit values
RUST_LOG=debug persist config show
```

**Solutions**:
1. **Export variables properly**:
   ```bash
   export AWS_ACCESS_KEY_ID="your_key"
   export AWS_SECRET_ACCESS_KEY="your_secret"
   export PERSIST_DEFAULT_STORAGE="s3"
   ```

2. **Use configuration file**:
   ```bash
   # Create ~/.persist/config.toml
   mkdir -p ~/.persist
   cat > ~/.persist/config.toml << EOF
   [default]
   storage = "s3"
   bucket = "my-bucket"
   region = "us-west-2"
   EOF
   ```

3. **Verify with CLI**:
   ```bash
   persist config show --explain
   ```

### AWS Credentials Issues

**Problem**: "Unable to load AWS credentials" error

**Diagnostic**:
```bash
# Test AWS CLI
aws sts get-caller-identity

# Check credential files
ls -la ~/.aws/
cat ~/.aws/credentials
cat ~/.aws/config
```

**Solutions**:
1. **Set credentials via environment**:
   ```bash
   export AWS_ACCESS_KEY_ID="AKIAIOSFODNN7EXAMPLE"
   export AWS_SECRET_ACCESS_KEY="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
   export AWS_DEFAULT_REGION="us-west-2"
   ```

2. **Use AWS credentials file**:
   ```bash
   aws configure
   # Enter your credentials when prompted
   ```

3. **Use IAM roles** (for EC2/ECS):
   ```bash
   # Ensure your instance has an IAM role with S3 permissions
   curl http://169.254.169.254/latest/meta-data/iam/security-credentials/
   ```

4. **Debug with verbose logging**:
   ```bash
   RUST_LOG=aws_config=debug persist snapshot --help
   ```

## Storage Backend Issues

### Local Storage Problems

**Problem**: Permission denied when writing snapshots

**Diagnostic**:
```bash
# Check directory permissions
ls -la ./snapshots/
mkdir -p ./snapshots && touch ./snapshots/test.txt
```

**Solutions**:
1. **Fix permissions**:
   ```bash
   chmod 755 ./snapshots/
   sudo chown -R $USER:$USER ./snapshots/
   ```

2. **Use absolute paths**:
   ```python
   import os
   import persist
   
   snapshot_dir = os.path.expanduser("~/persist_snapshots")
   os.makedirs(snapshot_dir, exist_ok=True)
   persist.snapshot(agent, f"{snapshot_dir}/agent.json.gz")
   ```

3. **Check disk space**:
   ```bash
   df -h .
   ```

**Problem**: Snapshots not found after saving

**Diagnostic**:
```bash
find . -name "*.json.gz" -type f
ls -la snapshots/
```

**Solutions**:
1. **Use absolute paths consistently**
2. **Check current working directory**:
   ```python
   import os
   print("Current directory:", os.getcwd())
   ```

### S3 Storage Problems

**Problem**: "Access denied to S3 bucket" error

**Diagnostic**:
```bash
# Test S3 access
aws s3 ls s3://your-bucket/
aws s3 cp test.txt s3://your-bucket/test.txt
```

**Solutions**:
1. **Check bucket permissions**:
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
           "arn:aws:s3:::your-bucket",
           "arn:aws:s3:::your-bucket/*"
         ]
       }
     ]
   }
   ```

2. **Verify bucket exists**:
   ```bash
   aws s3 mb s3://your-bucket  # Create if needed
   ```

3. **Check region configuration**:
   ```bash
   aws s3api get-bucket-location --bucket your-bucket
   ```

**Problem**: S3 operations timeout

**Solutions**:
1. **Increase timeout**:
   ```bash
   export PERSIST_S3_TIMEOUT=120  # 120 seconds
   ```

2. **Check network connectivity**:
   ```bash
   ping s3.amazonaws.com
   curl -I https://s3.amazonaws.com
   ```

3. **Use regional endpoint**:
   ```python
   persist.snapshot(agent, "snapshot.json.gz",
                   storage_mode="s3",
                   s3_bucket="your-bucket",
                   s3_region="us-west-2")
   ```

**Problem**: Large files fail to upload

**Solutions**:
1. **Enable multipart upload** (handled automatically for large files)
2. **Increase compression**:
   ```bash
   export PERSIST_COMPRESSION_LEVEL=9
   ```

3. **Monitor file sizes**:
   ```python
   metadata = persist.get_metadata("large_snapshot.json.gz")
   print(f"Compressed size: {metadata.get('compressed_size', 'unknown')} bytes")
   ```

## Performance Problems

### Slow Snapshot Creation

**Diagnostic**:
```bash
# Enable timing information
RUST_LOG=debug persist snapshot --help
```

**Solutions**:
1. **Reduce compression level**:
   ```bash
   export PERSIST_COMPRESSION_LEVEL=1  # Faster compression
   ```

2. **Profile Python serialization**:
   ```python
   import time
   import persist
   
   start = time.time()
   persist.snapshot(agent, "profile_test.json.gz")
   print(f"Snapshot took {time.time() - start:.2f} seconds")
   ```

3. **Use async operations** (for custom implementations):
   ```python
   import asyncio
   # If implementing custom async snapshot logic
   ```

### High Memory Usage

**Diagnostic**:
```bash
# Monitor memory usage
ps aux | grep python
top -p $(pgrep python)
```

**Solutions**:
1. **Stream large snapshots** (for custom storage adapters)
2. **Reduce agent state size**:
   ```python
   # Before snapshotting, clean up agent memory
   agent.memory.clear_old_messages(keep_last=100)
   ```

3. **Use compression**:
   ```bash
   export PERSIST_COMPRESSION_LEVEL=6  # Balance size/speed
   ```

### Slow Restore Operations

**Solutions**:
1. **Verify integrity separately**:
   ```python
   # Skip integrity check for faster restore (not recommended for production)
   # This is currently automatic, but future versions may allow skipping
   ```

2. **Use local caching**:
   ```python
   # Download from S3 to local cache first
   import shutil
   import tempfile
   
   with tempfile.NamedTemporaryFile() as tmp:
       # Custom logic to cache S3 objects locally
       pass
   ```

## Error Messages

### Common Error Messages and Solutions

#### `PersistIntegrityError: Integrity verification failed`

**Cause**: Snapshot file was corrupted or tampered with

**Solutions**:
1. **Re-create the snapshot**:
   ```python
   persist.snapshot(agent, "fresh_snapshot.json.gz")
   ```

2. **Check storage reliability**:
   ```bash
   # For S3, check if versioning is enabled
   aws s3api get-bucket-versioning --bucket your-bucket
   ```

3. **Verify manually**:
   ```bash
   persist verify-snapshot snapshot.json.gz
   ```

#### `PersistS3Error: S3 upload failed`

**Cause**: Network issues, permissions, or service outage

**Solutions**:
1. **Retry with exponential backoff** (handled automatically)
2. **Check AWS service status**: https://status.aws.amazon.com/
3. **Use different region**:
   ```python
   persist.snapshot(agent, "backup.json.gz",
                   storage_mode="s3",
                   s3_region="us-east-1")  # Try different region
   ```

#### `PersistConfigurationError: S3 configuration error`

**Cause**: Missing or invalid S3 configuration

**Solutions**:
1. **Provide all required parameters**:
   ```python
   persist.snapshot(agent, "snapshot.json.gz",
                   storage_mode="s3",
                   s3_bucket="required-bucket-name",
                   s3_region="us-west-2")
   ```

2. **Check environment variables**:
   ```bash
   echo $AWS_S3_BUCKET
   echo $AWS_REGION
   ```

#### `FileNotFoundError: Snapshot not found`

**Cause**: Snapshot doesn't exist or wrong path

**Solutions**:
1. **Check if snapshot exists**:
   ```python
   if persist.snapshot_exists("snapshot.json.gz"):
       agent = persist.restore("snapshot.json.gz")
   else:
       print("Snapshot not found")
   ```

2. **List available snapshots**:
   ```bash
   persist list --storage local --path ./snapshots/
   persist list --storage s3 --bucket your-bucket
   ```

3. **Use absolute paths**:
   ```python
   import os
   full_path = os.path.abspath("snapshots/agent.json.gz")
   agent = persist.restore(full_path)
   ```

## Development Issues

### Build Failures

**Problem**: `cargo build` fails during development

**Solutions**:
1. **Clean build**:
   ```bash
   cargo clean
   rm -rf target/
   cargo build
   ```

2. **Update toolchain**:
   ```bash
   rustup update
   rustup component add clippy rustfmt
   ```

3. **Check feature compatibility**:
   ```bash
   cargo build --no-default-features
   cargo build --features "s3"
   cargo build --features "metrics"
   ```

### Test Failures

**Problem**: Tests fail with network or permission errors

**Solutions**:
1. **Run tests with proper setup**:
   ```bash
   # Set up test environment
   export PERSIST_TEST_BUCKET="test-bucket-$(date +%s)"
   export AWS_ENDPOINT_URL="http://localhost:4566"  # For LocalStack
   
   # Run tests
   cargo test
   pytest
   ```

2. **Skip integration tests**:
   ```bash
   cargo test --lib  # Only unit tests
   pytest -m "not integration"  # Skip integration tests
   ```

3. **Use test fixtures**:
   ```python
   import pytest
   import tempfile
   
   @pytest.fixture
   def temp_snapshot_dir():
       with tempfile.TemporaryDirectory() as tmpdir:
           yield tmpdir
   ```

### Python Integration Issues

**Problem**: PyO3 binding errors during development

**Solutions**:
1. **Rebuild Python module**:
   ```bash
   cd persist-python
   maturin develop --release
   ```

2. **Check Python environment**:
   ```bash
   which python
   python -c "import sys; print(sys.path)"
   ```

3. **Debug import issues**:
   ```python
   import sys
   print(sys.modules.keys())
   
   try:
       import persist
   except ImportError as e:
       print(f"Import error: {e}")
   ```

## Frequently Asked Questions

### General Usage

**Q: Can I use Persist with agents other than LangChain?**

A: Yes, but you'll need custom serialization. LangChain is supported out-of-the-box, but you can serialize any Python object:

```python
import json
import persist

# For custom agents, serialize to JSON first
custom_agent = {"state": "data", "config": "values"}
persist.snapshot(custom_agent, "custom_agent.json.gz")
```

**Q: How do I migrate snapshots between storage backends?**

A: Use the CLI or write a migration script:

```bash
# Using CLI (if available in future versions)
persist migrate --from "local:./snapshots/" --to "s3:my-bucket"
```

```python
# Using Python
import persist

# Load from local
agent = persist.restore("local_snapshot.json.gz", storage_mode="local")

# Save to S3
persist.snapshot(agent, "migrated_snapshot.json.gz", 
                storage_mode="s3", s3_bucket="my-bucket")
```

**Q: Can I encrypt snapshots?**

A: Currently, encryption is not built-in. Use storage-level encryption:

- **S3**: Enable server-side encryption
- **Local**: Use filesystem-level encryption (LUKS, BitLocker, etc.)

**Q: How much storage space do snapshots typically use?**

A: It depends on your agent's complexity:
- Simple conversation agents: 1-10 KB compressed
- Complex agents with large memory: 100 KB - 1 MB compressed
- Enterprise agents with extensive context: 1-10 MB compressed

**Q: Can I version snapshots automatically?**

A: Yes, use the snapshot_index parameter:

```python
for i in range(10):
    # Agent does some work
    persist.snapshot(agent, f"agent_snapshot_{i}.json.gz", 
                    snapshot_index=i)
```

### Performance

**Q: How can I make snapshots faster?**

A: Several optimizations:
1. Reduce compression level: `export PERSIST_COMPRESSION_LEVEL=1`
2. Clean agent memory before snapshotting
3. Use local storage for development
4. Snapshot only when necessary

**Q: What's the maximum snapshot size?**

A: There's no hard limit, but consider:
- Local storage: Limited by available disk space
- S3: 5TB per object (with multipart upload)
- Memory: Large snapshots may use significant RAM during processing

**Q: Can I snapshot in the background?**

A: Not directly, but you can use threading:

```python
import threading
import persist

def background_snapshot(agent, path):
    persist.snapshot(agent, path)

# Create snapshot in background
thread = threading.Thread(target=background_snapshot, args=(agent, "bg_snapshot.json.gz"))
thread.start()
```

### Security

**Q: Are snapshots secure?**

A: Snapshots are as secure as your storage:
- **Local**: Protected by filesystem permissions
- **S3**: Benefit from AWS security features
- **Content**: No encryption by default - contains agent state as-is

**Q: Can I audit snapshot access?**

A: Yes, through underlying storage:
- **S3**: Enable CloudTrail
- **Local**: Use filesystem auditing
- **Application**: Add custom logging

### Troubleshooting

**Q: My snapshots are very large. How can I reduce size?**

A: Try these approaches:
1. Clean agent memory: Remove old conversations, temporary data
2. Increase compression: `export PERSIST_COMPRESSION_LEVEL=9`
3. Snapshot selectively: Only save essential state
4. Use differential snapshots (custom implementation)

**Q: Snapshots work locally but fail in production. Why?**

A: Common causes:
1. Different AWS credentials/permissions
2. Network restrictions (firewall, VPC settings)
3. Different Python/Rust versions
4. Missing environment variables
5. Different working directories

**Q: How do I debug snapshot corruption?**

A: Use the verification tools:

```bash
# Check integrity
persist verify-snapshot snapshot.json.gz

# View metadata
persist show snapshot.json.gz

# Manual inspection
gunzip -c snapshot.json.gz | jq '.'
```

### Integration

**Q: Can I use Persist in Docker containers?**

A: Yes, ensure proper configuration:

```dockerfile
FROM python:3.11

# Install Rust for building (if building from source)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install persist
RUN pip install persist

# Set up environment
ENV PERSIST_DEFAULT_STORAGE=s3
ENV AWS_DEFAULT_REGION=us-west-2

COPY your_app.py .
CMD python your_app.py
```

**Q: How do I use Persist in Kubernetes?**

A: Use ConfigMaps and Secrets:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: persist-config
data:
  PERSIST_DEFAULT_STORAGE: "s3"
  AWS_REGION: "us-west-2"
  AWS_S3_BUCKET: "k8s-snapshots"

---
apiVersion: v1
kind: Secret
metadata:
  name: aws-credentials
type: Opaque
data:
  AWS_ACCESS_KEY_ID: <base64-encoded-key>
  AWS_SECRET_ACCESS_KEY: <base64-encoded-secret>
```

**Q: Can multiple processes share snapshots?**

A: Yes, snapshots are designed for sharing:
- Each snapshot has a unique path/key
- Metadata includes agent_id, session_id for organization
- Concurrent access to different snapshots is safe
- Coordinate access to prevent overwriting the same snapshot

## Getting Additional Help

1. **Documentation**: Check other files in the `docs/` directory
2. **GitHub Issues**: Search existing issues or create a new one
3. **Logs**: Enable debug logging: `export RUST_LOG=debug`
4. **Community**: Join discussions (if community channels exist)
5. **Support**: Contact maintainers for enterprise support

Remember to include relevant logs, error messages, and configuration details when seeking help.
