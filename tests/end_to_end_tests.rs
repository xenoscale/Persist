/*!
End-to-end integration tests for the Persist system.
These tests verify the complete functionality from Python SDK through to storage.
*/

use persist_core::{
    create_default_engine, create_engine_from_config,
    SnapshotMetadata, StorageConfig, StorageBackend,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use rayon::prelude::*;

#[test]
fn test_complete_agent_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();
    
    // Simulate a realistic agent state
    let agent_state = serde_json::json!({
        "agent_type": "conversational_ai",
        "model_config": {
            "model": "gpt-4",
            "temperature": 0.7,
            "max_tokens": 2048,
            "system_prompt": "You are a helpful AI assistant."
        },
        "memory": {
            "conversation_history": [
                {"role": "user", "content": "What is the weather like?", "timestamp": 1640995200},
                {"role": "assistant", "content": "I'd be happy to help you with weather information, but I don't have access to real-time weather data.", "timestamp": 1640995210},
                {"role": "user", "content": "Can you help me with Python programming?", "timestamp": 1640995300},
                {"role": "assistant", "content": "Absolutely! I can help you with Python programming. What specific topic would you like to learn about?", "timestamp": 1640995305}
            ],
            "user_preferences": {
                "response_style": "detailed",
                "technical_level": "intermediate",
                "topics_of_interest": ["programming", "ai", "science"]
            },
            "session_context": {
                "current_task": "learning_python",
                "progress": 0.3,
                "last_interaction": 1640995305
            }
        },
        "tools": [
            {
                "name": "code_executor",
                "description": "Execute Python code safely",
                "enabled": true,
                "last_used": 1640995250
            },
            {
                "name": "web_search",
                "description": "Search the web for information",
                "enabled": true,
                "last_used": null
            }
        ],
        "state": {
            "session_id": "session_abc123",
            "agent_id": "conversational_ai_v1",
            "created_at": 1640995000,
            "last_updated": 1640995305,
            "total_interactions": 4,
            "active": true
        }
    });
    
    let agent_json = serde_json::to_string(&agent_state).unwrap();
    let metadata = SnapshotMetadata::new("conversational_ai_v1", "session_abc123", 0);
    let snapshot_path = temp_dir.path().join("agent_lifecycle_test.json.gz");
    
    // Phase 1: Save initial state
    let save_start = Instant::now();
    engine.save_snapshot(&agent_json, &metadata, snapshot_path.to_str().unwrap()).unwrap();
    let save_duration = save_start.elapsed();
    
    assert!(snapshot_path.exists());
    println!("Initial save took: {:?}", save_duration);
    
    // Phase 2: Load and verify state
    let load_start = Instant::now();
    let (loaded_metadata, loaded_data) = engine.load_snapshot(snapshot_path.to_str().unwrap()).unwrap();
    let load_duration = load_start.elapsed();
    
    assert_eq!(loaded_data, agent_json);
    assert_eq!(loaded_metadata.agent_id(), "conversational_ai_v1");
    assert_eq!(loaded_metadata.session_id(), "session_abc123");
    println!("Initial load took: {:?}", load_duration);
    
    // Phase 3: Simulate agent evolution (multiple snapshots over time)
    let mut evolved_state: serde_json::Value = serde_json::from_str(&agent_json).unwrap();
    
    for i in 1..=5 {
        // Simulate new interactions
        let new_interaction_user = serde_json::json!({
            "role": "user",
            "content": format!("This is user message number {}", i),
            "timestamp": 1640995305 + (i * 60)
        });
        let new_interaction_assistant = serde_json::json!({
            "role": "assistant", 
            "content": format!("This is assistant response number {}", i),
            "timestamp": 1640995305 + (i * 60) + 5
        });
        
        evolved_state["memory"]["conversation_history"].as_array_mut().unwrap().push(new_interaction_user);
        evolved_state["memory"]["conversation_history"].as_array_mut().unwrap().push(new_interaction_assistant);
        
        // Update counters
        evolved_state["state"]["total_interactions"] = serde_json::Value::Number(serde_json::Number::from(4 + (i * 2)));
        evolved_state["state"]["last_updated"] = serde_json::Value::Number(serde_json::Number::from(1640995305 + (i * 60) + 5));
        
        let evolved_json = serde_json::to_string(&evolved_state).unwrap();
        let evolved_metadata = SnapshotMetadata::new("conversational_ai_v1", "session_abc123", i as u64);
        let evolved_path = temp_dir.path().join(format!("agent_evolved_{}.json.gz", i));
        
        engine.save_snapshot(&evolved_json, &evolved_metadata, evolved_path.to_str().unwrap()).unwrap();
        
        // Verify each evolution
        let (verified_metadata, verified_data) = engine.load_snapshot(evolved_path.to_str().unwrap()).unwrap();
        assert_eq!(verified_data, evolved_json);
        assert_eq!(verified_metadata.snapshot_index(), i as u64);
    }
    
    println!("Successfully completed agent lifecycle with 5 evolution snapshots");
}

#[test]
fn test_multi_agent_system() {
    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();
    
    // Create multiple different types of agents
    let agent_types = vec![
        ("code_assistant", "Help with programming tasks"),
        ("data_analyst", "Analyze and visualize data"),
        ("creative_writer", "Generate creative content"),
        ("research_assistant", "Find and summarize information"),
        ("task_planner", "Plan and organize tasks"),
    ];
    
    let mut agent_snapshots = Vec::new();
    
    // Create and save snapshots for all agents
    for (i, (agent_type, description)) in agent_types.iter().enumerate() {
        let agent_state = serde_json::json!({
            "agent_type": agent_type,
            "description": description,
            "id": format!("{}_{}", agent_type, i),
            "session": format!("multi_session_{}", i),
            "config": {
                "specialized_for": agent_type,
                "capabilities": match *agent_type {
                    "code_assistant" => vec!["python", "javascript", "rust", "debugging"],
                    "data_analyst" => vec!["pandas", "matplotlib", "statistics", "visualization"],
                    "creative_writer" => vec!["storytelling", "poetry", "dialogue", "world_building"],
                    "research_assistant" => vec!["web_search", "summarization", "fact_checking"],
                    "task_planner" => vec!["scheduling", "prioritization", "project_management"],
                    _ => vec!["general"]
                }
            },
            "memory": {
                "expertise_level": i * 20, // Varying expertise levels
                "completed_tasks": i * 10,
                "user_satisfaction": 0.8 + (i as f64 * 0.04)
            },
            "timestamp": 1640995000 + (i * 3600) // 1 hour apart
        });
        
        let agent_json = serde_json::to_string(&agent_state).unwrap();
        let metadata = SnapshotMetadata::new(
            &format!("{}_{}", agent_type, i),
            &format!("multi_session_{}", i),
            0
        );
        let snapshot_path = temp_dir.path().join(format!("{}_agent.json.gz", agent_type));
        
        engine.save_snapshot(&agent_json, &metadata, snapshot_path.to_str().unwrap()).unwrap();
        agent_snapshots.push((snapshot_path, agent_json, metadata));
    }
    
    // Verify all agents can be loaded correctly
    for (snapshot_path, original_json, original_metadata) in agent_snapshots {
        let (loaded_metadata, loaded_data) = engine.load_snapshot(snapshot_path.to_str().unwrap()).unwrap();
        
        assert_eq!(loaded_data, original_json);
        assert_eq!(loaded_metadata.agent_id(), original_metadata.agent_id());
        assert_eq!(loaded_metadata.session_id(), original_metadata.session_id());
        
        // Verify the loaded data is valid JSON and contains expected fields
        let loaded_state: serde_json::Value = serde_json::from_str(&loaded_data).unwrap();
        assert!(loaded_state["agent_type"].is_string());
        assert!(loaded_state["config"]["capabilities"].is_array());
        assert!(loaded_state["memory"]["expertise_level"].is_number());
    }
    
    println!("Successfully tested multi-agent system with {} agents", agent_types.len());
}

#[test]
fn test_performance_under_load() {
    let temp_dir = TempDir::new().unwrap();
    let engine = std::sync::Arc::new(create_default_engine());
    
    // Create load test data
    let num_agents = 50;
    let operations_per_agent = 10;
    
    let agents: Vec<_> = (0..num_agents).map(|i| {
        // Create varying sized agent data
        let base_size = 1000 + (i * 100); // 1KB to 6KB
        let conversation_items = base_size / 50;
        
        let conversation: Vec<serde_json::Value> = (0..conversation_items)
            .map(|j| serde_json::json!({
                "role": if j % 2 == 0 { "user" } else { "assistant" },
                "content": format!("Message {} from agent {} - this is test content that makes the data larger", j, i),
                "timestamp": 1640995000 + (j * 30)
            }))
            .collect();
        
        let agent_state = serde_json::json!({
            "agent_id": format!("load_test_agent_{}", i),
            "session_id": format!("load_test_session_{}", i),
            "conversation": conversation,
            "metadata": {
                "created_at": 1640995000,
                "size_category": if base_size < 2000 { "small" } else if base_size < 4000 { "medium" } else { "large" },
                "performance_baseline": true
            }
        });
        
        serde_json::to_string(&agent_state).unwrap()
    }).collect();
    
    println!("Starting performance test with {} agents, {} operations each", num_agents, operations_per_agent);
    
    // Test 1: Sequential operations
    let sequential_start = Instant::now();
    for (agent_idx, agent_data) in agents.iter().enumerate() {
        for op_idx in 0..operations_per_agent {
            let metadata = SnapshotMetadata::new(
                &format!("load_test_agent_{}", agent_idx),
                &format!("load_test_session_{}", agent_idx),
                op_idx as u64
            );
            let file_path = temp_dir.path().join(format!("sequential_{}_{}.json.gz", agent_idx, op_idx));
            
            engine.save_snapshot(agent_data, &metadata, file_path.to_str().unwrap()).unwrap();
            let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
            
            assert_eq!(loaded_data, *agent_data);
            assert_eq!(loaded_metadata.snapshot_index(), op_idx as u64);
        }
    }
    let sequential_duration = sequential_start.elapsed();
    
    // Test 2: Parallel operations using rayon
    let parallel_start = Instant::now();
    (0..num_agents).into_par_iter().for_each(|agent_idx| {
        let agent_data = &agents[agent_idx];
        for op_idx in 0..operations_per_agent {
            let metadata = SnapshotMetadata::new(
                &format!("load_test_agent_{}", agent_idx),
                &format!("load_test_session_{}", agent_idx),
                op_idx as u64
            );
            let file_path = temp_dir.path().join(format!("parallel_{}_{}.json.gz", agent_idx, op_idx));
            
            engine.save_snapshot(agent_data, &metadata, file_path.to_str().unwrap()).unwrap();
            let (loaded_metadata, loaded_data) = engine.load_snapshot(file_path.to_str().unwrap()).unwrap();
            
            assert_eq!(loaded_data, *agent_data);
            assert_eq!(loaded_metadata.snapshot_index(), op_idx as u64);
        }
    });
    let parallel_duration = parallel_start.elapsed();
    
    let total_operations = num_agents * operations_per_agent;
    let sequential_ops_per_sec = total_operations as f64 / sequential_duration.as_secs_f64();
    let parallel_ops_per_sec = total_operations as f64 / parallel_duration.as_secs_f64();
    let speedup = parallel_ops_per_sec / sequential_ops_per_sec;
    
    println!("Performance Results:");
    println!("  Total operations: {}", total_operations);
    println!("  Sequential time: {:?} ({:.2} ops/sec)", sequential_duration, sequential_ops_per_sec);
    println!("  Parallel time: {:?} ({:.2} ops/sec)", parallel_duration, parallel_ops_per_sec);
    println!("  Speedup: {:.2}x", speedup);
    
    // Performance assertions
    assert!(parallel_duration < sequential_duration, "Parallel execution should be faster");
    assert!(speedup > 1.5, "Should achieve at least 1.5x speedup with parallelization");
    assert!(parallel_ops_per_sec > 10.0, "Should achieve at least 10 operations per second");
}

#[test]
fn test_memory_efficiency() {
    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();
    
    // Test with increasingly large data to verify memory efficiency
    let size_categories = vec![
        ("small", 10 * 1024),    // 10KB
        ("medium", 100 * 1024),  // 100KB  
        ("large", 1024 * 1024),  // 1MB
        ("xlarge", 5 * 1024 * 1024), // 5MB
    ];
    
    for (category, target_size) in size_categories {
        // Generate data of target size
        let base_content = "This is test content for memory efficiency testing. ".repeat(target_size / 60);
        let large_agent_state = serde_json::json!({
            "category": category,
            "content": base_content,
            "metadata": {
                "size_bytes": target_size,
                "test_type": "memory_efficiency",
                "timestamp": 1640995000
            },
            "large_array": vec![0; target_size / 10] // Additional bulk
        });
        
        let agent_json = serde_json::to_string(&large_agent_state).unwrap();
        let actual_size = agent_json.len();
        
        println!("Testing {} category: target={}KB, actual={}KB", 
                category, target_size / 1024, actual_size / 1024);
        
        let metadata = SnapshotMetadata::new(
            &format!("memory_test_{}", category),
            "memory_test_session",
            0
        );
        let snapshot_path = temp_dir.path().join(format!("memory_test_{}.json.gz", category));
        
        // Measure save operation
        let save_start = Instant::now();
        engine.save_snapshot(&agent_json, &metadata, snapshot_path.to_str().unwrap()).unwrap();
        let save_duration = save_start.elapsed();
        
        // Measure load operation
        let load_start = Instant::now();
        let (loaded_metadata, loaded_data) = engine.load_snapshot(snapshot_path.to_str().unwrap()).unwrap();
        let load_duration = load_start.elapsed();
        
        // Verify correctness
        assert_eq!(loaded_data, agent_json);
        assert_eq!(loaded_metadata.agent_id(), format!("memory_test_{}", category));
        
        // Check compression efficiency
        let file_size = std::fs::metadata(&snapshot_path).unwrap().len();
        let compression_ratio = file_size as f64 / actual_size as f64;
        
        println!("  Save: {:?}, Load: {:?}, Compression: {:.1}%", 
                save_duration, load_duration, compression_ratio * 100.0);
        
        // Performance expectations (adjust based on hardware)
        assert!(save_duration < Duration::from_secs(10), 
               "Save operation for {} should complete within 10 seconds", category);
        assert!(load_duration < Duration::from_secs(5), 
               "Load operation for {} should complete within 5 seconds", category);
        assert!(compression_ratio < 0.8, 
               "Compression should achieve at least 20% size reduction for {}", category);
    }
}

#[test]
fn test_configuration_variants() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test different storage configurations
    let local_config = StorageConfig {
        backend: StorageBackend::Local,
        bucket_name: None,
        region: None,
        access_key_id: None,
        secret_access_key: None,
        endpoint_url: None,
    };
    
    let local_engine = create_engine_from_config(local_config).unwrap();
    
    // Test with local storage
    let test_data = serde_json::json!({
        "config_test": true,
        "storage_type": "local",
        "test_data": vec!["item1", "item2", "item3"]
    });
    
    let agent_json = serde_json::to_string(&test_data).unwrap();
    let metadata = SnapshotMetadata::new("config_test_agent", "config_test_session", 0);
    let snapshot_path = temp_dir.path().join("config_test.json.gz");
    
    local_engine.save_snapshot(&agent_json, &metadata, snapshot_path.to_str().unwrap()).unwrap();
    let (loaded_metadata, loaded_data) = local_engine.load_snapshot(snapshot_path.to_str().unwrap()).unwrap();
    
    assert_eq!(loaded_data, agent_json);
    assert_eq!(loaded_metadata.agent_id(), "config_test_agent");
    
    // Test that different engines can read the same snapshots (compatibility)
    let default_engine = create_default_engine();
    let (compat_metadata, compat_data) = default_engine.load_snapshot(snapshot_path.to_str().unwrap()).unwrap();
    
    assert_eq!(compat_data, agent_json);
    assert_eq!(compat_metadata.agent_id(), loaded_metadata.agent_id());
    
    println!("Configuration variant testing completed successfully");
}

#[test]
fn test_real_world_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();
    
    // Simulate a real-world conversational AI scenario
    println!("Simulating real-world AI agent scenario...");
    
    // Phase 1: Agent initialization
    let mut agent_state = serde_json::json!({
        "agent_type": "customer_support_ai",
        "version": "2.1.0",
        "config": {
            "personality": "helpful_professional",
            "knowledge_cutoff": "2024-01-01",
            "response_style": "concise_friendly"
        },
        "session": {
            "session_id": "cs_session_12345",
            "customer_id": "cust_67890",
            "started_at": 1640995000,
            "channel": "web_chat"
        },
        "conversation": [],
        "customer_context": {
            "account_type": "premium",
            "previous_issues": 2,
            "satisfaction_score": 4.2
        },
        "tools_used": [],
        "metrics": {
            "response_time_avg_ms": 0,
            "customer_satisfaction": null,
            "issue_resolved": false
        }
    });
    
    // Phase 2: Simulate conversation with periodic snapshots
    let conversation_turns = vec![
        ("user", "Hi, I'm having trouble with my account login"),
        ("assistant", "I'd be happy to help you with your login issue. Can you tell me what specific error you're seeing?"),
        ("user", "It says 'invalid credentials' but I'm sure my password is correct"),
        ("assistant", "Let me check your account status. I can see you're a premium customer. Have you tried resetting your password recently?"),
        ("user", "No, I haven't. Should I try that?"),
        ("assistant", "Yes, let's try a password reset. I'll send you a secure link to your registered email address."),
        ("user", "Great, I got the email and reset it. Now I can log in!"),
        ("assistant", "Wonderful! I'm glad we could resolve that quickly. Is there anything else I can help you with today?"),
        ("user", "No, that's all. Thank you so much for your help!"),
        ("assistant", "You're very welcome! Have a great day, and don't hesitate to reach out if you need any further assistance."),
    ];
    
    for (turn_idx, (role, content)) in conversation_turns.iter().enumerate() {
        // Add conversation turn
        agent_state["conversation"].as_array_mut().unwrap().push(serde_json::json!({
            "role": role,
            "content": content,
            "timestamp": 1640995000 + (turn_idx * 120), // 2 minutes between turns
            "turn_id": turn_idx
        }));
        
        // Update metrics
        if *role == "assistant" {
            agent_state["metrics"]["response_time_avg_ms"] = serde_json::Value::Number(
                serde_json::Number::from(150 + (turn_idx * 10)) // Simulated response times
            );
            
            // Simulate tool usage
            if content.contains("check your account") {
                agent_state["tools_used"].as_array_mut().unwrap().push(serde_json::json!({
                    "tool": "account_lookup",
                    "timestamp": 1640995000 + (turn_idx * 120),
                    "success": true
                }));
            }
            if content.contains("password reset") {
                agent_state["tools_used"].as_array_mut().unwrap().push(serde_json::json!({
                    "tool": "password_reset_email",
                    "timestamp": 1640995000 + (turn_idx * 120),
                    "success": true
                }));
            }
        }
        
        // Create snapshot every few turns (simulating periodic saves)
        if turn_idx % 3 == 0 || turn_idx == conversation_turns.len() - 1 {
            let snapshot_idx = turn_idx / 3;
            let agent_json = serde_json::to_string(&agent_state).unwrap();
            let metadata = SnapshotMetadata::new(
                "customer_support_ai",
                "cs_session_12345",
                snapshot_idx as u64
            );
            let snapshot_path = temp_dir.path().join(format!("cs_session_snapshot_{}.json.gz", snapshot_idx));
            
            let save_start = Instant::now();
            engine.save_snapshot(&agent_json, &metadata, snapshot_path.to_str().unwrap()).unwrap();
            let save_time = save_start.elapsed();
            
            println!("  Snapshot {} saved after turn {} ({}ms)", snapshot_idx, turn_idx, save_time.as_millis());
        }
    }
    
    // Phase 3: Finalize session
    agent_state["metrics"]["issue_resolved"] = serde_json::Value::Bool(true);
    agent_state["metrics"]["customer_satisfaction"] = serde_json::Value::Number(serde_json::Number::from_f64(4.8).unwrap());
    agent_state["session"]["ended_at"] = serde_json::Value::Number(serde_json::Number::from(1640995000 + (conversation_turns.len() * 120)));
    
    let final_agent_json = serde_json::to_string(&agent_state).unwrap();
    let final_metadata = SnapshotMetadata::new(
        "customer_support_ai",
        "cs_session_12345",
        99 // Final snapshot
    );
    let final_snapshot_path = temp_dir.path().join("cs_session_final.json.gz");
    
    engine.save_snapshot(&final_agent_json, &final_metadata, final_snapshot_path.to_str().unwrap()).unwrap();
    
    // Phase 4: Verify complete session can be restored
    let (final_loaded_metadata, final_loaded_data) = engine.load_snapshot(final_snapshot_path.to_str().unwrap()).unwrap();
    
    assert_eq!(final_loaded_data, final_agent_json);
    assert_eq!(final_loaded_metadata.snapshot_index(), 99);
    
    // Verify conversation completeness
    let final_state: serde_json::Value = serde_json::from_str(&final_loaded_data).unwrap();
    assert_eq!(final_state["conversation"].as_array().unwrap().len(), conversation_turns.len());
    assert_eq!(final_state["metrics"]["issue_resolved"], true);
    assert_eq!(final_state["tools_used"].as_array().unwrap().len(), 2); // Two tools were used
    
    println!("Real-world scenario completed successfully:");
    println!("  - {} conversation turns processed", conversation_turns.len());
    println!("  - {} periodic snapshots created", conversation_turns.len() / 3 + 1);
    println!("  - Issue resolved with 4.8/5 satisfaction");
    println!("  - All data verified for consistency");
}
