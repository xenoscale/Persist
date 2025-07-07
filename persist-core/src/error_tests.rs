/*!
Comprehensive tests for error handling and error types.
*/

#[cfg(test)]
mod tests {
    use crate::error::PersistError;
    use std::io;

    #[test]
    fn test_persist_error_display() {
        let error = PersistError::validation("test validation error");
        assert_eq!(error.to_string(), "Validation error: test validation error");

        let error = PersistError::compression("test compression error");
        assert_eq!(error.to_string(), "Compression error: test compression error");

        let error = PersistError::Storage("test storage error".to_string());
        assert_eq!(error.to_string(), "Storage error: test storage error");
    }

    #[test]
    fn test_persist_error_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let persist_error = PersistError::from(io_error);
        
        match persist_error {
            PersistError::Io(_) => {}, // Expected
            _ => panic!("Expected Io error variant"),
        }
    }

    #[test]
    fn test_persist_error_from_json_error() {
        let json_error = serde_json::Error::syntax(serde_json::error::ErrorCode::EofWhileParsingValue, 0, 0);
        let persist_error = PersistError::from(json_error);
        
        match persist_error {
            PersistError::Json(_) => {}, // Expected
            _ => panic!("Expected Json error variant"),
        }
    }

    #[test]
    fn test_integrity_check_failed_error() {
        let error = PersistError::IntegrityCheckFailed {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        
        assert!(error.to_string().contains("abc123"));
        assert!(error.to_string().contains("def456"));
    }

    #[test]
    fn test_error_is_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        
        assert_send::<PersistError>();
        assert_sync::<PersistError>();
    }

    #[test]
    fn test_missing_metadata_error() {
        let error = PersistError::MissingMetadata("agent_id".to_string());
        assert!(error.to_string().contains("agent_id"));
    }

    #[test]
    fn test_invalid_format_error() {
        let error = PersistError::InvalidFormat("Invalid JSON structure".to_string());
        assert!(error.to_string().contains("Invalid JSON structure"));
    }

    #[test]
    fn test_error_chain() {
        let root_cause = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let persist_error = PersistError::from(root_cause);
        
        // Test that we can access the source error
        match persist_error {
            PersistError::Io(ref io_err) => {
                assert_eq!(io_err.kind(), io::ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_error_variants_coverage() {
        // Test all error variants to ensure they're constructible
        let _io_error = PersistError::Io(io::Error::new(io::ErrorKind::Other, "test"));
        let _json_error = PersistError::Json(serde_json::Error::syntax(
            serde_json::error::ErrorCode::EofWhileParsingValue, 0, 0
        ));
        let _compression_error = PersistError::Compression("test".to_string());
        let _integrity_error = PersistError::IntegrityCheckFailed {
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        let _format_error = PersistError::InvalidFormat("test".to_string());
        let _metadata_error = PersistError::MissingMetadata("test".to_string());
        let _storage_error = PersistError::Storage("test".to_string());
        let _validation_error = PersistError::Validation("test".to_string());
    }

    #[test]
    fn test_error_result_type() {
        fn returns_error() -> crate::Result<()> {
            Err(PersistError::validation("test error"))
        }
        
        let result = returns_error();
        assert!(result.is_err());
    }
}
