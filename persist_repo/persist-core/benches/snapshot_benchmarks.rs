/*!
Performance benchmarks for the Persist snapshot system.
These benchmarks help identify bottlenecks and measure performance improvements.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use persist_core::{
    compression::NoCompression, create_default_engine, GzipCompressor, LocalFileStorage,
    SnapshotEngine, SnapshotMetadata,
};
use rayon::prelude::*;
use std::collections::HashMap;
use tempfile::TempDir;

// Helper function to generate test data of various sizes
fn generate_test_data(size_kb: usize) -> String {
    let base_object = serde_json::json!({
        "type": "benchmark_agent",
        "config": {
            "model": "gpt-4",
            "temperature": 0.7,
            "max_tokens": 2048
        },
        "memory": {
            "conversation_history": [],
            "facts": {},
            "context": ""
        },
        "tools": [
            {"name": "calculator", "description": "Perform mathematical calculations"},
            {"name": "web_search", "description": "Search the web for information"},
            {"name": "file_reader", "description": "Read file contents"}
        ]
    });

    // Scale up the data to reach desired size
    let target_bytes = size_kb * 1024;
    let base_size = serde_json::to_string(&base_object).unwrap().len();
    let multiplier = (target_bytes / base_size).max(1);

    let mut scaled_object = base_object.clone();

    // Add conversation history to scale up size
    let conversation: Vec<serde_json::Value> = (0..multiplier)
        .map(|i| serde_json::json!({
            "role": if i % 2 == 0 { "user" } else { "assistant" },
            "content": format!("This is message {} in the conversation history. It contains some realistic text that an agent might encounter during normal operation.", i),
            "timestamp": 1640995200 + i * 60, // Incremental timestamps
            "metadata": {
                "message_id": format!("msg_{}", i),
                "processing_time_ms": 150 + (i % 100)
            }
        }))
        .collect();

    scaled_object["memory"]["conversation_history"] = serde_json::Value::Array(conversation);

    // Add some facts to memory
    let facts: HashMap<String, serde_json::Value> = (0..(multiplier / 10).max(1))
        .map(|i| {
            (
                format!("fact_{i}"),
                serde_json::json!({
                    "value": format!("This is fact number {i} that the agent has learned"),
                    "confidence": 0.8 + (i % 20) as f64 * 0.01,
                    "source": format!("source_{}", i % 5),
                    "learned_at": 1640995200 + i * 3600
                }),
            )
        })
        .collect();

    scaled_object["memory"]["facts"] = serde_json::to_value(facts).unwrap();

    serde_json::to_string(&scaled_object).unwrap()
}

fn benchmark_save_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_save");

    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();

    // Test different data sizes
    for size_kb in [1, 10, 100, 1000].iter() {
        let data = generate_test_data(*size_kb);
        group.throughput(Throughput::Bytes(data.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("save", format!("{size_kb}KB")),
            size_kb,
            |b, &size_kb| {
                let data = generate_test_data(size_kb);
                let metadata = SnapshotMetadata::new("benchmark_agent", "benchmark_session", 0);

                b.iter(|| {
                    let file_path = temp_dir.path().join(format!(
                        "bench_save_{}_{}.json.gz",
                        size_kb,
                        rand::random::<u32>()
                    ));
                    black_box(
                        engine
                            .save_snapshot(
                                black_box(&data),
                                black_box(&metadata),
                                black_box(file_path.to_str().unwrap()),
                            )
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

fn benchmark_load_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_load");

    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();

    // Pre-create snapshots for different sizes
    let mut snapshot_paths = Vec::new();
    for size_kb in [1, 10, 100, 1000].iter() {
        let data = generate_test_data(*size_kb);
        let metadata = SnapshotMetadata::new("benchmark_agent", "benchmark_session", 0);
        let file_path = temp_dir
            .path()
            .join(format!("bench_load_{size_kb}.json.gz"));

        engine
            .save_snapshot(&data, &metadata, file_path.to_str().unwrap())
            .unwrap();
        snapshot_paths.push((file_path, *size_kb, data.len()));
    }

    for (file_path, size_kb, data_len) in snapshot_paths {
        group.throughput(Throughput::Bytes(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("load", format!("{size_kb}KB")),
            &file_path,
            |b, file_path| {
                b.iter(|| {
                    black_box(
                        engine
                            .load_snapshot(black_box(file_path.to_str().unwrap()))
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

fn benchmark_compression_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_comparison");

    let temp_dir = TempDir::new().unwrap();
    let data = generate_test_data(100); // 100KB test data
    let metadata = SnapshotMetadata::new("compression_test", "session", 0);

    // Test different compression algorithms
    let gzip_engine = SnapshotEngine::new(LocalFileStorage::new(), GzipCompressor::new());
    let gzip_fast_engine = SnapshotEngine::new(LocalFileStorage::new(), GzipCompressor::fast());
    let gzip_max_engine = SnapshotEngine::new(LocalFileStorage::new(), GzipCompressor::max());
    let no_comp_engine = SnapshotEngine::new(LocalFileStorage::new(), NoCompression::new());

    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("gzip_default", |b| {
        b.iter(|| {
            let file_path = temp_dir
                .path()
                .join(format!("gzip_default_{}.json.gz", rand::random::<u32>()));
            black_box(
                gzip_engine
                    .save_snapshot(
                        black_box(&data),
                        black_box(&metadata),
                        black_box(file_path.to_str().unwrap()),
                    )
                    .unwrap(),
            );
        });
    });

    group.bench_function("gzip_fast", |b| {
        b.iter(|| {
            let file_path = temp_dir
                .path()
                .join(format!("gzip_fast_{}.json.gz", rand::random::<u32>()));
            black_box(
                gzip_fast_engine
                    .save_snapshot(
                        black_box(&data),
                        black_box(&metadata),
                        black_box(file_path.to_str().unwrap()),
                    )
                    .unwrap(),
            );
        });
    });

    group.bench_function("gzip_max", |b| {
        b.iter(|| {
            let file_path = temp_dir
                .path()
                .join(format!("gzip_max_{}.json.gz", rand::random::<u32>()));
            black_box(
                gzip_max_engine
                    .save_snapshot(
                        black_box(&data),
                        black_box(&metadata),
                        black_box(file_path.to_str().unwrap()),
                    )
                    .unwrap(),
            );
        });
    });

    group.bench_function("no_compression", |b| {
        b.iter(|| {
            let file_path = temp_dir
                .path()
                .join(format!("no_comp_{}.json", rand::random::<u32>()));
            black_box(
                no_comp_engine
                    .save_snapshot(
                        black_box(&data),
                        black_box(&metadata),
                        black_box(file_path.to_str().unwrap()),
                    )
                    .unwrap(),
            );
        });
    });

    group.finish();
}

fn benchmark_parallel_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_operations");

    let temp_dir = TempDir::new().unwrap();
    let engine = std::sync::Arc::new(create_default_engine());

    // Create test data for parallel operations
    let agents: Vec<_> = (0..20)
        .map(|i| {
            (
                generate_test_data(10), // 10KB per agent
                SnapshotMetadata::new(format!("parallel_agent_{i}"), "parallel_session", i as u64),
                temp_dir.path().join(format!("parallel_{i}.json.gz")),
            )
        })
        .collect();

    group.throughput(Throughput::Elements(agents.len() as u64));

    group.bench_function("sequential_save", |b| {
        b.iter(|| {
            for (i, (data, metadata, _)) in agents.iter().enumerate() {
                let file_path = temp_dir.path().join(format!(
                    "seq_save_{}_{}.json.gz",
                    i,
                    rand::random::<u32>()
                ));
                black_box(
                    engine
                        .save_snapshot(
                            black_box(data),
                            black_box(metadata),
                            black_box(file_path.to_str().unwrap()),
                        )
                        .unwrap(),
                );
            }
        });
    });

    group.bench_function("parallel_save", |b| {
        b.iter(|| {
            agents
                .par_iter()
                .enumerate()
                .for_each(|(i, (data, metadata, _))| {
                    let file_path = temp_dir.path().join(format!(
                        "par_save_{}_{}.json.gz",
                        i,
                        rand::random::<u32>()
                    ));
                    black_box(
                        engine
                            .save_snapshot(
                                black_box(data),
                                black_box(metadata),
                                black_box(file_path.to_str().unwrap()),
                            )
                            .unwrap(),
                    );
                });
        });
    });

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();

    // Test memory usage with large data
    for size_kb in [100, 500, 1000, 5000].iter() {
        let data = generate_test_data(*size_kb);
        let metadata = SnapshotMetadata::new("memory_test", "session", 0);

        group.bench_with_input(
            BenchmarkId::new("memory_save", format!("{size_kb}KB")),
            size_kb,
            |b, &size_kb| {
                b.iter(|| {
                    let file_path = temp_dir.path().join(format!(
                        "memory_test_{}_{}.json.gz",
                        size_kb,
                        rand::random::<u32>()
                    ));
                    black_box(
                        engine
                            .save_snapshot(
                                black_box(&data),
                                black_box(&metadata),
                                black_box(file_path.to_str().unwrap()),
                            )
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

fn benchmark_roundtrip_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_operations");

    let temp_dir = TempDir::new().unwrap();
    let engine = create_default_engine();

    for size_kb in [10, 100, 1000].iter() {
        let data = generate_test_data(*size_kb);
        group.throughput(Throughput::Bytes(data.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("save_load_roundtrip", format!("{size_kb}KB")),
            size_kb,
            |b, &size_kb| {
                let data = generate_test_data(size_kb);
                let metadata = SnapshotMetadata::new("roundtrip_agent", "roundtrip_session", 0);

                b.iter(|| {
                    let file_path = temp_dir.path().join(format!(
                        "roundtrip_{}_{}.json.gz",
                        size_kb,
                        rand::random::<u32>()
                    ));

                    // Save
                    black_box(
                        engine
                            .save_snapshot(
                                black_box(&data),
                                black_box(&metadata),
                                black_box(file_path.to_str().unwrap()),
                            )
                            .unwrap(),
                    );

                    // Load
                    let (loaded_metadata, loaded_data) = black_box(
                        engine
                            .load_snapshot(black_box(file_path.to_str().unwrap()))
                            .unwrap(),
                    );

                    // Verify by comparing the parsed JSON structures. The
                    // serialized length can vary because `HashMap` iteration
                    // order is nondeterministic across runs.
                    let loaded_json: serde_json::Value =
                        serde_json::from_str(&loaded_data).unwrap();
                    let original_json: serde_json::Value = serde_json::from_str(&data).unwrap();
                    assert_eq!(loaded_json, original_json);
                    assert_eq!(loaded_metadata.agent_id, "roundtrip_agent");
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_save_operations,
    benchmark_load_operations,
    benchmark_compression_algorithms,
    benchmark_parallel_operations,
    benchmark_memory_usage,
    benchmark_roundtrip_operations
);
criterion_main!(benches);
