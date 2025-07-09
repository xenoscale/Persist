"""
Integration tests for Persist S3 storage backend.

These tests verify the S3 integration functionality. 
Set RUN_S3_TESTS=1 and appropriate AWS environment variables to run S3 tests.
"""

import os
import sys
import tempfile
import pytest
import uuid
from unittest.mock import Mock, patch

# Add the persist-python package to the path for testing
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../persist-python'))

try:
    import persist
except ImportError:
    pytest.skip("persist module not built yet", allow_module_level=True)


class MockLangChainAgent:
    """Mock LangChain agent for testing."""
    
    def __init__(self, agent_id="test_agent", memory_data=None):
        self.agent_id = agent_id
        self.memory_data = memory_data or ["Hello", "How are you?"]
        
    def to_dict(self):
        return {
            "agent_id": self.agent_id,
            "memory": self.memory_data,
            "type": "MockAgent"
        }


@pytest.fixture
def mock_langchain_agent():
    """Provide a mock LangChain agent for testing."""
    return MockLangChainAgent()


@pytest.fixture
def temp_snapshot_path():
    """Provide a temporary file path for snapshots."""
    with tempfile.NamedTemporaryFile(suffix=".json.gz", delete=False) as tmp:
        yield tmp.name
    # Clean up
    if os.path.exists(tmp.name):
        os.unlink(tmp.name)


class TestPersistS3Integration:
    """Test S3 integration with mock data."""

    def test_s3_config_validation(self):
        """Test S3 configuration validation."""
        # Test invalid storage mode
        with pytest.raises(IOError, match="Invalid storage_mode"):
            persist.snapshot(
                MockLangChainAgent(), 
                "test_path", 
                storage_mode="invalid"
            )

    @patch('langchain_core.load.dumps')
    @patch('langchain_core.load.loads')
    def test_snapshot_local_mode(self, mock_loads, mock_dumps, mock_langchain_agent, temp_snapshot_path):
        """Test snapshot/restore with local storage mode (default)."""
        # Mock LangChain serialization
        agent_json = '{"agent_id": "test_agent", "memory": ["Hello", "How are you?"], "type": "MockAgent"}'
        mock_dumps.return_value = agent_json
        mock_loads.return_value = mock_langchain_agent
        
        # Test snapshot (local mode - default)
        persist.snapshot(mock_langchain_agent, temp_snapshot_path)
        
        # Verify file was created
        assert os.path.exists(temp_snapshot_path)
        
        # Test restore
        restored_agent = persist.restore(temp_snapshot_path)
        
        # Verify calls
        mock_dumps.assert_called_once()
        mock_loads.assert_called_once_with(agent_json)
        assert restored_agent == mock_langchain_agent

    @patch('langchain_core.load.dumps')
    @patch('langchain_core.load.loads')
    def test_snapshot_explicit_local_mode(self, mock_loads, mock_dumps, mock_langchain_agent, temp_snapshot_path):
        """Test snapshot/restore with explicitly specified local storage mode."""
        agent_json = '{"agent_id": "test_agent", "memory": ["Hello", "How are you?"], "type": "MockAgent"}'
        mock_dumps.return_value = agent_json
        mock_loads.return_value = mock_langchain_agent
        
        # Test snapshot with explicit local mode
        persist.snapshot(
            mock_langchain_agent, 
            temp_snapshot_path,
            storage_mode="local"
        )
        
        assert os.path.exists(temp_snapshot_path)
        
        # Test restore with explicit local mode
        restored_agent = persist.restore(
            temp_snapshot_path,
            storage_mode="local"
        )
        
        mock_dumps.assert_called_once()
        mock_loads.assert_called_once()

    def test_metadata_functions_local(self, temp_snapshot_path):
        """Test metadata functions with local storage."""
        # Create a simple snapshot first
        agent_json = '{"test": "data"}'
        
        with patch('langchain_core.load.dumps', return_value=agent_json):
            persist.snapshot(MockLangChainAgent(), temp_snapshot_path)
        
        # Test get_metadata
        metadata = persist.get_metadata(temp_snapshot_path)
        assert isinstance(metadata, dict)
        assert "agent_id" in metadata
        assert "session_id" in metadata
        assert "snapshot_index" in metadata
        assert "timestamp" in metadata
        
        # Test verify_snapshot
        persist.verify_snapshot(temp_snapshot_path)  # Should not raise
        
        # Test snapshot_exists
        assert persist.snapshot_exists(temp_snapshot_path) is True
        assert persist.snapshot_exists("/nonexistent/path") is False

    def test_snapshot_with_custom_metadata(self, temp_snapshot_path):
        """Test snapshot with custom metadata parameters."""
        agent_json = '{"test": "data"}'
        
        with patch('langchain_core.load.dumps', return_value=agent_json):
            persist.snapshot(
                MockLangChainAgent(),
                temp_snapshot_path,
                agent_id="custom_agent",
                session_id="session_123",
                snapshot_index=5,
                description="Test snapshot with custom metadata"
            )
        
        metadata = persist.get_metadata(temp_snapshot_path)
        assert metadata["agent_id"] == "custom_agent"
        assert metadata["session_id"] == "session_123"
        assert metadata["snapshot_index"] == 5
        assert metadata.get("description") == "Test snapshot with custom metadata"

    def test_delete_snapshot_local(self, temp_snapshot_path):
        """Test snapshot deletion with local storage."""
        # Create a snapshot
        agent_json = '{"test": "data"}'
        with patch('langchain_core.load.dumps', return_value=agent_json):
            persist.snapshot(MockLangChainAgent(), temp_snapshot_path)
        
        assert os.path.exists(temp_snapshot_path)
        
        # Delete the snapshot
        persist.delete_snapshot(temp_snapshot_path)
        
        # Verify it's gone
        assert not os.path.exists(temp_snapshot_path)
        assert persist.snapshot_exists(temp_snapshot_path) is False


@pytest.mark.skipif(
    os.environ.get("RUN_S3_TESTS") != "1",
    reason="S3 integration tests disabled. Set RUN_S3_TESTS=1 to enable."
)
class TestPersistS3RealIntegration:
    """Test S3 integration with real AWS (requires credentials)."""
    
    @pytest.fixture(scope="class")
    def s3_test_bucket(self):
        """Get S3 test bucket from environment."""
        bucket = os.environ.get("TEST_S3_BUCKET")
        if not bucket:
            pytest.skip("TEST_S3_BUCKET environment variable not set")
        return bucket
    
    def test_aws_credentials_available(self):
        """Test that AWS credentials are available."""
        required_vars = ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"]
        missing_vars = [var for var in required_vars if not os.environ.get(var)]
        
        if missing_vars:
            pytest.skip(f"AWS credentials not available. Missing: {missing_vars}")

    @patch('langchain_core.load.dumps')
    @patch('langchain_core.load.loads')
    def test_s3_snapshot_roundtrip(self, mock_loads, mock_dumps, s3_test_bucket):
        """Test S3 snapshot and restore roundtrip."""
        # Generate unique key for this test
        test_key = f"test_snapshots/test_{uuid.uuid4()}.json.gz"
        
        agent = MockLangChainAgent("s3_test_agent", ["S3 test message"])
        agent_json = '{"agent_id": "s3_test_agent", "memory": ["S3 test message"], "type": "MockAgent"}'
        
        mock_dumps.return_value = agent_json
        mock_loads.return_value = agent
        
        try:
            # Test snapshot to S3
            persist.snapshot(
                agent,
                test_key,
                storage_mode="s3",
                s3_bucket=s3_test_bucket,
                agent_id="s3_test_agent",
                description="S3 integration test"
            )
            
            # Verify snapshot exists
            assert persist.snapshot_exists(
                test_key, 
                storage_mode="s3", 
                s3_bucket=s3_test_bucket
            ) is True
            
            # Test restore from S3
            restored_agent = persist.restore(
                test_key,
                storage_mode="s3",
                s3_bucket=s3_test_bucket
            )
            
            # Verify LangChain calls
            mock_dumps.assert_called_once()
            mock_loads.assert_called_once_with(agent_json)
            
            # Test metadata retrieval
            metadata = persist.get_metadata(
                test_key,
                storage_mode="s3",
                s3_bucket=s3_test_bucket
            )
            
            assert metadata["agent_id"] == "s3_test_agent"
            assert metadata.get("description") == "S3 integration test"
            
            # Test verification
            persist.verify_snapshot(
                test_key,
                storage_mode="s3",
                s3_bucket=s3_test_bucket
            )
            
        finally:
            # Cleanup: delete the test snapshot
            try:
                persist.delete_snapshot(
                    test_key,
                    storage_mode="s3",
                    s3_bucket=s3_test_bucket
                )
            except Exception:
                pass  # Ignore cleanup errors

    def test_s3_with_region(self, s3_test_bucket):
        """Test S3 operations with explicit region."""
        test_key = f"test_snapshots/region_test_{uuid.uuid4()}.json.gz"
        region = os.environ.get("AWS_REGION", "us-east-1")
        
        agent_json = '{"test": "region_data"}'
        
        with patch('langchain_core.load.dumps', return_value=agent_json):
            try:
                persist.snapshot(
                    MockLangChainAgent(),
                    test_key,
                    storage_mode="s3",
                    s3_bucket=s3_test_bucket,
                    s3_region=region
                )
                
                assert persist.snapshot_exists(
                    test_key,
                    storage_mode="s3",
                    s3_bucket=s3_test_bucket,
                    s3_region=region
                ) is True
                
            finally:
                # Cleanup
                try:
                    persist.delete_snapshot(
                        test_key,
                        storage_mode="s3",
                        s3_bucket=s3_test_bucket,
                        s3_region=region
                    )
                except Exception:
                    pass

    def test_s3_error_handling(self, s3_test_bucket):
        """Test S3 error handling for non-existent objects."""
        non_existent_key = f"non_existent_{uuid.uuid4()}.json.gz"
        
        # Test restore of non-existent object
        with pytest.raises(IOError, match="Storage error"):
            persist.restore(
                non_existent_key,
                storage_mode="s3",
                s3_bucket=s3_test_bucket
            )
        
        # Test metadata of non-existent object
        with pytest.raises(IOError):
            persist.get_metadata(
                non_existent_key,
                storage_mode="s3",
                s3_bucket=s3_test_bucket
            )
        
        # Test exists check for non-existent object
        assert persist.snapshot_exists(
            non_existent_key,
            storage_mode="s3",
            s3_bucket=s3_test_bucket
        ) is False


if __name__ == "__main__":
    # Run tests
    pytest.main([__file__, "-v"])
