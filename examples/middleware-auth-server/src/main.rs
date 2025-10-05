//! Middleware Authentication Example
//!
//! This example demonstrates how to use middleware for API key authentication.
//! The middleware:
//! 1. Extracts the X-API-Key header from requests
//! 2. Validates the API key (simple hardcoded example)
//! 3. Stores the authenticated user_id in session state
//! 4. Tools can read the user_id from session to know who is calling
//!
//! # Usage
//!
//! ```bash
//! # Start server
//! cargo run --bin middleware-auth-server
//!
//! # Valid request with API key
//! curl -X POST http://localhost:8080/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json" \
//!   -H "X-API-Key: secret-key-123" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//! ```

use async_trait::async_trait;
use clap::Parser;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::prelude::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "8672")]
    port: u16,
}

/// Authentication middleware that validates X-API-Key header
struct AuthMiddleware {
    /// Valid API keys mapped to user IDs (in production, use a database)
    valid_keys: HashMap<String, String>,
}

impl AuthMiddleware {
    fn new() -> Self {
        let mut valid_keys = HashMap::new();
        valid_keys.insert("secret-key-123".to_string(), "user-alice".to_string());
        valid_keys.insert("secret-key-456".to_string(), "user-bob".to_string());

        Self { valid_keys }
    }
}

#[async_trait]
impl McpMiddleware for AuthMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn turul_mcp_session_storage::SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Skip authentication for initialize method (required for session creation)
        if ctx.method() == "initialize" {
            tracing::debug!("Skipping auth for initialize method");
            return Ok(());
        }

        // Extract X-API-Key from request metadata (HTTP headers are stored here)
        let api_key = ctx.metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str());

        match api_key {
            Some(key) => {
                // Validate API key
                if let Some(user_id) = self.valid_keys.get(key) {
                    tracing::info!("Authenticated user: {}", user_id);

                    // Store user_id in session state for tools to access
                    injection.set_state("user_id", json!(user_id));
                    injection.set_state("authenticated", json!(true));

                    // Store API key scope in metadata
                    injection.set_metadata("api_key_scope", json!("read_write"));

                    Ok(())
                } else {
                    tracing::warn!("Invalid API key provided");
                    Err(MiddlewareError::Unauthenticated(
                        "Invalid API key".to_string(),
                    ))
                }
            }
            None => {
                tracing::warn!("Missing X-API-Key header");
                Err(MiddlewareError::Unauthenticated(
                    "Missing X-API-Key header".to_string(),
                ))
            }
        }
    }

    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        tracing::debug!("Request {} completed", ctx.method());
        Ok(())
    }
}

/// Tool that reads the authenticated user_id from session
#[derive(McpTool, Clone, Default)]
#[tool(name = "whoami", description = "Returns the authenticated user ID from the session")]
struct WhoAmITool {}

impl WhoAmITool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or_else(|| McpError::InvalidRequest {
            message: "Session required".to_string()
        })?;

        // Read user_id from session state (written by AuthMiddleware)
        let user_id = session
            .get_typed_state::<String>("user_id")
            .await
            .ok_or_else(|| McpError::InvalidRequest {
                message: "Not authenticated".to_string()
            })?;

        Ok(json!({
            "user_id": user_id,
            "message": format!("You are logged in as: {}", user_id)
        }))
    }
}

#[tokio::main]
async fn main() -> McpResult<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("middleware_auth_server=debug,turul_mcp_server=debug,turul_http_mcp_server=debug")
        .init();

    tracing::info!("Starting middleware-auth-server example on port {}", args.port);
    tracing::info!("Valid API keys:");
    tracing::info!("  - secret-key-123 (user-alice)");
    tracing::info!("  - secret-key-456 (user-bob)");

    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", args.port)
        .parse()
        .expect("Failed to parse bind address");

    let server = McpServer::builder()
        .name("middleware-auth-server")
        .version("1.0.0")
        .title("Authentication Middleware Example")
        .instructions(
            "This server demonstrates middleware-based authentication. \
             Include X-API-Key header with valid key (secret-key-123 or secret-key-456). \
             Use 'whoami' tool to see your authenticated user ID."
        )
        // Register authentication middleware FIRST (executes before other middleware)
        .middleware(Arc::new(AuthMiddleware::new()))
        // Register tools that can access session state
        .tool(WhoAmITool::default())
        .bind_address(bind_address)
        .build()?;

    tracing::info!("Server listening on http://localhost:{}/mcp", args.port);
    tracing::info!("");
    tracing::info!("Try:");
    tracing::info!("  curl -X POST http://localhost:{}/mcp \\", args.port);
    tracing::info!("    -H 'Content-Type: application/json' \\");
    tracing::info!("    -H 'Accept: application/json' \\");
    tracing::info!("    -H 'X-API-Key: secret-key-123' \\");
    tracing::info!("    -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}'");

    server.run().await
}
