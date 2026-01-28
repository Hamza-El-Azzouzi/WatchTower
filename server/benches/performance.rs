// Performance benchmarks for the monitoring system
// These benchmarks demonstrate the Criterion framework setup
// Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

// Simple computation benchmarks
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn benchmark_fibonacci(c: &mut Criterion) {
    c.bench_function("fibonacci_20", |b| {
        b.iter(|| fibonacci(black_box(20)));
    });
}

// JSON parsing benchmarks (relevant to API performance)
fn benchmark_json_parsing(c: &mut Criterion) {
    let json_data = r#"{
        "agent_id": "test-agent",
        "timestamp": 1234567890,
        "metrics": {
            "cpu_usage": 45.5,
            "memory_usage": 60.0,
            "disk_usage": 75.0,
            "network_in": 1024.0,
            "network_out": 2048.0
        }
    }"#;

    c.bench_function("json_parse_metrics", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(json_data)).unwrap());
    });
}

fn benchmark_json_serialization(c: &mut Criterion) {
    let data = serde_json::json!({
        "agent_id": "test-agent",
        "timestamp": 1234567890,
        "metrics": {
            "cpu_usage": 45.5,
            "memory_usage": 60.0,
            "disk_usage": 75.0,
            "network_in": 1024.0,
            "network_out": 2048.0
        }
    });

    c.bench_function("json_serialize_metrics", |b| {
        b.iter(|| serde_json::to_string(&data).unwrap());
    });
}

// String operations (relevant to key generation and validation)
fn benchmark_string_operations(c: &mut Criterion) {
    let text = "msk_1234567890abcdefghijklmnopqrstuvwxyz1234567890";

    c.bench_function("string_starts_with", |b| {
        b.iter(|| black_box(text).starts_with("msk_"));
    });

    c.bench_function("string_length", |b| {
        b.iter(|| black_box(text).len());
    });
}

// Vector operations (relevant to metric storage)
fn benchmark_vector_operations(c: &mut Criterion) {
    c.bench_function("vec_push_1000", |b| {
        b.iter(|| {
            let mut vec = Vec::new();
            for i in 0..1000 {
                vec.push(black_box(i));
            }
            vec
        });
    });

    c.bench_function("vec_with_capacity_1000", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(1000);
            for i in 0..1000 {
                vec.push(black_box(i));
            }
            vec
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets =
        benchmark_fibonacci,
        benchmark_json_parsing,
        benchmark_json_serialization,
        benchmark_string_operations,
        benchmark_vector_operations
}

criterion_main!(benches);

// Note: For comprehensive end-to-end performance testing:
//
// 1. Start the server in release mode:
//    cargo run --release
//
// 2. Use load testing tools:
//    - wrk: wrk -t4 -c100 -d30s http://localhost:8080/health
//    - hey: hey -n 10000 -c 100 http://localhost:8080/api/metrics
//    - k6: for complex scenarios with authentication
//
// 3. Monitor performance:
//    - cargo flamegraph (CPU profiling)
//    - valgrind --tool=massif (memory profiling)
//    - Docker stats (container resource usage)
//
// 4. Database performance:
//    - Use EXPLAIN QUERY PLAN for SQLite queries
//    - Monitor query execution times
//    - Check index usage
//
// These micro-benchmarks demonstrate Criterion is working correctly.
// They measure operations relevant to the monitoring system.
