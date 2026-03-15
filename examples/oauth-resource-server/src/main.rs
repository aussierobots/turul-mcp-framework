//! OAuth 2.1 Resource Server Example
//!
//! Demonstrates an MCP server that validates Bearer tokens against an external
//! Authorization Server using JWKS. The server acts as an OAuth 2.1 Resource
//! Server (RS) per RFC 9728.
//!
//! # Architecture
//!
//! - External AS issues JWTs (this server does NOT issue tokens)
//! - Bearer tokens are validated via JWKS fetched from the AS
//! - RFC 9728 metadata is served at `/.well-known/oauth-protected-resource`
//! - Auth claims are available to tools via `SessionContext.extensions`
//!
//! # Usage
//!
//! ```bash
//! # Start server (requires a running Authorization Server with JWKS endpoint)
//! cargo run --bin oauth-resource-server -- \
//!   --jwks-uri https://auth.example.com/.well-known/jwks.json \
//!   --resource https://example.com/mcp \
//!   --auth-server https://auth.example.com
//!
//! # The server exposes:
//! #   POST /mcp                                          — MCP endpoint (requires Bearer token)
//! #   GET  /.well-known/oauth-protected-resource         — RFC 9728 metadata (root form)
//! #   GET  /.well-known/oauth-protected-resource/mcp     — RFC 9728 metadata (path form)
//!
//! # Discover the authorization server:
//! curl http://localhost:8080/.well-known/oauth-protected-resource
//!
//! # Call with a valid Bearer token:
//! curl -X POST http://localhost:8080/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json" \
//!   -H "Authorization: Bearer <JWT>" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//! ```

use clap::Parser;
use serde_json::json;
use turul_mcp_derive::McpTool;
use turul_mcp_oauth::{ProtectedResourceMetadata, TokenClaims};
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::prelude::*;

#[derive(Parser)]
#[command(author, version, about = "OAuth 2.1 Resource Server MCP Example")]
struct Args {
    #[arg(long, default_value = "8080")]
    port: u16,

    /// JWKS URI of the Authorization Server
    #[arg(long, default_value = "https://auth.example.com/.well-known/jwks.json")]
    jwks_uri: String,

    /// Resource identifier (this server's URL)
    #[arg(long, default_value = "https://example.com/mcp")]
    resource: String,

    /// Authorization Server URL
    #[arg(long, default_value = "https://auth.example.com")]
    auth_server: String,
}

/// Tool that reads authenticated user claims from the Bearer token
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "whoami",
    description = "Returns the authenticated user identity from the Bearer token"
)]
struct WhoAmITool {}

impl WhoAmITool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or_else(|| McpError::InvalidRequest {
            message: "Session required".to_string(),
        })?;

        // Read auth claims from extensions (written by OAuthResourceMiddleware)
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
            "message": format!("Authenticated as: {}", claims.sub),
        }))
    }
}

#[tokio::main]
async fn main() -> McpResult<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            "oauth_resource_server=debug,turul_mcp_server=debug,turul_http_mcp_server=debug",
        )
        .init();

    // Configure Protected Resource Metadata (RFC 9728)
    let metadata = ProtectedResourceMetadata::new(&args.resource, vec![args.auth_server.clone()])
        .map_err(|e| McpError::InvalidRequest {
            message: format!("Invalid OAuth metadata: {}", e),
        })?
        .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);

    // Create OAuth middleware + well-known route handlers (root + path form per RFC 9728 §3)
    let (auth_middleware, routes) =
        turul_mcp_oauth::oauth_resource_server(metadata, &args.jwks_uri).map_err(|e| {
            McpError::InvalidRequest {
                message: format!("OAuth setup failed: {}", e),
            }
        })?;

    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", args.port)
        .parse()
        .expect("Failed to parse bind address");

    let mut builder = McpServer::builder()
        .name("oauth-resource-server")
        .version("0.3.19")
        .title("OAuth 2.1 Resource Server Example")
        .instructions(
            "This server requires a valid Bearer token from the configured \
             Authorization Server. Discover the AS via \
             /.well-known/oauth-protected-resource.",
        )
        // OAuth middleware validates Bearer tokens before session creation
        .middleware(auth_middleware)
        .tool(WhoAmITool::default())
        .bind_address(bind_address);

    // Register all RFC 9728 metadata routes (root form + path form)
    for (path, handler) in routes {
        builder = builder.route(&path, handler);
    }

    let server = builder.build()?;

    tracing::info!(
        "OAuth Resource Server listening on http://localhost:{}/mcp",
        args.port
    );
    tracing::info!("JWKS URI: {}", args.jwks_uri);
    tracing::info!("Resource: {}", args.resource);
    tracing::info!("Auth Server: {}", args.auth_server);
    tracing::info!(
        "Metadata: http://localhost:{}/.well-known/oauth-protected-resource",
        args.port
    );

    server.run().await
}
