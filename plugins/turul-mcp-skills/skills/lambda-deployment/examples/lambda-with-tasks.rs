// turul-mcp-server v0.3
// Lambda MCP server with DynamoDB task storage for long-running tools
//
// Cargo.toml dependencies:
//   turul-mcp-aws-lambda = { version = "0.3", features = ["dynamodb"] }
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
//   turul-mcp-task-storage = { version = "0.3", features = ["dynamodb"] }
//   turul-mcp-derive = { version = "0.3" }
//   lambda_http = "0.13"
//   tokio = { version = "1", features = ["full"] }

use lambda_http::{Body, Error, Request, run, service_fn};
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_session_storage::DynamoDbSessionStorage;
use turul_mcp_task_storage::DynamoDbTaskStorage;

// --- Tool with task support ---

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "slow_process",
    description = "Process data (may take several seconds)",
    task_support = "optional"  // Client can choose sync or async execution
)]
struct SlowProcessTool {
    #[param(description = "Data to process")]
    data: String,
}

impl SlowProcessTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        // Simulate long-running work
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        Ok(format!("Processed: {}", self.data))
    }
}

// --- Lambda handler ---

static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

async fn create_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    // DynamoDB for sessions
    let session_storage = Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(e.to_string()))?,
    );

    // DynamoDB for tasks (default table name)
    let task_storage = Arc::new(
        DynamoDbTaskStorage::new()
            .await
            .map_err(|e| Error::from(e.to_string()))?,
    );

    let server = LambdaMcpServerBuilder::new()
        .name("task-lambda")
        .version("1.0.0")
        .tool(SlowProcessTool::default())
        .storage(session_storage)
        .with_task_storage(task_storage) // Enable task support
        .sse(false)
        .cors_allow_all_origins()
        .build()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    server.handler().await.map_err(|e| Error::from(e.to_string()))
}

async fn lambda_handler(req: Request) -> Result<lambda_http::Response<Body>, Error> {
    let handler = HANDLER
        .get_or_try_init(|| async { create_handler().await })
        .await?;

    handler.handle(req).await.map_err(|e| Error::from(e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .json()
        .init();

    run(service_fn(lambda_handler)).await
}
