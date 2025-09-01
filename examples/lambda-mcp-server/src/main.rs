//! AWS Lambda MCP Server using turul-mcp-aws-lambda
//!
//! A complete MCP server for AWS Lambda with:
//! - turul-mcp-aws-lambda integration
//! - MCP 2025-06-18 compliance  
//! - DynamoDB session storage
//! - CORS support
//! - AWS tools integration

mod tools;

use lambda_http::{run, service_fn, Error, Request, Body};
use std::env;
use tracing::{error, info};
use tracing_subscriber;

// Framework imports
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_session_storage::DynamoDbSessionStorage;

// Local imports
use tools::{DynamoDbQueryTool, SnsPublishTool, SqsSendMessageTool, CloudWatchMetricsTool};

/// Initialize CloudWatch-optimized logging for Lambda environment
fn init_logging() {
    let log_level = env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "INFO".to_string());
        
    tracing_subscriber::fmt()
        .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
        .with_target(false)      // No target for CloudWatch  
        .without_time()          // CloudWatch adds timestamps
        .json()                  // Structured logging for CloudWatch
        .init();
        
    info!("🚀 Logging initialized at level: {}", log_level);
}

/// Lambda handler function using turul-mcp-aws-lambda
async fn lambda_handler(
    request: Request
) -> Result<lambda_http::Response<Body>, Error> {
    info!("🌐 Lambda MCP request: {} {}", 
        request.method(), 
        request.uri().path()
    );

    // Create handler (could be cached globally for better performance)
    let handler = create_lambda_mcp_handler().await?;
    
    // Process request through the Lambda MCP handler
    handler.handle(request).await
        .map_err(|e| {
            error!("❌ Lambda MCP handler error: {}", e);
            Error::from(e.to_string())
        })
}

/// Create the Lambda MCP handler with AWS tools
async fn create_lambda_mcp_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    info!("🔧 Creating Lambda MCP handler with AWS tools");
    
    // Create DynamoDB session storage
    let storage = std::sync::Arc::new(
        DynamoDbSessionStorage::new().await
            .map_err(|e| Error::from(format!("Failed to create DynamoDB storage: {}", e)))?
    );
    
    info!("💾 DynamoDB session storage initialized");
    
    // Build Lambda MCP handler with all AWS tools
    let handler = LambdaMcpServerBuilder::new()
        .name("aws-lambda-mcp-server")
        .version("1.0.0")
        // AWS Lambda tools
        .tool(DynamoDbQueryTool::default())
        .tool(SnsPublishTool::default())
        .tool(SqsSendMessageTool::default())
        .tool(CloudWatchMetricsTool::default())
        // Session storage
        .storage(storage)
        // CORS configuration
        .cors_allow_all_origins()
        .build()
        .await
        .map_err(|e| Error::from(format!("Failed to build Lambda MCP handler: {}", e)))?;
    
    info!("✅ Lambda MCP handler created successfully");
    Ok(handler)
}

/// Main Lambda entry point
#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();

    info!("🚀 Starting AWS Lambda MCP Server with turul-mcp-aws-lambda");
    info!("Architecture: MCP 2025-06-18 Streamable HTTP compliance");
    info!("  - turul-mcp-aws-lambda integration");
    info!("  - DynamoDB session storage");
    info!("  - CORS support");
    info!("  - POST /mcp - JSON-RPC requests");
    info!("  - GET /mcp - SSE streaming");
    info!("  - OPTIONS * - CORS preflight");

    info!("📋 Environment variables:");
    info!("  - LOG_LEVEL: {}", env::var("LOG_LEVEL").unwrap_or("INFO".to_string()));
    info!("  - AWS_REGION: {}", env::var("AWS_REGION").unwrap_or("us-east-1".to_string()));
    info!("  - MCP_SESSION_TABLE: {}", env::var("MCP_SESSION_TABLE").unwrap_or("mcp-sessions".to_string()));
    
    info!("🎯 Lambda handler ready");

    // Run Lambda HTTP runtime
    run(service_fn(lambda_handler)).await
}