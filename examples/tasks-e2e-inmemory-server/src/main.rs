//! # Tasks E2E In-Memory Server
//!
//! MCP server with task support for end-to-end testing.
//! Tools:
//! - `slow_add`: Adds two numbers with a configurable delay (simulates async work)
//! - `slow_cancelable`: Long-running tool that checks for cancellation
//!
//! ## Usage
//! ```bash
//! cargo run --example tasks-e2e-inmemory-server -- --port 8080
//! ```

use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;
use turul_mcp_task_storage::InMemoryTaskStorage;

#[derive(Parser)]
#[command(name = "tasks-e2e-inmemory-server")]
#[command(about = "MCP server with in-memory task support for E2E testing")]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "8080")]
    port: u16,
}

/// Adds two numbers after a configurable delay (simulates async work).
/// When called with task augmentation, the delay causes the task to be in
/// Working state while the client can poll for status.
#[mcp_tool(
    name = "slow_add",
    description = "Add two numbers with a delay (for task testing)",
    task_support = "optional"
)]
async fn slow_add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
    #[param(description = "Delay in milliseconds (default 2000)")] delay_ms: Option<u64>,
) -> McpResult<f64> {
    let delay = delay_ms.unwrap_or(2000);
    tokio::time::sleep(Duration::from_millis(delay)).await;
    Ok(a + b)
}

/// Long-running tool that sleeps for the given duration.
/// Designed to be cancelled mid-flight via tasks/cancel.
#[mcp_tool(
    name = "slow_cancelable",
    description = "Long-running sleep tool (for cancellation testing)",
    task_support = "optional"
)]
async fn slow_cancelable(
    #[param(description = "Sleep duration in milliseconds (default 30000)")] duration_ms: Option<
        u64,
    >,
) -> McpResult<String> {
    let duration = duration_ms.unwrap_or(30_000);
    tokio::time::sleep(Duration::from_millis(duration)).await;
    Ok(format!("Completed after {}ms", duration))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();
    let addr = format!("127.0.0.1:{}", args.port);

    let server = McpServer::builder()
        .name("tasks-e2e-inmemory-server")
        .version("0.3.3")
        .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
        .tool_fn(slow_add)
        .tool_fn(slow_cancelable)
        .bind_address(addr.parse()?)
        .build()?;

    println!("Tasks E2E server running at: http://{}/mcp", addr);
    println!("Tools: slow_add, slow_cancelable");
    println!("Task support: enabled (in-memory storage)");

    server.run().await?;
    Ok(())
}
