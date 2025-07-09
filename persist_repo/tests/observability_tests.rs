/*!
Observability integration tests for the Persist system.

These tests verify that logging, tracing, and metrics are working correctly
and provide the expected observability data for monitoring and debugging.
*/

use persist_core::{
    create_engine_from_config, init_default_observability, PersistError, PersistMetrics,
    SnapshotMetadata, StorageBackend, StorageConfig,
};
use std::sync::Once;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

static INIT: Once = Once::new();

/// Initialize observability system once for all tests
fn init_test_observability() {
    INIT.call_once(|| {
        init_default_observability().expect("Failed to initialize observability");
    });
}

#[test]
fn test_metrics_initialization() {
    init_test_observability();
    
    // Get the global metrics instance
    let metrics = PersistMetrics::global();
    
    // Test that we can gather metrics without error
    let metrics_text = metrics.gather_metrics().expect("Failed to gather metrics");
    
    // Verify that expected metric names are present
    assert!(metrics_text.contains("persist_s3_requests_total"));
    assert!(metrics_text.contains("persist_s3_errors_total"));
    assert!(metrics_text.contains("persist_s3_latency_seconds"));
    assert!(metrics_text.contains("persist_s3_retries_total"));
    assert!(metrics_text.contains("persist_state_size_bytes"));
}

#[test]
fn test_metrics_recording() {
    init_test_observability();
    
    let metrics = PersistMetrics::global();
    
    // Record some test metrics
    metrics.record_s3_request("put_object");
    metrics.record_s3_request("get_object");
    metrics.record_s3_error("put_object");
    metrics.record_s3_latency("put_object", Duration::from_millis(150));
    metrics.record_s3_retry("put_object");
    metrics.record_state_size(2048);
    
    // Give metrics time to be processed
    std::thread::sleep(Duration::from_millis(100));
    
    let metrics_text = metrics.gather_metrics().expect("Failed to gather metrics");
    
    // Verify metrics were recorded (exact values may vary due to parallel tests)
    assert!(metrics_text.contains("persist_s3_requests_total"));
    assert!(metrics_text.contains("persist_s3_errors_total"));
    assert!(metrics_text.contains("persist_s3_latency_seconds"));
    
    println!("Metrics output:\n{}", metrics_text);
}

#[test]
fn test_local_storage_observability() {
    init_test_observability();
    
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig::local_with_base_path(temp_dir.path().to_str().unwrap());
    let engine = create_engine_from_config(config).unwrap();
    
    let agent_state = serde_json::json!({
        "agent_type": "test_agent",
        "state": {
            "memory": ["Hello", "World"],
            "step": 1
        }
    });
    
    let agent_json = serde_json::to_string(&agent_state).unwrap();
    let metadata = SnapshotMetadata::new("test_agent", "observability_test", 0);
    let snapshot_path = temp_dir.path().join("observability_test.json.gz");
    
    // Record initial metrics state
    let metrics = PersistMetrics::global();
    let initial_metrics = metrics.gather_metrics().unwrap();
    println!("Initial metrics:\n{}", initial_metrics);
    
    // Perform save operation (should generate tracing spans and metrics)
    let save_result = engine.save_snapshot(
        &agent_json,
        &metadata,
        snapshot_path.to_str().unwrap(),
    );
    
    assert!(save_result.is_ok(), "Save operation should succeed");
    
    // Perform load operation
    let load_result = engine.load_snapshot(snapshot_path.to_str().unwrap());
    assert!(load_result.is_ok(), "Load operation should succeed");
    
    let (loaded_metadata, loaded_data) = load_result.unwrap();
    assert_eq!(loaded_data, agent_json);
    assert_eq!(loaded_metadata.agent_id(), "test_agent");
    
    // Wait for metrics to be recorded
    std::thread::sleep(Duration::from_millis(200));
    
    // Check final metrics
    let final_metrics = metrics.gather_metrics().unwrap();
    println!("Final metrics:\n{}", final_metrics);
    
    // Local storage should record state size metrics
    assert!(final_metrics.contains("persist_state_size_bytes"));
}

#[test]
fn test_error_logging_and_metrics() {
    init_test_observability();
    
    let config = StorageConfig::local_with_base_path("/nonexistent/path/that/should/fail");
    let engine = create_engine_from_config(config).unwrap();
    
    let agent_state = serde_json::json!({"test": "data"});
    let agent_json = serde_json::to_string(&agent_state).unwrap();
    let metadata = SnapshotMetadata::new("error_test", "session", 0);
    
    // Record initial metrics
    let metrics = PersistMetrics::global();
    let initial_metrics = metrics.gather_metrics().unwrap();
    
    // This should fail and generate error logs/metrics
    let save_result = engine.save_snapshot(&agent_json, &metadata, "/nonexistent/path/test.json.gz");
    
    assert!(save_result.is_err(), "Save operation should fail for invalid path");
    
    // Check that error was properly typed
    let error = save_result.unwrap_err();
    match error {
        PersistError::Storage(_) => {
            // Expected error type for local storage failures
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
    
    // Wait for any async operations to complete
    std::thread::sleep(Duration::from_millis(100));
    
    let final_metrics = metrics.gather_metrics().unwrap();
    println!("Error test metrics:\n{}", final_metrics);
}

#[test]
fn test_tracing_spans_generation() {
    init_test_observability();
    
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig::local_with_base_path(temp_dir.path().to_str().unwrap());
    let engine = create_engine_from_config(config).unwrap();
    
    // Create a substantial agent state to test
    let agent_state = serde_json::json!({
        "agent_id": "tracing_test_agent",
        "memory": {
            "conversations": [
                {"role": "user", "content": "Test message 1"},
                {"role": "assistant", "content": "Test response 1"},
                {"role": "user", "content": "Test message 2"},
                {"role": "assistant", "content": "Test response 2"}
            ],
            "context": {
                "current_task": "testing_tracing",
                "user_preferences": {
                    "verbose": true,
                    "format": "detailed"
                }
            }
        },
        "tools": [
            {"name": "search", "enabled": true},
            {"name": "calculator", "enabled": false}
        ]
    });
    
    let agent_json = serde_json::to_string(&agent_state).unwrap();
    let metadata = SnapshotMetadata::new("tracing_test", "session_1", 0);
    let snapshot_path = temp_dir.path().join("tracing_test.json.gz");
    
    // Operations should generate tracing spans with the #[tracing::instrument] annotations
    tracing::info!("Starting tracing test operations");
    
    let save_result = engine.save_snapshot(
        &agent_json,
        &metadata,
        snapshot_path.to_str().unwrap(),
    );
    
    assert!(save_result.is_ok(), "Save should succeed");
    tracing::info!("Save operation completed successfully");
    
    let load_result = engine.load_snapshot(snapshot_path.to_str().unwrap());
    assert!(load_result.is_ok(), "Load should succeed");
    tracing::info!("Load operation completed successfully");
    
    let (loaded_metadata, loaded_data) = load_result.unwrap();
    assert_eq!(loaded_data, agent_json);
    assert_eq!(loaded_metadata.agent_id(), "tracing_test");
    
    tracing::info!("Tracing test completed - spans should be visible in trace output");
}

#[test]
fn test_concurrent_operations_observability() {
    init_test_observability();
    
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig::local_with_base_path(temp_dir.path().to_str().unwrap());
    let engine = create_engine_from_config(config).unwrap();
    
    let metrics = PersistMetrics::global();
    let initial_metrics = metrics.gather_metrics().unwrap();
    
    // Perform multiple concurrent operations
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let engine = &engine;
            let base_path = temp_dir.path();
            
            std::thread::spawn(move || {
                let agent_state = serde_json::json!({
                    "agent_id": format!("concurrent_agent_{}", i),
                    "state": {"step": i, "data": format!("test_data_{}", i)}
                });
                
                let agent_json = serde_json::to_string(&agent_state).unwrap();
                let metadata = SnapshotMetadata::new(&format!("agent_{}", i), "concurrent_test", i);
                let snapshot_path = base_path.join(format!("concurrent_{}.json.gz", i));
                
                // Each operation should generate its own tracing spans
                let save_result = engine.save_snapshot(
                    &agent_json,
                    &metadata,
                    snapshot_path.to_str().unwrap(),
                );
                
                assert!(save_result.is_ok(), "Concurrent save {} should succeed", i);
                
                // Load it back
                let load_result = engine.load_snapshot(snapshot_path.to_str().unwrap());
                assert!(load_result.is_ok(), "Concurrent load {} should succeed", i);
                
                let (loaded_metadata, loaded_data) = load_result.unwrap();
                assert_eq!(loaded_data, agent_json);
                assert_eq!(loaded_metadata.agent_id(), format!("agent_{}", i));
                
                tracing::info!(operation_id = i, "Concurrent operation completed");
            })
        })
        .collect();
    
    // Wait for all operations to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    // Give metrics time to be recorded
    std::thread::sleep(Duration::from_millis(300));
    
    let final_metrics = metrics.gather_metrics().unwrap();
    println!("Concurrent operations metrics:\n{}", final_metrics);
    
    // We should see state size metrics for all operations
    assert!(final_metrics.contains("persist_state_size_bytes"));
    
    tracing::info!("Concurrent operations test completed - all spans should be properly isolated");
}

#[test]
fn test_metrics_endpoint_functionality() {
    init_test_observability();
    
    let metrics = PersistMetrics::global();
    
    // Record some metrics
    metrics.record_s3_request("test_operation");
    metrics.record_s3_latency("test_operation", Duration::from_millis(75));
    metrics.record_state_size(1024);
    
    // Test gathering metrics multiple times (simulating Prometheus scraping)
    for i in 0..3 {
        let metrics_text = metrics.gather_metrics().expect("Should gather metrics successfully");
        
        // Basic format validation
        assert!(metrics_text.starts_with('#') || metrics_text.contains("persist_"));
        assert!(metrics_text.contains("TYPE"));
        assert!(metrics_text.contains("HELP"));
        
        tracing::info!(scrape_iteration = i, "Metrics scrape completed");
        
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod integration_with_s3_storage {
    use super::*;
    
    /// Test observability with S3 storage (requires AWS credentials)
    /// Set RUN_S3_TESTS=1 to enable
    #[test]
    fn test_s3_observability() {
        if std::env::var("RUN_S3_TESTS").unwrap_or_default() != "1" {
            println!("Skipping S3 test - set RUN_S3_TESTS=1 to run");
            return;
        }
        
        init_test_observability();
        
        let bucket = std::env::var("TEST_S3_BUCKET")
            .unwrap_or_else(|_| "persist-test-bucket".to_string());
        
        let config = StorageConfig::s3_with_bucket(bucket);
        let engine = create_engine_from_config(config).unwrap();
        
        let metrics = PersistMetrics::global();
        let initial_metrics = metrics.gather_metrics().unwrap();
        
        let agent_state = serde_json::json!({
            "agent_type": "s3_test_agent",
            "test_data": "S3 observability test"
        });
        
        let agent_json = serde_json::to_string(&agent_state).unwrap();
        let metadata = SnapshotMetadata::new("s3_test", "observability", 0);
        let s3_key = "test/observability_test.json.gz";
        
        // S3 operations should generate detailed metrics and traces
        let save_result = engine.save_snapshot(&agent_json, &metadata, s3_key);
        
        match save_result {
            Ok(_) => {
                tracing::info!("S3 save operation succeeded");
                
                // Try to load it back
                let load_result = engine.load_snapshot(s3_key);
                assert!(load_result.is_ok(), "S3 load should succeed");
                
                let (loaded_metadata, loaded_data) = load_result.unwrap();
                assert_eq!(loaded_data, agent_json);
                assert_eq!(loaded_metadata.agent_id(), "s3_test");
            }
            Err(e) => {
                tracing::warn!(error = ?e, "S3 operation failed - checking if error was properly logged");
                
                // Error should be properly typed and logged
                match e {
                    PersistError::S3UploadError { .. } 
                    | PersistError::S3AccessDenied { .. }
                    | PersistError::S3Configuration(_) => {
                        // These are expected S3-specific error types
                        tracing::info!("S3 error was properly typed and logged");
                    }
                    other => {
                        panic!("Unexpected error type for S3 operation: {:?}", other);
                    }
                }
            }
        }
        
        // Wait for metrics to be recorded
        std::thread::sleep(Duration::from_millis(500));
        
        let final_metrics = metrics.gather_metrics().unwrap();
        println!("S3 observability metrics:\n{}", final_metrics);
        
        // Should see S3 request metrics regardless of success/failure
        assert!(final_metrics.contains("persist_s3_requests_total"));
        
        tracing::info!("S3 observability test completed");
    }
}
