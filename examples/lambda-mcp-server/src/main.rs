//! AWS Lambda MCP Server (Non-Streaming)
//!
//! A complete MCP server for AWS Lambda with:
//! - turul-mcp-aws-lambda integration (snapshot-based SSE)
//! - MCP 2025-06-18 compliance with SSE notifications
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
use tokio::sync::OnceCell;
use tracing::{error, info};

// Framework imports
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_session_storage::DynamoDbSessionStorage;

// Local imports
use session_aware_logging_demo::{
    CheckLoggingStatusTool, SessionLoggingDemoTool, SetLoggingLevelTool,
};
use tools::{CloudWatchMetricsTool, DynamoDbQueryTool, SnsPublishTool, SqsSendMessageTool};

/// Global handler instance - created once per Lambda container, reused across invocations
static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

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

/// Lambda handler function using turul-mcp-aws-lambda
async fn lambda_handler(request: Request) -> Result<lambda_http::Response<Body>, Error> {
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
    info!("Architecture: MCP 2025-06-18 JSON-RPC compliance");
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

    info!("🎯 Lambda handler ready (snapshot-based SSE)");

    // Run Lambda HTTP runtime (regular, non-streaming)
    run(service_fn(lambda_handler)).await
}
