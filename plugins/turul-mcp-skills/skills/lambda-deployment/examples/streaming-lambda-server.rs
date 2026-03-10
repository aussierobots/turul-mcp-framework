// turul-mcp-server v0.3
// Lambda MCP server with real-time SSE streaming
//
// Cargo.toml dependencies:
//   turul-mcp-aws-lambda = { version = "0.3", features = ["streaming", "dynamodb"] }
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
//   turul-mcp-derive = { version = "0.3" }
//   lambda_http = "0.13"
//   tokio = { version = "1", features = ["full"] }
//   http-body-util = "0.1"
//   bytes = "1"
//   hyper = "1"
//
// Key differences from minimal example:
//   1. `streaming` feature enabled (implies `sse`)
//   2. `.sse(true)` on builder (default when sse feature is on, but explicit for clarity)
//   3. `run_streaming(handler)` instead of `run(service_fn(...))`
//   4. DynamoDB session storage for durability
//
// For custom pre-dispatch logic (e.g., `.well-known` routing), use
// `run_streaming_with(|req| async { handler.handle_streaming(req).await })`
// instead of `run_streaming(handler)`.

use lambda_http::Error;
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::{LambdaMcpServerBuilder, run_streaming};
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_session_storage::DynamoDbSessionStorage;

// --- Tool definition ---

#[derive(McpTool, Clone, Default)]
#[tool(name = "greet", description = "Greet someone by name")]
struct GreetTool {
    #[param(description = "Name to greet")]
    name: String,
}

impl GreetTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Hello, {}!", self.name))
    }
}

// --- Lambda handler with cold-start caching ---

static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

async fn create_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    // DynamoDB for session persistence across Lambda invocations
    let storage = Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(e.to_string()))?,
    );

    let server = LambdaMcpServerBuilder::new()
        .name("streaming-lambda")
        .version("1.0.0")
        .tool(GreetTool::default())
        .storage(storage)
        .sse(true)                // Enable SSE (explicit for clarity)
        .cors_allow_all_origins()
        .build()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    server.handler().await.map_err(|e| Error::from(e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .json()
        .init();

    // Pre-initialize handler during startup
    let handler = HANDLER
        .get_or_try_init(|| async { create_handler().await })
        .await?
        .clone();

    // run_streaming() handles completion invocations gracefully — no ERROR logs
    run_streaming(handler).await
}
