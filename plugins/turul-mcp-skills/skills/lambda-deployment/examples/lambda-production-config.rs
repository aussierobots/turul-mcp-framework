// turul-mcp-server v0.3
// Production-ready Lambda MCP server with auth middleware, env CORS, and logging
//
// Cargo.toml dependencies:
//   turul-mcp-aws-lambda = { version = "0.3", features = ["dynamodb"] }
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
//   turul-http-mcp-server = { version = "0.3" }
//   turul-mcp-derive = { version = "0.3" }
//   lambda_http = "0.13"
//   tokio = { version = "1", features = ["full"] }
//   async-trait = "0.1"
//   serde_json = "1"
//   tracing = "0.1"
//   tracing-subscriber = { version = "0.3", features = ["json"] }

use async_trait::async_trait;
use lambda_http::{Body, Error, Request, run, service_fn};
use std::env;
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};

// --- Auth middleware (reads API Gateway authorizer headers) ---

use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;

struct LambdaAuthMiddleware;

#[async_trait]
impl McpMiddleware for LambdaAuthMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Skip auth for initialize and ping
        if ctx.method() == "initialize" || ctx.method() == "ping" {
            return Ok(());
        }

        // API Gateway authorizer populates x-authorizer-* headers
        let user_id = ctx
            .metadata()
            .get("x-authorizer-principalid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MiddlewareError::unauthenticated(
                    "Missing authorizer principal — is the API Gateway authorizer configured?",
                )
            })?;

        injection.set_state("user_id", serde_json::json!(user_id));
        Ok(())
    }
}

// --- Tool definition ---

#[derive(McpTool, Clone, Default)]
#[tool(name = "whoami", description = "Return the authenticated user ID")]
struct WhoamiTool;

impl WhoamiTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let user_id = session
            .as_ref()
            .and_then(|s| {
                // get_typed_state would be used in real code with .await
                None::<String> // Placeholder for example
            })
            .unwrap_or_else(|| "anonymous".to_string());

        Ok(format!("You are: {}", user_id))
    }
}

// --- Lambda handler ---

static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

async fn create_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    // production_config() = DynamoDB sessions + env-based CORS
    let server = LambdaMcpServerBuilder::new()
        .name("production-lambda")
        .version("1.0.0")
        .production_config()
        .await
        .map_err(|e| Error::from(e.to_string()))?
        .middleware(Arc::new(LambdaAuthMiddleware))
        .tool(WhoamiTool::default())
        .sse(false)
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

// --- CloudWatch-optimized logging ---

fn init_logging() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());

    if env::var("AWS_EXECUTION_ENV").is_ok() {
        // Lambda: JSON for CloudWatch Logs Insights
        tracing_subscriber::fmt()
            .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
            .with_target(false)
            .without_time()
            .json()
            .init();
    } else {
        // Local dev: human-readable
        tracing_subscriber::fmt()
            .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();
    run(service_fn(lambda_handler)).await
}
