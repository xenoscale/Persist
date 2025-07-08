/*!
Simple benchmark example for hyperfine performance testing.
*/

use persist_core::{create_default_engine, SnapshotMetadata};
use std::time::Instant;

fn main() {
    let engine = create_default_engine();
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create sample agent data
    let test_data = serde_json::json!({
        "agent_type": "benchmark_agent",
        "config": {
            "model": "gpt-4",
            "temperature": 0.7,
            "max_tokens": 2048
        },
        "memory": {
            "conversation_history": [
                {"role": "user", "content": "Hello, how can you help me today?"},
                {"role": "assistant", "content": "I'm here to help you with any questions or tasks you have!"},
                {"role": "user", "content": "Can you explain how AI agents work?"},
                {"role": "assistant", "content": "AI agents are systems that perceive their environment and take actions to achieve specific goals..."}
            ],
            "facts": {
                "user_preference": "detailed explanations",
                "interaction_count": 25,
                "satisfaction_score": 4.8
            }
        },
        "tools": [
            {"name": "web_search", "enabled": true},
            {"name": "calculator", "enabled": true},
            {"name": "file_reader", "enabled": false}
        ],
        "state": {
            "session_id": "benchmark_session_001",
            "active": true,
            "last_interaction": 1640995200
        }
    });

    let agent_json = serde_json::to_string(&test_data).unwrap();
    let metadata = SnapshotMetadata::new("benchmark_agent", "benchmark_session", 0);
    let file_path = temp_dir.path().join("benchmark_snapshot.json.gz");

    let start = Instant::now();

    // Perform save operation
    engine
        .save_snapshot(&agent_json, &metadata, file_path.to_str().unwrap())
        .unwrap();

    // Perform load operation
    let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();

    let duration = start.elapsed();

    // Verify correctness
    assert_eq!(loaded_data, agent_json);
    assert_eq!(loaded_metadata.agent_id(), "benchmark_agent");

    println!("Benchmark operation completed in: {:?}", duration);
    println!("Data size: {} bytes", agent_json.len());
    println!(
        "File size: {} bytes",
        std::fs::metadata(&file_path).unwrap().len()
    );
    println!(
        "Compression ratio: {:.2}%",
        (std::fs::metadata(&file_path).unwrap().len() as f64 / agent_json.len() as f64) * 100.0
    );
}
