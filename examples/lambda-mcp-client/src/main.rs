//! Comprehensive Test Client for Lambda MCP Server
//!
//! This client provides comprehensive end-to-end testing of the lambda-turul-mcp-server
//! implementation, validating MCP protocol compliance, tool functionality, session
//! management, and infrastructure integration.

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

mod client;
mod mcp_spec_validator;
mod schema_validator;
mod test_runner;
mod test_suite;

use client::{McpClient, McpClientConfig};
use test_runner::{TestResult, TestRunner};
use test_suite::TestSuite;

#[derive(Parser)]
#[command(name = "lambda-mcp-client")]
#[command(about = "Comprehensive test client for lambda-mcp-server validation")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the full test suite
    Test(TestArgs),
    /// Connect to server and run interactive session
    Connect(ConnectArgs),
    /// Validate tool schemas
    ValidateSchemas(ValidateArgs),
    /// Benchmark server performance
    Benchmark(BenchmarkArgs),
    /// Monitor server health
    Monitor(MonitorArgs),
}

#[derive(Args)]
struct TestArgs {
    /// Server URL to test
    #[arg(long, default_value = "http://127.0.0.1:9000")]
    url: String,
    
    /// Test suite to run (all, protocol, tools, session, infrastructure)
    #[arg(long, default_value = "all")]
    suite: String,
    
    /// Number of concurrent test sessions
    #[arg(long, default_value = "1")]
    concurrency: u32,
    
    /// Test timeout in seconds
    #[arg(long, default_value = "120")]
    timeout: u64,
    
    /// Generate detailed test report
    #[arg(long)]
    detailed_report: bool,
    
    /// Continue on test failures
    #[arg(long)]
    continue_on_failure: bool,
    
    /// Test SSE streaming specifically
    #[arg(long)]
    test_sse_streaming: bool,
}

#[derive(Args)]
struct ConnectArgs {
    /// Server URL to connect to
    #[arg(long, default_value = "http://127.0.0.1:9000")]
    url: String,
    
    /// Session ID to use
    #[arg(long)]
    session_id: Option<String>,
    
    /// Enable debug output
    #[arg(long)]
    debug: bool,
}

#[derive(Args)]
struct ValidateArgs {
    /// Server URL to test
    #[arg(long, default_value = "http://127.0.0.1:9000")]
    url: String,
    
    /// Tool to validate (or 'all')
    #[arg(long, default_value = "all")]
    tool: String,
}

#[derive(Args)]
struct BenchmarkArgs {
    /// Server URL to benchmark
    #[arg(long, default_value = "http://127.0.0.1:9000")]
    url: String,
    
    /// Number of requests to send
    #[arg(long, default_value = "100")]
    requests: u32,
    
    /// Number of concurrent clients
    #[arg(long, default_value = "10")]
    concurrency: u32,
    
    /// Request rate limit (requests/second)
    #[arg(long)]
    rate_limit: Option<u32>,
}

#[derive(Args)]
struct MonitorArgs {
    /// Server URL to monitor
    #[arg(long, default_value = "http://127.0.0.1:9000")]
    url: String,
    
    /// Monitoring interval in seconds
    #[arg(long, default_value = "30")]
    interval: u64,
    
    /// Alert thresholds configuration file
    #[arg(long)]
    alert_config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("lambda_mcp_client=info".parse()?)
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Test(args) => run_tests(args).await,
        Commands::Connect(args) => run_interactive_session(args).await,
        Commands::ValidateSchemas(args) => validate_schemas(args).await,
        Commands::Benchmark(args) => run_benchmark(args).await,
        Commands::Monitor(args) => run_monitoring(args).await,
    }
}

/// Run the comprehensive test suite
async fn run_tests(args: TestArgs) -> Result<()> {
    println!("{}", "🧪 Lambda MCP Server Test Suite".bright_blue().bold());
    println!("Server URL: {}", args.url.bright_cyan());
    println!("Test Suite: {}", args.suite.bright_cyan());
    println!("Concurrency: {}", args.concurrency.to_string().bright_cyan());
    println!();

    let client_config = McpClientConfig {
        base_url: args.url.clone(),
        timeout: Duration::from_secs(args.timeout),
        user_agent: "lambda-mcp-client/0.1.0".to_string(),
    };

    let mut test_runner = TestRunner::new(client_config, args.concurrency);
    
    // Create test suite based on arguments
    let test_suite = if args.test_sse_streaming {
        TestSuite::streaming_only()
    } else {
        create_test_suite(&args.suite)?
    };
    
    let start_time = Instant::now();
    info!("Starting test execution...");
    
    // Create progress bar
    let pb = ProgressBar::new(test_suite.test_count() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    // Run tests
    let results = test_runner.run_test_suite(test_suite, Some(pb.clone())).await?;
    
    pb.finish_with_message("Tests completed");
    
    let duration = start_time.elapsed();
    
    // Print results summary
    print_test_results(&results, duration, args.detailed_report)?;
    
    // Exit with appropriate code
    let failed_count = results.iter().filter(|r| !r.passed).count();
    if failed_count > 0 && !args.continue_on_failure {
        std::process::exit(1);
    }
    
    Ok(())
}

/// Run an interactive session with the server
async fn run_interactive_session(args: ConnectArgs) -> Result<()> {
    println!("{}", "🔗 Interactive MCP Session".bright_blue().bold());
    println!("Connecting to: {}", args.url.bright_cyan());
    
    let client_config = McpClientConfig {
        base_url: args.url,
        timeout: Duration::from_secs(30),
        user_agent: "lambda-mcp-client-interactive/0.1.0".to_string(),
    };
    
    let session_id = args.session_id.unwrap_or_else(|| {
        format!("interactive-{}", Uuid::new_v4())
    });
    
    let mut client = McpClient::new(client_config, session_id.clone())?;
    
    println!("Session ID: {}", session_id.bright_green());
    println!();
    
    // Initialize session
    print!("Initializing session... ");
    let init_result = client.initialize().await?;
    println!("{}", "✅ Success".green());
    
    if args.debug {
        println!("Initialize result: {}", serde_json::to_string_pretty(&init_result)?);
    }
    
    // List available tools
    print!("Listing tools... ");
    let tools = client.list_tools().await?;
    println!("{}", "✅ Success".green());
    
    println!("\n{}", "Available Tools:".bright_yellow().bold());
    if let Some(tools_array) = tools.get("tools").and_then(|t| t.as_array()) {
        for tool in tools_array {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                let desc = tool.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("No description");
                println!("  {} - {}", name.bright_cyan(), desc);
            }
        }
    }
    
    println!("\n{}", "Type 'help' for commands, 'quit' to exit".bright_yellow());
    
    // Interactive command loop
    loop {
        print!("\n> ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input {
            "quit" | "exit" => break,
            "help" => {
                println!("Commands:");
                println!("  help - Show this help");
                println!("  tools - List available tools");
                println!("  call <tool_name> [args] - Call a tool");
                println!("  session - Show session info");
                println!("  quit - Exit interactive session");
            }
            "tools" => {
                let tools = client.list_tools().await?;
                println!("{}", serde_json::to_string_pretty(&tools)?);
            }
            "session" => {
                match client.call_tool("session_info", json!({})).await {
                    Ok(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                    Err(e) => println!("Error: {}", e.to_string().red()),
                }
            }
            input if input.starts_with("call ") => {
                let parts: Vec<&str> = input.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let tool_name = parts[1];
                    let args = if parts.len() > 2 {
                        match serde_json::from_str(parts[2]) {
                            Ok(args) => args,
                            Err(_) => json!({}),
                        }
                    } else {
                        json!({})
                    };
                    
                    match client.call_tool(tool_name, args).await {
                        Ok(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                        Err(e) => println!("Error: {}", e.to_string().red()),
                    }
                } else {
                    println!("Usage: call <tool_name> [json_args]");
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.", input.red());
            }
        }
    }
    
    println!("\n{}", "👋 Session ended".bright_blue());
    Ok(())
}

/// Validate tool schemas
async fn validate_schemas(args: ValidateArgs) -> Result<()> {
    println!("{}", "🔍 Schema Validation".bright_blue().bold());
    println!("Server URL: {}", args.url.bright_cyan());
    println!("Tool: {}", args.tool.bright_cyan());
    println!();
    
    // Implementation for schema validation
    todo!("Implement schema validation")
}

/// Run performance benchmark
async fn run_benchmark(args: BenchmarkArgs) -> Result<()> {
    println!("{}", "⚡ Performance Benchmark".bright_blue().bold());
    println!("Server URL: {}", args.url.bright_cyan());
    println!("Requests: {}", args.requests.to_string().bright_cyan());
    println!("Concurrency: {}", args.concurrency.to_string().bright_cyan());
    println!();
    
    // Implementation for benchmarking
    todo!("Implement benchmarking")
}

/// Run health monitoring
async fn run_monitoring(args: MonitorArgs) -> Result<()> {
    println!("{}", "📊 Health Monitoring".bright_blue().bold());
    println!("Server URL: {}", args.url.bright_cyan());
    println!("Interval: {}s", args.interval.to_string().bright_cyan());
    println!();
    
    // Implementation for monitoring
    todo!("Implement monitoring")
}

/// Create test suite based on suite name
fn create_test_suite(suite_name: &str) -> Result<TestSuite> {
    match suite_name {
        "all" => Ok(TestSuite::comprehensive()),
        "protocol" => Ok(TestSuite::protocol_only()),
        "tools" => Ok(TestSuite::tools_only()),
        "session" => Ok(TestSuite::session_only()),
        "infrastructure" => Ok(TestSuite::infrastructure_only()),
        "streaming" => Ok(TestSuite::streaming_only()),
        _ => Err(anyhow::anyhow!("Unknown test suite: {} (available: all, protocol, tools, session, infrastructure, streaming)", suite_name)),
    }
}

/// Print test results summary
fn print_test_results(
    results: &[TestResult],
    duration: Duration,
    detailed: bool,
) -> Result<()> {
    println!("\n{}", "📊 Test Results".bright_blue().bold());
    println!("{}", "=".repeat(60));
    
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    
    println!("Total tests: {}", total.to_string().bright_cyan());
    println!("Passed: {}", passed.to_string().bright_green());
    println!("Failed: {}", failed.to_string().bright_red());
    println!("Duration: {:.2}s", duration.as_secs_f64().to_string().bright_cyan());
    
    if failed > 0 {
        println!("\n{}", "❌ Failed Tests:".bright_red().bold());
        for result in results.iter().filter(|r| !r.passed) {
            println!("  {} - {}", result.name.red(), result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
            
            if detailed {
                if let Some(details) = &result.details {
                    println!("    Details: {}", serde_json::to_string_pretty(details)?);
                }
            }
        }
    }
    
    if passed > 0 {
        println!("\n{}", "✅ Passed Tests:".bright_green().bold());
        for result in results.iter().filter(|r| r.passed) {
            println!("  {} ({:.2}s)", result.name.green(), result.duration.as_secs_f64());
            
            if detailed && result.details.is_some() {
                println!("    Details: {}", serde_json::to_string_pretty(result.details.as_ref().unwrap())?);
            }
        }
    }
    
    println!("{}", "=".repeat(60));
    
    if failed == 0 {
        println!("{}", "🎉 All tests passed!".bright_green().bold());
    } else {
        println!("{}", format!("💥 {} test(s) failed", failed).bright_red().bold());
    }
    
    Ok(())
}