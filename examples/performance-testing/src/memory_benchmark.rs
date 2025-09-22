//! Memory Benchmark - Memory usage analysis and optimization testing
//!
//! This module provides detailed memory usage analysis for the MCP server,
//! including memory leak detection, allocation pattern analysis, and optimization recommendations.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use turul_mcp_client::{McpClient, McpClientBuilder};
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing::{info, warn};
use std::sync::Arc;

/// Custom allocator that tracks memory usage
struct TrackingAllocator {
    allocated: AtomicUsize,
    deallocated: AtomicUsize,
    peak_usage: AtomicUsize,
    allocation_count: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            allocated: AtomicUsize::new(0),
            deallocated: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
        }
    }
    
    fn current_usage(&self) -> usize {
        self.allocated.load(Ordering::Relaxed) - self.deallocated.load(Ordering::Relaxed)
    }
    
    fn peak_usage(&self) -> usize {
        self.peak_usage.load(Ordering::Relaxed)
    }
    
    fn total_allocated(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }
    
    fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Relaxed)
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let size = layout.size();
            self.allocated.fetch_add(size, Ordering::Relaxed);
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
            
            // Update peak usage
            let current = self.current_usage();
            let mut peak = self.peak_usage.load(Ordering::Relaxed);
            while current > peak {
                match self.peak_usage.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(x) => peak = x,
                }
            }
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        self.deallocated.fetch_add(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

#[derive(Parser)]
#[command(name = "memory_benchmark")]
#[command(about = "Memory usage analysis and benchmarking for MCP servers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "http://127.0.0.1:8080/mcp")]
    server_url: String,
    
    #[arg(long, default_value = "300")]
    duration_seconds: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Baseline memory usage measurement
    Baseline {
        #[arg(long, default_value = "100")]
        request_count: usize,
    },
    /// Memory leak detection
    LeakDetection {
        #[arg(long, default_value = "10000")]
        iterations: usize,
    },
    /// Memory growth analysis
    GrowthAnalysis {
        #[arg(long, default_value = "60")]
        measurement_interval_seconds: u64,
    },
    /// Memory allocation pattern analysis
    AllocationPatterns {
        #[arg(long, default_value = "1000")]
        operations: usize,
    },
    /// Memory optimization recommendations
    Optimization {
        #[arg(long, default_value = "500")]
        sample_size: usize,
    },
}

/// Memory usage snapshot
#[derive(Debug, Clone)]
struct MemorySnapshot {
    timestamp: Instant,
    current_usage_bytes: usize,
    peak_usage_bytes: usize,
    total_allocated_bytes: usize,
    allocation_count: usize,
    rss_bytes: Option<usize>, // Resident Set Size from system
}

impl MemorySnapshot {
    fn new() -> Self {
        Self {
            timestamp: Instant::now(),
            current_usage_bytes: ALLOCATOR.current_usage(),
            peak_usage_bytes: ALLOCATOR.peak_usage(),
            total_allocated_bytes: ALLOCATOR.total_allocated(),
            allocation_count: ALLOCATOR.allocation_count(),
            rss_bytes: get_rss_memory(),
        }
    }
    
    fn format_bytes(bytes: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        format!("{:.2} {}", size, UNITS[unit_index])
    }
    
    fn print(&self, label: &str) {
        info!("=== Memory Snapshot: {} ===", label);
        info!("Current usage: {}", Self::format_bytes(self.current_usage_bytes));
        info!("Peak usage: {}", Self::format_bytes(self.peak_usage_bytes));
        info!("Total allocated: {}", Self::format_bytes(self.total_allocated_bytes));
        info!("Allocation count: {}", self.allocation_count);
        if let Some(rss) = self.rss_bytes {
            info!("RSS (system): {}", Self::format_bytes(rss));
        }
    }
}

/// Get RSS memory usage from the system (Linux/macOS)
fn get_rss_memory() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(contents) = fs::read_to_string("/proc/self/status") {
            for line in contents.lines() {
                if line.starts_with("VmRSS:")
                    && let Some(kb_str) = line.split_whitespace().nth(1)
                        && let Ok(kb) = kb_str.parse::<usize>() {
                            return Some(kb * 1024); // Convert KB to bytes
                        }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ps")
            .args(&["-o", "rss=", "-p"])
            .arg(std::process::id().to_string())
            .output()
        {
            if let Ok(rss_str) = String::from_utf8(output.stdout) {
                if let Ok(kb) = rss_str.trim().parse::<usize>() {
                    return Some(kb * 1024); // Convert KB to bytes
                }
            }
        }
    }
    
    None
}

async fn send_memory_request(
    client: &McpClient,
    tool_name: &str,
    params: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let results = client
        .call_tool(tool_name, params)
        .await?;
    
    // Convert tool results to simple value for compatibility
    Ok(serde_json::to_value(results)?)
}

async fn baseline_memory_test(
    client: &McpClient,
    request_count: usize,
) -> anyhow::Result<()> {
    info!("Starting baseline memory measurement with {} requests", request_count);
    
    let initial = MemorySnapshot::new();
    initial.print("Initial");
    
    // Warm up
    for _ in 0..10 {
        let params = json!({"input": 42});
        let _ = send_memory_request(client, "fast_compute", params).await;
    }
    
    let warmup = MemorySnapshot::new();
    warmup.print("After Warmup");
    
    // Main test
    for i in 0..request_count {
        let params = json!({"input": i as i64});
        match send_memory_request(client, "fast_compute", params).await {
            Ok(_) => {},
            Err(e) => warn!("Request {} failed: {}", i, e),
        }
        
        if i % 100 == 0 {
            let progress = MemorySnapshot::new();
            info!("Progress {}/{}: Current memory {}", 
                i, request_count, 
                MemorySnapshot::format_bytes(progress.current_usage_bytes)
            );
        }
    }
    
    let final_snapshot = MemorySnapshot::new();
    final_snapshot.print("Final");
    
    // Analysis
    let memory_per_request = (final_snapshot.current_usage_bytes as i64 - warmup.current_usage_bytes as i64) / request_count as i64;
    info!("=== Baseline Analysis ===");
    info!("Memory per request: {} bytes", memory_per_request);
    info!("Memory efficiency: {:.2} requests per MB", 
        1024.0 * 1024.0 / memory_per_request.abs() as f64
    );
    
    Ok(())
}

async fn leak_detection_test(
    client: &McpClient,
    iterations: usize,
) -> anyhow::Result<()> {
    info!("Starting memory leak detection with {} iterations", iterations);
    
    let mut snapshots = Vec::new();
    let checkpoint_interval = iterations / 10;
    
    for i in 0..iterations {
        // Perform various operations that might leak memory
        let operations = [
            ("fast_compute", json!({"input": i as i64})),
            ("cpu_intensive", json!({"size": 10})),
            ("memory_allocate", json!({"mb_size": 1})),
            ("session_counter", json!({"increment": 1})),
        ];
        
        for (tool_name, params) in &operations {
            match send_memory_request(client, tool_name, params.clone()).await {
                Ok(_) => {},
                Err(e) => warn!("Operation {} failed: {}", tool_name, e),
            }
        }
        
        if i % checkpoint_interval == 0 {
            let snapshot = MemorySnapshot::new();
            snapshots.push(snapshot.clone());
            info!("Checkpoint {}/{}: {}", 
                i, iterations, 
                MemorySnapshot::format_bytes(snapshot.current_usage_bytes)
            );
        }
    }
    
    // Force garbage collection if possible
    for _ in 0..5 {
        tokio::task::yield_now().await;
        sleep(Duration::from_millis(100)).await;
    }
    
    let final_snapshot = MemorySnapshot::new();
    snapshots.push(final_snapshot);
    
    // Analyze for leaks
    info!("=== Leak Detection Analysis ===");
    
    if snapshots.len() >= 3 {
        let first = &snapshots[1]; // Skip initial baseline
        let last = &snapshots[snapshots.len() - 1];
        
        let growth = last.current_usage_bytes as i64 - first.current_usage_bytes as i64;
        let growth_rate = growth as f64 / (iterations as f64 - checkpoint_interval as f64);
        
        info!("Memory growth: {} over {} iterations", 
            MemorySnapshot::format_bytes(growth.unsigned_abs() as usize), 
            iterations - checkpoint_interval
        );
        info!("Growth rate: {:.2} bytes per operation", growth_rate);
        
        if growth_rate > 100.0 {
            warn!("⚠️  POTENTIAL MEMORY LEAK DETECTED - High growth rate");
        } else if growth_rate > 10.0 {
            warn!("⚠️  MODERATE MEMORY GROWTH - Monitor for leaks");
        } else {
            info!("✅ NO SIGNIFICANT MEMORY LEAKS DETECTED");
        }
        
        // Linear regression to detect trends
        let n = snapshots.len() as f64;
        let sum_x: f64 = (0..snapshots.len()).map(|i| i as f64).sum();
        let sum_y: f64 = snapshots.iter().map(|s| s.current_usage_bytes as f64).sum();
        let sum_xy: f64 = snapshots.iter().enumerate()
            .map(|(i, s)| i as f64 * s.current_usage_bytes as f64).sum();
        let sum_x2: f64 = (0..snapshots.len()).map(|i| (i as f64).powi(2)).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let correlation_strength = slope.abs() / (sum_y / n);
        
        info!("Linear trend slope: {:.2} bytes per checkpoint", slope);
        info!("Trend strength: {:.4}", correlation_strength);
    }
    
    Ok(())
}

async fn growth_analysis_test(
    client: Arc<McpClient>,
    measurement_interval: Duration,
    total_duration: Duration,
) -> anyhow::Result<()> {
    info!("Starting memory growth analysis over {} seconds", total_duration.as_secs());
    
    let mut snapshots = Vec::new();
    let start_time = Instant::now();
    let mut next_measurement = start_time + measurement_interval;
    
    // Background load generator
    let load_handle = {
        let client = client.clone();
        tokio::spawn(async move {
            let mut counter = 0;
            loop {
                let tools = ["fast_compute", "cpu_intensive", "memory_allocate"];
                for tool in &tools {
                    let params = match *tool {
                        "fast_compute" => json!({"input": counter}),
                        "cpu_intensive" => json!({"size": 50}),
                        "memory_allocate" => json!({"mb_size": 5}),
                        _ => json!({}),
                    };
                    
                    let _ = send_memory_request(&client, tool, params).await;
                    counter += 1;
                    
                    if start_time.elapsed() > total_duration {
                        return;
                    }
                }
                
                sleep(Duration::from_millis(10)).await;
            }
        })
    };
    
    // Memory measurement loop
    while start_time.elapsed() < total_duration {
        if Instant::now() >= next_measurement {
            let snapshot = MemorySnapshot::new();
            snapshots.push(snapshot.clone());
            
            info!("Memory at {:?}: {}", 
                start_time.elapsed(),
                MemorySnapshot::format_bytes(snapshot.current_usage_bytes)
            );
            
            next_measurement += measurement_interval;
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    // Stop background load
    load_handle.abort();
    let _ = load_handle.await;
    
    // Final measurement after cooldown
    sleep(Duration::from_secs(2)).await;
    let final_snapshot = MemorySnapshot::new();
    snapshots.push(final_snapshot);
    
    // Analysis
    info!("=== Memory Growth Analysis ===");
    
    if snapshots.len() >= 2 {
        let first = &snapshots[0];
        let last = &snapshots[snapshots.len() - 1];
        
        let total_growth = last.current_usage_bytes as i64 - first.current_usage_bytes as i64;
        let time_span = last.timestamp.duration_since(first.timestamp);
        let growth_rate_per_minute = (total_growth as f64) / time_span.as_secs_f64() * 60.0;
        
        info!("Total memory growth: {}", MemorySnapshot::format_bytes(total_growth.unsigned_abs() as usize));
        info!("Growth rate: {:.2} bytes per minute", growth_rate_per_minute);
        
        // Find peak usage
        let peak_snapshot = snapshots.iter().max_by_key(|s| s.current_usage_bytes).unwrap();
        info!("Peak usage: {} at {:?}", 
            MemorySnapshot::format_bytes(peak_snapshot.current_usage_bytes),
            peak_snapshot.timestamp.duration_since(start_time)
        );
        
        // Memory stability analysis
        let variance: f64 = snapshots.iter()
            .map(|s| s.current_usage_bytes as f64)
            .map(|x| (x - (last.current_usage_bytes as f64)).powi(2))
            .sum::<f64>() / snapshots.len() as f64;
        
        let std_dev = variance.sqrt();
        let stability_score = 100.0 - (std_dev / (last.current_usage_bytes as f64) * 100.0);
        
        info!("Memory stability score: {:.1}%", stability_score.max(0.0));
        
        if stability_score >= 90.0 {
            info!("✅ EXCELLENT - Very stable memory usage");
        } else if stability_score >= 80.0 {
            info!("✅ GOOD - Reasonably stable memory usage");
        } else if stability_score >= 70.0 {
            warn!("⚠️  MODERATE - Some memory fluctuation");
        } else {
            warn!("❌ POOR - Unstable memory usage pattern");
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();
    
    // Create MCP client and connect
    let client = McpClientBuilder::new()
        .with_url(&cli.server_url)?
        .build();
    
    client.connect().await?;
    let client = Arc::new(client);

    info!("Starting memory benchmark against: {}", cli.server_url);
    
    // Initial system check
    let initial = MemorySnapshot::new();
    info!("=== Initial System State ===");
    initial.print("System Startup");

    match cli.command {
        Commands::Baseline { request_count } => {
            baseline_memory_test(&client, request_count).await?;
        }
        Commands::LeakDetection { iterations } => {
            leak_detection_test(&client, iterations).await?;
        }
        Commands::GrowthAnalysis { measurement_interval_seconds } => {
            let measurement_interval = Duration::from_secs(measurement_interval_seconds);
            let total_duration = Duration::from_secs(cli.duration_seconds);
            growth_analysis_test(client.clone(), measurement_interval, total_duration).await?;
        }
        Commands::AllocationPatterns { operations: _ } => {
            info!("Allocation pattern analysis not yet implemented");
        }
        Commands::Optimization { sample_size: _ } => {
            info!("Memory optimization recommendations not yet implemented");
        }
    }

    // Final system state
    let final_state = MemorySnapshot::new();
    info!("=== Final System State ===");
    final_state.print("Test Completion");

    Ok(())
}