//! # Tasks E2E In-Memory Client
//!
//! Exercises the full MCP task lifecycle against the tasks-e2e-inmemory-server.
//! Prints PASS/FAIL output for each test scenario.
//!
//! ## Usage
//! ```bash
//! # Start the server first:
//! cargo run --example tasks-e2e-inmemory-server -- --port 8080
//! # Then run this client:
//! cargo run --example tasks-e2e-inmemory-client -- --url http://127.0.0.1:8080/mcp
//! ```

use std::time::Duration;

use clap::Parser;
use serde_json::json;
use tracing::info;
use turul_mcp_client::prelude::*;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_protocol::tasks::TaskStatus;

#[derive(Parser)]
#[command(name = "tasks-e2e-inmemory-client")]
#[command(about = "MCP task lifecycle E2E test client")]
struct Args {
    /// Server URL
    #[arg(long, default_value = "http://127.0.0.1:8080/mcp")]
    url: String,
}

fn pass(name: &str) {
    println!("PASS: {}", name);
}

fn fail(name: &str, reason: &str) {
    println!("FAIL: {} — {}", name, reason);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();
    info!(url = %args.url, "Connecting to MCP server");

    let transport = HttpTransport::new(&args.url)?;
    let client = McpClientBuilder::new()
        .with_transport(Box::new(transport))
        .build();
    client.connect().await?;

    // === Test 1: Task-augmented slow_add (success flow) ===
    let response = client
        .call_tool_with_task(
            "slow_add",
            json!({"a": 5, "b": 3, "delay_ms": 500}),
            Some(60_000),
        )
        .await?;

    match &response {
        ToolCallResponse::TaskCreated(task) => {
            pass("call_tool_with_task returns TaskCreated");
            info!(task_id = %task.task_id, "Task created");

            // Poll until complete
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let current = client.get_task(&task.task_id).await?;
                match current.status {
                    TaskStatus::Completed => {
                        pass("tasks/get shows Completed status");
                        break;
                    }
                    TaskStatus::Failed | TaskStatus::Cancelled => {
                        fail(
                            "tasks/get polling",
                            &format!("unexpected terminal status: {:?}", current.status),
                        );
                        break;
                    }
                    _ => continue,
                }
            }

            // Get final result via tasks/result (should return immediately since task is complete)
            let result = client.get_task_result(&task.task_id).await?;
            // The result should be a CallToolResult with content containing the sum
            if let Some(content) = result.get("content") {
                if content.to_string().contains("8") {
                    pass("tasks/result returns correct sum (5 + 3 = 8)");
                } else {
                    fail(
                        "tasks/result value",
                        &format!("expected 8, got: {}", content),
                    );
                }
            } else {
                fail(
                    "tasks/result shape",
                    &format!("missing content field: {}", result),
                );
            }
        }
        ToolCallResponse::Immediate(_) => {
            fail("call_tool_with_task", "expected TaskCreated, got Immediate");
        }
    }

    // === Test 2: List tasks ===
    let tasks = client.list_tasks().await?;
    if !tasks.is_empty() {
        pass(&format!("tasks/list returns {} task(s)", tasks.len()));
    } else {
        fail("tasks/list", "expected at least 1 task");
    }

    // === Test 3: Cancellation flow ===
    let cancel_response = client
        .call_tool_with_task(
            "slow_cancelable",
            json!({"duration_ms": 30000}),
            Some(60_000),
        )
        .await?;

    if let ToolCallResponse::TaskCreated(task) = cancel_response {
        // Wait a moment for the task to start executing
        tokio::time::sleep(Duration::from_millis(200)).await;

        match client.cancel_task(&task.task_id).await {
            Ok(cancelled) => {
                if cancelled.status == TaskStatus::Cancelled {
                    pass("tasks/cancel transitions to Cancelled");
                } else {
                    fail(
                        "tasks/cancel status",
                        &format!("expected Cancelled, got {:?}", cancelled.status),
                    );
                }
            }
            Err(e) => {
                fail("tasks/cancel", &format!("error: {}", e));
            }
        }
    } else {
        fail(
            "cancellation setup",
            "expected TaskCreated for slow_cancelable",
        );
    }

    // === Test 4: Synchronous call (no task augmentation) ===
    let sync_result = client
        .call_tool("slow_add", json!({"a": 1, "b": 2, "delay_ms": 100}))
        .await?;
    if !sync_result.is_empty() {
        pass("synchronous tools/call works without task augmentation");
    } else {
        fail("synchronous call", "returned empty result");
    }

    client.disconnect().await?;
    println!("\nE2E task lifecycle tests complete.");

    Ok(())
}
