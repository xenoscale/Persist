"""
Tests for Persist custom exception classes and error handling.
"""

from unittest.mock import patch

import pytest

# Try to import the persist module
try:
    import persist
    PERSIST_AVAILABLE = True
except ImportError:
    PERSIST_AVAILABLE = False
    print("Persist module not available - building with maturin first")


@pytest.mark.skipif(not PERSIST_AVAILABLE, reason="Persist module not available")
class TestCustomExceptions:
    """Test custom exception classes are properly exposed."""

    def test_exception_hierarchy(self):
        """Test that custom exceptions have proper inheritance."""
        # Test base exception
        assert issubclass(persist.PersistError, Exception)

        # Test specific exceptions inherit from base
        assert issubclass(persist.PersistConfigurationError, persist.PersistError)
        assert issubclass(persist.PersistIntegrityError, persist.PersistError)
        assert issubclass(persist.PersistS3Error, persist.PersistError)
        assert issubclass(persist.PersistCompressionError, persist.PersistError)

    def test_exceptions_are_instantiable(self):
        """Test that exceptions can be instantiated with messages."""
        exceptions = [
            persist.PersistError,
            persist.PersistConfigurationError,
            persist.PersistIntegrityError,
            persist.PersistS3Error,
            persist.PersistCompressionError,
        ]

        for exc_class in exceptions:
            exc = exc_class("Test message")
            assert str(exc) == "Test message"
            assert isinstance(exc, persist.PersistError)
            assert isinstance(exc, Exception)


@pytest.mark.skipif(not PERSIST_AVAILABLE, reason="Persist module not available")
class TestErrorHandling:
    """Test error handling in snapshot/restore operations."""

    def test_invalid_storage_mode_raises_configuration_error(self):
        """Test that invalid storage mode raises PersistConfigurationError."""
        dummy_agent = {"test": "data"}

        with pytest.raises((persist.PersistConfigurationError, ValueError, OSError)) as exc_info:
            persist.snapshot(dummy_agent, "test.json.gz", storage_mode="invalid_mode")

        # Check that the error message indicates invalid storage mode
        assert "Invalid storage_mode" in str(exc_info.value) or "invalid_mode" in str(exc_info.value)

    def test_missing_s3_bucket_raises_error(self):
        """Test that missing S3 bucket parameter raises appropriate error."""
        dummy_agent = {"test": "data"}

        with pytest.raises((persist.PersistS3Error, persist.PersistConfigurationError, ValueError)) as exc_info:
            persist.snapshot(dummy_agent, "test.json.gz", storage_mode="s3")

        # Should raise an error about missing bucket configuration
        error_msg = str(exc_info.value).lower()
        assert any(word in error_msg for word in ["bucket", "configuration", "s3"])

    def test_snapshot_nonexistent_path_raises_error(self):
        """Test that trying to restore from nonexistent path raises appropriate error."""
        with pytest.raises((FileNotFoundError, OSError, persist.PersistError)) as exc_info:
            persist.restore("/nonexistent/path/snapshot.json.gz")

        # Should be a file not found or IO error
        assert "not found" in str(exc_info.value).lower() or "no such file" in str(exc_info.value).lower()

    def test_snapshot_exists_with_invalid_path(self):
        """Test that snapshot_exists returns False for invalid paths."""
        # Should not raise an exception, just return False
        assert not persist.snapshot_exists("/nonexistent/path/snapshot.json.gz")
        assert not persist.snapshot_exists("")

    @pytest.mark.skip(reason="Requires mock implementation")
    def test_integrity_error_on_corrupted_data(self):
        """Test that corrupted snapshot data raises PersistIntegrityError."""
        # This would require mocking the Rust layer to simulate corruption
        # For now, we skip this test but keep it as a placeholder
        pass

    @pytest.mark.skip(reason="Requires AWS credentials or LocalStack")
    def test_s3_error_on_invalid_credentials(self):
        """Test that invalid S3 credentials raise PersistS3Error."""
        dummy_agent = {"test": "data"}

        with patch.dict('os.environ', {
            'AWS_ACCESS_KEY_ID': 'invalid_key',
            'AWS_SECRET_ACCESS_KEY': 'invalid_secret',
            'AWS_REGION': 'us-west-2'
        }):
            with pytest.raises((persist.PersistS3Error, OSError, PermissionError)) as exc_info:
                persist.snapshot(
                    dummy_agent,
                    "test.json.gz",
                    storage_mode="s3",
                    s3_bucket="test-bucket"
                )

            # Should be an S3-related error
            error_msg = str(exc_info.value).lower()
            assert any(word in error_msg for word in ["s3", "access", "credential", "denied"])


@pytest.mark.skipif(not PERSIST_AVAILABLE, reason="Persist module not available")
class TestTypeHints:
    """Test that type hints are properly exposed."""

    def test_function_signatures_are_accessible(self):
        """Test that function signatures can be inspected."""
        import inspect

        # Test snapshot function signature
        sig = inspect.signature(persist.snapshot)
        params = list(sig.parameters.keys())

        expected_params = [
            'agent', 'path', 'agent_id', 'session_id', 'snapshot_index',
            'description', 'storage_mode', 's3_bucket', 's3_region'
        ]

        for param in expected_params:
            assert param in params, f"Expected parameter '{param}' not found in snapshot signature"

        # Test restore function signature
        sig = inspect.signature(persist.restore)
        params = list(sig.parameters.keys())

        expected_params = ['path', 'storage_mode', 's3_bucket', 's3_region']

        for param in expected_params:
            assert param in params, f"Expected parameter '{param}' not found in restore signature"

    def test_module_has_version(self):
        """Test that module exposes version information."""
        assert hasattr(persist, '__version__')
        assert isinstance(persist.__version__, str)
        assert len(persist.__version__) > 0

    def test_module_has_docstring(self):
        """Test that module has proper documentation."""
        assert hasattr(persist, '__doc__')
        assert persist.__doc__ is not None
        assert len(persist.__doc__) > 0


@pytest.mark.skipif(not PERSIST_AVAILABLE, reason="Persist module not available")
class TestFunctionDefaults:
    """Test default parameter behavior."""

    def test_snapshot_defaults(self):
        """Test that snapshot function handles default parameters correctly."""
        import inspect

        sig = inspect.signature(persist.snapshot)

        # Check that defaults are set correctly
        assert sig.parameters['agent_id'].default == "default_agent"
        assert sig.parameters['session_id'].default == "default_session"
        assert sig.parameters['snapshot_index'].default == 0
        assert sig.parameters['description'].default is None
        assert sig.parameters['storage_mode'].default is None
        assert sig.parameters['s3_bucket'].default is None
        assert sig.parameters['s3_region'].default is None

    @pytest.mark.xfail(reason="Expected to fail without LangChain - tests parameter validation")
    def test_optional_parameters_work(self):
        """Test that optional parameters can be omitted."""
        dummy_agent = {"test": "data"}

        # This should work with minimal parameters (though will fail due to no LangChain)
        with pytest.raises(Exception):  # Expect LangChain import error  # noqa: B017
            persist.snapshot(dummy_agent, "test.json.gz")

        # The error should be about LangChain, not parameter validation
        try:
            persist.snapshot(dummy_agent, "test.json.gz")
        except Exception as e:
            error_msg = str(e).lower()
            # Should be LangChain-related error, not parameter error
            assert any(word in error_msg for word in ["langchain", "import", "load"])


@pytest.mark.skipif(not PERSIST_AVAILABLE, reason="Persist module not available")
class TestUtilityFunctions:
    """Test utility functions like metadata retrieval and verification."""

    def test_get_metadata_with_nonexistent_file(self):
        """Test get_metadata with nonexistent file."""
        with pytest.raises((FileNotFoundError, OSError, persist.PersistError)) as exc_info:
            persist.get_metadata("/nonexistent/file.json.gz")

        # Should raise file not found or similar error
        error_msg = str(exc_info.value).lower()
        assert any(word in error_msg for word in ["not found", "no such file", "does not exist"])

    def test_verify_snapshot_with_nonexistent_file(self):
        """Test verify_snapshot with nonexistent file."""
        with pytest.raises((FileNotFoundError, OSError, persist.PersistError)) as exc_info:
            persist.verify_snapshot("/nonexistent/file.json.gz")

        # Should raise file not found or similar error
        error_msg = str(exc_info.value).lower()
        assert any(word in error_msg for word in ["not found", "no such file", "does not exist"])

    def test_delete_snapshot_with_nonexistent_file(self):
        """Test delete_snapshot with nonexistent file."""
        with pytest.raises((FileNotFoundError, OSError, persist.PersistError)) as exc_info:
            persist.delete_snapshot("/nonexistent/file.json.gz")

        # Should raise file not found or similar error
        error_msg = str(exc_info.value).lower()
        assert any(word in error_msg for word in ["not found", "no such file", "does not exist"])


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
