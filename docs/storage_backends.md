# Storage Backends

Persist supports multiple storage backends for agent snapshots, providing flexibility for different deployment scenarios and requirements.

## Local Filesystem

The local filesystem backend stores snapshots as compressed files on the local disk.

### Configuration
```rust
let config = StorageConfig::default_local();
// or with custom path
let config = StorageConfig {
    backend: StorageBackend::Local,
    local_base_path: Some(PathBuf::from("/custom/path/to/snapshots")),
    ..Default::default()
};
```

### Environment Variables
- `PERSIST_LOCAL_PATH`: Base directory for snapshots (optional)

## Amazon S3

The S3 backend provides scalable cloud storage with enterprise-grade durability and availability.

### Configuration
```rust
let config = StorageConfig::s3_with_bucket("my-snapshots-bucket".to_string());
// or with region
let config = StorageConfig::s3_with_bucket_and_region(
    "my-snapshots-bucket".to_string(), 
    "us-west-2".to_string()
);
```

### Environment Variables
- `AWS_S3_BUCKET`: S3 bucket name for snapshots
- `AWS_REGION`: AWS region (defaults to us-east-1)
- `AWS_ACCESS_KEY_ID`: AWS access key
- `AWS_SECRET_ACCESS_KEY`: AWS secret key

### Required Permissions
The AWS credentials must have the following IAM permissions:
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
        "s3:HeadObject"
      ],
      "Resource": "arn:aws:s3:::your-bucket-name/*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "s3:ListBucket"
      ],
      "Resource": "arn:aws:s3:::your-bucket-name"
    }
  ]
}
```

## Google Cloud Storage

The GCS backend provides Google Cloud's enterprise storage with global availability and strong consistency.

### Configuration
```rust
// Basic configuration
let config = StorageConfig::gcs_with_bucket("my-snapshots-bucket".to_string());

// With service account credentials
let config = StorageConfig::gcs_with_bucket_and_credentials(
    "my-snapshots-bucket".to_string(),
    PathBuf::from("/path/to/service-account.json")
);

// With prefix for organization
let config = StorageConfig::gcs_with_bucket_prefix_and_credentials(
    "my-snapshots-bucket".to_string(),
    "snapshots/production".to_string(),
    Some(PathBuf::from("/path/to/service-account.json"))
);
```

### Environment Variables
- `PERSIST_GCS_BUCKET` or `GCS_BUCKET`: GCS bucket name for snapshots
- `PERSIST_GCS_PREFIX`: Optional prefix for organizing snapshots within the bucket
- `GOOGLE_APPLICATION_CREDENTIALS`: Path to service account JSON file
- `PERSIST_GCS_TIMEOUT`: Timeout for GCS operations in seconds (default: 30)

### Authentication Methods
1. **Service Account Key File**: Set `GOOGLE_APPLICATION_CREDENTIALS` to the path of your service account JSON file
2. **Application Default Credentials**: Use workload identity on GKE, service account on GCE, or gcloud credentials locally
3. **Explicit Credentials**: Pass the path directly to the constructor

### Setting up GCS Bucket
```bash
# Create bucket
gsutil mb gs://your-snapshots-bucket

# Set appropriate permissions
gsutil iam ch serviceAccount:your-service-account@project.iam.gserviceaccount.com:objectAdmin gs://your-snapshots-bucket
```

### Required IAM Permissions
The service account must have the following IAM roles or permissions:
- `roles/storage.objectAdmin` on the bucket, or custom role with:
  - `storage.objects.create`
  - `storage.objects.delete` 
  - `storage.objects.get`
  - `storage.objects.list`
  - `storage.buckets.get` (for bucket validation)

### Advanced Features

#### Streaming Support
GCS backend supports streaming for large snapshots:
```rust
use tokio::fs::File;
let file = File::open("large_snapshot.json").await?;
gcs_adapter.save_stream(file, "agent1/large_snapshot.json.gz").await?;

// Stream reading
let stream = gcs_adapter.load_stream("agent1/large_snapshot.json.gz").await?;
```

#### Error Handling and Retries
- Automatic retries for transient errors (5xx, network issues, timeouts)
- Exponential backoff with jitter
- Comprehensive error classification and mapping

#### Security Features
- KMS encryption support (when `PERSIST_GCS_KMS_KEY` is set)
- PII scrubbing in logs (unless `RUST_LOG=debug`)
- Least-privilege IAM recommendations

### Supported Backends Summary

| Backend | ✅ Implemented | Compression | Streaming | Retry Logic | Encryption |
|---------|---------------|-------------|-----------|-------------|------------|
| Local Filesystem | ✅ | ✅ | ⏳ | N/A | File-level |
| Amazon S3 | ✅ | ✅ | ✅ | ✅ | Server-side |
| **Google Cloud Storage** | ✅ | ✅ | ✅ | ✅ | KMS Support |

### Performance Considerations

- **Compression**: All backends use gzip compression by default to reduce storage costs and transfer time
- **Concurrent Operations**: Backends support concurrent read/write operations
- **Large Files**: Streaming support prevents memory exhaustion with large agent states
- **Network Optimization**: Retry logic with exponential backoff minimizes the impact of transient network issues

### Choosing a Backend

- **Local Filesystem**: Best for development, testing, and single-node deployments
- **Amazon S3**: Recommended for AWS-based deployments and high-scale production workloads
- **Google Cloud Storage**: Recommended for GCP-based deployments and applications requiring global consistency
