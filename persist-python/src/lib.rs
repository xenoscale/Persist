/*!
Python bindings for the Persist agent snapshot system with S3 support.

This module provides a Pythonic interface to the Rust-based persist-core library,
enabling easy snapshotting and restoration of LangChain agents with both local and S3 storage.

## Example Usage

```python
import persist

# Local storage (original behavior)
persist.snapshot(agent, "/path/to/snapshots/agent1_snapshot.json.gz")

# S3 storage
persist.snapshot(agent, "agent1/snapshot.json.gz",
                storage_mode="s3",
                s3_bucket="my-snapshots-bucket")

# Restore from S3
restored_agent = persist.restore("agent1/snapshot.json.gz",
                               storage_mode="s3",
                               s3_bucket="my-snapshots-bucket")
```
*/

use persist_core::{create_engine_from_config, PersistError, SnapshotMetadata, StorageConfig};
use pyo3::exceptions::{PyException, PyIOError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::create_exception;

// Define custom Python exception types
create_exception!(
    persist_python,
    PyPersistError,
    PyException,
    "Base exception for Persist operations"
);
create_exception!(
    persist_python,
    PyPersistConfigurationError,
    PyPersistError,
    "Configuration error"
);
create_exception!(
    persist_python,
    PyPersistIntegrityError,
    PyPersistError,
    "Data integrity verification failed"
);
create_exception!(
    persist_python,
    PyPersistS3Error,
    PyPersistError,
    "S3 storage operation failed"
);
create_exception!(
    persist_python,
    PyPersistCompressionError,
    PyPersistError,
    "Compression/decompression failed"
);

/// Convert a Rust PersistError to a Python exception
fn convert_error(err: PersistError) -> PyErr {
    match err {
        PersistError::Io(io_err) => PyIOError::new_err(format!("I/O error: {io_err}")),
        PersistError::Json(json_err) => {
            PyPersistError::new_err(format!("JSON serialization error: {json_err}"))
        }
        PersistError::Compression(msg) => {
            PyPersistCompressionError::new_err(format!("Compression error: {msg}"))
        }
        PersistError::IntegrityCheckFailed { expected, actual } => PyPersistIntegrityError::new_err(
            format!("Integrity verification failed: expected hash {expected}, got {actual}"),
        ),
        PersistError::InvalidFormat(msg) => {
            PyPersistError::new_err(format!("Invalid snapshot format: {msg}"))
        }
        PersistError::MissingMetadata(field) => {
            PyPersistError::new_err(format!("Missing required metadata field: {field}"))
        }
        PersistError::Storage(msg) => {
            PyPersistError::new_err(format!("Storage operation failed: {msg}"))
        }
        PersistError::Validation(msg) => PyPersistError::new_err(format!("Validation error: {msg}")),

        // S3-specific errors
        PersistError::S3UploadError {
            source,
            bucket,
            key,
        } => PyPersistS3Error::new_err(format!(
            "S3 upload failed (bucket: {bucket}, key: {key}): {source}"
        )),
        PersistError::S3DownloadError {
            source,
            bucket,
            key,
        } => PyPersistS3Error::new_err(format!(
            "S3 download failed (bucket: {bucket}, key: {key}): {source}"
        )),
        PersistError::S3NotFound { bucket, key } => {
            use pyo3::exceptions::PyFileNotFoundError;
            PyFileNotFoundError::new_err(format!(
                "Snapshot not found in S3 (bucket: {bucket}, key: {key})"
            ))
        }
        PersistError::S3AccessDenied { bucket } => {
            use pyo3::exceptions::PyPermissionError;
            PyPermissionError::new_err(format!(
                "Access denied to S3 bucket: {bucket}. Check your credentials and permissions."
            ))
        }
        PersistError::S3Configuration(msg) => {
            PyPersistConfigurationError::new_err(format!("S3 configuration error: {msg}"))
        }
    }
}

/// Create storage configuration from Python parameters
fn create_storage_config(
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<StorageConfig> {
    let mode = storage_mode.unwrap_or("local").to_lowercase();

    match mode.as_str() {
        "local" => Ok(StorageConfig::default_local()),
        "s3" => {
            let mut config = if let Some(bucket) = s3_bucket {
                StorageConfig::s3_with_bucket(bucket.to_string())
            } else {
                StorageConfig::default_s3()
            };

            if let Some(region) = s3_region {
                config.s3_region = Some(region.to_string());
            }

            Ok(config)
        }
        _ => Err(PyIOError::new_err(format!(
            "Invalid storage_mode '{mode}'. Must be 'local' or 's3'"
        ))),
    }
}

/// Save an agent snapshot with configurable storage backend
///
/// This function serializes a LangChain agent (or other compatible object) to a compressed
/// snapshot file. Supports both local filesystem and Amazon S3 storage backends.
///
/// # Arguments
/// * `agent` - The agent object to snapshot (must support LangChain serialization)
/// * `path` - Storage path/key for the snapshot
/// * `agent_id` - Optional unique identifier for the agent (default: "default_agent")
/// * `session_id` - Optional session identifier (default: "default_session")
/// * `snapshot_index` - Optional sequence number for this snapshot (default: 0)
/// * `description` - Optional human-readable description of the snapshot
/// * `storage_mode` - Storage backend: "local" or "s3" (default: "local")
/// * `s3_bucket` - S3 bucket name (required for S3 mode)
/// * `s3_region` - S3 region (optional, uses AWS environment default)
///
/// # Returns
/// None on success
///
/// # Raises
/// * IOError - If saving fails, JSON serialization fails, or integrity check fails
///
/// # Example
/// ```python
/// import persist
/// from langchain.chains import ConversationChain
///
/// # Local storage
/// persist.snapshot(agent, "snapshots/agent1.json.gz")
///
/// # S3 storage
/// persist.snapshot(agent, "agent1/session1/snapshot.json.gz",
///                 storage_mode="s3",
///                 s3_bucket="my-snapshots-bucket",
///                 agent_id="conversation_agent")
/// ```
#[pyfunction]
#[pyo3(signature = (agent, path, agent_id="default_agent", session_id="default_session", snapshot_index=0, description=None, storage_mode=None, s3_bucket=None, s3_region=None))]
#[allow(clippy::too_many_arguments)]
fn snapshot(
    py: Python<'_>,
    agent: &Bound<'_, PyAny>,
    path: &str,
    agent_id: &str,
    session_id: &str,
    snapshot_index: u64,
    description: Option<&str>,
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<()> {
    // Import LangChain's dump function
    let langchain_load = py.import("langchain_core.load")
        .or_else(|_| py.import("langchain.load"))  // Fallback for older versions
        .map_err(|_| PyIOError::new_err("Could not import langchain_core.load or langchain.load. Please ensure LangChain is installed."))?;

    let dumps_func = langchain_load.getattr("dumps").map_err(|_| {
        PyIOError::new_err("Could not find dumps function in LangChain load module")
    })?;

    // Serialize the agent to JSON string using LangChain's dumps
    let json_obj = dumps_func.call1((agent,)).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to serialize agent with LangChain dumps: {e}"
        ))
    })?;

    let agent_json: String = json_obj.extract().map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to extract JSON string from LangChain dumps result: {e}"
        ))
    })?;

    // Create metadata
    let mut metadata = SnapshotMetadata::new(agent_id, session_id, snapshot_index);
    if let Some(desc) = description {
        metadata = metadata.with_description(desc);
    }

    // Create storage configuration
    let config = create_storage_config(storage_mode, s3_bucket, s3_region)?;

    // Create appropriate engine based on storage configuration
    let engine = create_engine_from_config(config).map_err(convert_error)?;

    // Save snapshot
    let _saved_metadata = engine
        .save_snapshot(&agent_json, &metadata, path)
        .map_err(convert_error)?;

    Ok(())
}

/// Restore an agent snapshot with configurable storage backend
///
/// This function loads a compressed snapshot file and reconstructs the original agent
/// object using LangChain's loads() function. Supports both local and S3 storage.
///
/// # Arguments
/// * `path` - Storage path/key of the snapshot to restore
/// * `secrets_map` - Optional dictionary of secrets/API keys for the restored agent
/// * `storage_mode` - Storage backend: "local" or "s3" (default: "local")
/// * `s3_bucket` - S3 bucket name (required for S3 mode)
/// * `s3_region` - S3 region (optional, uses AWS environment default)
///
/// # Returns
/// The restored agent object
///
/// # Raises
/// * IOError - If loading fails, decompression fails, or integrity check fails
///
/// # Example
/// ```python
/// import persist
/// import os
///
/// # Set up AWS credentials for S3 access
/// os.environ["AWS_ACCESS_KEY_ID"] = "your-access-key"
/// os.environ["AWS_SECRET_ACCESS_KEY"] = "your-secret-key"
/// os.environ["AWS_REGION"] = "us-west-2"
///
/// # Restore from S3
/// restored_agent = persist.restore("agent1/session1/snapshot.json.gz",
///                                storage_mode="s3",
///                                s3_bucket="my-snapshots-bucket")
/// ```
#[pyfunction]
#[pyo3(signature = (path, secrets_map=None, storage_mode=None, s3_bucket=None, s3_region=None))]
fn restore(
    py: Python<'_>,
    path: &str,
    secrets_map: Option<&Bound<'_, PyDict>>,
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<PyObject> {
    // Create storage configuration
    let config = create_storage_config(storage_mode, s3_bucket, s3_region)?;

    // Create appropriate engine based on storage configuration
    let engine = create_engine_from_config(config).map_err(convert_error)?;

    // Load snapshot
    let (_metadata, agent_json) = engine.load_snapshot(path).map_err(convert_error)?;

    // Import LangChain's load function
    let langchain_load = py.import("langchain_core.load")
        .or_else(|_| py.import("langchain.load"))
        .map_err(|_| PyIOError::new_err("Could not import langchain_core.load or langchain.load. Please ensure LangChain is installed."))?;

    let loads_func = langchain_load.getattr("loads").map_err(|_| {
        PyIOError::new_err("Could not find loads function in LangChain load module")
    })?;

    // Deserialize the agent using LangChain's loads
    let agent_obj = if let Some(secrets) = secrets_map {
        loads_func.call1((agent_json, secrets))
    } else {
        loads_func.call1((agent_json,))
    }
    .map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to deserialize agent with LangChain loads: {e}"
        ))
    })?;

    Ok(agent_obj.into())
}

/// Get metadata for a snapshot without loading the full snapshot
///
/// # Arguments
/// * `path` - Storage path/key of the snapshot
/// * `storage_mode` - Storage backend: "local" or "s3" (default: "local")
/// * `s3_bucket` - S3 bucket name (required for S3 mode)
/// * `s3_region` - S3 region (optional, uses AWS environment default)
///
/// # Returns
/// Dictionary containing snapshot metadata
#[pyfunction]
#[pyo3(signature = (path, storage_mode=None, s3_bucket=None, s3_region=None))]
fn get_metadata(
    py: Python<'_>,
    path: &str,
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<PyObject> {
    let config = create_storage_config(storage_mode, s3_bucket, s3_region)?;
    let engine = create_engine_from_config(config).map_err(convert_error)?;

    let metadata = engine.get_snapshot_metadata(path).map_err(convert_error)?;

    // Convert metadata to Python dictionary
    let dict = PyDict::new(py);
    dict.set_item("agent_id", metadata.agent_id)?;
    dict.set_item("session_id", metadata.session_id)?;
    dict.set_item("snapshot_index", metadata.snapshot_index)?;
    dict.set_item("timestamp", metadata.timestamp.timestamp())?;
    dict.set_item("format_version", metadata.format_version)?;
    dict.set_item("content_hash", metadata.content_hash)?;
    dict.set_item("compression_algorithm", metadata.compression_algorithm)?;

    if let Some(desc) = &metadata.description {
        dict.set_item("description", desc)?;
    }
    if let Some(size) = metadata.compressed_size {
        dict.set_item("compressed_size", size)?;
    }
    if let Some(snapshot_id) = Some(&metadata.snapshot_id) {
        dict.set_item("snapshot_id", snapshot_id)?;
    }

    Ok(dict.into())
}

/// Verify the integrity of a snapshot
///
/// # Arguments
/// * `path` - Storage path/key of the snapshot to verify
/// * `storage_mode` - Storage backend: "local" or "s3" (default: "local")
/// * `s3_bucket` - S3 bucket name (required for S3 mode)
/// * `s3_region` - S3 region (optional, uses AWS environment default)
///
/// # Returns
/// None on success (integrity verified)
///
/// # Raises
/// * IOError - If verification fails or snapshot is corrupted
#[pyfunction]
#[pyo3(signature = (path, storage_mode=None, s3_bucket=None, s3_region=None))]
fn verify_snapshot(
    path: &str,
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<()> {
    let config = create_storage_config(storage_mode, s3_bucket, s3_region)?;
    let engine = create_engine_from_config(config).map_err(convert_error)?;

    engine.verify_snapshot(path).map_err(convert_error)?;

    Ok(())
}

/// Check if a snapshot exists
///
/// # Arguments
/// * `path` - Storage path/key to check
/// * `storage_mode` - Storage backend: "local" or "s3" (default: "local")
/// * `s3_bucket` - S3 bucket name (required for S3 mode)
/// * `s3_region` - S3 region (optional, uses AWS environment default)
///
/// # Returns
/// True if the snapshot exists, False otherwise
#[pyfunction]
#[pyo3(signature = (path, storage_mode=None, s3_bucket=None, s3_region=None))]
fn snapshot_exists(
    path: &str,
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<bool> {
    let config = create_storage_config(storage_mode, s3_bucket, s3_region)
        .unwrap_or_else(|_| StorageConfig::default_local()); // Fallback to local on error

    let engine = create_engine_from_config(config);
    match engine {
        Ok(e) => Ok(e.snapshot_exists(path)),
        Err(_) => Ok(false), // If engine creation fails, assume snapshot doesn't exist
    }
}

/// Delete a snapshot
///
/// # Arguments
/// * `path` - Storage path/key of the snapshot to delete
/// * `storage_mode` - Storage backend: "local" or "s3" (default: "local")
/// * `s3_bucket` - S3 bucket name (required for S3 mode)
/// * `s3_region` - S3 region (optional, uses AWS environment default)
///
/// # Returns
/// None on success
///
/// # Raises
/// * IOError - If deletion fails
#[pyfunction]
#[pyo3(signature = (path, storage_mode=None, s3_bucket=None, s3_region=None))]
fn delete_snapshot(
    path: &str,
    storage_mode: Option<&str>,
    s3_bucket: Option<&str>,
    s3_region: Option<&str>,
) -> PyResult<()> {
    let config = create_storage_config(storage_mode, s3_bucket, s3_region)?;
    let engine = create_engine_from_config(config).map_err(convert_error)?;

    engine.delete_snapshot(path).map_err(convert_error)?;

    Ok(())
}

/// Python module definition
#[pymodule]
fn persist(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add main functions
    m.add_function(wrap_pyfunction!(snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(restore, m)?)?;
    m.add_function(wrap_pyfunction!(get_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(verify_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(snapshot_exists, m)?)?;
    m.add_function(wrap_pyfunction!(delete_snapshot, m)?)?;

    // Add custom exception classes
    m.add("PersistError", m.py().get_type::<PyPersistError>())?;
    m.add(
        "PersistConfigurationError",
        m.py().get_type::<PyPersistConfigurationError>(),
    )?;
    m.add(
        "PersistIntegrityError",
        m.py().get_type::<PyPersistIntegrityError>(),
    )?;
    m.add("PersistS3Error", m.py().get_type::<PyPersistS3Error>())?;
    m.add(
        "PersistCompressionError",
        m.py().get_type::<PyPersistCompressionError>(),
    )?;

    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add(
        "__doc__",
        "Enterprise-grade agent snapshot and restore system with S3 support",
    )?;

    Ok(())
}
