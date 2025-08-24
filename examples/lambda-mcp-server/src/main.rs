//! AWS Lambda MCP Server using mcp-framework
//!
//! Complete Lambda MCP server with restored architecture including:
//! - DynamoDB session management
//! - Global event broadcasting via tokio channels
//! - SNS/SQS external event processing
//! - SSE streaming for real-time notifications
//! - Complete tool suite (AWS, Lambda, Session tools)

mod event_router;
mod global_events;
mod session_manager;
mod sns_processor;
mod streaming_response;
mod tools;

use event_router::EventRouter;
use global_events::initialize_global_events;
use lambda_http::{
    Body, Error, Request, RequestExt, Response, run_with_streaming_response, service_fn,
};
use lambda_runtime::Context as LambdaContext;
use sns_processor::initialize_sns_processor;
use std::env;
use std::sync::OnceLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Global EventRouter using OnceCell pattern - initialized once, reused for all requests
static EVENT_ROUTER: OnceLock<EventRouter> = OnceLock::new();

/// Initialize logging with comprehensive coverage
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lambda_mcp_server=info,mcp_server=debug,http_mcp_server=debug,aws_sdk_dynamodb=warn,aws_sdk_sns=warn,aws_sdk_sqs=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize global event system, EventRouter, and SNS processing
async fn initialize_background_services() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("🚀 Initializing Lambda MCP Server with complete restored architecture");

    // Initialize global event broadcasting system
    initialize_global_events();
    info!("✅ Global event broadcasting system initialized");

    // Initialize EventRouter once (OnceCell pattern) - will create SessionManager, ToolRegistry etc.
    match EventRouter::new().await {
        Ok(router) => {
            EVENT_ROUTER
                .set(router)
                .map_err(|_| "Failed to set global EventRouter")?;
            info!("✅ EventRouter initialized globally (includes SessionManager & ToolRegistry)");
        }
        Err(e) => {
            error!("❌ Failed to initialize EventRouter: {:?}", e);
            return Err(format!("EventRouter initialization failed: {}", e).into());
        }
    }

    // Initialize global SNS processor if topic is configured
    if let Ok(topic_arn) = env::var("SNS_TOPIC_ARN") {
        info!(
            "🔗 Initializing global SNS processor - Topic: {}",
            topic_arn
        );
        initialize_sns_processor(topic_arn).await?;
        info!("✅ Global SNS processor initialized and ready (OnceCell pattern)");
    } else {
        warn!("⚠️  SNS_TOPIC_ARN not configured - external event processing disabled");
    }

    info!("🎉 All background services initialized successfully");
    Ok(())
}

/// Lambda handler function using the global event router
async fn lambda_handler(request: Request) -> Result<Response<Body>, Error> {
    let method = request.method();
    let uri = request.uri();
    let lambda_context = request.lambda_context();

    info!(
        "🌐 Lambda MCP request: {} {} (request_id: {})",
        method,
        uri.path(),
        lambda_context.request_id
    );

    // Get the global EventRouter (initialized once at startup)
    let event_router = EVENT_ROUTER.get().ok_or_else(|| {
        error!("❌ Global EventRouter not initialized - this should never happen");
        Error::from("EventRouter not initialized")
    })?;

    // Route request through the complete architecture
    match event_router.route_request(request, lambda_context).await {
        Ok(response) => {
            info!("✅ Request processed successfully through global event router");
            Ok(response)
        }
        Err(e) => {
            error!("❌ Event router failed: {:?}", e);
            Ok(create_error_response(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Request processing failed: {}", e),
            ))
        }
    }
}

/// Create JSON-RPC 2.0 compliant error response
fn create_error_response(status: http::StatusCode, message: &str) -> Response<Body> {
    use serde_json::json;

    // Create proper JSON-RPC 2.0 error response
    let json_rpc_error = json!({
        "jsonrpc": "2.0",
        "id": null, // Use null when request ID is unknown/unparseable
        "error": {
            "code": match status {
                http::StatusCode::BAD_REQUEST => -32600, // Invalid Request
                http::StatusCode::NOT_FOUND => -32601,   // Method not found
                http::StatusCode::UNPROCESSABLE_ENTITY => -32602, // Invalid params
                _ => -32603, // Internal error
            },
            "message": message,
            "data": {
                "httpStatus": status.as_u16()
            }
        }
    });

    Response::builder()
        .status(http::StatusCode::OK) // Always return 200 for JSON-RPC responses
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .header(
            "access-control-allow-headers",
            "Content-Type, mcp-session-id",
        )
        .body(Body::from(json_rpc_error.to_string()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(http::StatusCode::OK)
                .body(Body::from(r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal server error"}}"#))
                .unwrap()
        })
}

/// Main Lambda entry point with complete restored architecture
#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();

    info!("🚀 Starting Lambda MCP Server with complete restored architecture");
    info!("Architecture components:");
    info!("  - DynamoDB session management");
    info!("  - Global event broadcasting (tokio channels)");
    info!("  - SNS external event processing");
    info!("  - SSE streaming for real-time notifications");
    info!("  - Complete tool suite (AWS, Lambda, Session tools)");
    info!("  - Event router with full request handling");
    info!("Protocol version: 2025-06-18");

    // Initialize background services
    if let Err(e) = initialize_background_services().await {
        warn!("Failed to initialize background services: {:?}", e);
        info!("Server will continue with limited functionality");
    }

    // Run Lambda HTTP runtime with streaming response support
    run_with_streaming_response(service_fn(lambda_handler)).await
}
