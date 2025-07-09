"""
Type stubs for the persist Python module.

This file provides type annotations for IDE support and static type checking.
"""

from typing import Any

__version__: str

class PersistError(Exception):
    """Base exception for Persist operations."""
    pass

class PersistConfigurationError(PersistError):
    """Raised when there's a configuration error."""
    pass

class PersistIntegrityError(PersistError):
    """Raised when data integrity verification fails."""
    pass

class PersistS3Error(PersistError):
    """Raised when S3 storage operations fail."""
    pass

class PersistCompressionError(PersistError):
    """Raised when compression/decompression operations fail."""
    pass

def snapshot(
    agent: Any,
    path: str,
    agent_id: str = "default_agent",
    session_id: str = "default_session",
    snapshot_index: int = 0,
    description: str | None = None,
    storage_mode: str | None = None,
    s3_bucket: str | None = None,
    s3_region: str | None = None,
) -> None:
    """
    Save an agent snapshot with configurable storage backend.
    
    This function serializes a LangChain agent (or other compatible object) to a compressed
    snapshot file. Supports both local filesystem and Amazon S3 storage backends.
    
    Args:
        agent: The agent object to snapshot (must support LangChain serialization)
        path: Storage path/key for the snapshot
        agent_id: Unique identifier for the agent (default: "default_agent")
        session_id: Session identifier (default: "default_session")
        snapshot_index: Sequence number for this snapshot (default: 0)
        description: Human-readable description of the snapshot
        storage_mode: Storage backend - "local" or "s3" (default: "local")
        s3_bucket: S3 bucket name (required for S3 mode)
        s3_region: S3 region (optional, uses AWS environment default)
    
    Raises:
        PersistError: If saving fails
        PersistConfigurationError: If configuration is invalid
        PersistS3Error: If S3 operations fail
        PersistCompressionError: If compression fails
        IOError: If I/O operations fail
        FileNotFoundError: If S3 object not found
        PermissionError: If access denied
    
    Example:
        >>> import persist
        >>> from langchain.chains import ConversationChain
        >>> 
        >>> # Local storage
        >>> persist.snapshot(agent, "snapshots/agent1.json.gz")
        >>> 
        >>> # S3 storage
        >>> persist.snapshot(agent, "agent1/session1/snapshot.json.gz",
        ...                 storage_mode="s3",
        ...                 s3_bucket="my-snapshots-bucket",
        ...                 agent_id="conversation_agent")
    """
    ...

def restore(
    path: str,
    storage_mode: str | None = None,
    s3_bucket: str | None = None,
    s3_region: str | None = None,
) -> Any:
    """
    Restore an agent from a snapshot.
    
    This function loads and deserializes a previously saved agent snapshot,
    reconstructing the original agent object using LangChain's loading mechanisms.
    
    Args:
        path: Storage path/key of the snapshot to restore
        storage_mode: Storage backend - "local" or "s3" (default: "local")
        s3_bucket: S3 bucket name (required for S3 mode)
        s3_region: S3 region (optional, uses AWS environment default)
    
    Returns:
        The restored agent object
    
    Raises:
        PersistError: If restoration fails
        PersistIntegrityError: If integrity verification fails
        PersistConfigurationError: If configuration is invalid
        PersistS3Error: If S3 operations fail
        PersistCompressionError: If decompression fails
        IOError: If I/O operations fail
        FileNotFoundError: If snapshot not found
        PermissionError: If access denied
    
    Example:
        >>> import persist
        >>> 
        >>> # Restore from local storage
        >>> agent = persist.restore("snapshots/agent1.json.gz")
        >>> 
        >>> # Restore from S3
        >>> agent = persist.restore("agent1/session1/snapshot.json.gz",
        ...                        storage_mode="s3",
        ...                        s3_bucket="my-snapshots-bucket")
    """
    ...

def get_metadata(
    path: str,
    storage_mode: str | None = None,
    s3_bucket: str | None = None,
    s3_region: str | None = None,
) -> dict[str, str | int | float]:
    """
    Get metadata for a snapshot without loading the full snapshot.
    
    Args:
        path: Storage path/key of the snapshot
        storage_mode: Storage backend - "local" or "s3" (default: "local")
        s3_bucket: S3 bucket name (required for S3 mode)
        s3_region: S3 region (optional, uses AWS environment default)
    
    Returns:
        Dictionary containing snapshot metadata with keys:
        - agent_id: Agent identifier
        - session_id: Session identifier  
        - snapshot_index: Sequence number
        - timestamp: Unix timestamp when snapshot was created
        - format_version: Snapshot format version
        - content_hash: SHA-256 hash of the content
        - compression_algorithm: Compression algorithm used
        - description: Optional description (if present)
        - compressed_size: Size of compressed data (if available)
        - snapshot_id: Unique snapshot identifier (if present)
    
    Raises:
        PersistError: If metadata retrieval fails
        PersistConfigurationError: If configuration is invalid
        PersistS3Error: If S3 operations fail
        IOError: If I/O operations fail
        FileNotFoundError: If snapshot not found
    
    Example:
        >>> metadata = persist.get_metadata("snapshots/agent1.json.gz")
        >>> print(f"Agent: {metadata['agent_id']}, Created: {metadata['timestamp']}")
    """
    ...

def verify_snapshot(
    path: str,
    storage_mode: str | None = None,
    s3_bucket: str | None = None,
    s3_region: str | None = None,
) -> None:
    """
    Verify the integrity of a snapshot.
    
    This function checks the integrity of a snapshot by verifying its hash
    and ensuring the data hasn't been corrupted.
    
    Args:
        path: Storage path/key of the snapshot to verify
        storage_mode: Storage backend - "local" or "s3" (default: "local")
        s3_bucket: S3 bucket name (required for S3 mode)
        s3_region: S3 region (optional, uses AWS environment default)
    
    Raises:
        PersistIntegrityError: If verification fails or snapshot is corrupted
        PersistConfigurationError: If configuration is invalid
        PersistS3Error: If S3 operations fail
        IOError: If I/O operations fail
        FileNotFoundError: If snapshot not found
    
    Example:
        >>> persist.verify_snapshot("snapshots/agent1.json.gz")
        >>> print("Snapshot integrity verified!")
    """
    ...

def snapshot_exists(
    path: str,
    storage_mode: str | None = None,
    s3_bucket: str | None = None,
    s3_region: str | None = None,
) -> bool:
    """
    Check if a snapshot exists.
    
    Args:
        path: Storage path/key to check
        storage_mode: Storage backend - "local" or "s3" (default: "local")
        s3_bucket: S3 bucket name (required for S3 mode)
        s3_region: S3 region (optional, uses AWS environment default)
    
    Returns:
        True if the snapshot exists, False otherwise
    
    Example:
        >>> if persist.snapshot_exists("snapshots/agent1.json.gz"):
        ...     print("Snapshot exists!")
        ... else:
        ...     print("Snapshot not found")
    """
    ...

def delete_snapshot(
    path: str,
    storage_mode: str | None = None,
    s3_bucket: str | None = None,
    s3_region: str | None = None,
) -> None:
    """
    Delete a snapshot.
    
    Args:
        path: Storage path/key of the snapshot to delete
        storage_mode: Storage backend - "local" or "s3" (default: "local")
        s3_bucket: S3 bucket name (required for S3 mode)
        s3_region: S3 region (optional, uses AWS environment default)
    
    Raises:
        PersistError: If deletion fails
        PersistConfigurationError: If configuration is invalid
        PersistS3Error: If S3 operations fail
        IOError: If I/O operations fail
        FileNotFoundError: If snapshot not found
        PermissionError: If access denied
    
    Example:
        >>> persist.delete_snapshot("snapshots/old_agent.json.gz")
        >>> print("Snapshot deleted!")
    """
    ...
