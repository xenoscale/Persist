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
# Install from pre-built package (when available)
pip install --pre persist

# Or build and install from source
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


### S3 Cloud Storage

```python
import persist
import os

# Configure AWS credentials
os.environ["AWS_ACCESS_KEY_ID"] = "your-access-key"
os.environ["AWS_SECRET_ACCESS_KEY"] = "your-secret-key"
os.environ["AWS_REGION"] = "us-west-2"

# Save snapshot to S3
persist.snapshot(
    agent, 
    "agents/conversation_bot/session_123/snapshot.json.gz",
    storage_mode="s3",
    s3_bucket="my-ai-snapshots-bucket",
    agent_id="conversation_bot",
    description="Production snapshot"
)

# Restore from S3
restored_agent = persist.restore(
    "agents/conversation_bot/session_123/snapshot.json.gz",
    storage_mode="s3", 
    s3_bucket="my-ai-snapshots-bucket"
)

# S3 metadata operations
metadata = persist.get_metadata(
    "agents/conversation_bot/session_123/snapshot.json.gz",
    storage_mode="s3",
    s3_bucket="my-ai-snapshots-bucket"
)
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

## 📊 Observability & Monitoring

Persist includes comprehensive observability features for production deployment, providing structured logging, distributed tracing, and detailed metrics.

### Structured Logging

All operations are automatically logged with rich context:

```python
import persist
import os

# Enable detailed logging
os.environ["RUST_LOG"] = "persist_core=info"

# Operations automatically generate structured logs
persist.snapshot(agent, "agent.json.gz", agent_id="prod_agent")
# Logs: INFO persist_core::snapshot: Save operation completed 
#       agent_id="prod_agent" duration_ms=150 size_bytes=2048
```

### Distributed Tracing

Trace complete operation flows with OpenTelemetry integration:

```bash
# Enable tracing export to Jaeger
export PERSIST_JAEGER_ENDPOINT="http://localhost:14268/api/traces"

# View traces at http://localhost:16686
```

### Metrics & Monitoring

Prometheus-compatible metrics for operational insights:

```bash
# Access metrics endpoint
curl http://localhost:9090/metrics

# Key metrics available:
# - persist_s3_requests_total: Total requests by operation
# - persist_s3_errors_total: Error count by operation  
# - persist_s3_latency_seconds: Operation latency histogram
# - persist_state_size_bytes: Agent state size distribution
```

### Error Handling

Enhanced error types provide detailed context:

```python
try:
    persist.restore("missing_snapshot.json.gz")
except FileNotFoundError as e:
    print(f"Snapshot not found: {e}")
except PermissionError as e:
    print(f"Access denied: {e}")
except persist.S3Error as e:
    print(f"S3 operation failed: {e}")
```

### Production Configuration

```bash
# Recommended production settings
export RUST_LOG="persist_core=info"           # Appropriate log level
export PERSIST_LOG_FORMAT="json"              # Machine-parseable logs
export PERSIST_METRICS_ENABLED="true"         # Enable metrics collection
export PERSIST_TRACING_ENABLED="true"         # Enable distributed tracing
```

For detailed observability setup and configuration, see [docs/observability.md](docs/observability.md).

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

### 🎯 One-Command Build and Test

The easiest way to build and test everything:

```bash
# Complete build and test automation
./build-and-test.sh

# Quick development cycle (fast iteration)
./build-and-test.sh --quick

# Show all options
./build-and-test.sh --help
```

Alternative using Make:

```bash
# Complete pipeline
make all

# Quick development cycle
make quick

# Show all available targets
make help
```

### 🛠 Manual Building

If you prefer manual control:

```bash
# Build Rust core
cargo build --release -p persist-core

# Build CLI tool
cargo build --release -p persist-cli

# Build Python package
cd persist-python
maturin develop --release
cd ..
```

### 🧪 Testing

```bash
# Run all tests with automation script
./build-and-test.sh --skip-build

# Or run tests manually
cargo test --workspace --all-features
cargo test --doc

# Python tests (after building extension)
cd persist-python && pytest && cd ..
```

### 📋 Code Quality

```bash
# Format code
./scripts/format.sh
# OR: make format

# Lint code  
./scripts/lint.sh
# OR: make lint

# Run comprehensive tests
./scripts/run_comprehensive_tests.sh
```

### ⚡ Development Workflow

For fast development iterations:

```bash
# Option 1: Use automation script
./build-and-test.sh --quick

# Option 2: Use Make
make quick

# Option 3: Use individual scripts
./scripts/format.sh && ./scripts/lint.sh && cargo test
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
- ✅ S3 cloud storage support
- ✅ Structured logging and error handling
- ✅ Distributed tracing with OpenTelemetry
- ✅ Prometheus metrics and monitoring
- ✅ Comprehensive end-to-end testing

### Future Enhancements
- 🔄 Automated periodic snapshots
- ☁️ Additional cloud storage backends (Azure Blob, GCP Storage)
- 🔐 Encryption at rest
- 🤖 Support for additional AI frameworks (Auto-GPT, HuggingFace)
- 📊 Snapshot management UI
- 🏃‍♂️ Streaming compression for large agents
- 🔍 Advanced observability dashboards
- 🚨 Built-in alerting and health checks

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
