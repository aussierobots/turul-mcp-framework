// turul-mcp-server v0.3
// Task support declaration: function macro, derive macro, and builder patterns

use turul_mcp_derive::{mcp_tool, McpTool};
use turul_mcp_protocol::tools::{TaskSupport, ToolExecution};
use turul_mcp_server::prelude::*;

// === Function Macro ===

#[mcp_tool(
    name = "slow_add",
    description = "Add two numbers with simulated delay",
    task_support = "optional"  // Client can choose sync or async
)]
async fn slow_add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    Ok(a + b)
}

// === Derive Macro ===

#[derive(McpTool, Default)]
#[tool(
    name = "batch_process",
    description = "Process a batch of items",
    task_support = "required"  // Must always run as task
)]
struct BatchProcess {
    #[param(description = "Items to process")]
    items: Vec<String>,
}

impl BatchProcess {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        // Long-running batch operation
        for item in &self.items {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            tracing::info!(item = %item, "Processing item");
        }
        Ok(format!("Processed {} items", self.items.len()))
    }
}

// === Builder ===

fn build_dynamic_task_tool() -> Result<impl std::any::Any, Box<dyn std::error::Error>> {
    let tool = ToolBuilder::new("dynamic_slow_op")
        .description("Dynamic slow operation")
        .string_param("input", "Input data")
        .execution(ToolExecution {
            task_support: Some(TaskSupport::Optional),
        })
        .execute(|args| async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            Ok(serde_json::json!({"processed": true}))
        })
        .build()?;
    Ok(tool)
}

// === Values reference ===
//
// task_support = "optional"  — Client chooses: sync call or async task
// task_support = "required"  — Always runs as task; sync calls are rejected
// task_support = "forbidden" — Never runs as task; task requests are rejected
// (omitted)                  — No task support; execution field not advertised
