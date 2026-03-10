// turul-mcp-server v0.3
// Lambda MCP server with OAuth 2.1 RS
//
// Cargo.toml dependencies:
//   turul-mcp-aws-lambda = { version = "0.3", features = ["streaming", "dynamodb"] }
//   turul-mcp-server = { version = "0.3" }
//   turul-mcp-oauth = { version = "0.3" }
//   turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
//   turul-mcp-derive = { version = "0.3" }
//   tokio = { version = "1", features = ["full"] }
//   serde_json = "1"
//
// This example demonstrates:
//   1. OAuth 2.1 RS on Lambda with DynamoDB session storage
//   2. .route() on LambdaMcpServerBuilder for RFC 9728 well-known endpoints
//   3. Cold-start caching with OnceCell
//   4. Reading TokenClaims in tools
//
// The well-known routes are registered via .route() on the builder.
// LambdaMcpHandler.handle_streaming() checks the route registry before
// dispatching to MCP, so run_streaming() works — no custom dispatch needed.

use lambda_http::Error;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::{LambdaMcpServerBuilder, run_streaming};
use turul_mcp_derive::McpTool;
use turul_mcp_oauth::{ProtectedResourceMetadata, TokenClaims, oauth_resource_server};
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::prelude::SessionContext;
use turul_mcp_session_storage::DynamoDbSessionStorage;

// --- Tool ---

#[derive(McpTool, Clone, Default)]
#[tool(name = "whoami", description = "Returns authenticated user identity")]
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
                message: "Not authenticated".to_string(),
            })?;

        Ok(json!({ "subject": claims.sub, "issuer": claims.iss }))
    }
}

// --- Handler creation (cached in OnceCell) ---

static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

async fn create_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    let jwks_uri = std::env::var("JWKS_URI")
        .unwrap_or_else(|_| "https://auth.example.com/.well-known/jwks.json".into());
    let resource_url = std::env::var("RESOURCE_URL")
        .unwrap_or_else(|_| "https://api.example.com/mcp".into());
    let auth_server = std::env::var("AUTH_SERVER")
        .unwrap_or_else(|_| "https://auth.example.com".into());

    let metadata = ProtectedResourceMetadata::new(
        &resource_url,
        vec![auth_server],
    ).map_err(|e| Error::from(e.to_string()))?;

    let (auth_middleware, routes) = oauth_resource_server(metadata, &jwks_uri)
        .map_err(|e| Error::from(e.to_string()))?;

    let storage = Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(e.to_string()))?,
    );

    // Register OAuth middleware and well-known routes on the builder.
    // handle_streaming() checks the route registry before MCP dispatch,
    // so .well-known requests are handled without custom dispatch logic.
    let mut builder = LambdaMcpServerBuilder::new()
        .name("oauth-lambda")
        .version("1.0.0")
        .middleware(auth_middleware)
        .tool(WhoAmITool::default())
        .storage(storage)
        .sse(true)
        .cors_allow_all_origins();

    for (path, handler) in routes {
        builder = builder.route(&path, handler);
    }

    let server = builder
        .build()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    server.handler().await.map_err(|e| Error::from(e.to_string()))
}

// --- Lambda entry point ---

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .json()
        .init();

    // Initialize handler on cold start
    let handler = HANDLER
        .get_or_try_init(|| async { create_handler().await })
        .await?
        .clone();

    // run_streaming() works here — well-known routes are in the route registry,
    // checked by handle_streaming() before MCP dispatch.
    run_streaming(handler).await
}
