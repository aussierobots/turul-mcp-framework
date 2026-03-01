// turul-mcp-server v0.3
// Task-enabled server: McpServer with InMemoryTaskStorage + task-supporting tools

use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;
use turul_mcp_task_storage::InMemoryTaskStorage;

#[mcp_tool(
    name = "long_computation",
    description = "Run a long computation",
    task_support = "optional"
)]
async fn long_computation(
    #[param(description = "Iterations to run")] iterations: f64,
) -> McpResult<String> {
    let n = iterations as u64;
    for i in 0..n {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if i % 10 == 0 {
            tracing::info!(progress = i, total = n, "Computing...");
        }
    }
    Ok(format!("Completed {} iterations", n))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();

    // Option A: Simple — let builder create runtime from storage
    let server = McpServer::builder()
        .name("task-server")
        .version("1.0.0")
        .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
        .tool_fn(long_computation)
        .build()?;

    // Option B: Custom runtime with recovery timeout
    // use turul_mcp_server::task::TaskRuntime;
    // let runtime = Arc::new(
    //     TaskRuntime::with_default_executor(Arc::new(InMemoryTaskStorage::new()))
    //         .with_recovery_timeout(600_000)  // 10 minutes
    // );
    // let server = McpServer::builder()
    //     .name("task-server")
    //     .with_task_runtime(runtime)
    //     .tool_fn(long_computation)
    //     .build()?;

    // Option C: Shortcut for in-memory + defaults
    // use turul_mcp_server::task::TaskRuntime;
    // let runtime = Arc::new(TaskRuntime::in_memory());

    server.run().await
}
