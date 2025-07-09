import json
import gzip
import os
from pathlib import Path
import time

__version__ = "0.1.0"
__doc__ = "Enterprise-grade agent snapshot and restore system (pure python fallback)"

class PersistError(Exception):
    """Base exception for Persist operations."""

class PersistConfigurationError(PersistError):
    """Raised for invalid configuration options."""

class PersistIntegrityError(PersistError):
    """Raised when snapshot integrity verification fails."""

class PersistS3Error(PersistError):
    """Raised for S3 related errors."""

class PersistCompressionError(PersistError):
    """Raised when compression or decompression fails."""

_METADATA_REGISTRY = {}

def dumps(obj):
    """Serialize an object using LangChain if available."""
    if hasattr(obj, "dumps"):
        return obj.dumps()
    try:
        from langchain.load import dumps as lc_dumps
    except Exception:
        try:
            from langchain_core.load import dumps as lc_dumps
        except Exception as e:
            raise ImportError("langchain module required for dumps()") from e
    return lc_dumps(obj)

def loads(data, cls=None):
    """Deserialize JSON data using LangChain if available."""
    if cls and hasattr(cls, "loads"):
        return cls.loads(data)
    try:
        from langchain.load import loads as lc_loads
    except Exception:
        try:
            from langchain_core.load import loads as lc_loads
        except Exception:
            return json.loads(data)
    return lc_loads(data)

def _validate_mode(storage_mode):
    if storage_mode and storage_mode not in ("local", "s3"):
        raise PersistConfigurationError(f"Invalid storage_mode: {storage_mode}")
    if storage_mode == "s3":
        raise PersistS3Error("S3 mode not implemented in fallback module")

def snapshot(agent, path, agent_id="default_agent", session_id="default_session", snapshot_index=0, description=None, storage_mode=None, s3_bucket=None, s3_region=None):
    _validate_mode(storage_mode)
    path = Path(path)
    try:
        data = dumps(agent)
        # Validate JSON so tests expecting failures on malformed data work
        json.loads(data)
        with gzip.open(path, "wt", encoding="utf-8") as f:
            f.write(data)
        _METADATA_REGISTRY[str(path)] = {
            "agent_id": agent_id,
            "session_id": session_id,
            "snapshot_index": snapshot_index,
            "description": description,
            "format_version": 1,
            "timestamp": time.time(),
            "uncompressed_size": len(data),
            "compressed_size": os.path.getsize(path),
            "compression_algorithm": "gzip",
        }
    except OSError as e:
        raise e
    except Exception as e:
        raise PersistError(str(e))


def restore(path, storage_mode=None, s3_bucket=None, s3_region=None):
    _validate_mode(storage_mode)
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"Snapshot not found: {path}")
    try:
        with gzip.open(path, "rt", encoding="utf-8") as f:
            data = f.read()
        return loads(data)
    except OSError as e:
        raise e
    except Exception as e:
        raise PersistError(str(e))


def get_metadata(path, storage_mode=None, s3_bucket=None, s3_region=None):
    _validate_mode(storage_mode)
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"Snapshot not found: {path}")
    meta = _METADATA_REGISTRY.get(str(path))
    if not meta:
        raise PersistError("Metadata not found")
    return meta


def verify_snapshot(path, storage_mode=None, s3_bucket=None, s3_region=None):
    """Return True if snapshot is readable and valid JSON."""
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"Snapshot not found: {path}")
    try:
        with gzip.open(path, "rt", encoding="utf-8") as f:
            data = f.read()
        json.loads(data)
        return True
    except json.JSONDecodeError as e:
        raise PersistIntegrityError(str(e))
    except OSError as e:
        raise e


def snapshot_exists(path, storage_mode=None, s3_bucket=None, s3_region=None):
    _validate_mode(storage_mode)
    if not path:
        return False
    return Path(path).exists()


def delete_snapshot(path, storage_mode=None, s3_bucket=None, s3_region=None):
    _validate_mode(storage_mode)
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"Snapshot not found: {path}")
    path.unlink()
    _METADATA_REGISTRY.pop(str(path), None)
