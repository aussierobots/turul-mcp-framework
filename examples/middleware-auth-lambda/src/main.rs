//! Middleware Authentication Example for AWS Lambda
//!
//! This example demonstrates middleware-based authentication in Lambda with
//! API Gateway authorizer context integration.
//!
//! The middleware:
//! 1. Extracts the X-API-Key header from Lambda requests
//! 2. Validates the API key (hardcoded for demo)
//! 3. Extracts Lambda authorizer context (x-authorizer-* headers)
//! 4. Stores the authenticated user_id and authorizer data in session state
//! 5. Tools can read the user_id and context from session
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
//! # With valid API key
//! curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json" \
//!   -H "X-API-Key: secret-key-123" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//!
//! # Without API key (should fail)
//! curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
//! ```

use async_trait::async_trait;
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::OnceCell;
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
        // Skip authentication for initialize method (required for session creation)
        if ctx.method() == "initialize" {
            debug!("Skipping auth for initialize method");
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
                        if let Some(field_name) = key.strip_prefix("x-authorizer-") {
                            if let Some(value_str) = value.as_str() {
                                debug!("📋 Authorizer context: {} = {}", field_name, value_str);
                                authorizer_context.insert(field_name.to_string(), value_str.to_string());
                            }
                        }
                    }

                    if !authorizer_context.is_empty() {
                        debug!("✅ Extracted {} authorizer fields", authorizer_context.len());
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

/// Global handler instance (created once, reused across invocations)
static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

/// Initialize logging
fn init_logging() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());

    tracing_subscriber::fmt()
        .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
        .with_env_filter("middleware_auth_lambda=info,turul_mcp_server=info")
        .init();

    info!("🚀 Logging initialized at level: {}", log_level);
}

/// Lambda handler function
async fn lambda_handler(request: Request) -> Result<Response<Body>, Error> {
    info!(
        "🌐 Lambda MCP request: {} {}",
        request.method(),
        request.uri().path()
    );

    // Get or create handler (cached globally for session persistence)
    let handler = HANDLER
        .get_or_try_init(|| async { create_lambda_mcp_handler().await })
        .await?;

    // Process request through the Lambda MCP handler
    handler
        .handle(request)
        .await
        .map_err(|e| Error::from(e.to_string()))
}

/// Create the Lambda MCP handler with authentication middleware
async fn create_lambda_mcp_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    use turul_mcp_session_storage::DynamoDbSessionStorage;

    info!("🔧 Creating Lambda MCP handler with auth middleware");

    // Create DynamoDB session storage
    let storage = Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(format!("Failed to create DynamoDB storage: {}", e)))?,
    );

    info!("💾 DynamoDB session storage initialized");

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
    info!("Architecture: MCP 2025-06-18 with middleware auth layer");
    info!("  - X-API-Key header validation");
    info!("  - Lambda authorizer context extraction");
    info!("  - User context injection");
    info!("  - DynamoDB session storage");
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

    info!("🎯 Lambda handler ready with auth middleware");

    // Run Lambda HTTP runtime
    run(service_fn(lambda_handler)).await
}
