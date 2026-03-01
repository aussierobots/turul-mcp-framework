// turul-mcp-server v0.3
// Cancellation: how the executor handles cancellation transparently
//
// Tools are normal async functions. The TokioTaskExecutor wraps your tool
// in cancellation logic externally — you don't check for cancellation signals.

use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

// This tool does NOT need to handle cancellation itself.
// The TokioTaskExecutor wraps it with tokio::select!:
//
//   tokio::select! {
//       outcome = tool_future => { /* store result */ }
//       _ = cancel_signal    => { /* mark Cancelled */ }
//   }
//
// If the client calls tasks/cancel while this tool is sleeping,
// the executor cancels the tokio task, and the sleep is dropped.

#[mcp_tool(
    name = "slow_report",
    description = "Generate a report over many steps",
    task_support = "optional"
)]
async fn slow_report(
    #[param(description = "Number of sections")] sections: f64,
) -> McpResult<String> {
    let n = sections as u64;
    let mut report = String::new();

    for i in 1..=n {
        // Each step is an await point — cancellation can take effect here
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        report.push_str(&format!("Section {i}: Lorem ipsum...\n"));
    }

    Ok(report)
}

// What happens on cancellation:
//
// 1. Client sends: POST /mcp  {"method": "tasks/cancel", "params": {"taskId": "..."}}
// 2. TaskRuntime::cancel_task() signals the executor
// 3. TokioTaskExecutor drops the running future (slow_report stops at next .await)
// 4. Task status transitions: Working → Cancelled
// 5. tasks/get returns: {"status": "cancelled", "statusMessage": "Cancelled by client"}
//
// The tool code is clean — no cancellation tokens, no special handling.
