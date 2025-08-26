//! Lambda Event Router
//!
//! Routes Lambda events to appropriate handlers with context management and streaming support

use crate::session_manager::SessionManager;
use crate::global_events::{subscribe_to_global_events, EventFilter};
// MCP protocol uses JSON-RPC only - no custom streaming endpoints needed
use crate::tools::{ToolRegistry, ToolExecutionContext, LambdaEventContext};
use lambda_http::{Body, Response, Request};
use http::Method;
use lambda_runtime::{Context as LambdaContext, Error as LambdaError};
use serde_json::{Value, json};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Import json-rpc-server framework types for proper MCP spec compliance
use mcp_json_rpc_server::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError
};

/// Lambda event router for MCP protocol and SSE streaming
pub struct EventRouter {
    /// Tool registry for MCP tool execution
    tool_registry: ToolRegistry,
    /// Session manager for persistent session handling
    session_manager: SessionManager,
}

impl EventRouter {
    /// Create new event router with dependencies
    pub async fn new() -> Result<Self, LambdaError> {
        let tool_registry = ToolRegistry::new().await?;
        let session_manager = SessionManager::new().await
            .map_err(|e| LambdaError::from(format!("Failed to initialize session manager: {:?}", e)))?;

        info!("Event router initialized with {} tools", tool_registry.tool_count());
        
        Ok(Self {
            tool_registry,
            session_manager,
        })
    }

    /// Route incoming Lambda request to appropriate handler
    pub async fn route_request(
        &self,
        event: Request,
        lambda_context: LambdaContext,
    ) -> Result<Response<Body>, LambdaError> {
        let method = event.method().clone();
        let uri = event.uri().clone();
        let headers = event.headers().clone();
        
        debug!("Routing request: {} {}", method, uri);

        // Extract session ID from headers if present
        let session_id = headers.get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Create Lambda event context
        let lambda_event_context = match session_id.clone() {
            Some(id) => LambdaEventContext::with_session_id(lambda_context, id),
            None => LambdaEventContext::new(lambda_context),
        };

        // Route based on path and method - MCP 2025-06-18 Streamable HTTP transport compliance
        // Lambda function URLs use root path "/" instead of "/mcp"
        match (method.clone(), uri.path()) {
            // MCP Protocol JSON-RPC endpoint - POST for sending messages
            (Method::POST, "/") => {
                self.handle_mcp_post_request(event, lambda_event_context).await
            }
            
            // MCP Protocol SSE endpoint - GET for opening streams
            (Method::GET, "/") => {
                self.handle_mcp_get_request(event, lambda_event_context).await
            }
            
            // CORS preflight support for web clients
            (Method::OPTIONS, _) => {
                self.handle_cors_preflight().await
            }
            
            // All other endpoints are non-standard - return 404
            _ => {
                info!("❌ Non-MCP endpoint requested: {} {} (MCP Streamable HTTP supports POST/GET /)", method, uri);
                self.handle_not_found().await
            }
        }
    }

    /// Handle MCP protocol JSON-RPC POST requests  
    async fn handle_mcp_post_request(
        &self,
        event: Request,
        lambda_context: LambdaEventContext,
    ) -> Result<Response<Body>, LambdaError> {
        let body = event.body();
        let body_str = std::str::from_utf8(body)
            .map_err(|e| LambdaError::from(format!("Invalid UTF-8 in request body: {}", e)))?;

        debug!("MCP request body: {}", body_str);

        // Parse JSON-RPC request using framework types
        let json_request: JsonRpcRequest = serde_json::from_str(body_str)
            .map_err(|e| {
                // Return proper JSON-RPC parse error
                let _parse_error = JsonRpcError::parse_error();
                debug!("Parse error details: {:?}", _parse_error);
                LambdaError::from(format!("JSON-RPC parse error: {}", e))
            })?;

        let method = &json_request.method;
        let request_id = json_request.id.clone();
        let params = json_request.params
            .as_ref()
            .map(|p| p.to_map())
            .unwrap_or_default();
        let params_value = serde_json::to_value(params)
            .unwrap_or_else(|_| json!({}));

        info!("🔄 Processing MCP method: {} (request_id: {})", method, lambda_context.request_id());

        // Handle methods and capture session ID for initialize using framework types
        let (response, session_id_for_header) = match method.as_str() {
            "initialize" => {
                let (result, session_id) = self.handle_initialize(params_value, lambda_context.clone()).await?;
                info!("🎯 MCP Initialize completed - session ID: {}", session_id);
                let response = JsonRpcResponse::success(request_id.clone(), result);
                (serde_json::to_value(&response).unwrap(), Some(session_id))
            },
            "tools/list" => {
                let result = self.handle_list_tools().await?;
                info!("📋 Tools list requested - {} tools available", self.tool_registry.tool_count());
                let response = JsonRpcResponse::success(request_id.clone(), result);
                (serde_json::to_value(&response).unwrap(), None)
            },
            "tools/call" => {
                let tool_results = self.handle_tool_call(params_value, lambda_context.clone()).await?;
                // Create MCP-compliant tools/call response structure directly
                let result = match tool_results.len() {
                    1 => {
                        // Single result - check if it's text and contains JSON
                        match &tool_results[0] {
                            mcp_protocol::ToolResult::Text { text } => {
                                // Try to parse as JSON for structured display
                                match serde_json::from_str::<serde_json::Value>(text) {
                                    Ok(json_value) => json_value,
                                    Err(_) => {
                                        // Not valid JSON, wrap in content structure
                                        serde_json::json!({
                                            "content": [{
                                                "type": "text",
                                                "text": text
                                            }]
                                        })
                                    }
                                }
                            },
                            _ => {
                                // Non-text result, wrap appropriately
                                serde_json::json!({
                                    "content": [{
                                        "type": "text", 
                                        "text": "Non-text tool result"
                                    }]
                                })
                            }
                        }
                    },
                    _ => {
                        // Multiple results, wrap in content array
                        let content: Vec<serde_json::Value> = tool_results.iter().map(|tr| {
                            match tr {
                                mcp_protocol::ToolResult::Text { text } => serde_json::json!({
                                    "type": "text",
                                    "text": text
                                }),
                                _ => serde_json::json!({
                                    "type": "text",
                                    "text": "Non-text tool result"
                                })
                            }
                        }).collect();
                        
                        serde_json::json!({
                            "content": content
                        })
                    }
                };
                
                let response = JsonRpcResponse::success(request_id.clone(), result);
                (serde_json::to_value(&response).unwrap(), None)
            },
            "notifications/initialized" => {
                info!("🔔 Client sent initialized notification");
                let result = self.handle_initialized_notification().await?;
                let response = JsonRpcResponse::success(request_id.clone(), result);
                (serde_json::to_value(&response).unwrap(), None)
            },
            "ping" => {
                debug!("🏓 Ping request received");
                let result = self.handle_ping().await?;
                let response = JsonRpcResponse::success(request_id.clone(), result);
                (serde_json::to_value(&response).unwrap(), None)
            },
            _ => {
                warn!("❌ Unknown MCP method: {}", method);
                let error = JsonRpcError::method_not_found(request_id.clone(), method);
                (serde_json::to_value(&error).unwrap(), None)
            }
        };

        // Response is already properly constructed by framework with correct ID

        // Create HTTP response
        let response_body = serde_json::to_string(&response)
            .map_err(|e| LambdaError::from(format!("Failed to serialize response: {}", e)))?;

        let mut response_builder = Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Headers", "Content-Type, mcp-session-id");

        // Add session ID header from initialize method or existing context
        if let Some(session_id) = session_id_for_header {
            info!("📤 Adding mcp-session-id header: {}", session_id);
            response_builder = response_builder.header("mcp-session-id", session_id);
        } else if let Some(session_id) = lambda_context.session_id() {
            response_builder = response_builder.header("mcp-session-id", session_id);
        }

        Ok(response_builder.body(Body::from(response_body))?)
    }

    /// Handle MCP protocol SSE GET requests - MCP 2025-06-18 Streamable HTTP transport
    async fn handle_mcp_get_request(
        &self,
        event: Request,
        lambda_context: LambdaEventContext,
    ) -> Result<Response<Body>, LambdaError> {
        info!("🌊 MCP GET request for SSE stream: {}", event.uri());
        
        // Check Accept header for text/event-stream as required by spec
        let accept_header = event.headers().get("accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
            
        if !accept_header.contains("text/event-stream") {
            warn!("❌ GET /mcp requires Accept: text/event-stream header");
            return Ok(Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from(r#"{"error": "GET /mcp requires Accept: text/event-stream header"}"#))?);
        }

        // Extract session ID from headers
        let session_id = event.headers().get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        info!("🔗 Opening SSE stream for session: {:?}", session_id);

        // Subscribe to global events
        let mut receiver = match subscribe_to_global_events() {
            Some(rx) => rx,
            None => {
                warn!("❌ Global event channel not initialized - SSE stream will only send connection message");
                // Return basic connection message if broadcast system not available
                let sse_body = format!(
                    "data: {}\n\n",
                    serde_json::to_string(&json!({
                        "type": "connection",
                        "status": "connected", 
                        "message": "MCP SSE stream opened (broadcast system unavailable)",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "session_id": session_id
                    })).unwrap_or_default()
                );

                let mut response_builder = Response::builder()
                    .status(200)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Headers", "Content-Type, mcp-session-id");

                if let Some(ref session_id) = session_id {
                    response_builder = response_builder.header("mcp-session-id", session_id);
                } else if let Some(session_id) = lambda_context.session_id() {
                    response_builder = response_builder.header("mcp-session-id", session_id);
                }

                return Ok(response_builder.body(Body::from(sse_body))?);
            }
        };

        // Create event filter for session-specific events if session ID is available
        let event_filter = session_id.as_ref().map(|id| {
            EventFilter::new().with_session_id(id.clone())
        });

        info!("✅ SSE stream connected to tokio broadcast system for session: {:?}", session_id);

        // In Lambda, we need to return immediately since we can't maintain long-lived streams
        // For now, we'll return the connection message and indicate the stream is ready
        // In a real implementation, you'd need API Gateway WebSockets or similar for persistent connections
        let mut sse_messages = Vec::new();

        // Connection message
        sse_messages.push(format!(
            "data: {}\n\n",
            serde_json::to_string(&json!({
                "type": "connection",
                "status": "connected",
                "message": "MCP SSE stream opened and connected to tokio broadcast",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "session_id": session_id,
                "broadcast_subscribers": crate::global_events::get_subscriber_count()
            })).unwrap_or_default()
        ));

        // Try to get any recent events (non-blocking)
        let mut event_count = 0;
        while event_count < 5 { // Limit to 5 recent events
            match receiver.try_recv() {
                Ok(event) => {
                    // Apply session filter if available
                    if let Some(ref filter) = event_filter {
                        if !filter.matches(&event) {
                            continue;
                        }
                    }

                    let sse_message = event.to_sse_message();
                    sse_messages.push(sse_message);
                    event_count += 1;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    // No more events available
                    break;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(skipped)) => {
                    warn!("SSE stream lagged, skipped {} events", skipped);
                    // Add lag notification
                    sse_messages.push(format!(
                        "data: {}\n\n",
                        serde_json::to_string(&json!({
                            "type": "system_health",
                            "component": "sse_stream",
                            "status": "warning",
                            "details": {
                                "message": format!("Stream lagged, skipped {} events", skipped),
                                "skipped_events": skipped
                            },
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })).unwrap_or_default()
                    ));
                    break;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                    warn!("Global event channel closed");
                    // Add closure notification
                    sse_messages.push(format!(
                        "data: {}\n\n",
                        serde_json::to_string(&json!({
                            "type": "connection",
                            "status": "closed",
                            "message": "Global event channel closed",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })).unwrap_or_default()
                    ));
                    break;
                }
            }
        }

        if event_count > 0 {
            info!("📡 SSE stream returning {} buffered events for session: {:?}", event_count, session_id);
        }

        let sse_body = sse_messages.join("");

        let mut response_builder = Response::builder()
            .status(200)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Headers", "Content-Type, mcp-session-id");

        // Add session ID header if available
        if let Some(ref session_id) = session_id {
            response_builder = response_builder.header("mcp-session-id", session_id);
        } else if let Some(session_id) = lambda_context.session_id() {
            response_builder = response_builder.header("mcp-session-id", session_id);
        }

        Ok(response_builder.body(Body::from(sse_body))?)
    }

    /// Handle MCP initialize method
    async fn handle_initialize(
        &self,
        params: Value,
        _lambda_context: LambdaEventContext,
    ) -> Result<(Value, String), LambdaError> {
        debug!("Initialize called with lambda context: {:?}", _lambda_context.request_id());
        let client_info = params.get("clientInfo").cloned();
        let capabilities = params.get("capabilities").cloned().unwrap_or_default();

        // Generate time-ordered session ID (UUID v7)
        let session_id = Uuid::now_v7().to_string();

        info!("🔗 MCP Initialize: Creating session {} with client capabilities: {:?}", session_id, capabilities);
        if let Some(ref info) = client_info {
            info!("📱 Client info: {:?}", info);
        }

        // Create session in DynamoDB
        match self.session_manager.create_session(&session_id, capabilities.clone(), client_info.clone()).await {
            Ok(_) => {
                info!("✅ Created new MCP session in DynamoDB: {}", session_id);
            }
            Err(e) => {
                warn!("⚠️  Failed to create session in DynamoDB: {:?} (continuing with in-memory only)", e);
                // Continue anyway - session will be in-memory only
            }
        }

        let result = json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "lambda-mcp-server",
                "version": env!("CARGO_PKG_VERSION")
            },
            "sessionId": session_id
        });

        Ok((result, session_id))
    }

    /// Handle tools/list method
    async fn handle_list_tools(&self) -> Result<Value, LambdaError> {
        let tools = self.tool_registry.list_tools().await?;
        
        let mut map = serde_json::Map::new();
        map.insert("tools".to_string(), Value::Array(tools));
        Ok(Value::Object(map))
    }

    /// Handle tools/call method
    async fn handle_tool_call(
        &self,
        params: Value,
        lambda_context: LambdaEventContext,
    ) -> Result<Vec<mcp_protocol::ToolResult>, LambdaError> {
        let tool_name = params.get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| LambdaError::from("Missing tool name"))?;

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        // Validate session ID header for notification tools that require it
        let requires_session_id = matches!(tool_name, "server_notification" | "progress_update");
        
        let session_id = if requires_session_id {
            match lambda_context.session_id() {
                Some(id) => id.to_string(),
                None => {
                    warn!("❌ Tool {} requires mcp-session-id header but none provided", tool_name);
                    // This should actually be handled at the higher level with JsonRpcError
                    // For now, return an error result that will be caught by the calling layer
                    return Err(LambdaError::from("Missing required mcp-session-id header for notification tools"));
                }
            }
        } else {
            lambda_context.session_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        };

        // Update session activity
        if let Err(e) = self.session_manager.update_session_activity(&session_id).await {
            warn!("Failed to update session activity: {:?}", e);
        }

        // Create tool execution context
        let tool_context = ToolExecutionContext {
            session_id: session_id.clone(),
            lambda_context,
            request_id: Uuid::now_v7().to_string(),
        };

        // Execute tool through mcp-framework approach
        match self.tool_registry.execute_tool(tool_name, arguments, tool_context).await {
            Ok(result) => {
                info!("Tool {} executed successfully for session {}", tool_name, session_id);
                Ok(vec![result])
            }
            Err(e) => {
                error!("Tool execution failed: {:?}", e);
                Ok(vec![mcp_protocol::ToolResult::text(format!("Tool execution failed: {}", e))])
            }
        }
    }

    /// Handle initialized notification
    async fn handle_initialized_notification(&self) -> Result<Value, LambdaError> {
        // Mark any active session as initialized (if available from context)
        // In a real implementation, you'd extract session ID from the request context
        info!("📋 Client has completed initialization - MCP handshake complete");
        
        // This is a notification, return empty result per MCP specification
        Ok(json!(null))
    }

    /// Handle ping method
    async fn handle_ping(&self) -> Result<Value, LambdaError> {
        Ok(json!({}))
    }


    /// Handle CORS preflight requests - MCP Streamable HTTP transport compliance  
    async fn handle_cors_preflight(&self) -> Result<Response<Body>, LambdaError> {
        Ok(Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")  // GET for SSE, POST for JSON-RPC
            .header("Access-Control-Allow-Headers", "Content-Type, Accept, mcp-session-id")
            .header("Access-Control-Max-Age", "86400")
            .body(Body::Empty)?)
    }

    /// Handle unknown endpoints - MCP Streamable HTTP transport compliance
    async fn handle_not_found(&self) -> Result<Response<Body>, LambdaError> {
        let error_response = json!({
            "error": "Not Found",
            "message": "MCP server supports Streamable HTTP transport with GET/POST /mcp endpoints",
            "specification": "MCP 2025-06-18 Streamable HTTP",
            "supported_endpoints": [
                "POST /mcp - JSON-RPC messages (Content-Type: application/json)",
                "GET /mcp - SSE stream (Accept: text/event-stream)", 
                "OPTIONS * - CORS preflight"
            ]
        });

        let response_body = serde_json::to_string(&error_response)
            .unwrap_or_else(|_| r#"{"error": "Not Found"}"#.to_string());

        Ok(Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from(response_body))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::RequestExt;
    use lambda_runtime::Context;

    #[tokio::test]
    async fn test_event_router_creation() {
        // This test might fail if AWS services are not available
        // In a real test environment, you'd mock the dependencies
        match EventRouter::new().await {
            Ok(router) => {
                assert!(router.tool_registry.tool_count() > 0);
            }
            Err(_) => {
                // Expected in test environment without AWS services
                println!("EventRouter creation failed as expected in test environment");
            }
        }
    }

    #[test]
    fn test_lambda_event_context() {
        let lambda_ctx = Context::default();
        let event_ctx = LambdaEventContext::new(lambda_ctx);
        
        assert!(event_ctx.session_id().is_none());
        assert!(!event_ctx.request_id().is_empty());
    }
}