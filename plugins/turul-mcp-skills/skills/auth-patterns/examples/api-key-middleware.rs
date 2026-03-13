// turul-mcp-server v0.3
// Simple API key authentication middleware
//
// Cargo.toml dependencies:
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-derive = { version = "0.3" }
//   tokio = { version = "1", features = ["full"] }
//   async-trait = "0.1"
//
// This example demonstrates:
//   1. Custom McpMiddleware for API key validation
//   2. Pre-session middleware (runs_before_session = true)
//   3. Early 401 rejection without session allocation

use std::sync::Arc;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpResult;
use turul_mcp_server::prelude::*;

// --- API Key Middleware ---

struct ApiKeyMiddleware {
    valid_keys: Vec<String>,
}

#[async_trait::async_trait]
impl McpMiddleware for ApiKeyMiddleware {
    fn runs_before_session(&self) -> bool {
        true // Reject invalid keys before allocating a session
    }

    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let key = ctx
            .metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MiddlewareError::Unauthorized {
                message: "Missing X-API-Key header".into(),
            })?;

        if !self.valid_keys.contains(&key.to_string()) {
            return Err(MiddlewareError::Unauthorized {
                message: "Invalid API key".into(),
            });
        }

        Ok(())
    }

    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

// --- Simple tool ---

#[derive(McpTool, Clone, Default)]
#[tool(name = "hello", description = "Greet the caller")]
struct HelloTool {}

impl HelloTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("Hello, authenticated user!".to_string())
    }
}

// --- Server setup ---

#[tokio::main]
async fn main() -> McpResult<()> {
    let keys = vec!["secret-key-123".to_string(), "dev-key-456".to_string()];

    let server = McpServer::builder()
        .name("api-key-server")
        .version("0.3.13")
        .middleware(Arc::new(ApiKeyMiddleware { valid_keys: keys }))
        .tool(HelloTool::default())
        .build()?;

    server.run().await
}
