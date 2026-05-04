//! AWS Lambda MCP Server (Non-Streaming)
//!
//! A complete MCP server for AWS Lambda with:
//! - turul-mcp-aws-lambda integration (snapshot-based SSE)
//! - MCP 2025-11-25 compliance with SSE notifications
//! - DynamoDB session storage with automatic table creation
//! - CORS support for browser clients
//! - AWS tools integration (DynamoDB, SNS, SQS, CloudWatch)
//!
//! ## SSE Support
//!
//! This version uses SSE snapshot approach - returns recent events when requested
//! rather than real-time streaming. This is compatible with standard Lambda
//! runtime and doesn't require `run_with_streaming_response`.
//!
//! **Note**: For real-time SSE streaming, see the `lambda-mcp-server-streaming` example
//! which uses `run_with_streaming_response` (may incur higher Lambda costs).

mod session_aware_logging_demo;
mod tools;

use lambda_http::{Body, Error, Request, run, service_fn};
use std::env;
use tracing::{debug, error, info};

// Framework imports
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_session_storage::DynamoDbSessionStorage;

// Local imports
use session_aware_logging_demo::{
    CheckLoggingStatusTool, SessionLoggingDemoTool, SetLoggingLevelTool,
};
use tools::{CloudWatchMetricsTool, DynamoDbQueryTool, SnsPublishTool, SqsSendMessageTool};

/// Initialize CloudWatch-optimized logging for Lambda environment
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

/// Lambda handler function — receives a clone of the prebuilt handler per request.
async fn lambda_handler(
    handler: turul_mcp_aws_lambda::LambdaMcpHandler,
    request: Request,
) -> Result<lambda_http::Response<Body>, Error> {
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
        // Disable SSE for snapshot-only mode (compatible with non-streaming runtime)
        .sse(false)
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

    info!("🚀 Starting AWS Lambda MCP Server (non-SSE mode)");
    info!("Architecture: MCP 2025-11-25 JSON-RPC compliance");
    info!("  - turul-mcp-aws-lambda integration");
    info!("  - DynamoDB session storage");
    info!("  - CORS support");
    info!("  - POST /mcp - JSON-RPC requests");
    info!("  - GET /mcp - 405 Method Not Allowed (SSE disabled)");
    info!("  - OPTIONS * - CORS preflight");
    info!("  - For SSE support, use lambda-mcp-server-streaming with streaming features");

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
    // — not inside the first invocation's handler_total. See ADR-024.
    let handler = create_lambda_mcp_handler().await?;
    info!("🎯 Lambda handler ready (snapshot-based SSE)");

    run(service_fn(move |req| lambda_handler(handler.clone(), req))).await
}
