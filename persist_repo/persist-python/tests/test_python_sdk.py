"""
Comprehensive tests for the Python SDK of Persist.
These tests verify the Python interface and integration with LangChain.
"""

import json
import os
import tempfile
import threading
import time
from pathlib import Path
from unittest.mock import patch

import pytest

# Mock LangChain for testing if not available
try:
    from langchain.memory import ConversationBufferMemory
    from langchain.schema import AIMessage, BaseMessage, HumanMessage
    from langchain.schema.runnable import Runnable

    LANGCHAIN_AVAILABLE = True
except ImportError:
    LANGCHAIN_AVAILABLE = False
    # Create mock classes for testing
    class BaseMessage:
        def __init__(self, content, **kwargs):
            self.content = content
            for k, v in kwargs.items():
                setattr(self, k, v)

    class HumanMessage(BaseMessage):
        pass

    class AIMessage(BaseMessage):
        pass

    class ConversationBufferMemory:
        def __init__(self):
            self.chat_memory = []

    class Runnable:
        pass

# Try to import the persist module
try:
    import persist
    PERSIST_AVAILABLE = True
except ImportError:
    PERSIST_AVAILABLE = False
    print("Persist module not available - building with maturin first")

@pytest.fixture
def temp_dir():
    """Create a temporary directory for tests."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield tmpdir

@pytest.fixture
def sample_agent_data():
    """Create sample agent data for testing."""
    return {
        "type": "test_agent",
        "config": {
            "model": "gpt-3.5-turbo",
            "temperature": 0.7
        },
        "memory": {
            "conversation_history": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there!"}
            ]
        },
        "tools": ["calculator", "web_search"],
        "state": {
            "active": True,
            "session_id": "test_session_123"
        }
    }

@pytest.fixture
def mock_langchain_agent(sample_agent_data):
    """Create a mock LangChain agent for testing."""
    class MockAgent(Runnable):
        def __init__(self, data):
            self.data = data
            self.memory = ConversationBufferMemory()

        def dumps(self):
            return json.dumps(self.data)

        @classmethod
        def loads(cls, data_str, **kwargs):
            data = json.loads(data_str)
            return cls(data)

    return MockAgent(sample_agent_data)

@pytest.mark.skipif(not PERSIST_AVAILABLE, reason="Persist module not available")
class TestPersistSDK:
    """Test cases for the Persist Python SDK."""

    def test_snapshot_and_restore_basic(self, temp_dir, sample_agent_data):
        """Test basic snapshot and restore functionality."""
        # Create a mock agent with dumps/loads methods
        class MockAgent:
            def __init__(self, data):
                self.data = data

            def dumps(self):
                return json.dumps(self.data)

            @classmethod
            def loads(cls, data_str, **kwargs):
                data = json.loads(data_str)
                return cls(data)

        agent = MockAgent(sample_agent_data)
        snapshot_path = os.path.join(temp_dir, "test_snapshot.json.gz")

        # Mock the LangChain integration
        with patch('persist.dumps', return_value=agent.dumps()):
            with patch('persist.loads', return_value=agent):
                # Test snapshot
                persist.snapshot(agent, snapshot_path)

                # Verify file exists
                assert os.path.exists(snapshot_path)
                assert os.path.getsize(snapshot_path) > 0

                # Test restore
                restored_agent = persist.restore(snapshot_path)

                # Verify restored agent
                assert restored_agent is not None
                assert isinstance(restored_agent, MockAgent)
                assert restored_agent.data == agent.data

    def test_snapshot_with_invalid_path(self, sample_agent_data):
        """Test snapshot with invalid file path."""
        class MockAgent:
            def dumps(self):
                return json.dumps(sample_agent_data)

        agent = MockAgent()
        invalid_path = "/invalid/nonexistent/path/snapshot.json.gz"

        with patch('persist.dumps', return_value=agent.dumps()):
            with pytest.raises((OSError, PermissionError)):  # Should raise an IO error
                persist.snapshot(agent, invalid_path)

    def test_restore_nonexistent_file(self):
        """Test restore with nonexistent file."""
        nonexistent_path = "/nonexistent/file.json.gz"

        with pytest.raises((FileNotFoundError, OSError)):  # Should raise an IO error
            persist.restore(nonexistent_path)

    def test_snapshot_empty_data(self, temp_dir):
        """Test snapshot with empty agent data."""
        class MockAgent:
            def dumps(self):
                return "{}"

        agent = MockAgent()
        snapshot_path = os.path.join(temp_dir, "empty_snapshot.json.gz")

        with patch('persist.dumps', return_value="{}"):
            with patch('persist.loads', return_value=agent):
                persist.snapshot(agent, snapshot_path)

                assert os.path.exists(snapshot_path)

                restored_agent = persist.restore(snapshot_path)
                assert restored_agent is not None

    def test_snapshot_large_data(self, temp_dir):
        """Test snapshot with large agent data."""
        # Create large test data
        large_data = {
            "type": "large_agent",
            "conversation": [{"message": f"Message {i}"} for i in range(10000)],
            "embeddings": [0.1] * 50000
        }

        class MockAgent:
            def dumps(self):
                return json.dumps(large_data)

        agent = MockAgent()
        snapshot_path = os.path.join(temp_dir, "large_snapshot.json.gz")

        with patch('persist.dumps', return_value=json.dumps(large_data)):
            with patch('persist.loads', return_value=agent):
                start_time = time.time()
                persist.snapshot(agent, snapshot_path)
                save_time = time.time() - start_time

                assert os.path.exists(snapshot_path)

                start_time = time.time()
                restored_agent = persist.restore(snapshot_path)
                load_time = time.time() - start_time

                assert restored_agent is not None

                # Performance assertions (adjust based on requirements)
                assert save_time < 10.0, f"Save took too long: {save_time}s"
                assert load_time < 5.0, f"Load took too long: {load_time}s"

    def test_snapshot_special_characters(self, temp_dir):
        """Test snapshot with special characters and unicode."""
        special_data = {
            "agent_name": "🤖 AI Assistant",
            "messages": [
                {"content": "Hello 世界! How are you today? 🌍"},
                {"content": "Special chars: àáâãäåæçèéêë ñoël"},
                {"content": "Math symbols: ∑∏∆∇∀∃∈∉⊂⊃⊆⊇"}
            ],
            "config": {
                "émojis_enabled": True,
                "språk": "multiple",
                "🔧 tools": ["calculator", "web_search"]
            }
        }

        class MockAgent:
            def dumps(self):
                return json.dumps(special_data, ensure_ascii=False)

        agent = MockAgent()
        snapshot_path = os.path.join(temp_dir, "unicode_snapshot.json.gz")

        with patch('persist.dumps', return_value=json.dumps(special_data, ensure_ascii=False)):
            with patch('persist.loads', return_value=agent):
                persist.snapshot(agent, snapshot_path)

                assert os.path.exists(snapshot_path)

                restored_agent = persist.restore(snapshot_path)
                assert restored_agent is not None

    def test_concurrent_snapshots(self, temp_dir):
        """Test concurrent snapshot operations."""
        def create_snapshot(agent_id, temp_dir):
            data = {"agent_id": agent_id, "data": f"Agent {agent_id} data"}

            class MockAgent:
                def dumps(self):
                    return json.dumps(data)

            agent = MockAgent()
            snapshot_path = os.path.join(temp_dir, f"concurrent_{agent_id}.json.gz")

            with patch('persist.dumps', return_value=json.dumps(data)):
                with patch('persist.loads', return_value=agent):
                    persist.snapshot(agent, snapshot_path)

                    # Verify snapshot
                    restored_agent = persist.restore(snapshot_path)
                    assert restored_agent is not None

                    return snapshot_path

        # Create multiple threads for concurrent operations
        threads = []
        results = {}

        def thread_worker(agent_id, temp_dir):
            try:
                results[agent_id] = create_snapshot(agent_id, temp_dir)
            except Exception as e:
                results[agent_id] = e

        # Start 10 concurrent snapshot operations
        for i in range(10):
            thread = threading.Thread(target=thread_worker, args=(i, temp_dir))
            threads.append(thread)
            thread.start()

        # Wait for all threads to complete
        for thread in threads:
            thread.join()

        # Verify all operations succeeded
        for agent_id, result in results.items():
            assert not isinstance(result, Exception), f"Agent {agent_id} failed: {result}"
            assert os.path.exists(result), f"Snapshot file not created for agent {agent_id}"

    def test_snapshot_with_path_object(self, temp_dir, sample_agent_data):
        """Test snapshot with pathlib.Path object."""
        class MockAgent:
            def dumps(self):
                return json.dumps(sample_agent_data)

        agent = MockAgent()
        snapshot_path = Path(temp_dir) / "pathlib_snapshot.json.gz"

        with patch('persist.dumps', return_value=json.dumps(sample_agent_data)):
            with patch('persist.loads', return_value=agent):
                # Test with Path object
                persist.snapshot(agent, snapshot_path)

                assert snapshot_path.exists()

                # Test restore with Path object
                restored_agent = persist.restore(snapshot_path)
                assert restored_agent is not None

    def test_multiple_snapshots_same_agent(self, temp_dir, sample_agent_data):
        """Test multiple snapshots of the same agent (versioning)."""
        class MockAgent:
            def __init__(self, data):
                self.data = data

            def dumps(self):
                return json.dumps(self.data)

        agent = MockAgent(sample_agent_data)

        # Create multiple snapshots with different data
        for i in range(5):
            # Modify agent data
            agent.data["version"] = i
            agent.data["timestamp"] = time.time()

            snapshot_path = os.path.join(temp_dir, f"versioned_snapshot_{i}.json.gz")

            with patch('persist.dumps', return_value=agent.dumps()):
                with patch('persist.loads', return_value=agent):
                    persist.snapshot(agent, snapshot_path)

                    assert os.path.exists(snapshot_path)

                    # Verify each snapshot
                    restored_agent = persist.restore(snapshot_path)
                    assert restored_agent is not None

    def test_error_handling_malformed_data(self, temp_dir):
        """Test error handling with malformed agent data."""
        class BadAgent:
            def dumps(self):
                # Return invalid JSON
                return "{ invalid json structure"

        agent = BadAgent()
        snapshot_path = os.path.join(temp_dir, "bad_snapshot.json.gz")

        with patch('persist.dumps', return_value="{ invalid json"):
            with pytest.raises((ValueError, json.JSONDecodeError)):  # Should raise JSON parsing error
                persist.snapshot(agent, snapshot_path)

    def test_performance_benchmarks(self, temp_dir):
        """Performance benchmark tests."""
        # Test different data sizes
        data_sizes = [
            ("small", 1024),      # 1KB
            ("medium", 102400),   # 100KB
            ("large", 1048576),   # 1MB
        ]

        performance_results = {}

        for size_name, target_size in data_sizes:
            # Create data of approximately target size
            item_size = 100
            num_items = target_size // item_size

            data = {
                "size_category": size_name,
                "items": [f"Item {i} with some content to reach target size" for i in range(num_items)]
            }

            class MockAgent:
                def __init__(self, data_to_dump):
                    self.data_to_dump = data_to_dump

                def dumps(self):
                    return json.dumps(self.data_to_dump)

            agent = MockAgent(data)
            snapshot_path = os.path.join(temp_dir, f"perf_{size_name}.json.gz")

            with patch('persist.dumps', return_value=json.dumps(data)):
                with patch('persist.loads', return_value=agent):
                    # Measure save performance
                    start_time = time.time()
                    persist.snapshot(agent, snapshot_path)
                    save_time = time.time() - start_time

                    # Measure load performance
                    start_time = time.time()
                    _ = persist.restore(snapshot_path)
                    load_time = time.time() - start_time

                    file_size = os.path.getsize(snapshot_path)
                    data_size = len(json.dumps(data))
                    compression_ratio = file_size / data_size

                    performance_results[size_name] = {
                        "data_size": data_size,
                        "file_size": file_size,
                        "save_time": save_time,
                        "load_time": load_time,
                        "compression_ratio": compression_ratio
                    }

        # Print performance results for analysis
        print("\nPerformance Benchmark Results:")
        print("-" * 70)
        print(f"{'Size':<10} {'Data Size':<12} {'File Size':<12} {'Save Time':<10} {'Load Time':<10} {'Compression':<12}")
        print("-" * 70)

        for size_name, results in performance_results.items():
            print(f"{size_name:<10} "
                  f"{results['data_size']//1024:<11}KB "
                  f"{results['file_size']//1024:<11}KB "
                  f"{results['save_time']:<9.3f}s "
                  f"{results['load_time']:<9.3f}s "
                  f"{results['compression_ratio']:<11.2%}")

        # Basic performance assertions
        for size_name, results in performance_results.items():
            assert results['save_time'] < 5.0, f"Save time too slow for {size_name}: {results['save_time']}s"
            assert results['load_time'] < 3.0, f"Load time too slow for {size_name}: {results['load_time']}s"
            assert results['compression_ratio'] < 1.0, f"No compression achieved for {size_name}"

@pytest.mark.skipif(not LANGCHAIN_AVAILABLE, reason="LangChain not available")
class TestLangChainIntegration:
    """Test cases for LangChain integration (if available)."""

    def test_conversation_memory_persistence(self, temp_dir):
        """Test persisting LangChain conversation memory."""
        # Create a conversation memory with some history
        memory = ConversationBufferMemory()
        memory.chat_memory.add_user_message("Hello, how are you?")
        memory.chat_memory.add_ai_message("I'm doing well, thank you for asking!")
        memory.chat_memory.add_user_message("Can you help me with Python?")
        memory.chat_memory.add_ai_message("Of course! I'd be happy to help you with Python.")

        # Mock the serialization
        class MockMemory:
            def __init__(self, memory):
                self.memory = memory

            def dumps(self):
                return json.dumps({
                    "type": "ConversationBufferMemory",
                    "messages": [
                        {"type": "human", "content": "Hello, how are you?"},
                        {"type": "ai", "content": "I'm doing well, thank you for asking!"},
                        {"type": "human", "content": "Can you help me with Python?"},
                        {"type": "ai", "content": "Of course! I'd be happy to help you with Python."}
                    ]
                })

        mock_memory = MockMemory(memory)
        snapshot_path = os.path.join(temp_dir, "memory_snapshot.json.gz")

        with patch('persist.dumps', return_value=mock_memory.dumps()):
            with patch('persist.loads', return_value=mock_memory):
                persist.snapshot(mock_memory, snapshot_path)

                assert os.path.exists(snapshot_path)

                restored_memory = persist.restore(snapshot_path)
                assert restored_memory is not None

if __name__ == "__main__":
    # Run tests if called directly
    pytest.main([__file__, "-v"])
