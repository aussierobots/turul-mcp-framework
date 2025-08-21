//! Notification Broadcasting Benchmarks
//!
//! Benchmarks for measuring notification system performance including SSE,
//! broadcast patterns, and concurrent notification delivery.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use http_mcp_server::sse::{SseManager, SseEvent};

async fn create_sse_connections(manager: &SseManager, count: usize) -> Vec<String> {
    let mut connection_ids = Vec::new();
    
    for i in 0..count {
        let id = format!("bench_conn_{}", i);
        manager.create_connection(id.clone()).await;
        connection_ids.push(id);
    }
    
    connection_ids
}

fn sse_manager_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sse_manager");
    
    group.bench_function("create_manager", |b| {
        b.iter(|| {
            let manager = SseManager::new();
            black_box(manager)
        });
    });
    
    group.bench_function("create_connection", |b| {
        let manager = SseManager::new();
        let mut counter = 0;
        
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let id = format!("conn_{}", counter);
            let connection = manager.create_connection(id).await;
            black_box(connection)
        });
    });
    
    group.bench_function("remove_connection", |b| {
        let manager = SseManager::new();
        let mut counter = 0;
        
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let id = format!("conn_{}", counter);
            manager.create_connection(id.clone()).await;
            manager.remove_connection(&id).await;
        });
    });
    
    // Benchmark connection count tracking
    group.bench_function("connection_count", |b| {
        let manager = SseManager::new();
        
        b.to_async(&rt).iter(|| async {
            let count = manager.connection_count().await;
            black_box(count)
        });
    });
    
    group.finish();
}

fn notification_broadcasting_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("notification_broadcasting");
    
    // Benchmark broadcasting to different numbers of connections
    for connections in [1, 5, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("broadcast_data", connections),
            connections,
            |b, &connections| {
                b.to_async(&rt).iter(|| async {
                    let manager = SseManager::new();
                    let _connection_ids = create_sse_connections(&manager, connections).await;
                    
                    let data = json!({
                        "type": "benchmark_notification",
                        "payload": "test_data",
                        "timestamp": chrono::Utc::now().timestamp()
                    });
                    
                    manager.send_data(black_box(data)).await;
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("broadcast_error", connections),
            connections,
            |b, &connections| {
                b.to_async(&rt).iter(|| async {
                    let manager = SseManager::new();
                    let _connection_ids = create_sse_connections(&manager, connections).await;
                    
                    let error = "Benchmark error message".to_string();
                    manager.send_error(black_box(error)).await;
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("broadcast_keep_alive", connections),
            connections,
            |b, &connections| {
                b.to_async(&rt).iter(|| async {
                    let manager = SseManager::new();
                    let _connection_ids = create_sse_connections(&manager, connections).await;
                    
                    manager.send_keep_alive().await;
                });
            },
        );
    }
    
    group.finish();
}

fn sse_event_formatting_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("sse_event_formatting");
    
    group.bench_function("format_connected", |b| {
        b.iter(|| {
            let event = SseEvent::Connected;
            let formatted = event.format();
            black_box(formatted)
        });
    });
    
    group.bench_function("format_keep_alive", |b| {
        b.iter(|| {
            let event = SseEvent::KeepAlive;
            let formatted = event.format();
            black_box(formatted)
        });
    });
    
    group.bench_function("format_error", |b| {
        b.iter(|| {
            let event = SseEvent::Error("Test error message".to_string());
            let formatted = event.format();
            black_box(formatted)
        });
    });
    
    // Benchmark formatting data events of different sizes
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("format_data", size),
            size,
            |b, &size| {
                let large_data = json!({
                    "type": "large_notification",
                    "data": (0..size).map(|i| format!("item_{}", i)).collect::<Vec<_>>(),
                    "metadata": {
                        "size": size,
                        "type": "benchmark"
                    }
                });
                
                b.iter(|| {
                    let event = SseEvent::Data(black_box(large_data.clone()));
                    let formatted = event.format();
                    black_box(formatted)
                });
            },
        );
    }
    
    group.finish();
}

fn concurrent_broadcasting_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_broadcasting");
    
    // Benchmark concurrent broadcasting from multiple sources
    for broadcasters in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_senders", broadcasters),
            broadcasters,
            |b, &broadcasters| {
                b.to_async(&rt).iter(|| async {
                    let manager = Arc::new(SseManager::new());
                    let _connection_ids = create_sse_connections(&manager, 20).await;
                    
                    let mut handles = Vec::new();
                    
                    for i in 0..broadcasters {
                        let manager_clone = manager.clone();
                        let handle = tokio::spawn(async move {
                            for j in 0..10 {
                                let data = json!({
                                    "broadcaster": i,
                                    "message": j,
                                    "data": format!("message_{}_{}", i, j)
                                });
                                
                                manager_clone.send_data(data).await;
                            }
                        });
                        handles.push(handle);
                    }
                    
                    futures::future::join_all(handles).await
                });
            },
        );
    }
    
    group.finish();
}

fn notification_throughput_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("notification_throughput");
    
    // Benchmark sustained notification throughput
    for rate_per_second in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("sustained_rate", rate_per_second),
            rate_per_second,
            |b, &rate_per_second| {
                b.to_async(&rt).iter(|| async {
                    let manager = SseManager::new();
                    let _connection_ids = create_sse_connections(&manager, 10).await;
                    
                    let interval = Duration::from_nanos(1_000_000_000 / rate_per_second);
                    let total_messages = 100;
                    
                    for i in 0..total_messages {
                        let data = json!({
                            "message_id": i,
                            "timestamp": chrono::Utc::now().timestamp_millis(),
                            "payload": format!("message_{}", i)
                        });
                        
                        manager.send_data(data).await;
                        
                        if i < total_messages - 1 {
                            tokio::time::sleep(interval).await;
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn notification_reliability_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("notification_reliability");
    
    // Benchmark connection stability during high load
    group.bench_function("connection_stability", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = SseManager::new();
            
            // Create connections
            let connection_ids = create_sse_connections(&manager, 50).await;
            
            // Send many notifications
            for i in 0..100 {
                let data = json!({
                    "message": i,
                    "data": format!("stability_test_{}", i)
                });
                manager.send_data(data).await;
            }
            
            // Verify connections still exist
            let final_count = manager.connection_count().await;
            black_box((connection_ids, final_count))
        });
    });
    
    // Benchmark rapid connection create/destroy
    group.bench_function("rapid_connection_churn", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = SseManager::new();
            
            for i in 0..50 {
                // Create connection
                let id = format!("churn_conn_{}", i);
                manager.create_connection(id.clone()).await;
                
                // Send notification
                let data = json!({"churn_test": i});
                manager.send_data(data).await;
                
                // Remove connection
                manager.remove_connection(&id).await;
            }
            
            let final_count = manager.connection_count().await;
            black_box(final_count)
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    sse_manager_benchmarks,
    notification_broadcasting_benchmarks,
    sse_event_formatting_benchmarks,
    concurrent_broadcasting_benchmarks,
    notification_throughput_benchmarks,
    notification_reliability_benchmarks
);

criterion_main!(benches);