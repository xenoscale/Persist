/*!
Test module aggregation for persist-core.
This module includes all comprehensive test modules.
*/

#[cfg(test)]
pub mod error_tests;

#[cfg(test)]
pub mod metadata_tests;

#[cfg(test)]
pub mod storage_tests;

#[cfg(test)]
pub mod snapshot_tests;

// Re-export test utilities if needed
#[cfg(test)]
pub use tempfile::TempDir;

#[cfg(test)]
pub use std::sync::Arc;

#[cfg(test)]
pub use rayon::prelude::*;

/// Common test utilities
#[cfg(test)]
pub mod utils {
    use super::*;
    use crate::{SnapshotMetadata, create_default_engine};
    use std::collections::HashMap;
    
    /// Create test agent data of specified size (approximately)
    pub fn create_test_agent_data(size_kb: usize) -> String {
        let target_bytes = size_kb * 1024;
        let base_content = "test content for size scaling ".repeat(target_bytes / 100);
        
        let agent_data = serde_json::json!({
            "type": "test_agent",
            "size_category": format!("{}KB", size_kb),
            "content": base_content,
            "config": {
                "model": "test-model",
                "temperature": 0.7
            },
            "memory": {
                "conversation": (0..size_kb).map(|i| {
                    serde_json::json!({
                        "turn": i,
                        "content": format!("Test message {}", i)
                    })
                }).collect::<Vec<_>>()
            },
            "metadata": {
                "created_at": 1640995000,
                "test_marker": true
            }
        });
        
        serde_json::to_string(&agent_data).unwrap()
    }
    
    /// Create test metadata with default values
    pub fn create_test_metadata(agent_id: &str, session_id: &str, index: u64) -> SnapshotMetadata {
        SnapshotMetadata::new(agent_id, session_id, index)
    }
    
    /// Helper to run performance test
    pub fn measure_operation<F, R>(operation: F) -> (R, std::time::Duration)
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = operation();
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Helper to create temporary test files
    pub fn create_temp_snapshot_path(temp_dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        temp_dir.join(format!("{}.json.gz", name))
    }
}

/// Integration test helpers
#[cfg(test)]
pub mod integration {
    use super::*;
    use crate::{SnapshotEngine, create_default_engine};
    
    /// Test a complete save/load cycle
    pub fn test_roundtrip(data: &str, metadata: &crate::SnapshotMetadata, file_path: &str) -> crate::Result<(crate::SnapshotMetadata, String)> {
        let engine = create_default_engine();
        
        // Save
        engine.save_snapshot(data, metadata, file_path)?;
        
        // Verify file exists
        assert!(std::path::Path::new(file_path).exists());
        
        // Load
        let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path)?;
        
        // Verify data integrity
        assert_eq!(loaded_data, data);
        assert_eq!(loaded_metadata.agent_id(), metadata.agent_id());
        assert_eq!(loaded_metadata.session_id(), metadata.session_id());
        assert_eq!(loaded_metadata.snapshot_index(), metadata.snapshot_index());
        
        Ok((loaded_metadata, loaded_data))
    }
    
    /// Performance test helper
    pub fn performance_test_multiple_agents(num_agents: usize, operations_per_agent: usize) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let engine = std::sync::Arc::new(create_default_engine());
        
        let mut metrics = HashMap::new();
        
        // Sequential test
        let start = std::time::Instant::now();
        for agent_idx in 0..num_agents {
            for op_idx in 0..operations_per_agent {
                let data = utils::create_test_agent_data(10); // 10KB per agent
                let metadata = utils::create_test_metadata(
                    &format!("agent_{}", agent_idx),
                    &format!("session_{}", agent_idx),
                    op_idx as u64
                );
                let file_path = utils::create_temp_snapshot_path(
                    temp_dir.path(),
                    &format!("seq_{}_{}", agent_idx, op_idx)
                );
                
                engine.save_snapshot(&data, &metadata, file_path.to_str().unwrap())?;
                let (_meta, _data) = engine.load_snapshot(file_path.to_str().unwrap())?;
            }
        }
        let sequential_time = start.elapsed().as_secs_f64();
        
        // Parallel test
        let start = std::time::Instant::now();
        (0..num_agents).into_par_iter().try_for_each(|agent_idx| -> crate::Result<()> {
            for op_idx in 0..operations_per_agent {
                let data = utils::create_test_agent_data(10);
                let metadata = utils::create_test_metadata(
                    &format!("agent_{}", agent_idx),
                    &format!("session_{}", agent_idx),
                    op_idx as u64
                );
                let file_path = utils::create_temp_snapshot_path(
                    temp_dir.path(),
                    &format!("par_{}_{}", agent_idx, op_idx)
                );
                
                engine.save_snapshot(&data, &metadata, file_path.to_str().unwrap())?;
                let (_meta, _data) = engine.load_snapshot(file_path.to_str().unwrap())?;
            }
            Ok(())
        }).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let parallel_time = start.elapsed().as_secs_f64();
        
        let total_ops = num_agents * operations_per_agent;
        metrics.insert("sequential_ops_per_sec".to_string(), total_ops as f64 / sequential_time);
        metrics.insert("parallel_ops_per_sec".to_string(), total_ops as f64 / parallel_time);
        metrics.insert("speedup".to_string(), sequential_time / parallel_time);
        
        Ok(metrics)
    }
}
