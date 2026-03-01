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
//   3. `handle_streaming()` instead of `handle()`
//   4. `run_with_streaming_response()` instead of `run()`
//   5. DynamoDB session storage for durability

use lambda_http::{Error, Request, run_with_streaming_response, service_fn};
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
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

// Return type: streaming body (not LambdaBody)
async fn lambda_handler(
    req: Request,
) -> Result<
    lambda_http::Response<http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let handler = HANDLER
        .get_or_try_init(|| async { create_handler().await })
        .await?;

    // handle_streaming() instead of handle()
    handler.handle_streaming(req).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .json()
        .init();

    // Pre-initialize handler during startup
    HANDLER
        .get_or_try_init(|| async { create_handler().await })
        .await?;

    // run_with_streaming_response() instead of run()
    run_with_streaming_response(service_fn(lambda_handler)).await
}
