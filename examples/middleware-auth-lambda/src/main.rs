//! Middleware Authentication Example for AWS Lambda (Streamable HTTP + REST/HTTP API)
//!
//! This example demonstrates middleware-based authentication in Lambda with
//! API Gateway authorizer context integration. It supports all three authorizer
//! context shapes that API Gateway can produce:
//!
//! - **V1 Nested**: REST API with `requestContext.authorizer.lambda.{field}` (standard Lambda proxy)
//! - **V1 Flat**: REST API with `requestContext.authorizer.{field}` (simple Lambda authorizer)
//! - **V2**: HTTP API with `requestContext.authorizer.{field}` (HTTP API authorizer)
//!
//! The middleware:
//! 1. Extracts the X-API-Key header from Lambda requests
//! 2. Validates the API key (hardcoded for demo)
//! 3. Extracts Lambda authorizer context (x-authorizer-* headers)
//! 4. Stores the authenticated user_id and authorizer data in session state
//! 5. Tools can read the user_id and context from session
//!
//! # Transport: Streamable HTTP (REST API V1)
//!
//! This example uses the MCP 2026-07-28 Streamable HTTP transport via REST API (V1).
//! REST API supports standard HTTP POST with full request/response control, making it
//! compatible with Streamable HTTP. The Lambda adapter converts the API Gateway event
//! into a standard `hyper::Request`, which the framework's `StreamableHttpHandler`
//! processes normally. The 2026 core is stateless: there is no `initialize`/
//! `notifications/initialized` handshake and no `Mcp-Session-Id` — every request
//! carries its own `_meta` (protocolVersion, clientInfo, clientCapabilities).
//!
//! **Note**: HTTP API (V2) authorizer context extraction is fully supported, but
//! Streamable HTTP transport requires REST API (V1).
//!
//! # Deployment
//!
//! ```bash
//! # Build for Lambda
//! cargo lambda build --release --package middleware-auth-lambda
//!
//! # Deploy to AWS
//! cargo lambda deploy middleware-auth-lambda
//!
//! # Test locally
//! cargo lambda watch --package middleware-auth-lambda
//! ```
//!
//! # How Authorizer Context Works
//!
//! **Pattern**: API Gateway Authorizer → Lambda Extensions → Middleware → Session State
//!
//! 1. API Gateway authorizer adds context (userId, tenantId, role, etc.)
//! 2. turul-mcp-aws-lambda adapter extracts context → injects `x-authorizer-*` headers
//! 3. Middleware reads headers → stores in session state
//! 4. Your tools access via `session.get_typed_state("authorizer")`
//!
//! # API Gateway Authorizer Context Shapes
//!
//! The adapter handles three distinct JSON shapes from API Gateway:
//!
//! **V1 Nested** (REST API, Lambda proxy integration):
//! ```json
//! { "requestContext": { "authorizer": { "lambda": { "userId": "user-123" } } } }
//! ```
//!
//! **V1 Flat** (REST API, simple Lambda authorizer):
//! ```json
//! { "requestContext": { "authorizer": { "userId": "user-123", "principalId": "..." } } }
//! ```
//! Internal fields (`principalId`, `integrationLatency`, `usageIdentifierKey`) are
//! filtered out automatically — only your custom context fields are extracted.
//!
//! **V2** (HTTP API):
//! ```json
//! { "requestContext": { "authorizer": { "userId": "user-123" } } }
//! ```
//!
//! **Example tool using authorizer context**:
//! ```rust,ignore
//! #[mcp_tool(name = "get_account", description = "Get account info")]
//! async fn get_account(
//!     #[param(session)] session: SessionContext,
//! ) -> McpResult<serde_json::Value> {
//!     // Read authorizer context from session (fields are snake_case)
//!     let authorizer: Option<HashMap<String, String>> =
//!         session.get_typed_state("authorizer").await.ok().flatten();
//!
//!     // Field names are converted: "userId" → "user_id" (snake_case)
//!     let user_id = authorizer
//!         .and_then(|ctx| ctx.get("user_id").cloned())  // snake_case!
//!         .ok_or_else(|| McpError::validation("Missing user_id from authorizer"))?;
//!
//!     Ok(json!({ "userId": user_id }))
//! }
//! ```
//!
//! # Usage
//!
//! ```bash
//! # With valid API key (2026-07-28 stateless: no session handshake;
//! # each request carries its own `_meta`)
//! curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json" \
//!   -H "MCP-Protocol-Version: 2026-07-28" \
//!   -H "X-API-Key: secret-key-123" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}}}'
//!
//! # Without API key (should fail at the auth layer)
//! curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json" \
//!   -H "MCP-Protocol-Version: 2026-07-28" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}}}'
//! ```

use async_trait::async_trait;
use lambda_http::{Body, Error, Request, Response, run, service_fn};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};
use turul_http_mcp_server::middleware::{
    DispatcherResult, McpMiddleware, MiddlewareError, RequestContext, SessionInjection,
};
use turul_mcp_server::prelude::*;

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
        // Skip authentication for ping health checks only
        if ctx.method() == "ping" {
            debug!("Skipping auth for {} method", ctx.method());
            return Ok(());
        }

        // Extract X-API-Key from request metadata
        let api_key = ctx.metadata().get("x-api-key").and_then(|v| v.as_str());

        match api_key {
            Some(key) => {
                // Validate API key
                if let Some(user_id) = self.valid_keys.get(key) {
                    debug!("✅ Authenticated user: {}", user_id);

                    // Store user_id in session state for tools to access
                    injection.set_state("user_id", json!(user_id));
                    injection.set_state("authenticated", json!(true));

                    // Store API key scope in metadata
                    injection.set_metadata("api_key_scope", json!("read_write"));

                    // Extract Lambda authorizer context from x-authorizer-* headers
                    // These are injected by turul-mcp-aws-lambda adapter
                    let metadata: &serde_json::Map<String, serde_json::Value> = ctx.metadata();
                    let mut authorizer_context = HashMap::new();

                    // Iterate over metadata entries
                    for (key, value) in metadata.iter() {
                        if let Some(field_name) = key.strip_prefix("x-authorizer-")
                            && let Some(value_str) = value.as_str()
                        {
                            debug!("📋 Authorizer context: {} = {}", field_name, value_str);
                            authorizer_context
                                .insert(field_name.to_string(), value_str.to_string());
                        }
                    }

                    if !authorizer_context.is_empty() {
                        debug!(
                            "✅ Extracted {} authorizer fields",
                            authorizer_context.len()
                        );
                        injection.set_state("authorizer", json!(authorizer_context));
                    }

                    Ok(())
                } else {
                    error!("❌ Invalid API key provided");
                    Err(MiddlewareError::Unauthenticated(
                        "Invalid API key".to_string(),
                    ))
                }
            }
            None => {
                error!("❌ Missing X-API-Key header");
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
        debug!("Request {} completed", ctx.method());
        Ok(())
    }
}

/// Initialize logging
fn init_logging() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());

    tracing_subscriber::fmt()
        .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
        .with_env_filter("middleware_auth_lambda=info,turul_mcp_server=info")
        .init();

    info!("🚀 Logging initialized at level: {}", log_level);
}

/// Lambda handler function — receives a clone of the prebuilt handler per request.
async fn lambda_handler(
    handler: turul_mcp_aws_lambda::LambdaMcpHandler,
    request: Request,
) -> Result<Response<Body>, Error> {
    debug!(
        method = %request.method(),
        path = %request.uri().path(),
        "Lambda MCP request"
    );
    handler.handle(request).await.map_err(|e| {
        error!("❌ Lambda MCP handler error: {}", e);
        Error::from(e.to_string())
    })
}

/// Create the Lambda MCP handler with authentication middleware
async fn create_lambda_mcp_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    use turul_mcp_session_storage::DynamoDbSessionStorage;

    info!("🔧 Creating Lambda MCP handler with auth middleware");

    // DynamoDB backs the framework's internal per-request contexts and event
    // streams on the 2026 stateless lane — there is no client-visible session.
    // Each request writes an ephemeral row; for auth-only deployments the
    // in-memory default avoids that per-request DynamoDB cost.
    let storage = Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(format!("Failed to create DynamoDB storage: {}", e)))?,
    );

    info!("💾 DynamoDB storage initialized (per-request internal contexts)");

    // Create authentication middleware
    let auth_middleware = Arc::new(AuthMiddleware::new());

    info!("🔐 Authentication middleware registered");
    info!("Valid API keys:");
    info!("  - secret-key-123 (user-alice)");
    info!("  - secret-key-456 (user-bob)");

    // Build server with middleware using builder pattern
    let server = turul_mcp_aws_lambda::LambdaMcpServerBuilder::new()
        .name("middleware-auth-lambda")
        .version("1.0.0")
        .middleware(auth_middleware)
        .storage(storage)
        .cors_allow_all_origins()
        .build()
        .await
        .map_err(|e| Error::from(format!("{}", e)))?;

    info!("✅ Lambda MCP server created successfully with middleware and CORS");

    // Create handler from server
    server
        .handler()
        .await
        .map_err(|e| Error::from(format!("{}", e)))
}

/// Main Lambda entry point
#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();

    info!("🚀 Starting AWS Lambda MCP Server with Authentication Middleware");
    info!("Architecture: MCP 2026-07-28 (stateless) with middleware auth layer");
    info!("  - X-API-Key header validation");
    info!("  - Lambda authorizer context extraction");
    info!("  - User context injection");
    info!("  - DynamoDB-backed per-request contexts (no client sessions on 2026)");
    info!("  - CORS support");

    info!("📋 Environment variables:");
    info!(
        "  - LOG_LEVEL: {}",
        env::var("LOG_LEVEL").unwrap_or("INFO".to_string())
    );
    info!(
        "  - AWS_REGION: {}",
        env::var("AWS_REGION").unwrap_or("us-east-1".to_string())
    );
    info!(
        "  - MCP_SESSION_TABLE: {}",
        env::var("MCP_SESSION_TABLE").unwrap_or("mcp-sessions".to_string())
    );

    // Build the handler eagerly in main() so DDB session storage init,
    // server build, and tool registration land in Lambda's Init Duration
    // — not inside the first invocation's handler_total.
    let handler = create_lambda_mcp_handler().await?;
    info!("🎯 Lambda handler ready with auth middleware");

    // Run Lambda HTTP runtime (non-streaming).
    // For streaming with completion-invocation handling, use:
    //   turul_mcp_aws_lambda::run_streaming(handler).await          // standard
    //   turul_mcp_aws_lambda::run_streaming_with(my_dispatch).await  // custom dispatch
    run(service_fn(move |req| lambda_handler(handler.clone(), req))).await
}
