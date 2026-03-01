// turul-mcp-client v0.3
// Task workflow: call_tool_with_task, poll status, handle all TaskStatus variants.

use serde_json::json;
use std::time::Duration;
use turul_mcp_client::{McpClientBuilder, McpClientResult, ToolCallResponse};
use turul_mcp_protocol::TaskStatus;

#[tokio::main]
async fn main() -> McpClientResult<()> {
    let client = McpClientBuilder::new()
        .with_url("http://localhost:8080/mcp")?
        .build();
    client.connect().await?;

    // call_tool_with_task returns ToolCallResponse — either immediate or task
    let response = client
        .call_tool_with_task("slow_add", json!({"a": 10, "b": 20}), None)
        .await?;

    match response {
        ToolCallResponse::Immediate(result) => {
            println!("Got immediate result: {result:?}");
        }
        ToolCallResponse::TaskCreated(task) => {
            println!("Task created: {}", task.id);

            // --- Option A: Block until terminal (per MCP spec) ---
            // get_task_result blocks until the task reaches a terminal state.
            // let value = client.get_task_result(&task.id).await?;
            // println!("Task result: {value}");

            // --- Option B: Poll for status ---
            let max_polls = 60;
            for _ in 0..max_polls {
                let current = client.get_task(&task.id).await?;
                match current.status {
                    TaskStatus::Working => {
                        println!("Task still working...");
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                    TaskStatus::Completed => {
                        println!("Task completed!");
                        // Fetch the final result
                        let value = client.get_task_result(&current.id).await?;
                        println!("Result: {value}");
                        break;
                    }
                    TaskStatus::Failed => {
                        eprintln!("Task failed");
                        break;
                    }
                    TaskStatus::InputRequired => {
                        // Server needs client input (elicitation).
                        // McpClient does not yet expose an elicitation response API.
                        // In production, surface this to the application layer.
                        eprintln!("Task requires input — handle at application level");
                        break;
                    }
                    TaskStatus::Cancelled => {
                        println!("Task was cancelled");
                        break;
                    }
                }
            }
        }
    }

    // You can also list and cancel tasks
    let tasks = client.list_tasks().await?;
    println!("Active tasks: {}", tasks.len());

    // Cancel a task (if needed)
    // let cancelled = client.cancel_task("some-task-id").await?;

    client.disconnect().await?;
    Ok(())
}
