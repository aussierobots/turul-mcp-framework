//! Middleware Authentication Example for AWS Lambda
//!
//! This example demonstrates middleware-based authentication in Lambda.
//! The middleware:
//! 1. Extracts the X-API-Key header from Lambda requests
//! 2. Validates the API key (hardcoded for demo)
//! 3. Stores the authenticated user_id in session state
//! 4. Tools can read the user_id from session
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
use tracing::{error, info};
use turul_http_mcp_server::middleware::{
    DispatcherResult, McpMiddleware, MiddlewareError, MiddlewareStack, RequestContext,
    SessionInjection,
};
use turul_mcp_protocol::McpError;
use turul_mcp_session_storage::DynamoDbSessionStorage;

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
            info!("Skipping auth for initialize method");
            return Ok(());
        }

        // Extract X-API-Key from request metadata
        let api_key = ctx.metadata().get("x-api-key").and_then(|v| v.as_str());

        match api_key {
            Some(key) => {
                // Validate API key
                if let Some(user_id) = self.valid_keys.get(key) {
                    info!("✅ Authenticated user: {}", user_id);

                    // Store user_id in session state for tools to access
                    injection.set_state("user_id", json!(user_id));
                    injection.set_state("authenticated", json!(true));

                    // Store API key scope in metadata
                    injection.set_metadata("api_key_scope", json!("read_write"));

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
        info!("Request {} completed", ctx.method());
        Ok(())
    }
}

/// Initialize CloudWatch-optimized logging for Lambda
fn init_logging() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());

    tracing_subscriber::fmt()
        .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
        .with_target(false) // No target for CloudWatch
        .without_time() // CloudWatch adds timestamps
        .json() // Structured logging for CloudWatch
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

    // Create handler (could be cached globally for better performance)
    let handler = create_lambda_mcp_handler()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    // Process request through the Lambda MCP handler
    handler.handle(request).await.map_err(|e| {
        error!("❌ Lambda MCP handler error: {}", e);
        Error::from(e.to_string())
    })
}

/// Create the Lambda MCP handler with authentication middleware
async fn create_lambda_mcp_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    use turul_http_mcp_server::{ServerConfig, StreamConfig, StreamManager};
    use turul_mcp_json_rpc_server::JsonRpcDispatcher;
    use turul_mcp_protocol::ServerCapabilities;

    info!("🔧 Creating Lambda MCP handler with auth middleware");

    // Create DynamoDB session storage
    let storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(format!("Failed to create DynamoDB storage: {}", e)))?,
    );

    info!("💾 DynamoDB session storage initialized");

    // Create middleware stack with authentication
    let mut middleware_stack = MiddlewareStack::new();
    middleware_stack.push(Arc::new(AuthMiddleware::new()));

    info!("🔐 Authentication middleware registered");
    info!("Valid API keys:");
    info!("  - secret-key-123 (user-alice)");
    info!("  - secret-key-456 (user-bob)");

    // NOTE: For production with tools, use LambdaMcpServerBuilder then manually construct handler.
    // This example focuses on demonstrating middleware authentication in Lambda.
    // To add tools, build a server with .tool(), get the dispatcher, then use with_middleware().

    // Create basic dispatcher (tools would be registered here in production)
    let dispatcher = JsonRpcDispatcher::<McpError>::new();

    info!("🔧 Dispatcher created (add tools via builder in production)");

    // Create server configuration
    let config = ServerConfig::default();
    let stream_config = StreamConfig::default();
    let stream_manager = Arc::new(StreamManager::new(Arc::clone(&storage)));

    // Build capabilities
    let capabilities = ServerCapabilities::default();

    // Create Lambda MCP handler with middleware
    // This is the key method - same middleware stack works in both HTTP and Lambda!
    let handler = turul_mcp_aws_lambda::LambdaMcpHandler::with_middleware(
        config,
        Arc::new(dispatcher),
        storage,
        stream_manager,
        stream_config,
        capabilities,
        Arc::new(middleware_stack),
        false, // sse_enabled
    );

    info!("✅ Lambda MCP handler created successfully with middleware");
    Ok(handler)
}

/// Main Lambda entry point
#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();

    info!("🚀 Starting AWS Lambda MCP Server with Authentication Middleware");
    info!("Architecture: MCP 2025-06-18 with middleware auth layer");
    info!("  - X-API-Key header validation");
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
