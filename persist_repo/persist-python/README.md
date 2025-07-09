# Persist Python SDK

Enterprise-grade agent snapshot and restore system for Python.

## Quick Start

```python
import persist
from langchain.chains import ConversationChain

# Create and use an agent
agent = ConversationChain(...)
agent.predict("Hello, how are you?")

# Save snapshot
persist.snapshot(agent, "agent_snapshot.json.gz")

# Later, restore the agent
restored_agent = persist.restore("agent_snapshot.json.gz")
```

## Installation

```bash
pip install persist
```

## Features

- **LangChain Integration**: Seamless serialization/deserialization of LangChain agents
- **Compression**: Automatic gzip compression to reduce file sizes
- **Integrity Verification**: SHA-256 hash verification ensures data integrity
- **Rich Metadata**: Comprehensive snapshot metadata for tracking and management
- **Fast**: Rust-powered core for high performance

## API Reference

### `snapshot(agent, path, **kwargs)`

Save an agent to a snapshot file.

**Parameters:**
- `agent`: LangChain agent or chain object
- `path`: File path for the snapshot
- `agent_id`: Optional agent identifier (default: "default_agent")
- `session_id`: Optional session identifier (default: "default_session")
- `snapshot_index`: Optional sequence number (default: 0)
- `description`: Optional description

### `restore(path, secrets_map=None)`

Restore an agent from a snapshot file.

**Parameters:**
- `path`: Path to the snapshot file
- `secrets_map`: Optional dictionary of secrets/API keys

**Returns:** Restored agent object

### `get_metadata(path)`

Get snapshot metadata without loading the agent.

**Returns:** Dictionary with metadata information

### `verify_snapshot(path)`

Verify snapshot file integrity.

**Returns:** `True` if valid, `False` otherwise

### `snapshot_exists(path)`

Check if a snapshot file exists.

### `delete_snapshot(path)`

Delete a snapshot file.

## License

Proprietary - Internal use only.
