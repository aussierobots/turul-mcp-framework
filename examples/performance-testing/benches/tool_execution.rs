//! Tool Execution Benchmarks
//!
//! Benchmarks for measuring tool execution performance across different scenarios.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::hint::black_box;
use tokio::runtime::Runtime;

use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Simple computation tool for benchmarking
#[derive(McpTool, Clone)]
#[tool(name = "benchmark_compute", description = "Simple computation for benchmarking")]
struct BenchmarkComputeTool {
    #[param(description = "Input value")]
    input: i64,
}

impl BenchmarkComputeTool {
    async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> McpResult<String> {
        let result = (0..100).fold(self.input, |acc, i| acc.wrapping_mul(i + 1));
        Ok(format!("Result: {}", result))
    }
}

/// CPU-intensive tool for stress benchmarking
#[derive(McpTool, Clone)]
#[tool(name = "benchmark_cpu", description = "CPU-intensive computation for benchmarking")]
struct BenchmarkCpuTool {
    #[param(description = "Computation size")]
    size: u32,
}

impl BenchmarkCpuTool {
    async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> McpResult<String> {
        let mut result = 0u64;
        for i in 0..self.size {
            for j in 0..100 {
                result = result.wrapping_add((i as u64).wrapping_mul(j));
            }
        }
        Ok(format!("CPU result: {}", result))
    }
}

/// Session-aware tool for session overhead benchmarking
#[derive(McpTool, Clone)]
#[tool(name = "benchmark_session", description = "Session-aware tool for benchmarking")]
struct BenchmarkSessionTool {
    #[param(description = "Value to store")]
    value: i64,
}

impl BenchmarkSessionTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            let current: i64 = (session.get_state)("counter")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            
            let new_value = current + self.value;
            (session.set_state)("counter", json!(new_value));
            
            Ok(format!("Session value: {}", new_value))
        } else {
            Ok(format!("No session value: {}", self.value))
        }
    }
}

fn tool_execution_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("tool_execution");
    
    // Benchmark simple computation tool
    let compute_tool = BenchmarkComputeTool { input: 42 };
    group.bench_function("simple_compute", |b| {
        b.to_async(&rt).iter(|| async {
            let args = json!({"input": black_box(42)});
            let result = compute_tool.call(args, None).await;
            black_box(result)
        });
    });
    
    // Benchmark CPU-intensive tool with different sizes
    for size in [10, 50, 100, 500].iter() {
        let cpu_tool = BenchmarkCpuTool { size: *size };
        group.bench_with_input(
            BenchmarkId::new("cpu_intensive", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let args = json!({"size": black_box(size)});
                    let result = cpu_tool.call(args, None).await;
                    black_box(result)
                });
            },
        );
    }
    
    // Benchmark session-aware tool without session
    let session_tool = BenchmarkSessionTool { value: 1 };
    group.bench_function("session_without_context", |b| {
        b.to_async(&rt).iter(|| async {
            let args = json!({"value": black_box(1)});
            let result = session_tool.call(args, None).await;
            black_box(result)
        });
    });
    
    // Benchmark session-aware tool with session
    group.bench_function("session_with_context", |b| {
        b.to_async(&rt).iter(|| async {
            use std::sync::{Arc, Mutex};
            
            let state = Arc::new(Mutex::new(HashMap::<String, Value>::new()));
            let state_clone = state.clone();
            let state_clone2 = state.clone();

            let session = SessionContext {
                session_id: uuid::Uuid::new_v4().to_string(),
                get_state: Arc::new(move |key: &str| {
                    state.lock().unwrap().get(key).cloned()
                }),
                set_state: Arc::new(move |key: &str, value: Value| {
                    state_clone.lock().unwrap().insert(key.to_string(), value);
                }),
                remove_state: Arc::new(move |key: &str| {
                    state_clone2.lock().unwrap().remove(key)
                }),
                is_initialized: Arc::new(|| true),
                send_notification: Arc::new(|_| {}),
                broadcaster: None,
            };
            
            let args = json!({"value": black_box(1)});
            let result = session_tool.call(args, Some(session)).await;
            black_box(result)
        });
    });
    
    group.finish();
}

fn concurrent_execution_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_execution");
    
    // Benchmark concurrent tool execution
    for concurrency in [1, 2, 4, 8, 16].iter() {
        let compute_tool = BenchmarkComputeTool { input: 42 };
        group.bench_with_input(
            BenchmarkId::new("concurrent_compute", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrency {
                        let tool = compute_tool.clone();
                        let handle = tokio::spawn(async move {
                            let args = json!({"input": black_box(42 + i)});
                            tool.call(args, None).await
                        });
                        handles.push(handle);
                    }
                    
                    let results = futures::future::join_all(handles).await;
                    black_box(results)
                });
            },
        );
    }
    
    group.finish();
}

fn schema_generation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_generation");
    
    let compute_tool = BenchmarkComputeTool { input: 0 };
    let cpu_tool = BenchmarkCpuTool { size: 0 };
    let session_tool = BenchmarkSessionTool { value: 0 };
    
    group.bench_function("simple_schema", |b| {
        b.iter(|| {
            let schema = compute_tool.input_schema();
            black_box(schema)
        });
    });
    
    group.bench_function("cpu_schema", |b| {
        b.iter(|| {
            let schema = cpu_tool.input_schema();
            black_box(schema)
        });
    });
    
    group.bench_function("session_schema", |b| {
        b.iter(|| {
            let schema = session_tool.input_schema();
            black_box(schema)
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    tool_execution_benchmarks,
    concurrent_execution_benchmarks,
    schema_generation_benchmarks
);

criterion_main!(benches);