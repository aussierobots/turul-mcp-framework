//! # Tasks Extension Client (SEP-2663)
//!
//! The client leg against `ext-tasks-server`:
//!
//! 1. Declare the extension (`declared_capabilities.ext_tasks = true`).
//! 2. `call_tool_or_task("crunch")` → a task handle; `task_wait` polls
//!    (honoring `pollIntervalMs`) to the completed result.
//! 3. `call_tool_or_task("deploy")` → the task parks in `input_required`;
//!    answer the elicited approval via `task_update`; poll to completion.
//! 4. The same `crunch` from an UNDECLARED client → ordinary synchronous
//!    result (progressive enhancement).

use serde_json::json;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{ClientConfig, McpClient, McpVersion, ToolCallOutcome};
use turul_mcp_ext_tasks::DetailedTask;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8645/mcp".to_string());
    println!("Connecting to {url} (start it with: cargo run -p ext-tasks-server)");

    let mut config = ClientConfig::default();
    config.declared_capabilities.ext_tasks = true;
    let client = McpClient::new(Box::new(HttpTransport::new(&url)?), config);
    client.connect().await?;
    if client.negotiated_version().await != Some(McpVersion::V2026_07_28) {
        println!("Peer is not a 2026-07-28 server — the Tasks extension needs the 2026 wire.");
        return Ok(());
    }

    // 1. Long-running call → durable task handle, poll to completion.
    println!("\n→ crunch(7) with the extension declared");
    match client
        .call_tool_or_task("crunch", json!({ "n": 7 }))
        .await?
    {
        ToolCallOutcome::Task(task) => {
            println!(
                "← task {} ({:?}, pollIntervalMs {:?}) — polling…",
                task.task.fields.task_id, task.task.status, task.task.fields.poll_interval_ms
            );
            let done = client.task_wait(&task.task.fields.task_id).await?;
            if let DetailedTask::Completed { result, .. } = done {
                println!("← completed: {}", result["content"][0]["text"]);
            }
        }
        ToolCallOutcome::Completed(r) => println!("← completed synchronously: {r:?}"),
    }

    // 2. Mid-task input: deploy parks in input_required until tasks/update.
    println!("\n→ deploy(billing-api)");
    let ToolCallOutcome::Task(task) = client
        .call_tool_or_task("deploy", json!({ "service": "billing-api" }))
        .await?
    else {
        println!("← unexpectedly synchronous");
        return Ok(());
    };
    let task_id = task.task.fields.task_id.clone();
    println!("← task {task_id} — polling for the approval request…");
    loop {
        let t = client.task_get(&task_id).await?;
        match &t {
            DetailedTask::InputRequired { input_requests, .. } => {
                let msg = input_requests
                    .get("approval")
                    .and_then(|r| serde_json::to_value(r).ok())
                    .and_then(|v| {
                        v.pointer("/params/message")
                            .and_then(|m| m.as_str().map(String::from))
                    })
                    .unwrap_or_default();
                println!("← input_required: {msg}");
                println!("→ tasks/update: approved = true");
                client
                    .task_update(
                        &task_id,
                        json!({ "approval": { "action": "accept", "content": { "approved": true } } }),
                    )
                    .await?;
                break;
            }
            t if t.status().is_terminal() => break,
            _ => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    let done = client.task_wait(&task_id).await?;
    if let DetailedTask::Completed { result, .. } = done {
        println!("← completed: {}", result["content"][0]["text"]);
    }

    // 3. Progressive enhancement: an UNDECLARED client gets the sync result.
    println!("\n→ crunch(3) WITHOUT declaring the extension");
    let plain = McpClient::new(Box::new(HttpTransport::new(&url)?), ClientConfig::default());
    plain.connect().await?;
    match plain.call_tool_or_task("crunch", json!({ "n": 3 })).await? {
        ToolCallOutcome::Completed(r) => {
            let v = serde_json::to_value(&r)?;
            println!("← synchronous (blocked ~2s): {}", v["content"][0]["text"]);
        }
        ToolCallOutcome::Task(_) => println!("← BUG: task handle for an undeclared client"),
    }

    println!("\nDone — task ids are durable handles; re-run tasks/get any time before TTL.");
    Ok(())
}
