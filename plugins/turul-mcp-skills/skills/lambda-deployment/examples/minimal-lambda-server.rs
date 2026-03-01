// turul-mcp-server v0.3
// Minimal Lambda MCP server — smallest working example
//
// Cargo.toml dependencies:
//   turul-mcp-aws-lambda = { version = "0.3" }
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-session-storage = { version = "0.3" }
//   turul-mcp-derive = { version = "0.3" }
//   lambda_http = "0.13"
//   tokio = { version = "1", features = ["full"] }

use lambda_http::{Body, Error, Request, run, service_fn};
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_session_storage::InMemorySessionStorage;

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
    let server = LambdaMcpServerBuilder::new()
        .name("minimal-lambda")
        .version("1.0.0")
        .tool(GreetTool::default())
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)               // Explicitly disable SSE (on by default)
        .cors_allow_all_origins() // CORS for browser clients
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
