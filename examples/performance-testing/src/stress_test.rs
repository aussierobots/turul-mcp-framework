//! Stress Test - Comprehensive stress testing for MCP server
//!
//! This module provides comprehensive stress testing scenarios including
//! resource exhaustion, error recovery, and system stability under extreme load.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use turul_mcp_client::{McpClient, McpClientBuilder};
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing::{info, warn};
use rand::Rng;

#[derive(Parser)]
#[command(name = "stress_test")]
#[command(about = "Comprehensive stress testing for MCP servers")]
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
    /// Memory exhaustion test
    Memory {
        #[arg(long, default_value = "100")]
        max_concurrent: usize,

        #[arg(long, default_value = "50")]
        mb_per_request: u32,
    },
    /// CPU exhaustion test
    Cpu {
        #[arg(long, default_value = "200")]
        max_concurrent: usize,

        #[arg(long, default_value = "1000")]
        computation_size: u32,
    },
    /// Connection flooding test
    Flood {
        #[arg(long, default_value = "1000")]
        connections_per_second: u64,

        #[arg(long, default_value = "5000")]
        max_connections: usize,
    },
    /// Error recovery test
    Recovery {
        #[arg(long, default_value = "50")]
        error_rate_percent: u32,
    },
    /// Chaos test with mixed stressors
    Chaos {
        #[arg(long, default_value = "100")]
        max_concurrent: usize,
    },
    /// Session exhaustion test
    Sessions {
        #[arg(long, default_value = "10000")]
        max_sessions: usize,

        #[arg(long, default_value = "100")]
        operations_per_session: usize,
    },
}

/// Comprehensive stress test metrics
#[derive(Default)]
struct StressMetrics {
    // Request metrics
    requests_sent: AtomicU64,
    requests_completed: AtomicU64,
    requests_failed: AtomicU64,
    requests_timeout: AtomicU64,

    // Performance metrics
    total_response_time_ms: AtomicU64,
    min_response_time_ms: AtomicU64,
    max_response_time_ms: AtomicU64,

    // Resource metrics
    #[allow(dead_code)] // TODO: Implement memory usage tracking
    peak_memory_mb: AtomicU64,
    #[allow(dead_code)] // TODO: Implement connection pool tracking
    peak_connections: AtomicU64,

    // Error metrics
    connection_errors: AtomicU64,
    server_errors: AtomicU64,
    client_errors: AtomicU64,
}

impl StressMetrics {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            min_response_time_ms: AtomicU64::new(u64::MAX),
            ..Default::default()
        })
    }

    fn record_request(&self, response_time_ms: u64, success: bool, error_type: Option<&str>) {
        self.requests_completed.fetch_add(1, Ordering::Relaxed);

        if success {
            self.total_response_time_ms.fetch_add(response_time_ms, Ordering::Relaxed);

            // Update min/max response times
            let mut current_min = self.min_response_time_ms.load(Ordering::Relaxed);
            while response_time_ms < current_min {
                match self.min_response_time_ms.compare_exchange_weak(
                    current_min, response_time_ms, Ordering::Relaxed, Ordering::Relaxed
                ) {
                    Ok(_) => break,
                    Err(x) => current_min = x,
                }
            }

            let mut current_max = self.max_response_time_ms.load(Ordering::Relaxed);
            while response_time_ms > current_max {
                match self.max_response_time_ms.compare_exchange_weak(
                    current_max, response_time_ms, Ordering::Relaxed, Ordering::Relaxed
                ) {
                    Ok(_) => break,
                    Err(x) => current_max = x,
                }
            }
        } else {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);

            match error_type {
                Some("timeout") => { self.requests_timeout.fetch_add(1, Ordering::Relaxed); },
                Some("connection") => { self.connection_errors.fetch_add(1, Ordering::Relaxed); },
                Some("server") => { self.server_errors.fetch_add(1, Ordering::Relaxed); },
                Some("client") => { self.client_errors.fetch_add(1, Ordering::Relaxed); },
                _ => {}
            };
        }
    }

    fn print_comprehensive_stats(&self) {
        let sent = self.requests_sent.load(Ordering::Relaxed);
        let completed = self.requests_completed.load(Ordering::Relaxed);
        let failed = self.requests_failed.load(Ordering::Relaxed);
        let timeout = self.requests_timeout.load(Ordering::Relaxed);
        let total_time = self.total_response_time_ms.load(Ordering::Relaxed);
        let min_time = self.min_response_time_ms.load(Ordering::Relaxed);
        let max_time = self.max_response_time_ms.load(Ordering::Relaxed);

        let connection_errors = self.connection_errors.load(Ordering::Relaxed);
        let server_errors = self.server_errors.load(Ordering::Relaxed);
        let client_errors = self.client_errors.load(Ordering::Relaxed);

        let successful = completed - failed;
        let avg_time = if successful > 0 { total_time / successful } else { 0 };

        info!("=== Comprehensive Stress Test Results ===");
        info!("Request Statistics:");
        info!("  Requests sent: {}", sent);
        info!("  Requests completed: {} ({:.1}%)", completed, (completed as f64 / sent as f64) * 100.0);
        info!("  Requests successful: {} ({:.1}%)", successful, (successful as f64 / sent as f64) * 100.0);
        info!("  Requests failed: {} ({:.1}%)", failed, (failed as f64 / sent as f64) * 100.0);
        info!("  Requests timed out: {} ({:.1}%)", timeout, (timeout as f64 / sent as f64) * 100.0);

        info!("Performance Statistics:");
        info!("  Average response time: {} ms", avg_time);
        info!("  Min response time: {} ms", if min_time == u64::MAX { 0 } else { min_time });
        info!("  Max response time: {} ms", max_time);
        if successful > 0 {
            info!("  Requests per second: {:.2}", successful as f64 / (total_time as f64 / 1000.0));
        }

        info!("Error Breakdown:");
        info!("  Connection errors: {}", connection_errors);
        info!("  Server errors (5xx): {}", server_errors);
        info!("  Client errors (4xx): {}", client_errors);

        // Calculate reliability score
        let reliability = if sent > 0 {
            (successful as f64 / sent as f64) * 100.0
        } else {
            0.0
        };

        info!("Overall Reliability Score: {:.2}%", reliability);

        if reliability >= 99.0 {
            info!("✅ EXCELLENT - Server handled stress very well");
        } else if reliability >= 95.0 {
            info!("✅ GOOD - Server handled stress adequately");
        } else if reliability >= 90.0 {
            info!("⚠️  MODERATE - Server struggled under stress");
        } else {
            info!("❌ POOR - Server failed to handle stress");
        }
    }
}

async fn send_stress_request(
    client: &Client,
    url: &str,
    tool_name: &str,
    params: Value,
    timeout: Duration,
) -> Result<(Value, Duration), Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": params
        },
        "id": Uuid::new_v4().to_string()
    });

    let response = tokio::time::timeout(
        timeout,
        client.post(url).json(&request_body).send()
    ).await??;

    let elapsed = start.elapsed();
    let response_json: Value = response.json().await?;

    Ok((response_json, elapsed))
}

async fn memory_exhaustion_test(
    client: &Client,
    url: &str,
    max_concurrent: usize,
    mb_per_request: u32,
    duration: Duration,
    metrics: Arc<StressMetrics>,
) {
    info!("Starting memory exhaustion test: {} concurrent, {} MB per request", max_concurrent, mb_per_request);

    let end_time = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = url.to_string();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let params = json!({
                "mb_size": mb_per_request
            });

            match send_stress_request(&client, &url, "memory_allocate", params, Duration::from_secs(30)).await {
                Ok((_, duration)) => {
                    metrics.record_request(duration.as_millis() as u64, true, None);
                }
                Err(e) => {
                    let error_type = if e.to_string().contains("timeout") {
                        "timeout"
                    } else if e.to_string().contains("connection") {
                        "connection"
                    } else {
                        "server"
                    };
                    warn!("Memory test request failed: {}", e);
                    metrics.record_request(0, false, Some(error_type));
                }
            }
        });

        sleep(Duration::from_millis(10)).await; // Small delay to prevent overwhelming
    }

    // Wait for remaining requests
    let _all_permits = semaphore.acquire_many(max_concurrent as u32).await.unwrap();
}

async fn cpu_exhaustion_test(
    client: &Client,
    url: &str,
    max_concurrent: usize,
    computation_size: u32,
    duration: Duration,
    metrics: Arc<StressMetrics>,
) {
    info!("Starting CPU exhaustion test: {} concurrent, computation size {}", max_concurrent, computation_size);

    let end_time = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = url.to_string();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let params = json!({
                "size": computation_size
            });

            match send_stress_request(&client, &url, "cpu_intensive", params, Duration::from_secs(60)).await {
                Ok((_, duration)) => {
                    metrics.record_request(duration.as_millis() as u64, true, None);
                }
                Err(e) => {
                    let error_type = if e.to_string().contains("timeout") {
                        "timeout"
                    } else if e.to_string().contains("connection") {
                        "connection"
                    } else {
                        "server"
                    };
                    warn!("CPU test request failed: {}", e);
                    metrics.record_request(0, false, Some(error_type));
                }
            }
        });

        sleep(Duration::from_millis(5)).await; // Very short delay for CPU stress
    }

    let _all_permits = semaphore.acquire_many(max_concurrent as u32).await.unwrap();
}

async fn connection_flood_test(
    url: &str,
    connections_per_second: u64,
    max_connections: usize,
    duration: Duration,
    metrics: Arc<StressMetrics>,
) {
    info!("Starting connection flood test: {} connections/sec, max {} connections", connections_per_second, max_connections);

    let interval = Duration::from_nanos(1_000_000_000 / connections_per_second);
    let end_time = Instant::now() + duration;
    let active_connections = Arc::new(tokio::sync::Semaphore::new(max_connections));

    while Instant::now() < end_time {
        let permit = match active_connections.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                // Max connections reached, wait a bit
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        let url = url.to_string();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            let client = Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();

            let params = json!({
                "input": rand::rng().random::<i64>()
            });

            match send_stress_request(&client, &url, "fast_compute", params, Duration::from_secs(5)).await {
                Ok((_, duration)) => {
                    metrics.record_request(duration.as_millis() as u64, true, None);
                }
                Err(e) => {
                    let error_type = if e.to_string().contains("timeout") {
                        "timeout"
                    } else if e.to_string().contains("connection") {
                        "connection"
                    } else {
                        "client"
                    };
                    metrics.record_request(0, false, Some(error_type));
                }
            }
        });

        sleep(interval).await;
    }
}

async fn error_recovery_test(
    client: &Client,
    url: &str,
    error_rate_percent: u32,
    duration: Duration,
    metrics: Arc<StressMetrics>,
) {
    info!("Starting error recovery test: {}% error rate", error_rate_percent);

    let end_time = Instant::now() + duration;
    while Instant::now() < end_time {
        metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

        let should_error = rand::rng().random_range(0..100) < error_rate_percent;

        let (tool_name, params) = if should_error {
            ("error_simulation", json!({
                "error_type": match rand::rng().random_range(0..4) {
                    0 => "timeout",
                    1 => "panic",
                    2 => "invalid",
                    _ => "success"
                }
            }))
        } else {
            ("fast_compute", json!({
                "input": rand::rng().random::<i64>()
            }))
        };

        let client = client.clone();
        let url = url.to_string();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            match send_stress_request(&client, &url, tool_name, params, Duration::from_secs(10)).await {
                Ok((response, duration)) => {
                    let success = !response.get("error").is_some();
                    metrics.record_request(duration.as_millis() as u64, success, if success { None } else { Some("server") });
                }
                Err(e) => {
                    let error_type = if e.to_string().contains("timeout") {
                        "timeout"
                    } else if e.to_string().contains("connection") {
                        "connection"
                    } else {
                        "server"
                    };
                    metrics.record_request(0, false, Some(error_type));
                }
            }
        });

        sleep(Duration::from_millis(50)).await;
    }
}

async fn chaos_test(
    client: &Client,
    url: &str,
    max_concurrent: usize,
    duration: Duration,
    metrics: Arc<StressMetrics>,
) {
    info!("Starting chaos test with mixed stressors: {} max concurrent", max_concurrent);

    let end_time = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let _rng = rand::rng();

    while Instant::now() < end_time {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = url.to_string();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let _permit = permit;
            metrics.requests_sent.fetch_add(1, Ordering::Relaxed);

            // Randomly choose a stress scenario
            let (tool_name, params, timeout) = match rand::rng().random_range(0..5) {
                0 => ("fast_compute", json!({"input": rand::rng().random::<i64>()}), Duration::from_secs(5)),
                1 => ("cpu_intensive", json!({"size": rand::rng().random_range(100..2000)}), Duration::from_secs(30)),
                2 => ("memory_allocate", json!({"mb_size": rand::rng().random_range(1..100)}), Duration::from_secs(20)),
                3 => ("async_io", json!({"delay_ms": rand::rng().random_range(10..1000)}), Duration::from_secs(15)),
                _ => ("error_simulation", json!({"error_type": "invalid"}), Duration::from_secs(5)),
            };

            match send_stress_request(&client, &url, tool_name, params, timeout).await {
                Ok((response, duration)) => {
                    let success = !response.get("error").is_some();
                    metrics.record_request(duration.as_millis() as u64, success, if success { None } else { Some("server") });
                }
                Err(e) => {
                    let error_type = if e.to_string().contains("timeout") {
                        "timeout"
                    } else if e.to_string().contains("connection") {
                        "connection"
                    } else {
                        "server"
                    };
                    metrics.record_request(0, false, Some(error_type));
                }
            }
        });

        // Variable delay to create chaotic patterns
        let delay = rand::rng().random_range(1..100);
        sleep(Duration::from_millis(delay)).await;
    }

    let _all_permits = semaphore.acquire_many(max_concurrent as u32).await.unwrap();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let metrics = StressMetrics::new();
    let duration = Duration::from_secs(cli.duration_seconds);

    // Start real-time monitoring
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            let sent = metrics_clone.requests_sent.load(Ordering::Relaxed);
            let completed = metrics_clone.requests_completed.load(Ordering::Relaxed);
            let failed = metrics_clone.requests_failed.load(Ordering::Relaxed);

            if sent > 0 {
                let success_rate = ((completed - failed) as f64 / sent as f64) * 100.0;
                info!("Real-time: {} sent, {} completed, {:.1}% success rate", sent, completed, success_rate);
            }
        }
    });

    info!("Starting stress test against: {}", cli.server_url);
    info!("Test duration: {} seconds", cli.duration_seconds);

    match cli.command {
        Commands::Memory { max_concurrent, mb_per_request } => {
            memory_exhaustion_test(&client, &cli.server_url, max_concurrent, mb_per_request, duration, metrics.clone()).await;
        }
        Commands::Cpu { max_concurrent, computation_size } => {
            cpu_exhaustion_test(&client, &cli.server_url, max_concurrent, computation_size, duration, metrics.clone()).await;
        }
        Commands::Flood { connections_per_second, max_connections } => {
            connection_flood_test(&cli.server_url, connections_per_second, max_connections, duration, metrics.clone()).await;
        }
        Commands::Recovery { error_rate_percent } => {
            error_recovery_test(&client, &cli.server_url, error_rate_percent, duration, metrics.clone()).await;
        }
        Commands::Chaos { max_concurrent } => {
            chaos_test(&client, &cli.server_url, max_concurrent, duration, metrics.clone()).await;
        }
        Commands::Sessions { max_sessions: _, operations_per_session: _ } => {
            // TODO: Implement session exhaustion test
            info!("Session exhaustion test not yet implemented");
        }
    }

    // Allow final requests to complete
    sleep(Duration::from_secs(5)).await;

    // Print comprehensive results
    metrics.print_comprehensive_stats();

    Ok(())
}