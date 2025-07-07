/*!
Python bindings for the Persist agent snapshot system.

This module provides a Pythonic interface to the Rust-based persist-core library,
enabling easy snapshotting and restoration of LangChain agents and other AI agents.

## Example Usage

```python
import persist

# Assuming `agent` is a LangChain agent or chain object
persist.snapshot(agent, "/path/to/snapshots/agent1_snapshot.json.gz")

# Later or in another process
restored_agent = persist.restore("/path/to/snapshots/agent1_snapshot.json.gz")
```
*/

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;
use pyo3::types::{PyDict, PyModule};
use persist_core::{
    SnapshotMetadata, create_default_engine, PersistError
};

/// Convert a Rust PersistError to a Python exception
fn convert_error(err: PersistError) -> PyErr {
    match err {
        PersistError::Io(io_err) => PyIOError::new_err(format!("I/O error: {}", io_err)),
        PersistError::Json(json_err) => PyIOError::new_err(format!("JSON error: {}", json_err)),
        PersistError::Compression(msg) => PyIOError::new_err(format!("Compression error: {}", msg)),
        PersistError::IntegrityCheckFailed { expected, actual } => {
            PyIOError::new_err(format!("Integrity check failed: expected {}, got {}", expected, actual))
        },
        PersistError::InvalidFormat(msg) => PyIOError::new_err(format!("Invalid format: {}", msg)),
        PersistError::MissingMetadata(field) => PyIOError::new_err(format!("Missing metadata: {}", field)),
        PersistError::Storage(msg) => PyIOError::new_err(format!("Storage error: {}", msg)),
        PersistError::Validation(msg) => PyIOError::new_err(format!("Validation error: {}", msg)),
    }
}

/// Save an agent snapshot to disk
/// 
/// This function serializes a LangChain agent (or other compatible object) to a compressed
/// snapshot file on disk. The agent must be serializable using LangChain's dumps() function.
/// 
/// # Arguments
/// * `agent` - The agent object to snapshot (must support LangChain serialization)
/// * `path` - File path where the snapshot should be saved (e.g., "agent_snapshot.json.gz")
/// * `agent_id` - Optional unique identifier for the agent (default: "default_agent")
/// * `session_id` - Optional session identifier (default: "default_session")
/// * `snapshot_index` - Optional sequence number for this snapshot (default: 0)
/// * `description` - Optional human-readable description of the snapshot
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
/// # Create and use an agent
/// agent = ConversationChain(...)
/// agent.predict("Hello, how are you?")
/// 
/// # Save snapshot
/// persist.snapshot(agent, "my_agent_snapshot.json.gz", 
///                 agent_id="conversation_agent", 
///                 description="After greeting interaction")
/// ```
#[pyfunction]
#[pyo3(signature = (agent, path, agent_id="default_agent", session_id="default_session", snapshot_index=0, description=None))]
fn snapshot(
    py: Python<'_>,
    agent: &Bound<'_, PyAny>,
    path: &str,
    agent_id: &str,
    session_id: &str,
    snapshot_index: u64,
    description: Option<&str>,
) -> PyResult<()> {
    // Import LangChain's dump function
    let langchain_load = py.import_bound("langchain_core.load")
        .or_else(|_| py.import_bound("langchain.load"))  // Fallback for older versions
        .map_err(|_| PyIOError::new_err("Could not import langchain_core.load or langchain.load. Please ensure LangChain is installed."))?;
    
    let dumps_func = langchain_load.getattr("dumps")
        .map_err(|_| PyIOError::new_err("Could not find dumps function in LangChain load module"))?;
    
    // Serialize the agent to JSON string using LangChain's dumps
    let json_obj = dumps_func.call1((agent,))
        .map_err(|e| PyIOError::new_err(format!("Failed to serialize agent with LangChain dumps: {}", e)))?;
    
    let agent_json: String = json_obj.extract()
        .map_err(|e| PyIOError::new_err(format!("Failed to extract JSON string from LangChain dumps result: {}", e)))?;
    
    // Create metadata
    let mut metadata = SnapshotMetadata::new(agent_id, session_id, snapshot_index);
    if let Some(desc) = description {
        metadata = metadata.with_description(desc);
    }
    
    // Create engine and save snapshot
    let engine = create_default_engine();
    let _saved_metadata = engine.save_snapshot(&agent_json, &metadata, path)
        .map_err(convert_error)?;
    
    Ok(())
}

/// Restore an agent snapshot from disk
/// 
/// This function loads a compressed snapshot file and reconstructs the original agent
/// object using LangChain's loads() function.
/// 
/// # Arguments
/// * `path` - File path of the snapshot to restore
/// * `secrets_map` - Optional dictionary of secrets/API keys for the restored agent
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
/// # Set up any required API keys in environment
/// os.environ["OPENAI_API_KEY"] = "your-api-key"
/// 
/// # Restore snapshot
/// restored_agent = persist.restore("my_agent_snapshot.json.gz")
/// 
/// # Use the restored agent
/// response = restored_agent.predict("Continue our conversation")
/// ```
#[pyfunction]
#[pyo3(signature = (path, secrets_map=None))]
fn restore(
    py: Python<'_>,
    path: &str,
    secrets_map: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    // Load snapshot
    let engine = create_default_engine();
    let (_metadata, agent_json) = engine.load_snapshot(path)
        .map_err(convert_error)?;
    
    // Import LangChain's load function
    let langchain_load = py.import_bound("langchain_core.load")
        .or_else(|_| py.import_bound("langchain.load"))  // Fallback for older versions
        .map_err(|_| PyIOError::new_err("Could not import langchain_core.load or langchain.load. Please ensure LangChain is installed."))?;
    
    let loads_func = langchain_load.getattr("loads")
        .map_err(|_| PyIOError::new_err("Could not find loads function in LangChain load module"))?;
    
    // Call loads with the JSON string and optional secrets_map
    let py_agent = if let Some(secrets) = secrets_map {
        loads_func.call1((agent_json, secrets))
            .map_err(|e| PyIOError::new_err(format!("Failed to deserialize agent with LangChain loads: {}", e)))?
    } else {
        loads_func.call1((agent_json,))
            .map_err(|e| PyIOError::new_err(format!("Failed to deserialize agent with LangChain loads: {}", e)))?
    };
    
    Ok(py_agent.into())
}

/// Get metadata from a snapshot without loading the full agent
/// 
/// This function is useful for inspecting snapshot information without the overhead
/// of deserializing the complete agent state.
/// 
/// # Arguments
/// * `path` - File path of the snapshot
/// 
/// # Returns
/// Dictionary containing snapshot metadata
/// 
/// # Example
/// ```python
/// import persist
/// 
/// metadata = persist.get_metadata("my_agent_snapshot.json.gz")
/// print(f"Agent ID: {metadata['agent_id']}")
/// print(f"Created: {metadata['timestamp']}")
/// print(f"Description: {metadata['description']}")
/// ```
#[pyfunction]
fn get_metadata(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let engine = create_default_engine();
    let metadata = engine.get_snapshot_metadata(path)
        .map_err(convert_error)?;
    
    // Convert metadata to Python dictionary
    let dict = PyDict::new_bound(py);
    dict.set_item("agent_id", metadata.agent_id)?;
    dict.set_item("session_id", metadata.session_id)?;
    dict.set_item("snapshot_index", metadata.snapshot_index)?;
    dict.set_item("timestamp", metadata.timestamp.to_rfc3339())?;
    dict.set_item("content_hash", metadata.content_hash)?;
    dict.set_item("format_version", metadata.format_version)?;
    dict.set_item("snapshot_id", metadata.snapshot_id)?;
    dict.set_item("description", metadata.description)?;
    dict.set_item("uncompressed_size", metadata.uncompressed_size)?;
    dict.set_item("compressed_size", metadata.compressed_size)?;
    dict.set_item("compression_algorithm", metadata.compression_algorithm)?;
    
    Ok(dict.into())
}

/// Verify the integrity of a snapshot file
/// 
/// This function checks that a snapshot file is valid and can be loaded successfully.
/// It verifies compression, JSON format, and content integrity.
/// 
/// # Arguments
/// * `path` - File path of the snapshot to verify
/// 
/// # Returns
/// True if the snapshot is valid, False otherwise
/// 
/// # Example
/// ```python
/// import persist
/// 
/// if persist.verify_snapshot("my_agent_snapshot.json.gz"):
///     print("Snapshot is valid")
/// else:
///     print("Snapshot is corrupted or invalid")
/// ```
#[pyfunction]
fn verify_snapshot(path: &str) -> PyResult<bool> {
    let engine = create_default_engine();
    match engine.verify_snapshot(path) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Check if a snapshot file exists
/// 
/// # Arguments
/// * `path` - File path to check
/// 
/// # Returns
/// True if the snapshot exists, False otherwise
#[pyfunction]
fn snapshot_exists(path: &str) -> bool {
    let engine = create_default_engine();
    engine.snapshot_exists(path)
}

/// Delete a snapshot file
/// 
/// # Arguments
/// * `path` - File path of the snapshot to delete
/// 
/// # Returns
/// None on success
/// 
/// # Raises
/// * IOError - If deletion fails
#[pyfunction]
fn delete_snapshot(path: &str) -> PyResult<()> {
    let engine = create_default_engine();
    engine.delete_snapshot(path)
        .map_err(convert_error)?;
    Ok(())
}

/// Python module definition
#[pymodule]
fn persist(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(restore, m)?)?;
    m.add_function(wrap_pyfunction!(get_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(verify_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(snapshot_exists, m)?)?;
    m.add_function(wrap_pyfunction!(delete_snapshot, m)?)?;
    
    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add("__doc__", "Enterprise-grade agent snapshot and restore system")?;
    
    Ok(())
}
