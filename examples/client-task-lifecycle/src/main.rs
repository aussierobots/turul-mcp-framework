//! # MCP Task Lifecycle Client Example
//!
//! Demonstrates the full MCP 2025-11-25 task lifecycle using the client library:
//! 1. Connect to a server with task support
//! 2. Call a tool with task augmentation (`call_tool_with_task`)
//! 3. Poll the task status (`get_task`)
//! 4. List all tasks (`list_tasks`)
//! 5. Retrieve the final result (`get_task_result`)
//! 6. Cancel a task (`cancel_task`)
//!
//! ## Usage
//! ```bash
//! # Start a server with task support, then:
//! cargo run --example client-task-lifecycle -- --url http://127.0.0.1:8080/mcp
//! ```

use clap::Parser;
use serde_json::json;
use std::time::Duration;
use tracing::info;
use turul_mcp_client::prelude::*;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_protocol::tasks::TaskStatus;

#[derive(Parser)]
#[command(name = "client-task-lifecycle")]
#[command(about = "MCP task lifecycle demonstration client")]
struct Args {
    /// Server URL (e.g., http://127.0.0.1:8080/mcp)
    #[arg(long, default_value = "http://127.0.0.1:8080/mcp")]
    url: String,

    /// Tool name to call
    #[arg(long, default_value = "add")]
    tool: String,

    /// TTL in milliseconds for the task
    #[arg(long, default_value = "60000")]
    ttl: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!(url = %args.url, "Connecting to MCP server");

    // 1. Connect
    let transport = HttpTransport::new(&args.url)?;
    let client = McpClientBuilder::new()
        .with_transport(Box::new(transport))
        .build();

    client.connect().await?;
    info!("Connected successfully");

    // 2. List available tools
    let tools = client.list_tools().await?;
    info!(count = tools.len(), "Available tools");
    for tool in &tools {
        info!(name = %tool.name, "  Tool");
    }

    // 3. Call tool with task augmentation
    info!(
        tool = %args.tool,
        ttl = args.ttl,
        "Calling tool with task augmentation"
    );

    let response = client
        .call_tool_with_task(&args.tool, json!({"a": 5, "b": 3}), Some(args.ttl))
        .await?;

    match &response {
        ToolCallResponse::TaskCreated(task) => {
            info!(
                task_id = %task.task_id,
                status = ?task.status,
                "Server created a task (async execution)"
            );

            // 4. Poll task status
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let current = client.get_task(&task.task_id).await?;
                info!(
                    task_id = %current.task_id,
                    status = ?current.status,
                    message = current.status_message.as_deref().unwrap_or("-"),
                    "Task status"
                );

                match current.status {
                    TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => break,
                    _ => continue,
                }
            }

            // 5. List all tasks
            let tasks = client.list_tasks().await?;
            info!(count = tasks.len(), "All tasks");

            // 6. Get the final result
            info!(task_id = %task.task_id, "Retrieving task result");
            let result = client.get_task_result(&task.task_id).await?;
            info!(result = %result, "Task result");
        }
        ToolCallResponse::Immediate(result) => {
            info!(
                is_error = result.is_error,
                "Tool executed synchronously (server does not support tasks for this tool)"
            );
            for content in &result.content {
                info!(content = ?content, "  Content");
            }
        }
    }

    // 7. Demonstrate cancellation (call another task and cancel it)
    info!("Demonstrating task cancellation...");
    let cancel_response = client
        .call_tool_with_task(&args.tool, json!({"a": 100, "b": 200}), Some(args.ttl))
        .await?;

    if let ToolCallResponse::TaskCreated(task) = cancel_response {
        info!(task_id = %task.task_id, "Cancelling task");
        match client.cancel_task(&task.task_id).await {
            Ok(cancelled) => {
                info!(
                    task_id = %cancelled.task_id,
                    status = ?cancelled.status,
                    "Task cancelled"
                );
            }
            Err(e) => {
                // Task may have already completed before we could cancel
                info!(error = %e, "Cancel returned error (task may have already completed)");
            }
        }
    }

    // 8. Disconnect
    client.disconnect().await?;
    info!("Disconnected. Task lifecycle demo complete.");

    Ok(())
}
