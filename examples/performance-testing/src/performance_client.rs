//! Performance Client - Load Testing Client for MCP Server
//!
//! This client generates various load patterns to test MCP server performance,
//! including concurrent requests, burst patterns, and sustained load.

use clap::{Parser, Subcommand};
use futures::future::join_all;
use rand::RngExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};
use turul_mcp_client::{McpClient, McpClientBuilder};

#[derive(Parser)]
#[command(name = "performance_client")]
#[command(about = "Performance testing client for MCP servers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "http://127.0.0.1:8080/mcp")]
    server_url: String,

    #[arg(long, default_value = "10")]
    concurrency: usize,

    #[arg(long, default_value = "60")]
    duration_seconds: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Throughput test with fast operations
    Throughput {
        #[arg(long, default_value = "1000")]
        requests_per_second: u64,
    },
    /// Stress test with CPU-intensive operations
    Stress {
        #[arg(long, default_value = "100")]
        computation_size: u32,
    },
    /// Memory test with memory allocation
    Memory {
        #[arg(long, default_value = "10")]
        mb_per_request: u32,
    },
    /// Latency test with async I/O
    Latency {
        #[arg(long, default_value = "100")]
        io_delay_ms: u64,
    },
    /// Session test with session-aware operations
    Session {
        #[arg(long, default_value = "100")]
        sessions: usize,
    },
    /// Mixed workload test
    Mixed,
    /// Burst test with sudden load spikes
    Burst {
        #[arg(long, default_value = "5")]
        burst_interval_seconds: u64,

        #[arg(long, default_value = "100")]
        burst_size: usize,
    },
}

/// Performance metrics collector
#[derive(Default)]
struct Metrics {
    requests_sent: AtomicU64,
    requests_completed: AtomicU64,
    requests_failed: AtomicU64,
    total_response_time_ms: AtomicU64,
    min_response_time_ms: AtomicU64,
    max_response_time_ms: AtomicU64,
}

impl Metrics {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            min_response_time_ms: AtomicU64::new(u64::MAX),
            ..Default::default()
        })
    }

    fn record_request(&self, response_time_ms: u64, success: bool) {
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        if success {
            self.total_response_time_ms
                .fetch_add(response_time_ms, Ordering::Relaxed);

            // Update min/max response times
            let mut current_min = self.min_response_time_ms.load(Ordering::Relaxed);
            while response_time_ms < current_min {
                match self.min_response_time_ms.compare_exchange_weak(
                    current_min,
                    response_time_ms,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => current_min = x,
                }
            }

            let mut current_max = self.max_response_time_ms.load(Ordering::Relaxed);
            while response_time_ms > current_max {
                match self.max_response_time_ms.compare_exchange_weak(
                    current_max,
                    response_time_ms,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => current_max = x,
                }
            }
        } else {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn print_stats(&self) {
        let sent = self.requests_sent.load(Ordering::Relaxed);
        let completed = self.requests_completed.load(Ordering::Relaxed);
        let failed = self.requests_failed.load(Ordering::Relaxed);
        let total_time = self.total_response_time_ms.load(Ordering::Relaxed);
        let min_time = self.min_response_time_ms.load(Ordering::Relaxed);
        let max_time = self.max_response_time_ms.load(Ordering::Relaxed);

        let successful = completed - failed;
        let avg_time = if successful > 0 {
            total_time / successful
        } else {
            0
        };

        info!("=== Performance Test Results ===");
        info!("Requests sent: {}", sent);
        info!(
            "Requests completed: {} ({:.1}%)",
            completed,
            (completed as f64 / sent as f64) * 100.0
        );
        info!(
            "Requests failed: {} ({:.1}%)",
            failed,
            (failed as f64 / sent as f64) * 100.0
        );
        info!("Average response time: {} ms", avg_time);
        info!(
            "Min response time: {} ms",
            if min_time == u64::MAX { 0 } else { min_time }
        );
        info!("Max response time: {} ms", max_time);
        info!(
            "Requests per second: {:.2}",
            completed as f64 / (total_time as f64 / 1000.0)
        );
    }
}

async fn send_mcp_tool_call(
    client: &McpClient,
    tool_name: &str,
    arguments: Value,
) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();

    let _results = client.call_tool(tool_name, arguments).await?;

    let elapsed = start.elapsed();
    Ok(elapsed)
}

async fn throughput_test(
    client: Arc<McpClient>,
    rps: u64,
    duration: Duration,
    concurrency: usize,
    metrics: Arc<Metrics>,
) {
    info!(
        "Starting throughput test: {} RPS for {} seconds",
        rps,
        duration.as_secs()
    );

    let interval = Duration::from_nanos(1_000_000_000 / rps);
    let end_time = Instant::now() + duration;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let arguments = json!({
                "input": rand::rng().random::<i64>()
            });

            match send_mcp_tool_call(&client, "fast_compute", arguments).await {
                Ok(duration) => {
                    metrics.record_request(duration.as_millis() as u64, true);
                }
                Err(e) => {
                    warn!("Request failed: {}", e);
                    metrics.record_request(0, false);
                }
            }
        });

        sleep(interval).await;
    }

    // Wait for remaining requests to complete
    let _all_permits = semaphore.acquire_many(concurrency as u32).await.unwrap();
}

async fn stress_test(
    client: Arc<McpClient>,
    computation_size: u32,
    duration: Duration,
    concurrency: usize,
    metrics: Arc<Metrics>,
) {
    info!(
        "Starting stress test: computation size {} for {} seconds",
        computation_size,
        duration.as_secs()
    );

    let end_time = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let arguments = json!({
                "size": computation_size
            });

            match send_mcp_tool_call(&client, "cpu_intensive", arguments).await {
                Ok(duration) => {
                    metrics.record_request(duration.as_millis() as u64, true);
                }
                Err(e) => {
                    warn!("Request failed: {}", e);
                    metrics.record_request(0, false);
                }
            }
        });

        sleep(Duration::from_millis(100)).await; // Throttle stress test
    }

    let _all_permits = semaphore.acquire_many(concurrency as u32).await.unwrap();
}

async fn memory_test(
    client: Arc<McpClient>,
    mb_per_request: u32,
    duration: Duration,
    concurrency: usize,
    metrics: Arc<Metrics>,
) {
    info!(
        "Starting memory test: {} MB per request for {} seconds",
        mb_per_request,
        duration.as_secs()
    );

    let end_time = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let arguments = json!({
                "mb_size": mb_per_request
            });

            match send_mcp_tool_call(&client, "memory_allocate", arguments).await {
                Ok(duration) => {
                    metrics.record_request(duration.as_millis() as u64, true);
                }
                Err(e) => {
                    warn!("Request failed: {}", e);
                    metrics.record_request(0, false);
                }
            }
        });

        sleep(Duration::from_secs(2)).await; // Throttle memory test
    }

    let _all_permits = semaphore.acquire_many(concurrency as u32).await.unwrap();
}

async fn latency_test(
    client: Arc<McpClient>,
    io_delay_ms: u64,
    duration: Duration,
    concurrency: usize,
    metrics: Arc<Metrics>,
) {
    info!(
        "Starting latency test: {} ms I/O delay for {} seconds",
        io_delay_ms,
        duration.as_secs()
    );

    let end_time = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let arguments = json!({
                "delay_ms": io_delay_ms
            });

            match send_mcp_tool_call(&client, "async_io", arguments).await {
                Ok(duration) => {
                    metrics.record_request(duration.as_millis() as u64, true);
                }
                Err(e) => {
                    warn!("Request failed: {}", e);
                    metrics.record_request(0, false);
                }
            }
        });

        sleep(Duration::from_millis(500)).await;
    }

    let _all_permits = semaphore.acquire_many(concurrency as u32).await.unwrap();
}

async fn burst_test(
    client: Arc<McpClient>,
    burst_interval: Duration,
    burst_size: usize,
    duration: Duration,
    metrics: Arc<Metrics>,
) {
    info!(
        "Starting burst test: {} requests every {} seconds",
        burst_size,
        burst_interval.as_secs()
    );

    let end_time = Instant::now() + duration;

    while Instant::now() < end_time {
        info!("Sending burst of {} requests...", burst_size);

        let mut handles = Vec::new();
        for _ in 0..burst_size {
            let client = client.clone();
            let metrics = metrics.clone();

            let handle = tokio::spawn(async move {
                metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

                let arguments = json!({
                    "input": rand::rng().random::<i64>()
                });

                match send_mcp_tool_call(&client, "fast_compute", arguments).await {
                    Ok(duration) => {
                        metrics.record_request(duration.as_millis() as u64, true);
                    }
                    Err(e) => {
                        warn!("Request failed: {}", e);
                        metrics.record_request(0, false);
                    }
                }
            });
            handles.push(handle);
        }

        join_all(handles).await;
        sleep(burst_interval).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();

    // Create MCP client and connect
    let client = McpClientBuilder::new().with_url(&cli.server_url)?.build();

    client.connect().await?;
    let client = Arc::new(client);

    let metrics = Metrics::new();
    let duration = Duration::from_secs(cli.duration_seconds);

    // Start metrics reporting
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let sent = metrics_clone.requests_sent.load(Ordering::Relaxed);
            let completed = metrics_clone.requests_completed.load(Ordering::Relaxed);
            let failed = metrics_clone.requests_failed.load(Ordering::Relaxed);

            if sent > 0 {
                info!(
                    "Progress: {} sent, {} completed, {} failed",
                    sent, completed, failed
                );
            }
        }
    });

    match cli.command {
        Commands::Throughput {
            requests_per_second,
        } => {
            throughput_test(
                client.clone(),
                requests_per_second,
                duration,
                cli.concurrency,
                metrics.clone(),
            )
            .await;
        }
        Commands::Stress { computation_size } => {
            stress_test(
                client.clone(),
                computation_size,
                duration,
                cli.concurrency,
                metrics.clone(),
            )
            .await;
        }
        Commands::Memory { mb_per_request } => {
            memory_test(
                client.clone(),
                mb_per_request,
                duration,
                cli.concurrency,
                metrics.clone(),
            )
            .await;
        }
        Commands::Latency { io_delay_ms } => {
            latency_test(
                client.clone(),
                io_delay_ms,
                duration,
                cli.concurrency,
                metrics.clone(),
            )
            .await;
        }
        Commands::Session { sessions: _ } => {
            // TODO: Implement session test using MCP client session awareness
            info!("Session test not yet implemented");
        }
        Commands::Mixed => {
            // TODO: Implement mixed workload test
            info!("Mixed workload test not yet implemented");
        }
        Commands::Burst {
            burst_interval_seconds,
            burst_size,
        } => {
            burst_test(
                client.clone(),
                Duration::from_secs(burst_interval_seconds),
                burst_size,
                duration,
                metrics.clone(),
            )
            .await;
        }
    }

    // Final statistics
    sleep(Duration::from_secs(2)).await; // Allow final requests to complete
    metrics.print_stats();

    Ok(())
}
