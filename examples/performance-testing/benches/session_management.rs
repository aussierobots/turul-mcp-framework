//! Session Management Benchmarks
//!
//! Benchmarks for measuring session creation, state management, and cleanup performance.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::hint::black_box;
use tokio::runtime::Runtime;

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use turul_mcp_server::SessionContext;
use turul_mcp_protocol::logging::LoggingLevel;

/// Create a mock session context for benchmarking
fn create_session_context() -> SessionContext {
    let state = Arc::new(Mutex::new(HashMap::<String, Value>::new()));
    let state_clone = state.clone();
    let state_clone2 = state.clone();

    SessionContext {
        session_id: Uuid::new_v4().to_string(),
        get_state: Arc::new(move |key: &str| {
            let state = state.clone();
            let key = key.to_string();
            Box::pin(async move { state.lock().unwrap().get(&key).cloned() })
        }),
        set_state: Arc::new(move |key: &str, value: Value| {
            let state = state_clone.clone();
            let key = key.to_string();
            Box::pin(async move {
                state.lock().unwrap().insert(key, value);
            })
        }),
        remove_state: Arc::new(move |key: &str| {
            let state = state_clone2.clone();
            let key = key.to_string();
            Box::pin(async move { state.lock().unwrap().remove(&key) })
        }),
        is_initialized: Arc::new(|| Box::pin(async { true })),
        send_notification: Arc::new(|_| Box::pin(async {})),
        broadcaster: None,
    }
}

fn session_creation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_creation");
    
    group.bench_function("create_session", |b| {
        b.iter(|| {
            let session = create_session_context();
            black_box(session)
        });
    });
    
    group.bench_function("create_multiple_sessions", |b| {
        b.iter(|| {
            let sessions: Vec<_> = (0..100)
                .map(|_| create_session_context())
                .collect();
            black_box(sessions)
        });
    });
    
    group.finish();
}

fn session_state_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_state");
    
    // Benchmark state operations
    group.bench_function("set_state", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on((session.set_state)("test_key", black_box(json!(42))))
        });
    });
    
    group.bench_function("get_state", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        rt.block_on((session.set_state)("test_key", json!(42)));
        
        b.iter(|| {
            let value = rt.block_on((session.get_state)("test_key"));
            black_box(value)
        });
    });
    
    group.bench_function("get_missing_state", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        
        b.iter(|| {
            let value = rt.block_on((session.get_state)("missing_key"));
            black_box(value)
        });
    });
    
    // Benchmark state operations with different data sizes
    for size in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("set_large_state", size),
            size,
            |b, &size| {
                let rt = Runtime::new().unwrap();
                let session = create_session_context();
                let large_data = json!({
                    "data": (0..size).map(|i| format!("item_{}", i)).collect::<Vec<_>>(),
                    "metadata": {
                        "size": size,
                        "type": "benchmark_data"
                    }
                });
                
                b.iter(|| {
                    rt.block_on((session.set_state)("large_data", black_box(large_data.clone())));
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("get_large_state", size),
            size,
            |b, &size| {
                let rt = Runtime::new().unwrap();
                let session = create_session_context();
                let large_data = json!({
                    "data": (0..size).map(|i| format!("item_{}", i)).collect::<Vec<_>>(),
                    "metadata": {
                        "size": size,
                        "type": "benchmark_data"
                    }
                });
                rt.block_on((session.set_state)("large_data", large_data));
                
                b.iter(|| {
                    let value = rt.block_on((session.get_state)("large_data"));
                    black_box(value)
                });
            },
        );
    }
    
    group.finish();
}

fn concurrent_session_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_sessions");
    
    // Benchmark concurrent session operations
    for sessions in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_state_operations", sessions),
            sessions,
            |b, &sessions| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    
                    for i in 0..sessions {
                        let handle = tokio::spawn(async move {
                            let session = create_session_context();
                            
                            // Perform multiple operations per session
                            for j in 0..10 {
                                let key = format!("key_{}_{}", i, j);
                                let value = json!({"session": i, "operation": j});
                                (session.set_state)(&key, value).await;
                                
                                let retrieved = (session.get_state)(&key).await;
                                black_box(retrieved);
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

fn session_memory_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_memory");
    
    // Benchmark memory usage with many keys
    for keys in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("many_keys", keys),
            keys,
            |b, &keys| {
                let rt = Runtime::new().unwrap();
                b.iter(|| {
                    let session = create_session_context();
                    
                    // Set many keys
                    for i in 0..keys {
                        let key = format!("key_{}", i);
                        let value = json!({
                            "index": i,
                            "data": format!("value_{}", i),
                            "timestamp": chrono::Utc::now().timestamp()
                        });
                        rt.block_on((session.set_state)(&key, value));
                    }
                    
                    // Retrieve some keys
                    for i in (0..keys).step_by(10) {
                        let key = format!("key_{}", i);
                        let value = rt.block_on((session.get_state)(&key));
                        black_box(value);
                    }
                    
                    black_box(session)
                });
            },
        );
    }
    
    group.finish();
}

fn session_notification_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("session_notifications");

    // Test MCP spec notifications - logging
    group.bench_function("notify_log", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on(session.notify_log(
                LoggingLevel::Info,
                json!("benchmark log message"),
                Some("bench".to_string()),
                None
            ))
        });
    });

    // Test MCP spec notifications - progress
    group.bench_function("notify_progress", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on(session.notify_progress("bench-token", 50));
        });
    });

    // Test MCP spec notifications - progress with total
    group.bench_function("notify_progress_with_total", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on(session.notify_progress_with_total("bench-token", 75, 100));
        });
    });

    // Test MCP spec notifications - resources changed
    group.bench_function("notify_resources_changed", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on(session.notify_resources_changed());
        });
    });

    // Test MCP spec notifications - resource updated
    group.bench_function("notify_resource_updated", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on(session.notify_resource_updated("file:///test/resource.txt"));
        });
    });

    // Test MCP spec notifications - tools changed
    group.bench_function("notify_tools_changed", |b| {
        let rt = Runtime::new().unwrap();
        let session = create_session_context();
        b.iter(|| {
            rt.block_on(session.notify_tools_changed());
        });
    });


    // Benchmark concurrent MCP notifications
    for sessions in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_mcp_notifications", sessions),
            sessions,
            |b, &sessions| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();

                    for i in 0..sessions {
                        let handle = tokio::spawn(async move {
                            let session = create_session_context();

                            // Send variety of MCP notifications per session
                            session
                                .notify_log(
                                LoggingLevel::Info,
                                json!(format!("Concurrent session {} log", i)),
                                Some("bench".to_string()),
                                None
                            )
                            .await;
                            session
                                .notify_progress(&format!("task-{}", i), (i as u64) * 10)
                                .await;
                            session.notify_resources_changed().await;
                            session.notify_tools_changed().await;
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

criterion_group!(
    benches,
    session_creation_benchmarks,
    session_state_benchmarks,
    concurrent_session_benchmarks,
    session_memory_benchmarks,
    session_notification_benchmarks
);

criterion_main!(benches);
