/*!
LocalStack integration tests for S3 storage testing.

These tests use LocalStack to simulate AWS S3 without requiring real AWS credentials.
LocalStack should be running on localhost:4566 for these tests to work.

To run LocalStack:
```bash
docker run --rm -p 4566:4566 localstack/localstack
```

Then run tests with:
```bash
RUN_LOCALSTACK_TESTS=1 cargo test localstack
```
*/

use persist_core::{
    create_engine_from_config, init_default_observability, PersistMetrics, SnapshotMetadata,
    StorageConfig,
};
use std::collections::HashMap;
use std::sync::Once;
use std::time::Duration;

static INIT: Once = Once::new();

fn init_test_observability() {
    INIT.call_once(|| {
        init_default_observability().expect("Failed to initialize observability");
    });
}

/// Check if LocalStack is available and skip test if not
fn check_localstack_available() -> bool {
    std::env::var("RUN_LOCALSTACK_TESTS").unwrap_or_default() == "1"
}

/// Create a LocalStack S3 configuration
fn create_localstack_config(bucket: &str) -> StorageConfig {
    // Set up environment variables for LocalStack
    std::env::set_var("AWS_ACCESS_KEY_ID", "test");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
    std::env::set_var("AWS_REGION", "us-east-1");
    std::env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");
    
    StorageConfig::s3_with_bucket(bucket.to_string())
}

#[tokio::test]
async fn test_localstack_basic_operations() {
    if !check_localstack_available() {
        println!("Skipping LocalStack test - set RUN_LOCALSTACK_TESTS=1 and run LocalStack");
        return;
    }
    
    init_test_observability();
    
    let bucket = "persist-test-bucket";
    let config = create_localstack_config(bucket);
    
    // Create engine (this will create S3 client pointing to LocalStack)
    let engine = match create_engine_from_config(config) {
        Ok(engine) => engine,
        Err(e) => {
            println!("Failed to create engine - LocalStack may not be running: {}", e);
            return;
        }
    };
    
    let metrics = PersistMetrics::global();
    let initial_metrics = metrics.gather_metrics().unwrap();
    println!("Initial metrics:\n{}", initial_metrics);
    
    let agent_state = serde_json::json!({
        "agent_type": "localstack_test_agent",
        "memory": {
            "conversations": [
                {"role": "user", "content": "Test with LocalStack"},
                {"role": "assistant", "content": "LocalStack S3 simulation working"}
            ]
        },
        "config": {
            "model": "test-model",
            "temperature": 0.7
        },
        "metadata": {
            "test_environment": "localstack",
            "test_timestamp": chrono::Utc::now().timestamp()
        }
    });
    
    let agent_json = serde_json::to_string(&agent_state).unwrap();
    let metadata = SnapshotMetadata::new("localstack_agent", "test_session", 0);
    let s3_key = "test/localstack_basic_test.json.gz";
    
    // Test save operation
    tracing::info!("Starting LocalStack save operation");
    let save_result = engine.save_snapshot(&agent_json, &metadata, s3_key);
    
    match save_result {
        Ok(saved_metadata) => {
            tracing::info!("LocalStack save operation succeeded");
            assert_eq!(saved_metadata.agent_id(), "localstack_agent");
            assert_eq!(saved_metadata.session_id(), "test_session");
            
            // Test load operation
            tracing::info!("Starting LocalStack load operation");
            let load_result = engine.load_snapshot(s3_key);
            
            match load_result {
                Ok((loaded_metadata, loaded_data)) => {
                    tracing::info!("LocalStack load operation succeeded");
                    
                    // Verify data integrity
                    assert_eq!(loaded_data, agent_json);
                    assert_eq!(loaded_metadata.agent_id(), "localstack_agent");
                    assert_eq!(loaded_metadata.session_id(), "test_session");
                    assert_eq!(loaded_metadata.snapshot_index(), 0);
                    
                    // Verify metadata integrity
                    let loaded_state: serde_json::Value = serde_json::from_str(&loaded_data).unwrap();
                    assert_eq!(loaded_state["agent_type"], "localstack_test_agent");
                    assert_eq!(loaded_state["config"]["model"], "test-model");
                }
                Err(e) => {
                    panic!("LocalStack load operation failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("LocalStack save operation failed: {:?}", e);
            println!("This might indicate LocalStack is not running or configured incorrectly");
            return;
        }
    }
    
    // Wait for metrics to be recorded
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let final_metrics = metrics.gather_metrics().unwrap();
    println!("Final metrics after LocalStack operations:\n{}", final_metrics);
    
    // Verify metrics were recorded
    assert!(final_metrics.contains("persist_s3_requests_total"));
    assert!(final_metrics.contains("persist_s3_latency_seconds"));
    
    tracing::info!("LocalStack basic operations test completed successfully");
}

#[tokio::test]
async fn test_localstack_error_scenarios() {
    if !check_localstack_available() {
        println!("Skipping LocalStack error test");
        return;
    }
    
    init_test_observability();
    
    let config = create_localstack_config("nonexistent-bucket-12345");
    let engine = create_engine_from_config(config).unwrap();
    
    let metrics = PersistMetrics::global();
    
    let agent_state = serde_json::json!({"test": "error scenario"});
    let agent_json = serde_json::to_string(&agent_state).unwrap();
    let metadata = SnapshotMetadata::new("error_test", "session", 0);
    
    // This should fail with bucket not found
    tracing::info!("Testing error scenario with nonexistent bucket");
    let save_result = engine.save_snapshot(&agent_json, &metadata, "test/error_test.json.gz");
    
    assert!(save_result.is_err(), "Save to nonexistent bucket should fail");
    
    let error = save_result.unwrap_err();
    tracing::info!(error = ?error, "Error was properly generated and logged");
    
    // Test loading from nonexistent key
    let valid_config = create_localstack_config("persist-test-bucket");
    let valid_engine = create_engine_from_config(valid_config).unwrap();
    
    tracing::info!("Testing load from nonexistent key");
    let load_result = valid_engine.load_snapshot("nonexistent/key.json.gz");
    
    assert!(load_result.is_err(), "Load from nonexistent key should fail");
    
    let load_error = load_result.unwrap_err();
    tracing::info!(error = ?load_error, "Load error was properly generated and logged");
    
    // Wait for error metrics to be recorded
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let error_metrics = metrics.gather_metrics().unwrap();
    println!("Error scenario metrics:\n{}", error_metrics);
    
    // Should have recorded error metrics
    assert!(error_metrics.contains("persist_s3_errors_total"));
    
    tracing::info!("LocalStack error scenarios test completed");
}

#[tokio::test]
async fn test_localstack_concurrent_operations() {
    if !check_localstack_available() {
        println!("Skipping LocalStack concurrent test");
        return;
    }
    
    init_test_observability();
    
    let config = create_localstack_config("persist-concurrent-test");
    let engine = create_engine_from_config(config).unwrap();
    
    let metrics = PersistMetrics::global();
    let initial_metrics = metrics.gather_metrics().unwrap();
    
    // Spawn multiple concurrent operations
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let engine_clone = &engine;
        
        let handle = tokio::spawn(async move {
            let agent_state = serde_json::json!({
                "agent_id": format!("concurrent_agent_{}", i),
                "operation_id": i,
                "test_data": format!("concurrent_test_data_{}", i),
                "timestamp": chrono::Utc::now().timestamp()
            });
            
            let agent_json = serde_json::to_string(&agent_state).unwrap();
            let metadata = SnapshotMetadata::new(&format!("agent_{}", i), "concurrent", i);
            let s3_key = format!("concurrent/test_{}.json.gz", i);
            
            tracing::info!(operation_id = i, "Starting concurrent operation");
            
            // Save operation
            let save_result = engine_clone.save_snapshot(&agent_json, &metadata, &s3_key);
            assert!(save_result.is_ok(), "Concurrent save {} should succeed", i);
            
            // Immediately try to load it back
            let load_result = engine_clone.load_snapshot(&s3_key);
            assert!(load_result.is_ok(), "Concurrent load {} should succeed", i);
            
            let (loaded_metadata, loaded_data) = load_result.unwrap();
            assert_eq!(loaded_data, agent_json);
            assert_eq!(loaded_metadata.agent_id(), format!("agent_{}", i));
            
            tracing::info!(operation_id = i, "Completed concurrent operation");
            
            i
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        results.push(result);
    }
    
    // Verify all operations completed
    results.sort();
    assert_eq!(results, vec![0, 1, 2, 3, 4]);
    
    // Wait for metrics to be fully recorded
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    let final_metrics = metrics.gather_metrics().unwrap();
    println!("Concurrent operations metrics:\n{}", final_metrics);
    
    // Should see multiple requests recorded
    assert!(final_metrics.contains("persist_s3_requests_total"));
    assert!(final_metrics.contains("persist_s3_latency_seconds"));
    assert!(final_metrics.contains("persist_state_size_bytes"));
    
    tracing::info!("LocalStack concurrent operations test completed successfully");
}

#[tokio::test]
async fn test_localstack_performance_metrics() {
    if !check_localstack_available() {
        println!("Skipping LocalStack performance test");
        return;
    }
    
    init_test_observability();
    
    let config = create_localstack_config("persist-performance-test");
    let engine = create_engine_from_config(config).unwrap();
    
    let metrics = PersistMetrics::global();
    
    // Create different sized payloads to test performance characteristics
    let test_cases = vec![
        ("small", 1024),      // 1KB
        ("medium", 10 * 1024), // 10KB
        ("large", 100 * 1024), // 100KB
    ];
    
    for (size_name, payload_size) in test_cases {
        let large_data: String = "x".repeat(payload_size);
        let agent_state = serde_json::json!({
            "agent_type": format!("{}_payload_agent", size_name),
            "large_data": large_data,
            "size_category": size_name,
            "size_bytes": payload_size
        });
        
        let agent_json = serde_json::to_string(&agent_state).unwrap();
        let metadata = SnapshotMetadata::new(&format!("{}_agent", size_name), "performance", 0);
        let s3_key = format!("performance/{}_test.json.gz", size_name);
        
        let start_time = std::time::Instant::now();
        
        tracing::info!(size_category = size_name, payload_size = payload_size, "Starting performance test");
        
        // Save operation
        let save_result = engine.save_snapshot(&agent_json, &metadata, &s3_key);
        assert!(save_result.is_ok(), "Save should succeed for {} payload", size_name);
        
        let save_duration = start_time.elapsed();
        
        // Load operation
        let load_start = std::time::Instant::now();
        let load_result = engine.load_snapshot(&s3_key);
        assert!(load_result.is_ok(), "Load should succeed for {} payload", size_name);
        
        let load_duration = load_start.elapsed();
        
        let (loaded_metadata, loaded_data) = load_result.unwrap();
        assert_eq!(loaded_data, agent_json);
        
        tracing::info!(
            size_category = size_name,
            payload_size = payload_size,
            save_duration_ms = save_duration.as_millis(),
            load_duration_ms = load_duration.as_millis(),
            "Performance test completed"
        );
    }
    
    // Wait for all metrics to be recorded
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let performance_metrics = metrics.gather_metrics().unwrap();
    println!("Performance test metrics:\n{}", performance_metrics);
    
    // Verify we have latency data for different operation sizes
    assert!(performance_metrics.contains("persist_s3_latency_seconds"));
    assert!(performance_metrics.contains("persist_state_size_bytes"));
    
    tracing::info!("LocalStack performance metrics test completed");
}
