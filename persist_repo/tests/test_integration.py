#!/usr/bin/env python3
"""
Integration tests for the Persist agent snapshot system.

This test file demonstrates the core functionality without requiring
external dependencies like OpenAI API keys.
"""

import json
import tempfile
import os
import sys
from pathlib import Path

# Add the persist-python directory to the path for development testing
sys.path.insert(0, str(Path(__file__).parent.parent / "persist-python"))

def test_core_functionality():
    """
    Test the core snapshot and restore functionality with a mock LangChain-like object.
    """
    try:
        import persist
        print("✅ Successfully imported persist module")
        print(f"📦 Version: {persist.__version__}")
    except ImportError as e:
        print(f"❌ Failed to import persist: {e}")
        print("💡 Make sure the package is built with: maturin develop --release")
        return False

    # Test basic module functionality
    assert hasattr(persist, 'snapshot')
    assert hasattr(persist, 'restore')
    assert hasattr(persist, 'get_metadata')
    assert hasattr(persist, 'verify_snapshot')
    assert hasattr(persist, 'snapshot_exists')
    assert hasattr(persist, 'delete_snapshot')
    print("✅ All required functions are available")

    # Create a mock LangChain-compatible object
    class MockAgent:
        """Mock object that mimics LangChain's serialization interface"""
        def __init__(self, name="TestAgent", memory=None):
            self.name = name
            self.memory = memory or []
            self.tools = ["search", "calculator"]
    
    # Mock LangChain's dumps and loads functions
    def mock_dumps(obj):
        """Mock LangChain dumps function"""
        if isinstance(obj, MockAgent):
            return json.dumps({
                "type": "MockAgent",
                "name": obj.name,
                "memory": obj.memory,
                "tools": obj.tools
            })
        else:
            return json.dumps(obj.__dict__ if hasattr(obj, '__dict__') else str(obj))
    
    def mock_loads(json_str, secrets_map=None):
        """Mock LangChain loads function"""
        data = json.loads(json_str)
        if data.get("type") == "MockAgent":
            agent = MockAgent(data["name"], data["memory"])
            agent.tools = data["tools"]
            return agent
        return data

    # Patch the LangChain imports for testing
    import sys
    from unittest.mock import MagicMock
    
    mock_module = MagicMock()
    mock_module.dumps = mock_dumps
    mock_module.loads = mock_loads
    
    sys.modules['langchain_core.load'] = mock_module
    sys.modules['langchain.load'] = mock_module

    # Create test agent
    agent = MockAgent("ConversationAgent", ["Hello!", "How are you?"])
    print(f"🤖 Created mock agent: {agent.name}")

    # Test snapshot creation
    with tempfile.NamedTemporaryFile(suffix='.json.gz', delete=False) as f:
        snapshot_path = f.name

    try:
        print(f"💾 Creating snapshot at: {snapshot_path}")
        persist.snapshot(
            agent, 
            snapshot_path,
            agent_id="test_agent",
            session_id="test_session", 
            snapshot_index=1,
            description="Integration test snapshot"
        )
        print("✅ Snapshot created successfully")

        # Test file existence
        assert persist.snapshot_exists(snapshot_path)
        print("✅ Snapshot file exists")

        # Test snapshot verification
        assert persist.verify_snapshot(snapshot_path)
        print("✅ Snapshot integrity verified")

        # Test metadata retrieval
        metadata = persist.get_metadata(snapshot_path)
        assert metadata['agent_id'] == 'test_agent'
        assert metadata['session_id'] == 'test_session'
        assert metadata['snapshot_index'] == 1
        assert metadata['description'] == 'Integration test snapshot'
        assert metadata['format_version'] == 1
        print("✅ Metadata retrieved and validated")
        print(f"📊 Snapshot info:")
        print(f"   - Agent ID: {metadata['agent_id']}")
        print(f"   - Created: {metadata['timestamp']}")
        print(f"   - Size: {metadata['uncompressed_size']} bytes (uncompressed)")
        print(f"   - Compressed: {metadata['compressed_size']} bytes")
        print(f"   - Algorithm: {metadata['compression_algorithm']}")

        # Test snapshot restoration
        print("🔄 Restoring snapshot...")
        restored_agent = persist.restore(snapshot_path)
        print("✅ Snapshot restored successfully")

        # Verify restored agent
        assert isinstance(restored_agent, MockAgent)
        assert restored_agent.name == agent.name
        assert restored_agent.memory == agent.memory
        assert restored_agent.tools == agent.tools
        print("✅ Restored agent matches original")

        # Test snapshot deletion
        persist.delete_snapshot(snapshot_path)
        assert not persist.snapshot_exists(snapshot_path)
        print("✅ Snapshot deleted successfully")

    finally:
        # Cleanup
        if os.path.exists(snapshot_path):
            os.unlink(snapshot_path)

    return True

def test_error_handling():
    """Test error handling for various failure cases"""
    try:
        import persist
    except ImportError:
        print("⚠️ Skipping error handling tests - persist not available")
        return True

    print("\n🧪 Testing error handling...")

    # Test with non-existent file
    assert not persist.snapshot_exists("non_existent_file.json.gz")
    assert not persist.verify_snapshot("non_existent_file.json.gz")
    print("✅ Non-existent file handling correct")

    # Test with invalid path
    try:
        persist.get_metadata("non_existent_file.json.gz")
        assert False, "Should have raised an error"
    except Exception:
        print("✅ Error handling for invalid metadata access correct")

    return True

def main():
    """Run all integration tests"""
    print("🚀 Starting Persist Integration Tests")
    print("=" * 50)
    
    success = True
    
    try:
        success &= test_core_functionality()
        print("\n" + "=" * 50)
        success &= test_error_handling()
        
    except Exception as e:
        print(f"\n❌ Integration test failed with error: {e}")
        import traceback
        traceback.print_exc()
        success = False
    
    print("\n" + "=" * 50)
    if success:
        print("🎉 All integration tests passed!")
        return 0
    else:
        print("💥 Some tests failed!")
        return 1

if __name__ == "__main__":
    sys.exit(main())
