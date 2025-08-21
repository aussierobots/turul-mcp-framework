//! Load Test Server - High-Performance MCP Server for Load Testing
//!
//! This server is optimized for high throughput and concurrent connections,
//! providing various tools and resources for performance testing scenarios.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use mcp_derive::McpTool;
use mcp_server::{McpServer, SessionContext, McpResult};
use mcp_protocol::McpError;
use serde_json::json;
use async_trait::async_trait;
use tokio::time::sleep;
use tracing::info;
use rand::Rng;

/// Global request counter for performance monitoring
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);
static TOTAL_PROCESSING_TIME: AtomicU64 = AtomicU64::new(0);

/// Fast computation tool - minimal overhead for throughput testing
#[derive(McpTool, Clone)]
#[tool(name = "fast_compute", description = "Performs fast computation for throughput testing")]
struct FastComputeTool {
    #[param(description = "Input number")]
    input: i64,
}

impl FastComputeTool {
    async fn execute(&self) -> McpResult<String> {
        let start = Instant::now();
        REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        // Simple computation that exercises CPU without being too heavy
        let result = (0..100).fold(self.input, |acc, i| acc.wrapping_mul(i + 1));
        
        let elapsed = start.elapsed().as_micros() as u64;
        TOTAL_PROCESSING_TIME.fetch_add(elapsed, Ordering::Relaxed);
        
        Ok(format!("Computed result: {}", result))
    }
}

/// CPU-intensive tool for stress testing
#[derive(McpTool, Clone)]
#[tool(name = "cpu_intensive", description = "CPU-intensive operation for stress testing")]
struct CpuIntensiveTool {
    #[param(description = "Computation size")]
    size: u32,
}

impl CpuIntensiveTool {
    async fn execute(&self) -> McpResult<String> {
        let start = Instant::now();
        REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        // CPU-intensive computation
        let mut result = 0u64;
        for i in 0..self.size {
            for j in 0..1000 {
                result = result.wrapping_add((i as u64).wrapping_mul(j));
            }
        }
        
        let elapsed = start.elapsed().as_micros() as u64;
        TOTAL_PROCESSING_TIME.fetch_add(elapsed, Ordering::Relaxed);
        
        Ok(format!("CPU-intensive result: {}", result))
    }
}

/// Memory allocation tool for memory testing
#[derive(McpTool, Clone)]
#[tool(name = "memory_allocate", description = "Allocates memory for memory testing")]
struct MemoryAllocateTool {
    #[param(description = "Number of MB to allocate")]
    mb_size: u32,
}

impl MemoryAllocateTool {
    async fn execute(&self) -> McpResult<String> {
        let start = Instant::now();
        REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        // Allocate memory and touch it to ensure it's actually allocated
        let size = (self.mb_size as usize) * 1024 * 1024;
        let mut data = vec![0u8; size];
        
        // Touch memory to ensure allocation
        let mut rng = rand::thread_rng();
        for i in (0..size).step_by(4096) {
            data[i] = rng.r#gen();
        }
        
        let checksum: u64 = data.iter().map(|&b| b as u64).sum();
        
        let elapsed = start.elapsed().as_micros() as u64;
        TOTAL_PROCESSING_TIME.fetch_add(elapsed, Ordering::Relaxed);
        
        Ok(format!("Allocated {} MB, checksum: {}", self.mb_size, checksum))
    }
}

/// Async I/O simulation tool
#[derive(McpTool, Clone)]
#[tool(name = "async_io", description = "Simulates async I/O operations")]
struct AsyncIoTool {
    #[param(description = "Delay in milliseconds")]
    delay_ms: u64,
}

impl AsyncIoTool {
    async fn execute(&self) -> McpResult<String> {
        let start = Instant::now();
        REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        // Simulate async I/O with sleep
        sleep(Duration::from_millis(self.delay_ms)).await;
        
        let elapsed = start.elapsed().as_micros() as u64;
        TOTAL_PROCESSING_TIME.fetch_add(elapsed, Ordering::Relaxed);
        
        Ok(format!("Async I/O completed after {}ms", self.delay_ms))
    }
}

/// Session-aware counter tool
#[derive(Clone)]
struct SessionCounterTool {
    increment: i64,
}

#[async_trait]
impl mcp_server::McpTool for SessionCounterTool {
    fn name(&self) -> &str {
        "session_counter"
    }
    
    fn description(&self) -> &str {
        "Session-aware counter for session testing"
    }
    
    fn input_schema(&self) -> mcp_protocol::ToolSchema {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        mcp_protocol::ToolSchema::object()
            .with_properties(HashMap::from([
                ("increment".to_string(), JsonSchema::integer().with_description("Increment amount")),
            ]))
            .with_required(vec!["increment".to_string()])
    }
    
    async fn call(&self, args: serde_json::Value, session: Option<SessionContext>) -> McpResult<Vec<mcp_protocol::ToolResult>> {
        let increment = args["increment"].as_i64().ok_or_else(|| McpError::missing_param("increment"))?;
        let tool = SessionCounterTool { increment };
        let result = tool.execute(session).await?;
        Ok(vec![mcp_protocol::ToolResult::text(result)])
    }
}

impl SessionCounterTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let start = Instant::now();
        REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        if let Some(session) = session {
            let current: i64 = (session.get_state)("counter")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            
            let new_value = current + self.increment;
            (session.set_state)("counter", json!(new_value));
            
            let elapsed = start.elapsed().as_micros() as u64;
            TOTAL_PROCESSING_TIME.fetch_add(elapsed, Ordering::Relaxed);
            
            Ok(format!("Session counter: {} (incremented by {})", new_value, self.increment))
        } else {
            let elapsed = start.elapsed().as_micros() as u64;
            TOTAL_PROCESSING_TIME.fetch_add(elapsed, Ordering::Relaxed);
            
            Ok(format!("No session - standalone increment: {}", self.increment))
        }
    }
}

/// Performance statistics tool
#[derive(McpTool, Clone)]
#[tool(name = "perf_stats", description = "Returns current performance statistics")]
struct PerfStatsTool {
    #[param(description = "Dummy parameter to make it a struct with fields")]
    _dummy: Option<bool>,
}

impl PerfStatsTool {
    async fn execute(&self) -> McpResult<String> {
        let total_requests = REQUEST_COUNTER.load(Ordering::Relaxed);
        let total_time_micros = TOTAL_PROCESSING_TIME.load(Ordering::Relaxed);
        
        let avg_time_micros = if total_requests > 0 {
            total_time_micros / total_requests
        } else {
            0
        };
        
        let stats = json!({
            "total_requests": total_requests,
            "total_processing_time_ms": total_time_micros as f64 / 1000.0,
            "average_processing_time_micros": avg_time_micros,
            "requests_per_second": if avg_time_micros > 0 { 1_000_000.0 / (avg_time_micros as f64) } else { 0.0 }
        });
        
        Ok(stats.to_string())
    }
}

/// Error simulation tool for error handling testing
#[derive(McpTool, Clone)]
#[tool(name = "error_simulation", description = "Simulates various error conditions")]
struct ErrorSimulationTool {
    #[param(description = "Error type: timeout, panic, invalid, or success")]
    error_type: String,
}

impl ErrorSimulationTool {
    async fn execute(&self) -> McpResult<String> {
        REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        match self.error_type.as_str() {
            "timeout" => {
                sleep(Duration::from_secs(5)).await;
                Ok("Should have timed out".to_string())
            }
            "panic" => {
                panic!("Simulated panic for testing");
            }
            "invalid" => {
                Err(McpError::tool_execution("Simulated invalid input error"))
            }
            "success" => {
                Ok("Error simulation completed successfully".to_string())
            }
            _ => {
                Err(McpError::invalid_param_type("error_type", "timeout|panic|invalid|success", &self.error_type))
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting load test server...");

    // Build server with performance-optimized tools
    let server = McpServer::builder()
        .name("load-test-server")
        .version("1.0.0")
        .title("MCP Load Test Server")
        .instructions("High-performance MCP server for load testing and performance benchmarking")
        .tool(FastComputeTool { input: 0 })
        .tool(CpuIntensiveTool { size: 0 })
        .tool(MemoryAllocateTool { mb_size: 0 })
        .tool(AsyncIoTool { delay_ms: 0 })
        .tool(SessionCounterTool { increment: 1 })
        .tool(PerfStatsTool { _dummy: None })
        .tool(ErrorSimulationTool { error_type: String::new() })
        .with_notifications()
        .with_logging()
        .with_completion()
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()?;

    // Create a simple notification system (placeholder)
    // In a real implementation, you would integrate with the server's SSE system
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            
            let total_requests = REQUEST_COUNTER.load(Ordering::Relaxed);
            let total_time = TOTAL_PROCESSING_TIME.load(Ordering::Relaxed);
            let avg_time = if total_requests > 0 { total_time / total_requests } else { 0 };
            
            info!(
                "Performance stats - Requests: {}, Avg time: {}μs, RPS: {:.2}",
                total_requests,
                avg_time,
                if avg_time > 0 { 1_000_000.0 / (avg_time as f64) } else { 0.0 }
            );
            
            // Log performance stats (placeholder for SSE notification)
            info!(
                "Performance update - Total: {}, Avg: {}μs, Est RPS: {:.2}",
                total_requests,
                avg_time,
                if avg_time > 0 { 1_000_000.0 / (avg_time as f64) } else { 0.0 }
            );
        }
    });

    info!("Load test server listening on http://127.0.0.1:8080/mcp");
    info!("Available tools for performance testing:");
    info!("  - fast_compute: Minimal overhead computation");
    info!("  - cpu_intensive: CPU stress testing");
    info!("  - memory_allocate: Memory allocation testing");
    info!("  - async_io: Async I/O simulation");
    info!("  - session_counter: Session-aware operations");
    info!("  - perf_stats: Performance statistics");
    info!("  - error_simulation: Error condition testing");

    // Run the server
    server.run().await?;

    Ok(())
}