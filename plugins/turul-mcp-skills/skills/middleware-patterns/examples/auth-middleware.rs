// turul-mcp-server v0.3
// Auth middleware: validate API key from transport metadata, inject user state

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use turul_http_mcp_server::middleware::*;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::SessionView;

struct ApiKeyAuth {
    valid_keys: Vec<String>,
}

#[async_trait]
impl McpMiddleware for ApiKeyAuth {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Skip auth for initialize (session doesn't exist yet) and ping
        if ctx.method() == "initialize" || ctx.method() == "ping" {
            return Ok(());
        }

        // Extract API key from transport metadata (HTTP header: x-api-key)
        let key = ctx
            .metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MiddlewareError::unauthenticated("Missing x-api-key header"))?;

        // Validate against known keys
        if !self.valid_keys.iter().any(|k| k == key) {
            return Err(MiddlewareError::unauthorized("Invalid API key"));
        }

        // Inject authenticated user state into session
        // Tools can read this via: session.get_typed_state::<String>("api_key_id").await
        injection.set_state("api_key_id", json!(key));
        injection.set_metadata("authenticated", json!(true));

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("auth-server")
        .version("1.0.0")
        .middleware(Arc::new(ApiKeyAuth {
            valid_keys: vec!["sk-test-123".to_string(), "sk-prod-456".to_string()],
        }))
        // .tool_fn(my_protected_tool)
        .build()?;

    server.run().await
}
