// turul-mcp-server v0.3
// OAuth 2.1 Resource Server — single Authorization Server
//
// Cargo.toml dependencies:
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-oauth = { version = "0.3" }
//   turul-mcp-derive = { version = "0.3" }
//   tokio = { version = "1", features = ["full"] }
//   serde_json = "1"
//   async-trait = "0.1"
//
// This example demonstrates:
//   1. ProtectedResourceMetadata setup (RFC 9728)
//   2. oauth_resource_server() convenience function
//   3. Reading TokenClaims in tools via get_typed_extension()
//   4. Well-known endpoint registration

use serde_json::json;
use turul_mcp_derive::McpTool;
use turul_mcp_oauth::{ProtectedResourceMetadata, TokenClaims};
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::prelude::*;

// --- Tool that reads authenticated user claims ---

#[derive(McpTool, Clone, Default)]
#[tool(name = "whoami", description = "Returns the authenticated user identity")]
struct WhoAmITool {}

impl WhoAmITool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or_else(|| McpError::InvalidRequest {
            message: "Session required".to_string(),
        })?;

        // Read claims injected by OAuthResourceMiddleware.
        // This key is the current internal convention used by the middleware.
        let claims: TokenClaims = session
            .get_typed_extension("__turul_internal.auth_claims")
            .ok_or_else(|| McpError::InvalidRequest {
                message: "Not authenticated — no Bearer token claims found".to_string(),
            })?;

        let scopes: Vec<&str> = claims
            .scope
            .as_deref()
            .map(|s| s.split_whitespace().collect())
            .unwrap_or_default();

        Ok(json!({
            "subject": claims.sub,
            "issuer": claims.iss,
            "scopes": scopes,
        }))
    }
}

// --- Server setup ---

#[tokio::main]
async fn main() -> McpResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    let jwks_uri = "https://auth.example.com/.well-known/jwks.json";

    // 1. Configure RFC 9728 metadata
    let metadata = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec!["https://auth.example.com".to_string()],
    )
    .map_err(|e| McpError::InvalidRequest {
        message: format!("Invalid OAuth metadata: {}", e),
    })?
    .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);

    // 2. Create middleware + well-known routes
    let (auth_middleware, routes) =
        turul_mcp_oauth::oauth_resource_server(metadata, jwks_uri).map_err(|e| {
            McpError::InvalidRequest {
                message: format!("OAuth setup failed: {}", e),
            }
        })?;

    // 3. Build server with OAuth middleware
    let mut builder = McpServer::builder()
        .name("oauth-resource-server")
        .version("0.3.11")
        .middleware(auth_middleware)
        .tool(WhoAmITool::default());

    // 4. Register well-known routes (root form + path form)
    for (path, handler) in routes {
        builder = builder.route(&path, handler);
    }

    let server = builder.build()?;
    server.run().await
}
