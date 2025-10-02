//! AWS Lambda MCP Server with Streaming SSE Support
//!
//! A complete streaming-enabled MCP server for AWS Lambda with:
//! - Real-time SSE streaming (requires `run_with_streaming_response`)
//! - turul-mcp-aws-lambda integration with "streaming" feature enabled
//! - MCP 2025-06-18 compliance with proper SSE notifications
//! - DynamoDB session storage with automatic table creation
//! - CORS support for browser clients
//! - AWS tools integration (DynamoDB, SNS, SQS, CloudWatch)
//!
//! ## SSE Streaming Support
//!
//! This example demonstrates real-time SSE streaming in Lambda using:
//! - `lambda_http::run_with_streaming_response` runtime
//! - `handle_streaming()` method for streaming-compatible responses
//! - StreamManager integration for proper SSE event delivery
//!
//! For non-streaming Lambda deployments, see `lambda-mcp-server` example.

mod session_aware_logging_demo;
mod tools;

use lambda_http::{Error, Request, run_with_streaming_response, service_fn};
use std::env;
use tokio::sync::OnceCell;
use tracing::info;

// HTTP body types for streaming
use bytes;
use http_body_util;
use hyper;

// Framework imports
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_session_storage::DynamoDbSessionStorage;

// Local imports
use session_aware_logging_demo::{
    CheckLoggingStatusTool, SessionLoggingDemoTool, SetLoggingLevelTool,
};
use tools::{
    CloudWatchMetricsTool, DynamoDbQueryTool, EchoTool, SnsPublishTool, SqsSendMessageTool,
};

/// Initialize logging - JSON for Lambda/CloudWatch, human-readable for local development
fn init_logging() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());

    if env::var("AWS_EXECUTION_ENV").is_ok() {
        // Running in Lambda → JSON for CloudWatch
        tracing_subscriber::fmt()
            .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
            .with_target(false) // No target for CloudWatch
            .without_time() // CloudWatch adds timestamps
            .json() // Structured logging for CloudWatch
            .init();
    } else {
        // Running locally → human readable
        tracing_subscriber::fmt()
            .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
            .init();
    }

    info!("🚀 Logging initialized at level: {}", log_level);
}

/// Global handler instance - created once and reused across all Lambda invocations
static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

/// Lambda handler function using turul-mcp-aws-lambda with streaming support
async fn lambda_handler(
    request: Request,
) -> Result<
    lambda_http::Response<http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    info!(
        "🌊 Lambda streaming MCP request: {} {}",
        request.method(),
        request.uri().path()
    );

    // Get or initialize handler (cached globally for performance)
    let handler = HANDLER
        .get_or_try_init(|| async { create_lambda_mcp_handler().await })
        .await?;

    // Process request through the Lambda MCP streaming handler
    handler.handle_streaming(request).await
}

/// Create the Lambda MCP handler with AWS tools
async fn create_lambda_mcp_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    info!("🔧 Creating Lambda MCP handler with AWS tools");

    // Create DynamoDB session storage
    let storage = std::sync::Arc::new(
        DynamoDbSessionStorage::new()
            .await
            .map_err(|e| Error::from(format!("Failed to create DynamoDB storage: {}", e)))?,
    );

    info!("💾 DynamoDB session storage initialized");

    // Build Lambda MCP server with all AWS tools
    let server = LambdaMcpServerBuilder::new()
        .name("aws-lambda-mcp-server")
        .version("1.0.0")
        // Echo tool (for testing notifications)
        .tool(EchoTool::default())
        // AWS Lambda tools
        .tool(DynamoDbQueryTool::default())
        .tool(SnsPublishTool::default())
        .tool(SqsSendMessageTool::default())
        .tool(CloudWatchMetricsTool::default())
        // Session-aware logging demo tools
        .tool(SessionLoggingDemoTool::default())
        .tool(SetLoggingLevelTool::default())
        .tool(CheckLoggingStatusTool::default())
        // Session storage
        .storage(storage)
        // Enable SSE streaming
        .sse(true)
        // CORS configuration
        .cors_allow_all_origins()
        .build()
        .await
        .map_err(|e| Error::from(format!("Failed to build Lambda MCP server: {}", e)))?;

    // Create handler from server
    let handler = server
        .handler()
        .await
        .map_err(|e| Error::from(format!("Failed to create Lambda MCP handler: {}", e)))?;

    info!("✅ Lambda MCP handler created successfully");
    Ok(handler)
}

/// Main Lambda entry point
#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();

    info!("🚀 Starting AWS Lambda MCP Server with STREAMING support");
    info!("Architecture: MCP 2025-06-18 Streamable HTTP compliance with real-time SSE");
    info!("  - turul-mcp-aws-lambda integration with streaming feature");
    info!("  - DynamoDB session storage");
    info!("  - CORS support");
    info!("  - POST /mcp - JSON-RPC requests");
    info!("  - GET /mcp - Real-time SSE streaming (not snapshots)");
    info!("  - OPTIONS * - CORS preflight");
    info!("  - Lambda streaming response support enabled");

    info!("📋 Environment variables:");
    info!(
        "  - LOG_LEVEL: {}",
        env::var("LOG_LEVEL").unwrap_or("INFO".to_string())
    );
    info!(
        "  - AWS_REGION: {}",
        env::var("AWS_REGION").unwrap_or_else(|_| "(not set, will use SDK default)".to_string())
    );
    info!(
        "  - MCP_SESSION_TABLE: {}",
        env::var("MCP_SESSION_TABLE")
            .unwrap_or_else(|_| "(not set, using default 'mcp-sessions')".to_string())
    );
    info!(
        "  - MCP_SESSION_EVENT_TABLE: {}",
        env::var("MCP_SESSION_EVENT_TABLE")
            .unwrap_or_else(|_| "(not set, will use '{table_name}-events')".to_string())
    );

    // Pre-initialize handler during startup (not on first request)
    info!("⚙️  Pre-initializing MCP handler...");
    HANDLER
        .get_or_try_init(|| async { create_lambda_mcp_handler().await })
        .await?;
    info!("🎯 Lambda handler ready and initialized");

    // Run Lambda HTTP runtime with streaming response support
    run_with_streaming_response(service_fn(lambda_handler)).await
}
