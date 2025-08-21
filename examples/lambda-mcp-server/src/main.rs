//! AWS Lambda MCP Server using mcp-framework
//!
//! This Lambda MCP server properly uses mcp-framework components for correct
//! MCP 2025-06-18 protocol implementation with proper McpTool implementations
//! and framework-based JSON-RPC handling.

use std::collections::HashMap;
use lambda_http::{run_with_streaming_response, service_fn, Error, Request, RequestExt, Body, Response};
use serde_json::{Value, json};
use tracing::{info, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use http::StatusCode;
use async_trait::async_trait;

// MCP framework imports
use mcp_server::{McpTool, SessionContext, McpResult};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema};
use http_mcp_server::StreamableHttpContext;

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lambda_mcp_server=info,mcp_server=debug,http_mcp_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// AWS Real-time Monitoring Tool
struct AwsRealTimeMonitor;

#[async_trait]
impl McpTool for AwsRealTimeMonitor {
    fn name(&self) -> &str {
        "aws_real_time_monitor"
    }

    fn description(&self) -> &str {
        "Monitor AWS resources in real-time"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("resource_type".to_string(), JsonSchema::string_with_description("AWS resource type to monitor")),
                ("region".to_string(), JsonSchema::string_with_description("AWS region")),
            ]))
            .with_required(vec!["resource_type".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let resource_type = args.get("resource_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
            
        let region = args.get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1");

        let result = format!(
            "Monitoring {} resources in {} region. Real-time data would be fetched from CloudWatch/AWS APIs.",
            resource_type, region
        );

        Ok(vec![ToolResult::text(result)])
    }
}

/// Lambda Diagnostics Tool
struct LambdaDiagnostics;

#[async_trait]
impl McpTool for LambdaDiagnostics {
    fn name(&self) -> &str {
        "lambda_diagnostics"
    }

    fn description(&self) -> &str {
        "Get Lambda function diagnostics and runtime information"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("include_metrics".to_string(), JsonSchema::boolean_with_description("Include CloudWatch metrics")),
                ("include_environment".to_string(), JsonSchema::boolean_with_description("Include environment variables")),
                ("include_aws_info".to_string(), JsonSchema::boolean_with_description("Include AWS context information")),
            ]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let include_metrics = args.get("include_metrics")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let include_env = args.get("include_environment")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let include_aws = args.get("include_aws_info")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut diagnostics = json!({
            "function_name": std::env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap_or_else(|_| "unknown".to_string()),
            "function_version": std::env::var("AWS_LAMBDA_FUNCTION_VERSION").unwrap_or_else(|_| "$LATEST".to_string()),
            "runtime": std::env::var("AWS_EXECUTION_ENV").unwrap_or_else(|_| "unknown".to_string()),
            "memory_size": std::env::var("AWS_LAMBDA_FUNCTION_MEMORY_SIZE").unwrap_or_else(|_| "unknown".to_string()),
        });

        if include_metrics {
            diagnostics["metrics"] = json!({
                "note": "CloudWatch metrics would be fetched from AWS APIs",
                "available_metrics": ["Duration", "BilledDuration", "MemoryUtilized", "MaxMemoryUsed"]
            });
        }

        if include_env {
            let env_vars: HashMap<String, String> = std::env::vars()
                .filter(|(k, _)| !k.contains("SECRET") && !k.contains("KEY") && !k.contains("TOKEN"))
                .collect();
            diagnostics["environment"] = serde_json::to_value(env_vars).unwrap_or_default();
        }

        if include_aws {
            diagnostics["aws_context"] = json!({
                "region": std::env::var("AWS_DEFAULT_REGION").unwrap_or_else(|_| "unknown".to_string()),
                "request_id": "will-be-available-at-runtime",
                "log_group": std::env::var("AWS_LAMBDA_LOG_GROUP_NAME").unwrap_or_else(|_| "unknown".to_string()),
                "log_stream": std::env::var("AWS_LAMBDA_LOG_STREAM_NAME").unwrap_or_else(|_| "unknown".to_string()),
            });
        }

        Ok(vec![ToolResult::text(
            serde_json::to_string_pretty(&diagnostics)
                .unwrap_or_else(|_| "Failed to serialize diagnostics".to_string())
        )])
    }
}

/// Session Info Tool
struct SessionInfo;

#[async_trait]
impl McpTool for SessionInfo {
    fn name(&self) -> &str {
        "session_info"
    }

    fn description(&self) -> &str {
        "Get current MCP session information and statistics"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("include_capabilities".to_string(), JsonSchema::boolean_with_description("Include server capabilities")),
                ("include_statistics".to_string(), JsonSchema::boolean_with_description("Include session statistics")),
            ]))
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let include_capabilities = args.get("include_capabilities")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let include_statistics = args.get("include_statistics")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut session_info = json!({
            "session_active": session.is_some(),
            "protocol_version": "2025-06-18",
            "transport": "lambda-http"
        });

        if let Some(session_ctx) = session {
            session_info["session_id"] = json!(session_ctx.session_id);
            // Note: In Lambda, session state is ephemeral
            session_info["note"] = json!("Session context available but state is request-scoped in Lambda");
        }

        if include_capabilities {
            session_info["server_capabilities"] = json!({
                "tools": true,
                "resources": false,
                "prompts": false,
                "logging": false,
                "notifications": false
            });
        }

        if include_statistics {
            session_info["statistics"] = json!({
                "lambda_invocations": "tracked-per-container",
                "session_duration": "ephemeral-per-request",
                "note": "Lambda is stateless - session persists only during request"
            });
        }

        Ok(vec![ToolResult::text(
            serde_json::to_string_pretty(&session_info)
                .unwrap_or_else(|_| "Failed to serialize session info".to_string())
        )])
    }
}

/// List Active Sessions Tool
struct ListActiveSessions;

#[async_trait]
impl McpTool for ListActiveSessions {
    fn name(&self) -> &str {
        "list_active_sessions"
    }

    fn description(&self) -> &str {
        "List all active MCP sessions (Lambda-specific implementation)"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
    }

    async fn call(&self, _args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let current_session = if let Some(session_ctx) = session {
            json!({
                "session_id": session_ctx.session_id,
                "status": "active",
                "transport": "lambda-http",
                "lifecycle": "request-scoped"
            })
        } else {
            json!({
                "status": "no_session",
                "note": "No active session context"
            })
        };

        let response = json!({
            "active_sessions": [current_session],
            "total_count": 1,
            "lambda_note": "Lambda functions are stateless - only current request session is tracked"
        });

        Ok(vec![ToolResult::text(
            serde_json::to_string_pretty(&response)
                .unwrap_or_else(|_| "Failed to serialize session list".to_string())
        )])
    }
}

/// Lambda handler function
async fn lambda_handler(request: Request) -> Result<Response<Body>, Error> {
    let method = request.method();
    let uri = request.uri();
    let lambda_context = request.lambda_context();
    
    info!(
        "Lambda MCP request: {} {} (request_id: {})",
        method,
        uri.path(),
        lambda_context.request_id
    );

    // Handle CORS preflight
    if method == http::Method::OPTIONS {
        return Ok(create_cors_response());
    }

    // For Lambda, we need to run the MCP server in a different way
    // since we can't bind to a socket address. Instead, we'll handle
    // the HTTP request directly using the framework components.
    
    match handle_mcp_request(request).await {
        Ok(response) => {
            info!("MCP request processed successfully");
            Ok(add_cors_headers(response))
        }
        Err(e) => {
            error!("MCP request failed: {:?}", e);
            Ok(create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("MCP request processing failed: {}", e)
            ))
        }
    }
}

/// Lambda-adapted MCP handler using framework components
struct LambdaMcpHandler {
    tools: HashMap<String, Box<dyn McpTool + Send + Sync>>,
}

impl LambdaMcpHandler {
    fn new() -> Self {
        let mut tools: HashMap<String, Box<dyn McpTool + Send + Sync>> = HashMap::new();
        tools.insert("aws_real_time_monitor".to_string(), Box::new(AwsRealTimeMonitor));
        tools.insert("lambda_diagnostics".to_string(), Box::new(LambdaDiagnostics));
        tools.insert("session_info".to_string(), Box::new(SessionInfo));
        tools.insert("list_active_sessions".to_string(), Box::new(ListActiveSessions));
        
        Self { tools }
    }

    /// Handle MCP JSON-RPC request using framework patterns
    async fn handle_json_rpc_request(&self, request_json: &Value, context: &StreamableHttpContext) -> Result<Value, Box<dyn std::error::Error>> {
        let method = request_json.get("method")
            .and_then(|m| m.as_str())
            .ok_or("Missing method in JSON-RPC request")?;
            
        let id = request_json.get("id").cloned().unwrap_or(Value::Null);
        let params = request_json.get("params").cloned();
        
        debug!("Processing JSON-RPC method: {} with session: {:?}", method, context.session_id);
        
        match method {
            "initialize" => self.handle_initialize(id, params, context).await,
            "notifications/initialized" => {
                // Notifications don't return responses per JSON-RPC spec
                info!("Session initialized notification received");
                Ok(Value::Null)
            },
            "tools/list" => self.handle_tools_list(id).await,
            "tools/call" => self.handle_tools_call(id, params, context).await,
            "ping" => {
                // Simple ping method - just return empty result per MCP spec
                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                }))
            },
            _ => Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                    "data": format!("Unknown method: {}", method)
                }
            }))
        }
    }

    /// Handle initialize using framework patterns
    async fn handle_initialize(&self, id: Value, params: Option<Value>, _context: &StreamableHttpContext) -> Result<Value, Box<dyn std::error::Error>> {
        info!("Handling initialize request with mcp-framework patterns");
        
        // Generate session ID (server-generated per MCP spec)
        let session_id = uuid::Uuid::new_v4().to_string();
        
        // Parse client info
        let client_info = params
            .as_ref()
            .and_then(|p| p.get("clientInfo"))
            .cloned();
        
        let protocol_version = params
            .as_ref()
            .and_then(|p| p.get("protocolVersion"))
            .and_then(|v| v.as_str())
            .unwrap_or("2025-06-18");
        
        info!("Initialize: session_id={}, client_info={:?}, protocol={}", session_id, client_info, protocol_version);
        
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": protocol_version,
                "serverInfo": {
                    "name": "lambda-mcp-server",
                    "version": "1.0.0",
                    "description": "AWS Lambda serverless MCP server using mcp-framework"
                },
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    },
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    },
                    "prompts": {
                        "listChanged": false
                    }
                }
            },
            // Store session ID separately to be added to response header
            "_internal_sessionId": session_id
        });
        
        Ok(response)
    }

    /// Handle tools/list using framework patterns
    async fn handle_tools_list(&self, id: Value) -> Result<Value, Box<dyn std::error::Error>> {
        info!("Handling tools/list request");
        
        let tools: Vec<Value> = self.tools.values()
            .map(|tool| {
                let schema = tool.input_schema();
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "inputSchema": serde_json::to_value(&schema).unwrap_or_default()
                })
            })
            .collect();

        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools
            }
        }))
    }

    /// Handle tools/call using framework patterns with session context
    async fn handle_tools_call(&self, id: Value, params: Option<Value>, context: &StreamableHttpContext) -> Result<Value, Box<dyn std::error::Error>> {
        let tool_name = params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .ok_or("Missing tool name in tools/call request")?;
            
        let arguments = params
            .as_ref()
            .and_then(|p| p.get("arguments"))
            .cloned()
            .unwrap_or(Value::Null);
            
        info!("Handling tools/call: {} with args: {}", tool_name, arguments);
        
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| format!("Tool not found: {}", tool_name))?;
        
        // In Lambda environment, we have limited session support
        // For a full implementation, you'd need persistent session storage
        let session_context: Option<SessionContext> = None;
        
        debug!("Tool execution with session support: session_id={:?}", context.session_id);
        
        match tool.call(arguments, session_context).await {
            Ok(content) => {
                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": content.into_iter().map(|result| {
                            match result {
                                ToolResult::Text { text } => json!({
                                    "type": "text",
                                    "text": text
                                }),
                                ToolResult::Image { data, mime_type } => json!({
                                    "type": "image",
                                    "data": data,
                                    "mimeType": mime_type
                                }),
                                ToolResult::Resource { resource, .. } => json!({
                                    "type": "resource",
                                    "resource": resource
                                }),
                                ToolResult::Audio { data, mime_type, .. } => json!({
                                    "type": "audio",
                                    "data": data,
                                    "mimeType": mime_type
                                }),
                            }
                        }).collect::<Vec<_>>(),
                        "isError": false
                    }
                }))
            }
            Err(error) => {
                error!("Tool execution error: {}", error);
                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": format!("Error: {}", error)
                        }],
                        "isError": true
                    }
                }))
            }
        }
    }
}

/// Handle MCP request using framework components with proper Accept header negotiation
async fn handle_mcp_request(request: Request) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    // Check if this is an MCP request 
    let path = request.uri().path();
    
    // Accept requests at root path (for MCP Inspector) or Lambda function path
    let is_mcp_request = path == "/" || 
                        path.contains("/mcp") || 
                        path.contains("/lambda-url/lambda-mcp-server");
    
    if !is_mcp_request {
        return Ok(create_error_response(
            StatusCode::NOT_FOUND,
            "MCP endpoint not found. Use / or /lambda-url/lambda-mcp-server"
        ));
    }

    // Per MCP spec: The client MUST include an Accept header, listing both 
    // application/json and text/event-stream as supported content types
    let accept_header = request.headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
        
    let accepts_json = accept_header.contains("application/json");
    let accepts_sse = accept_header.contains("text/event-stream");
    
    debug!("Accept header: '{}', JSON: {}, SSE: {}", accept_header, accepts_json, accepts_sse);

    // Handle GET requests for SSE
    if request.method() == http::Method::GET {
        if accepts_sse {
            return handle_sse_request(request).await;
        } else {
            return Ok(create_error_response(
                StatusCode::NOT_ACCEPTABLE,
                "GET requests require Accept header with text/event-stream"
            ));
        }
    }

    // Handle POST requests for JSON-RPC
    if request.method() == http::Method::POST {
        // Per MCP spec: If the input contains JSON-RPC requests, the server MUST either:
        // - Return Content-Type: text/event-stream (to initiate SSE stream)
        // - Return Content-Type: application/json (to return one JSON object)
        
        // For Lambda environment, we'll default to JSON responses unless SSE is specifically preferred
        // and JSON is also acceptable (client MUST accept both per spec)
        if !accepts_json {
            return Ok(create_error_response(
                StatusCode::NOT_ACCEPTABLE, 
                "POST requests require Accept header with application/json"
            ));
        }
        
        return handle_json_rpc_request(request).await;
    }

    Ok(create_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "Only GET (for SSE) and POST (for JSON-RPC) are supported"
    ))
}

/// Handle JSON-RPC POST requests using framework components
async fn handle_json_rpc_request(mut request: Request) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    use lambda_http::Body as LambdaBody;
    
    // Extract request body
    let body = std::mem::replace(request.body_mut(), LambdaBody::Empty);
    let body_str = match body {
        LambdaBody::Text(text) => text,
        LambdaBody::Binary(bytes) => String::from_utf8(bytes)?,
        LambdaBody::Empty => String::new(),
    };
    
    let request_json: Value = serde_json::from_str(&body_str)?;
    
    info!("Processing JSON-RPC request using mcp-framework patterns: {}", 
          request_json.get("method").and_then(|m| m.as_str()).unwrap_or("unknown"));
    
    // Parse streamable HTTP context from Lambda request
    let context = StreamableHttpContext::from_request(&request);
    debug!("Streamable HTTP context: protocol={}, session={:?}, streaming={}", 
           context.protocol_version.as_str(), context.session_id, context.wants_streaming);
    
    // Create MCP handler with framework patterns
    let mcp_handler = LambdaMcpHandler::new();
    let mut response_json = mcp_handler.handle_json_rpc_request(&request_json, &context).await?;
    
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json");
        
    // Extract session ID and clean it from response before serialization
    if let Some(session_id) = extract_session_id_from_response(&mut response_json) {
        response = response.header("mcp-session-id", session_id);
    }
    
    let response_body = if response_json == Value::Null {
        // Notifications - return null for compatibility with client expectations
        "null".to_string()
    } else {
        serde_json::to_string(&response_json)?
    };
    
    Ok(response.body(Body::from(response_body))?)
}


/// Handle SSE requests (placeholder)
async fn handle_sse_request(_request: Request) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    info!("Handling SSE request");
    
    // For Lambda, true SSE is challenging since Lambda functions are stateless
    // This would need WebSocket API or persistent connections
    let sse_response = "data: {\"type\":\"connection\",\"message\":\"SSE not fully supported in Lambda\"}\n\n";
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .body(Body::from(sse_response))?)
}

/// Extract session ID from JSON-RPC response and clean it from the response
fn extract_session_id_from_response(response: &mut Value) -> Option<String> {
    // Extract session ID from internal field
    let session_id = response.get("_internal_sessionId")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    
    // Remove internal session ID field from response to keep JSON-RPC compliant
    if let Some(obj) = response.as_object_mut() {
        obj.remove("_internal_sessionId");
    }
    
    session_id
}

/// Create CORS preflight response
fn create_cors_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, OPTIONS")
        .header("access-control-allow-headers", "Content-Type, Authorization, x-api-key, mcp-session-id")
        .header("access-control-max-age", "600")
        .body(Body::Empty)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to create CORS response"))
                .unwrap()
        })
}

/// Add CORS headers to response
fn add_cors_headers(mut response: Response<Body>) -> Response<Body> {
    let headers = response.headers_mut();
    headers.insert("access-control-allow-origin", "*".parse().unwrap());
    headers.insert("access-control-allow-methods", "GET, POST, OPTIONS".parse().unwrap());
    headers.insert("access-control-allow-headers", "Content-Type, Authorization, x-api-key, mcp-session-id".parse().unwrap());
    response
}

/// Create error response
fn create_error_response(status: StatusCode, message: &str) -> Response<Body> {
    let error_json = json!({
        "error": message,
        "status": status.as_u16()
    });
    
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(error_json.to_string()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal server error"))
                .unwrap()
        })
}

/// Main Lambda entry point
#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logging();
    
    info!("🚀 Starting Lambda MCP Server using proper mcp-framework components");
    info!("Protocol version: 2025-06-18");
    info!("Framework pattern: McpTool implementations with StreamableHttpContext");
    
    // Run Lambda HTTP runtime with streaming response support
    run_with_streaming_response(service_fn(lambda_handler)).await
}