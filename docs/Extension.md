# Extension and Integration Guide

This guide explains how to extend Persist with new storage backends, compression algorithms, and integrate it with other frameworks or languages.

## Table of Contents

- [Adding New Storage Backends](#adding-new-storage-backends)
- [Adding Compression Algorithms](#adding-compression-algorithms)
- [Framework Integrations](#framework-integrations)
- [Language Bindings](#language-bindings)
- [Custom Serialization](#custom-serialization)
- [Metrics and Observability](#metrics-and-observability)
- [Testing Extensions](#testing-extensions)

## Adding New Storage Backends

Persist uses a trait-based architecture that makes adding new storage backends straightforward.

### 1. Implement the StorageAdapter Trait

Create a new struct that implements the `StorageAdapter` trait:

```rust
use persist_core::{StorageAdapter, PersistError, SnapshotMetadata};
use async_trait::async_trait;

pub struct MyCustomStorage {
    config: MyCustomConfig,
}

#[async_trait]
impl StorageAdapter for MyCustomStorage {
    async fn save(&self, data: &[u8], metadata: &SnapshotMetadata, path: &str) -> Result<(), PersistError> {
        // Implementation here
        todo!("Implement save logic")
    }

    async fn load(&self, path: &str) -> Result<Vec<u8>, PersistError> {
        // Implementation here
        todo!("Implement load logic")
    }

    async fn exists(&self, path: &str) -> bool {
        // Implementation here
        todo!("Implement exists check")
    }

    async fn delete(&self, path: &str) -> Result<(), PersistError> {
        // Implementation here
        todo!("Implement delete logic")
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, PersistError> {
        // Implementation here
        todo!("Implement list logic")
    }
}
```

### 2. Add Feature Flags

Add your storage backend behind a feature flag in `persist-core/Cargo.toml`:

```toml
[features]
default = ["s3", "metrics"]
s3 = ["aws-config", "aws-sdk-s3", "aws-smithy-runtime-api"]
my_custom = ["my-custom-sdk"]  # Add your feature

[dependencies]
# Add your dependencies as optional
my-custom-sdk = { version = "1.0", optional = true }
```

### 3. Update the StorageConfig

Extend the `StorageConfig` enum to include your backend:

```rust
// In persist-core/src/storage/mod.rs
#[derive(Debug, Clone)]
pub enum StorageBackend {
    Local,
    #[cfg(feature = "s3")]
    S3,
    #[cfg(feature = "my_custom")]
    MyCustom,
}

impl StorageConfig {
    #[cfg(feature = "my_custom")]
    pub fn my_custom_with_config(config: MyCustomConfig) -> Self {
        Self {
            backend: StorageBackend::MyCustom,
            // ... other fields
        }
    }
}
```

### 4. Update the Engine Factory

Modify the engine creation logic to handle your backend:

```rust
// In persist-core/src/engine.rs
pub fn create_engine_from_config(config: StorageConfig) -> Result<Box<dyn SnapshotEngine>, PersistError> {
    match config.backend {
        StorageBackend::Local => {
            // Local implementation
        }
        #[cfg(feature = "s3")]
        StorageBackend::S3 => {
            // S3 implementation
        }
        #[cfg(feature = "my_custom")]
        StorageBackend::MyCustom => {
            let storage = MyCustomStorage::new(config)?;
            Ok(Box::new(PersistEngine::new(storage)))
        }
    }
}
```

### 5. Update Python Bindings

Add support in the Python module:

```rust
// In persist-python/src/lib.rs
fn create_storage_config(
    storage_mode: Option<&str>,
    // ... other params
) -> PyResult<StorageConfig> {
    let mode = storage_mode.unwrap_or("local").to_lowercase();
    
    match mode.as_str() {
        "local" => Ok(StorageConfig::default_local()),
        "s3" => Ok(StorageConfig::default_s3()),
        "my_custom" => Ok(StorageConfig::my_custom_with_config(config)),
        _ => Err(PyIOError::new_err("Invalid storage mode")),
    }
}
```

### Example: Azure Blob Storage

Here's a complete example for Azure Blob Storage:

```rust
// persist-core/src/storage/azure.rs
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use async_trait::async_trait;

pub struct AzureBlobStorage {
    client: BlobServiceClient,
    container: String,
}

impl AzureBlobStorage {
    pub fn new(account: &str, key: &str, container: &str) -> Result<Self, PersistError> {
        let credentials = StorageCredentials::access_key(account, key);
        let client = BlobServiceClient::new(account, credentials);
        
        Ok(Self {
            client,
            container: container.to_string(),
        })
    }
}

#[async_trait]
impl StorageAdapter for AzureBlobStorage {
    async fn save(&self, data: &[u8], metadata: &SnapshotMetadata, path: &str) -> Result<(), PersistError> {
        self.client
            .container_client(&self.container)
            .blob_client(path)
            .put_block_blob(data)
            .content_type("application/gzip")
            .execute()
            .await
            .map_err(|e| PersistError::Storage(format!("Azure upload failed: {}", e)))?;
            
        Ok(())
    }

    async fn load(&self, path: &str) -> Result<Vec<u8>, PersistError> {
        let response = self.client
            .container_client(&self.container)
            .blob_client(path)
            .get()
            .execute()
            .await
            .map_err(|e| PersistError::Storage(format!("Azure download failed: {}", e)))?;
            
        Ok(response.data.collect().await)
    }

    // ... other methods
}
```

## Adding Compression Algorithms

### 1. Implement the CompressionAdapter Trait

```rust
use persist_core::{CompressionAdapter, PersistError};

pub struct ZstdCompression {
    level: i32,
}

impl CompressionAdapter for ZstdCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, PersistError> {
        zstd::encode_all(data, self.level)
            .map_err(|e| PersistError::Compression(format!("Zstd compression failed: {}", e)))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, PersistError> {
        zstd::decode_all(data)
            .map_err(|e| PersistError::Compression(format!("Zstd decompression failed: {}", e)))
    }

    fn algorithm_name(&self) -> &'static str {
        "zstd"
    }
}
```

### 2. Update Engine Configuration

```rust
#[derive(Debug, Clone)]
pub enum CompressionAlgorithm {
    Gzip,
    Zstd,
    None,
}

impl SnapshotEngine {
    pub fn with_compression(mut self, algorithm: CompressionAlgorithm) -> Self {
        self.compression = match algorithm {
            CompressionAlgorithm::Gzip => Box::new(GzipCompression::new()),
            CompressionAlgorithm::Zstd => Box::new(ZstdCompression::new(3)),
            CompressionAlgorithm::None => Box::new(NoCompression::new()),
        };
        self
    }
}
```

## Framework Integrations

### LangChain (Enhanced)

The default integration supports LangChain. To enhance it:

```python
# persist/langchain.py
import persist
from langchain.schema.runnable import Runnable
from typing import Dict, Any

class PersistentRunnable(Runnable):
    """A LangChain Runnable that automatically persists state."""
    
    def __init__(self, runnable: Runnable, agent_id: str, auto_save: bool = True):
        self.runnable = runnable
        self.agent_id = agent_id
        self.auto_save = auto_save
        self.snapshot_counter = 0
    
    def invoke(self, input: Dict[str, Any], config=None) -> Dict[str, Any]:
        result = self.runnable.invoke(input, config)
        
        if self.auto_save:
            persist.snapshot(
                self.runnable,
                f"auto_snapshots/{self.agent_id}_snapshot_{self.snapshot_counter}.json.gz",
                agent_id=self.agent_id,
                snapshot_index=self.snapshot_counter
            )
            self.snapshot_counter += 1
            
        return result
    
    @classmethod
    def restore(cls, snapshot_path: str, auto_save: bool = True) -> 'PersistentRunnable':
        """Restore a PersistentRunnable from a snapshot."""
        metadata = persist.get_metadata(snapshot_path)
        runnable = persist.restore(snapshot_path)
        
        instance = cls(runnable, metadata['agent_id'], auto_save)
        instance.snapshot_counter = metadata['snapshot_index'] + 1
        return instance
```

### Auto-GPT Integration

```python
# persist/autogpt.py
import persist
from autogpt.agent import Agent
from typing import Optional

def save_autogpt_agent(agent: Agent, checkpoint_name: str) -> None:
    """Save an Auto-GPT agent state."""
    state = {
        'memory': agent.memory.to_dict(),
        'goals': agent.goals,
        'config': agent.config.to_dict(),
        'workspace': agent.workspace.state if hasattr(agent.workspace, 'state') else None,
    }
    
    persist.snapshot(
        state,
        f"autogpt_checkpoints/{checkpoint_name}.json.gz",
        agent_id=agent.config.ai_name,
        description=f"Auto-GPT checkpoint: {checkpoint_name}"
    )

def restore_autogpt_agent(checkpoint_name: str) -> dict:
    """Restore an Auto-GPT agent from checkpoint."""
    return persist.restore(f"autogpt_checkpoints/{checkpoint_name}.json.gz")
```

### HuggingFace Transformers

```python
# persist/transformers.py
import persist
import torch
from transformers import AutoModel, AutoTokenizer
from typing import Dict, Any

def save_model_checkpoint(model, tokenizer, checkpoint_name: str, metadata: Dict[str, Any] = None):
    """Save a HuggingFace model and tokenizer."""
    state = {
        'model_state_dict': model.state_dict(),
        'model_config': model.config.to_dict(),
        'tokenizer_config': tokenizer.to_dict() if hasattr(tokenizer, 'to_dict') else str(tokenizer),
        'metadata': metadata or {}
    }
    
    persist.snapshot(
        state,
        f"model_checkpoints/{checkpoint_name}.json.gz",
        agent_id=f"hf_model_{model.config.model_type}",
        description=f"HuggingFace model checkpoint: {checkpoint_name}"
    )

def restore_model_checkpoint(checkpoint_name: str, model_class=None):
    """Restore a HuggingFace model from checkpoint."""
    state = persist.restore(f"model_checkpoints/{checkpoint_name}.json.gz")
    
    if model_class:
        model = model_class.from_pretrained(state['model_config'])
        model.load_state_dict(state['model_state_dict'])
        return model, state['tokenizer_config']
    
    return state
```

## Language Bindings

### Go Bindings (using CGO)

First, create a C-compatible interface:

```rust
// persist-c/src/lib.rs
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use persist_core::*;

#[no_mangle]
pub extern "C" fn persist_snapshot(
    agent_json: *const c_char,
    path: *const c_char,
    agent_id: *const c_char,
) -> i32 {
    // Convert C strings to Rust strings
    // Implement snapshot logic
    // Return 0 for success, error code for failure
}

#[no_mangle]
pub extern "C" fn persist_restore(
    path: *const c_char,
    output: *mut c_char,
    output_len: usize,
) -> i32 {
    // Implement restore logic
    // Write result to output buffer
}
```

Then create Go bindings:

```go
// persist.go
package persist

/*
#cgo LDFLAGS: -L. -lpersist_c
#include <stdlib.h>
#include <string.h>

extern int persist_snapshot(const char* agent_json, const char* path, const char* agent_id);
extern int persist_restore(const char* path, char* output, size_t output_len);
*/
import "C"
import (
    "errors"
    "unsafe"
)

func Snapshot(agentJSON, path, agentID string) error {
    cAgentJSON := C.CString(agentJSON)
    cPath := C.CString(path)
    cAgentID := C.CString(agentID)
    defer C.free(unsafe.Pointer(cAgentJSON))
    defer C.free(unsafe.Pointer(cPath))
    defer C.free(unsafe.Pointer(cAgentID))
    
    result := C.persist_snapshot(cAgentJSON, cPath, cAgentID)
    if result != 0 {
        return errors.New("snapshot failed")
    }
    return nil
}

func Restore(path string) (string, error) {
    cPath := C.CString(path)
    defer C.free(unsafe.Pointer(cPath))
    
    output := make([]byte, 1024*1024) // 1MB buffer
    result := C.persist_restore(cPath, (*C.char)(unsafe.Pointer(&output[0])), C.size_t(len(output)))
    
    if result != 0 {
        return "", errors.New("restore failed")
    }
    
    return string(output), nil
}
```

### Node.js Bindings (using N-API)

```rust
// persist-node/src/lib.rs
use napi::bindgen_prelude::*;
use persist_core::*;

#[napi]
pub struct PersistEngine {
    engine: Box<dyn SnapshotEngine>,
}

#[napi]
impl PersistEngine {
    #[napi(constructor)]
    pub fn new(storage_mode: String) -> Result<Self> {
        let config = match storage_mode.as_str() {
            "local" => StorageConfig::default_local(),
            "s3" => StorageConfig::default_s3(),
            _ => return Err(Error::new(Status::InvalidArg, "Invalid storage mode")),
        };
        
        let engine = create_engine_from_config(config)
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;
            
        Ok(Self { engine })
    }
    
    #[napi]
    pub async fn snapshot(&self, agent_json: String, path: String) -> Result<()> {
        let metadata = SnapshotMetadata::new("default_agent", "default_session", 0);
        
        self.engine.save_snapshot(&agent_json, &metadata, &path).await
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;
            
        Ok(())
    }
    
    #[napi]
    pub async fn restore(&self, path: String) -> Result<String> {
        let (_, json) = self.engine.load_snapshot(&path).await
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;
            
        Ok(json)
    }
}
```

## Custom Serialization

### Adding Support for Custom Objects

```rust
// persist-core/src/serialization/mod.rs
use serde::{Serialize, Deserialize};

pub trait PersistSerializable {
    fn serialize_for_persist(&self) -> Result<String, PersistError>;
    fn deserialize_from_persist(data: &str) -> Result<Self, PersistError> 
    where Self: Sized;
}

// Example implementation for a custom agent type
#[derive(Serialize, Deserialize)]
pub struct CustomAgent {
    pub state: serde_json::Value,
    pub version: String,
}

impl PersistSerializable for CustomAgent {
    fn serialize_for_persist(&self) -> Result<String, PersistError> {
        serde_json::to_string(self)
            .map_err(|e| PersistError::Json(e))
    }
    
    fn deserialize_from_persist(data: &str) -> Result<Self, PersistError> {
        serde_json::from_str(data)
            .map_err(|e| PersistError::Json(e))
    }
}
```

### Schema Evolution

```rust
// Handle versioned serialization for backward compatibility
#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
pub enum VersionedAgent {
    #[serde(rename = "1.0")]
    V1(AgentV1),
    #[serde(rename = "2.0")]
    V2(AgentV2),
}

impl VersionedAgent {
    pub fn upgrade_to_latest(self) -> AgentV2 {
        match self {
            VersionedAgent::V1(v1) => v1.upgrade(),
            VersionedAgent::V2(v2) => v2,
        }
    }
}
```

## Metrics and Observability

### Custom Metrics

```rust
// persist-core/src/metrics/custom.rs
use prometheus::{Counter, Histogram, Registry};

pub struct CustomMetrics {
    pub snapshots_created: Counter,
    pub restore_duration: Histogram,
    pub custom_operations: Counter,
}

impl CustomMetrics {
    pub fn new() -> Self {
        Self {
            snapshots_created: Counter::new("persist_custom_snapshots_total", "Custom snapshots created").unwrap(),
            restore_duration: Histogram::new("persist_custom_restore_duration_seconds", "Custom restore duration").unwrap(),
            custom_operations: Counter::new("persist_custom_operations_total", "Custom operations").unwrap(),
        }
    }
    
    pub fn register(&self, registry: &Registry) -> Result<(), prometheus::Error> {
        registry.register(Box::new(self.snapshots_created.clone()))?;
        registry.register(Box::new(self.restore_duration.clone()))?;
        registry.register(Box::new(self.custom_operations.clone()))?;
        Ok(())
    }
}
```

### Custom Tracing

```rust
// Add custom tracing spans for detailed observability
use tracing::{info_span, Instrument};

impl CustomStorageAdapter {
    #[tracing::instrument(name = "custom_save", skip(self, data))]
    async fn save(&self, data: &[u8], metadata: &SnapshotMetadata, path: &str) -> Result<(), PersistError> {
        let span = info_span!("custom_storage_save", 
            path = %path, 
            size = data.len(),
            agent_id = %metadata.agent_id
        );
        
        async move {
            // Implementation here
            tracing::info!("Starting custom save operation");
            // ... save logic
            tracing::info!("Custom save completed successfully");
            Ok(())
        }.instrument(span).await
    }
}
```

## Testing Extensions

### Unit Tests for Storage Backends

```rust
// tests/storage_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use persist_core::*;
    
    #[tokio::test]
    async fn test_custom_storage_roundtrip() {
        let storage = MyCustomStorage::new(config).unwrap();
        let metadata = SnapshotMetadata::new("test_agent", "test_session", 0);
        let test_data = b"test_snapshot_data";
        
        // Test save
        storage.save(test_data, &metadata, "test_path").await.unwrap();
        
        // Test exists
        assert!(storage.exists("test_path").await);
        
        // Test load
        let loaded_data = storage.load("test_path").await.unwrap();
        assert_eq!(loaded_data, test_data);
        
        // Test delete
        storage.delete("test_path").await.unwrap();
        assert!(!storage.exists("test_path").await);
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use persist_core::*;
use testcontainers::*;

#[tokio::test]
async fn test_end_to_end_with_custom_backend() {
    // Set up test environment (e.g., docker container)
    let container = clients::Cli::default()
        .run(images::generic::GenericImage::new("custom-storage-mock", "latest"));
    
    let config = StorageConfig::custom_with_endpoint(
        format!("http://localhost:{}", container.get_host_port(8080))
    );
    
    let engine = create_engine_from_config(config).unwrap();
    
    // Test full workflow
    let test_json = r#"{"test": "data"}"#;
    let metadata = SnapshotMetadata::new("test_agent", "session", 0);
    
    engine.save_snapshot(test_json, &metadata, "test_snapshot").await.unwrap();
    let (loaded_metadata, loaded_json) = engine.load_snapshot("test_snapshot").await.unwrap();
    
    assert_eq!(loaded_json, test_json);
    assert_eq!(loaded_metadata.agent_id, "test_agent");
}
```

### Python Integration Tests

```python
# tests/test_custom_integration.py
import pytest
import persist
from unittest.mock import patch

class TestCustomIntegration:
    def test_custom_serialization(self):
        """Test custom object serialization."""
        custom_agent = {
            'type': 'custom_agent',
            'state': {'memory': [], 'context': 'test'},
            'version': '2.0'
        }
        
        persist.snapshot(custom_agent, 'test_custom.json.gz')
        restored = persist.restore('test_custom.json.gz')
        
        assert restored == custom_agent
    
    @pytest.mark.integration
    def test_custom_storage_backend(self):
        """Test custom storage backend integration."""
        with patch('persist.create_storage_config') as mock_config:
            mock_config.return_value = create_custom_config()
            
            persist.snapshot({'test': 'data'}, 'test_path', storage_mode='custom')
            result = persist.restore('test_path', storage_mode='custom')
            
            assert result == {'test': 'data'}
```

## Best Practices for Extensions

1. **Feature Flags**: Always put new backends behind feature flags
2. **Error Handling**: Use the existing `PersistError` types and add new variants if needed
3. **Testing**: Write comprehensive unit and integration tests
4. **Documentation**: Document your extension with examples
5. **Metrics**: Add relevant metrics for observability
6. **Backward Compatibility**: Ensure existing snapshots remain loadable
7. **Configuration**: Use environment variables for configuration when possible
8. **Security**: Consider security implications of new storage backends
9. **Performance**: Benchmark your implementation against existing backends
10. **CI/CD**: Ensure your extension works in CI environments

## Getting Help

- Check existing implementations in `persist-core/src/storage/`
- Review tests in `tests/` for examples
- Open an issue on GitHub for design discussions
- Contribute back useful extensions to the main project

This guide should get you started with extending Persist. The modular architecture makes it straightforward to add new capabilities while maintaining backward compatibility and code quality.
