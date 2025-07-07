# Persist - Agent Snapshot & Restore System (MVP)

[![Rust CI](https://github.com/xenoscale/Persist/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/xenoscale/Persist/actions/workflows/rust-ci.yml)
[![Python CI](https://github.com/xenoscale/Persist/actions/workflows/python-ci.yml/badge.svg)](https://github.com/xenoscale/Persist/actions/workflows/python-ci.yml)

Enterprise-grade agent snapshot and restore system for AI agents, designed to capture an agent's state and later reconstruct it with perfect fidelity.

## 🎯 Key Features

- **Manual Snapshot/Restore APIs**: Explicit save/load functions for agent state management
- **LangChain Integration**: Native support for LangChain agents and chains
- **Local Disk Storage**: Efficient local filesystem storage with pluggable architecture for future cloud support
- **Compressed Snapshots**: Automatic gzip compression to minimize storage footprint
- **Rich Metadata**: Comprehensive tracking with unique IDs, timestamps, and integrity verification
- **Python SDK**: Ergonomic Python interface powered by a high-performance Rust core
- **Hexagonal Architecture**: Modular, extensible design for easy customization

## 🚀 Quick Start

### Installation

```bash
# Install from wheel (after building)
pip install persist-0.1.0-cp312-cp312-manylinux_2_34_x86_64.whl

# Or build from source
cd persist-python
maturin develop --release
```

### Basic Usage

```python
import persist
from langchain.chains import ConversationChain

# Create and use an agent
agent = ConversationChain(...)
agent.predict("Hello, how are you?")

# Save snapshot
persist.snapshot(agent, "agent_snapshot.json.gz", 
                agent_id="conversation_agent",
                description="After greeting interaction")

# Later, restore the agent
restored_agent = persist.restore("agent_snapshot.json.gz")

# Continue using the restored agent
response = restored_agent.predict("What did we talk about?")
```

### Advanced Usage

```python
import persist

# Get snapshot metadata
metadata = persist.get_metadata("agent_snapshot.json.gz")
print(f"Agent: {metadata['agent_id']}")
print(f"Created: {metadata['timestamp']}")
print(f"Size: {metadata['uncompressed_size']} bytes")

# Verify snapshot integrity
if persist.verify_snapshot("agent_snapshot.json.gz"):
    print("Snapshot is valid")

# Manage snapshots
if persist.snapshot_exists("old_snapshot.json.gz"):
    persist.delete_snapshot("old_snapshot.json.gz")
```

## 🏗 Architecture

Persist follows hexagonal architecture principles with clear separation of concerns:

### Core Components

- **`persist-core`** (Rust): High-performance core engine with pluggable storage and compression
- **`persist-python`** (Python): PyO3-based bindings providing a native Python experience
- **Storage Adapters**: Local filesystem (included), S3 and other cloud storage (future)
- **Compression Adapters**: Gzip (included), Zstandard and others (future)

### Design Principles

1. **Domain Logic Isolation**: Core business logic independent of infrastructure
2. **Pluggable Components**: Easy to extend with new storage backends or compression algorithms
3. **Type Safety**: Rust's type system ensures memory safety and prevents data corruption
4. **Performance**: Zero-copy operations and efficient compression for minimal overhead

## 🔧 Development

### Prerequisites

- Rust 1.65+ (stable)
- Python 3.8+
- Maturin for Python package building

### Building

```bash
# Build Rust core
cargo build --release -p persist-core

# Run Rust tests
cargo test -p persist-core

# Build Python package
cd persist-python
maturin build --release

# Install for development
maturin develop --release
```

### Testing

```bash
# Rust tests
cargo test --workspace

# Python tests (after installing package)
pytest tests/
```

## 📁 Repository Structure

```
Persist/
├── persist-core/          # Rust core engine
│   ├── src/
│   │   ├── lib.rs         # Public API
│   │   ├── snapshot.rs    # Main engine
│   │   ├── metadata.rs    # Metadata management
│   │   ├── storage.rs     # Storage adapters
│   │   ├── compression.rs # Compression adapters
│   │   └── error.rs       # Error handling
│   └── Cargo.toml
├── persist-python/        # Python SDK
│   ├── src/lib.rs         # PyO3 bindings
│   ├── Cargo.toml         # Rust configuration
│   └── pyproject.toml     # Python packaging
├── .github/workflows/     # CI/CD pipelines
├── tests/                 # Integration tests
└── docs/                  # Documentation
```

## 🔒 Snapshot Format

Snapshots are stored as compressed JSON files containing:

- **Metadata**: Agent ID, session info, timestamps, integrity hashes
- **Agent State**: Serialized agent data (via LangChain's dumps/loads)
- **Format Version**: For backward compatibility

### Metadata Schema

```json
{
  "metadata": {
    "agent_id": "conversation_agent",
    "session_id": "default_session",
    "snapshot_index": 0,
    "timestamp": "2025-07-08T02:11:15Z",
    "content_hash": "sha256_hash_here",
    "format_version": 1,
    "snapshot_id": "uuid_here",
    "description": "After greeting interaction",
    "uncompressed_size": 1024,
    "compressed_size": 512,
    "compression_algorithm": "gzip"
  },
  "agent_state": { /* LangChain serialized data */ }
}
```

## 🛣 Roadmap

### MVP (Current)
- ✅ Manual snapshot/restore APIs
- ✅ LangChain integration
- ✅ Local filesystem storage
- ✅ Gzip compression
- ✅ Python SDK with PyO3

### Future Enhancements
- 🔄 Automated periodic snapshots
- ☁️ Cloud storage backends (S3, Azure Blob, GCP Storage)
- 🔐 Encryption at rest
- 🤖 Support for additional AI frameworks (Auto-GPT, HuggingFace)
- 📊 Snapshot management UI
- 🏃‍♂️ Streaming compression for large agents

## 🤝 Contributing

This is a private repository for internal use. For questions or issues, please contact the development team.

### Code Standards

- **Rust**: Use `cargo fmt` and `cargo clippy`
- **Python**: Follow PEP 8, use type hints
- **Tests**: Maintain high test coverage
- **Documentation**: Update docs for any API changes

## 📄 License

Proprietary - Internal use only. All rights reserved.

## 🙋‍♂️ Support

For technical support or questions:
- Check the [documentation](docs/)
- Review existing [issues](../../issues)
- Contact: MiniMax Agent

---

**Built with ❤️ using Rust and Python**
