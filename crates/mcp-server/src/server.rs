//! MCP Server Implementation
//!
//! This module provides the main MCP server implementation.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, debug, error};

use crate::{McpTool, McpServerBuilder, Result, tool::tool_to_descriptor};
use crate::handlers::McpHandler;
use crate::session::SessionManager;
use json_rpc_server::JsonRpcHandler;
use mcp_protocol::*;

/// Main MCP server
pub struct McpServer {
    /// Server implementation information
    pub implementation: Implementation,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Registered tools
    tools: HashMap<String, Arc<dyn McpTool>>,
    /// Registered handlers
    handlers: HashMap<String, Arc<dyn McpHandler>>,
    /// Session manager for state persistence
    session_manager: Arc<SessionManager>,
    /// Optional client instructions
    instructions: Option<String>,
    
    // HTTP configuration (if enabled)
    #[cfg(feature = "http")]
    bind_address: SocketAddr,
    #[cfg(feature = "http")]
    mcp_path: String,
    #[cfg(feature = "http")]
    enable_cors: bool,
    #[cfg(feature = "http")]
    enable_sse: bool,
}

impl McpServer {
    /// Create a new MCP server (use builder instead)
    pub(crate) fn new(
        implementation: Implementation,
        capabilities: ServerCapabilities,
        tools: HashMap<String, Arc<dyn McpTool>>,
        handlers: HashMap<String, Arc<dyn McpHandler>>,
        instructions: Option<String>,
        session_timeout_minutes: Option<u64>,
        session_cleanup_interval_seconds: Option<u64>,
        #[cfg(feature = "http")]
        bind_address: SocketAddr,
        #[cfg(feature = "http")]
        mcp_path: String,
        #[cfg(feature = "http")]
        enable_cors: bool,
        #[cfg(feature = "http")]
        enable_sse: bool,
    ) -> Self {
        // Create session manager with server capabilities and custom timeouts
        let session_manager = if let (Some(timeout_mins), Some(cleanup_secs)) = (session_timeout_minutes, session_cleanup_interval_seconds) {
            Arc::new(SessionManager::with_timeouts(
                capabilities.clone(),
                std::time::Duration::from_secs(timeout_mins * 60),
                std::time::Duration::from_secs(cleanup_secs),
            ))
        } else {
            Arc::new(SessionManager::new(capabilities.clone()))
        };
        
        Self {
            implementation,
            capabilities,
            tools,
            handlers,
            session_manager,
            instructions,
            #[cfg(feature = "http")]
            bind_address,
            #[cfg(feature = "http")]
            mcp_path,
            #[cfg(feature = "http")]
            enable_cors,
            #[cfg(feature = "http")]
            enable_sse,
        }
    }

    /// Create a new builder
    pub fn builder() -> McpServerBuilder {
        McpServerBuilder::new()
    }

    /// Run the server with the default transport (HTTP if available)
    pub async fn run(&self) -> Result<()> {
        #[cfg(feature = "http")]
        {
            self.run_http().await
        }
        #[cfg(not(feature = "http"))]
        {
            // If no HTTP feature, we can't run without transport
            Err(McpFrameworkError::Config(
                "No transport available. Enable the 'http' feature to use HTTP transport.".to_string()
            ))
        }
    }

    /// Run the server with HTTP transport (requires "http" feature)
    #[cfg(feature = "http")]
    pub async fn run_http(&self) -> Result<()> {
        info!("Starting MCP server: {} v{}", self.implementation.name, self.implementation.version);
        info!("Session management: enabled with automatic cleanup");
        
        if self.enable_sse {
            info!("SSE notifications: enabled at GET {}/sse", self.mcp_path);
        }

        // Start session cleanup task
        let _cleanup_task = self.session_manager.clone().start_cleanup_task();

        // Create session-aware tool handler
        let tool_handler = SessionAwareToolHandler::new(
            self.tools.clone(), 
            self.session_manager.clone()
        );

        // Create session-aware initialize handler
        let init_handler = SessionAwareInitializeHandler::new(
            self.implementation.clone(),
            self.capabilities.clone(),
            self.instructions.clone(),
            self.session_manager.clone(),
        );

        // Build HTTP server
        let mut builder = http_mcp_server::HttpMcpServer::builder()
            .bind_address(self.bind_address)
            .mcp_path(&self.mcp_path)
            .cors(self.enable_cors)
            .sse(self.enable_sse)
            .register_handler(vec!["initialize".to_string()], init_handler)
            .register_handler(vec!["tools/list".to_string()], ListToolsHandler::new(self.tools.clone()))
            .register_handler(vec!["tools/call".to_string()], tool_handler);

        // Register all MCP handlers with session awareness
        for (method, handler) in &self.handlers {
            let bridge_handler = SessionAwareMcpHandlerBridge::new(
                handler.clone(),
                self.session_manager.clone()
            );
            builder = builder.register_handler(vec![method.clone()], bridge_handler);
        }

        let http_server = builder.build();
        
        // SSE is now integrated directly into the session management
        if self.enable_sse {
            debug!("SSE support enabled with integrated session management");
        }
        
        http_server.run().await?;
        Ok(())
    }

    /// Run the server and return the HTTP server handle for SSE access (requires "http" feature)  
    #[cfg(feature = "http")]
    pub async fn run_with_sse_access(&self) -> Result<(http_mcp_server::HttpMcpServer, tokio::task::JoinHandle<http_mcp_server::Result<()>>)> {
        info!("Starting MCP server: {} v{}", self.implementation.name, self.implementation.version);
        info!("Session management: enabled with automatic cleanup");
        
        if self.enable_sse {
            info!("SSE notifications: enabled - SSE manager available for notifications");
        }

        // Start session cleanup task
        let _cleanup_task = self.session_manager.clone().start_cleanup_task();

        // Create session-aware tool handler
        let tool_handler = SessionAwareToolHandler::new(
            self.tools.clone(), 
            self.session_manager.clone()
        );

        // Create session-aware initialize handler
        let init_handler = SessionAwareInitializeHandler::new(
            self.implementation.clone(),
            self.capabilities.clone(),
            self.instructions.clone(),
            self.session_manager.clone(),
        );

        // Build HTTP server
        let mut builder = http_mcp_server::HttpMcpServer::builder()
            .bind_address(self.bind_address)
            .mcp_path(&self.mcp_path)
            .cors(self.enable_cors)
            .sse(self.enable_sse)
            .register_handler(vec!["initialize".to_string()], init_handler)
            .register_handler(vec!["tools/list".to_string()], ListToolsHandler::new(self.tools.clone()))
            .register_handler(vec!["tools/call".to_string()], tool_handler);

        // Register all MCP handlers with session awareness
        for (method, handler) in &self.handlers {
            let bridge_handler = SessionAwareMcpHandlerBridge::new(
                handler.clone(),
                self.session_manager.clone()
            );
            builder = builder.register_handler(vec![method.clone()], bridge_handler);
        }

        let http_server = builder.build();
        
        // Run server in background task
        let server_task = {
            let server = http_server.clone();
            tokio::spawn(async move {
                server.run().await
            })
        };
        
        Ok((http_server, server_task))
    }
}

/// Session-aware bridge handler to adapt McpHandler to JsonRpcHandler
struct SessionAwareMcpHandlerBridge {
    handler: Arc<dyn McpHandler>,
    session_manager: Arc<SessionManager>,
}

impl SessionAwareMcpHandlerBridge {
    fn new(handler: Arc<dyn McpHandler>, session_manager: Arc<SessionManager>) -> Self {
        Self { handler, session_manager }
    }
}

#[async_trait]
impl JsonRpcHandler for SessionAwareMcpHandlerBridge {
    async fn handle(&self, method: &str, params: Option<json_rpc_server::RequestParams>) -> json_rpc_server::r#async::JsonRpcResult<serde_json::Value> {
        debug!("Handling {} request via session-aware bridge", method);

        // Extract session ID from request (would come from headers in real implementation)
        // For now, we'll extract it from params if available
        let session_id = extract_session_id_from_params(&params);
        
        // Create session context if session exists
        let session_context = if let Some(sid) = session_id {
            self.session_manager.create_session_context(&sid)
        } else {
            None
        };

        // Convert JSON-RPC params to Value
        let mcp_params = params.map(|p| p.to_value());

        // Call the MCP handler with session context
        match self.handler.handle_with_session(mcp_params, session_context).await {
            Ok(result) => Ok(result),
            Err(error_msg) => {
                error!("MCP handler error: {}", error_msg);
                Err(json_rpc_server::error::JsonRpcProcessingError::HandlerError(error_msg.to_string()))
            }
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        self.handler.supported_methods()
    }
}

/// Extract session ID from request parameters (placeholder implementation)
fn extract_session_id_from_params(_params: &Option<json_rpc_server::RequestParams>) -> Option<String> {
    // In a real implementation, this would extract session ID from HTTP headers
    // For now, return None as we'll implement proper session extraction later
    None
}

/// Session-aware handler for initialize requests
struct SessionAwareInitializeHandler {
    implementation: Implementation,
    capabilities: ServerCapabilities,
    instructions: Option<String>,
    session_manager: Arc<SessionManager>,
}

impl SessionAwareInitializeHandler {
    fn new(
        implementation: Implementation, 
        capabilities: ServerCapabilities, 
        instructions: Option<String>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self { implementation, capabilities, instructions, session_manager }
    }

    /// Negotiate protocol version with client
    /// 
    /// Server supports backward compatibility with older protocol versions.
    /// The negotiation follows this priority:
    /// 1. Use client's requested version if server supports it
    /// 2. Use the highest version both client and server support
    /// 3. Fall back to minimum compatible version
    fn negotiate_version(&self, client_version: &str) -> std::result::Result<McpVersion, String> {
        use mcp_protocol::version::McpVersion;

        // Parse client's requested version
        let requested_version = McpVersion::from_str(client_version)
            .ok_or_else(|| format!("Unsupported protocol version: {}", client_version))?;

        // Define server's supported versions (all versions from 2024-11-05 to current)
        let supported_versions = vec![
            McpVersion::V2024_11_05,
            McpVersion::V2025_03_26,
            McpVersion::V2025_06_18,
        ];

        // Strategy 1: If server supports client's requested version, use it
        if supported_versions.contains(&requested_version) {
            return Ok(requested_version);
        }

        // Strategy 2: Use the highest version the server supports that's <= client version
        // This allows clients to request newer versions while falling back gracefully
        let negotiated = match requested_version {
            McpVersion::V2025_06_18 => McpVersion::V2025_06_18, // Latest
            McpVersion::V2025_03_26 => McpVersion::V2025_03_26, // Streamable HTTP
            McpVersion::V2024_11_05 => McpVersion::V2024_11_05, // Base version
        };

        // Verify the negotiated version is in our supported list
        if supported_versions.contains(&negotiated) {
            Ok(negotiated)
        } else {
            // Strategy 3: Fall back to minimum supported version
            Err(format!("Cannot negotiate compatible version with client version {}", client_version))
        }
    }

    /// Adjust server capabilities based on negotiated protocol version
    /// 
    /// Some capabilities are only available in newer protocol versions.
    /// This method filters capabilities to match what the negotiated version supports.
    fn adjust_capabilities_for_version(&self, version: McpVersion) -> ServerCapabilities {
        let adjusted = self.capabilities.clone();

        // Before version 2025-06-18, _meta field support wasn't available
        // So we don't need to adjust capabilities for that specifically since it's 
        // handled at the protocol level.

        // Before version 2025-03-26, streamable HTTP wasn't available
        // But HTTP transport capability isn't explicitly declared in ServerCapabilities,
        // so no adjustment needed here.

        // All other capabilities (tools, resources, prompts, etc.) are version-independent
        // in terms of their basic functionality.

        info!("Server capabilities adjusted for protocol version {}", version);
        debug!("Capabilities: logging={}, tools={}, resources={}, prompts={}", 
               adjusted.logging.is_some(),
               adjusted.tools.is_some(), 
               adjusted.resources.is_some(),
               adjusted.prompts.is_some());

        adjusted
    }
}

#[async_trait]
impl JsonRpcHandler for SessionAwareInitializeHandler {
    async fn handle(&self, method: &str, params: Option<json_rpc_server::RequestParams>) -> json_rpc_server::r#async::JsonRpcResult<serde_json::Value> {
        debug!("Handling {} request with session support", method);

        if method != "initialize" {
            return Err(json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                format!("Method not supported: {}", method)
            ));
        }

        // Parse initialize request
        let request = if let Some(params) = params {
            let params_value = params.to_value();
            serde_json::from_value::<InitializeRequest>(params_value)
                .map_err(|e| json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                    format!("Invalid initialize request: {}", e)
                ))?
        } else {
            return Err(json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                "Missing parameters for initialize".to_string()
            ));
        };

        // Perform protocol version negotiation
        let negotiated_version = match self.negotiate_version(&request.protocol_version) {
            Ok(version) => {
                info!("Protocol version negotiated: {} (client requested: {})", 
                      version, request.protocol_version);
                version
            }
            Err(e) => {
                error!("Protocol version negotiation failed: {}", e);
                return Err(json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                    format!("Version negotiation failed: {}", e)
                ));
            }
        };

        // Create new session for this initialization
        let session_id = self.session_manager.create_session().await;
        
        // Initialize the session with client info and negotiated version
        if let Err(e) = self.session_manager.initialize_session_with_version(
            &session_id,
            request.client_info,
            request.capabilities,
            negotiated_version,
        ).await {
            error!("Failed to initialize session: {}", e);
            return Err(json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                format!("Session initialization failed: {}", e)
            ));
        }

        // Store the negotiated version in session state for tools to access
        self.session_manager.set_session_state(
            &session_id,
            "mcp_version",
            serde_json::json!(negotiated_version.as_str()),
        ).await;

        info!("Session {} initialized for client with protocol version {}", session_id, negotiated_version);

        // Create response with negotiated version and adjusted capabilities
        let adjusted_capabilities = self.adjust_capabilities_for_version(negotiated_version);
        let mut response = InitializeResponse::new(
            negotiated_version,
            adjusted_capabilities,
            self.implementation.clone(),
        );

        if let Some(instructions) = &self.instructions {
            response = response.with_instructions(instructions.clone());
        }

        // TODO: Add session ID to response headers for Streamable HTTP transport
        // This will enable proper session management as specified in MCP 2025-03-26+
        // Session IDs should be cryptographically secure and globally unique

        serde_json::to_value(response)
            .map_err(|e| json_rpc_server::error::JsonRpcProcessingError::HandlerError(e.to_string()))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["initialize".to_string()]
    }
}

/// Handler for tools/list requests
struct ListToolsHandler {
    tools: HashMap<String, Arc<dyn McpTool>>,
}

impl ListToolsHandler {
    fn new(tools: HashMap<String, Arc<dyn McpTool>>) -> Self {
        Self { tools }
    }
}

#[async_trait]
impl JsonRpcHandler for ListToolsHandler {
    async fn handle(&self, method: &str, params: Option<json_rpc_server::RequestParams>) -> json_rpc_server::r#async::JsonRpcResult<serde_json::Value> {
        use mcp_protocol_2025_06_18::meta::{PaginatedResponse, Cursor};
        
        debug!("Handling {} request", method);

        if method != "tools/list" {
            return Err(json_rpc_server::error::JsonRpcProcessingError::RpcError(
                json_rpc_server::JsonRpcError::method_not_found(
                    json_rpc_server::types::RequestId::Number(0), method
                )
            ));
        }

        // Parse cursor from params if provided
        let cursor = params.as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);

        let tools: Vec<Tool> = self.tools.values()
            .map(|tool| tool_to_descriptor(tool.as_ref()))
            .collect();

        let base_response = ListToolsResponse::new(tools.clone());
        
        // Add pagination metadata
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(tools.len() as u64);
        
        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more
        );

        serde_json::to_value(paginated_response)
            .map_err(|e| json_rpc_server::error::JsonRpcProcessingError::HandlerError(e.to_string()))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tools/list".to_string()]
    }
}

/// Session-aware handler for tool execution
struct SessionAwareToolHandler {
    tools: HashMap<String, Arc<dyn McpTool>>,
    session_manager: Arc<SessionManager>,
}

impl SessionAwareToolHandler {
    fn new(tools: HashMap<String, Arc<dyn McpTool>>, session_manager: Arc<SessionManager>) -> Self {
        Self { tools, session_manager }
    }
}

#[async_trait]
impl JsonRpcHandler for SessionAwareToolHandler {
    async fn handle(&self, method: &str, params: Option<json_rpc_server::RequestParams>) -> json_rpc_server::r#async::JsonRpcResult<serde_json::Value> {
        debug!("Handling {} request with session support", method);

        if method != "tools/call" {
            return Err(json_rpc_server::error::JsonRpcProcessingError::RpcError(
                json_rpc_server::JsonRpcError::method_not_found(
                    json_rpc_server::types::RequestId::Number(0), method
                )
            ));
        }

        let params = params.ok_or_else(|| {
            let mcp_error = mcp_protocol::McpError::MissingParameter("CallToolRequest".to_string());
            json_rpc_server::error::JsonRpcProcessingError::RpcError(
                json_rpc_server::JsonRpcError::new(None, mcp_error.to_json_rpc_error())
            )
        })?;

        // Parse the tool call request
        let call_request: CallToolRequest = serde_json::from_value(params.to_value())
            .map_err(|e| {
                let mcp_error = mcp_protocol::McpError::InvalidParameters(format!("Invalid parameters for tools/call: {}", e));
                json_rpc_server::error::JsonRpcProcessingError::RpcError(
                    json_rpc_server::JsonRpcError::new(None, mcp_error.to_json_rpc_error())
                )
            })?;

        // Find the tool
        let tool = self.tools.get(&call_request.name)
            .ok_or_else(|| {
                let mcp_error = mcp_protocol::McpError::ToolNotFound(call_request.name.clone());
                json_rpc_server::error::JsonRpcProcessingError::RpcError(
                    json_rpc_server::JsonRpcError::new(None, mcp_error.to_json_rpc_error())
                )
            })?;

        // Extract session ID from request (placeholder - will be from headers)
        let session_id = extract_session_id_from_params(&Some(params.clone()));
        
        // Create session context if session exists
        let session_context = if let Some(sid) = session_id {
            self.session_manager.create_session_context(&sid)
        } else {
            None
        };

        // Execute the tool with session context
        let args = call_request.arguments.unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        match tool.execute(args, session_context).await {
            Ok(response) => {
                serde_json::to_value(response)
                    .map_err(|e| json_rpc_server::error::JsonRpcProcessingError::HandlerError(e.to_string()))
            }
            Err(error_msg) => {
                error!("Tool execution error: {}", error_msg);
                let error_content = vec![ToolResult::text(format!("Error: {}", error_msg))];
                let response = CallToolResponse::error(error_content);
                serde_json::to_value(response)
                    .map_err(|e| json_rpc_server::error::JsonRpcProcessingError::HandlerError(e.to_string()))
            }
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tools/call".to_string()]
    }
}

impl std::fmt::Debug for McpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServer")
            .field("implementation", &self.implementation)
            .field("capabilities", &self.capabilities)
            .field("tools", &format!("HashMap with {} tools", self.tools.len()))
            .field("instructions", &self.instructions)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::McpTool;
    use async_trait::async_trait;
    use mcp_protocol::{ToolSchema, ToolResult};
    use serde_json::Value;

    struct TestTool;

    #[async_trait]
    impl McpTool for TestTool {
        fn name(&self) -> &str { "test" }
        fn description(&self) -> &str { "Test tool" }
        fn input_schema(&self) -> ToolSchema { ToolSchema::object() }
        async fn call(&self, _args: Value, _session: Option<crate::SessionContext>) -> crate::McpResult<Vec<ToolResult>> {
            Ok(vec![ToolResult::text("test result")])
        }
    }

    #[test]
    fn test_server_creation() {
        let server = McpServer::builder()
            .name("test-server")
            .version("1.0.0")
            .tool(TestTool)
            .build()
            .unwrap();

        assert_eq!(server.implementation.name, "test-server");
        assert_eq!(server.implementation.version, "1.0.0");
        assert_eq!(server.tools.len(), 1);
    }

    #[tokio::test]
    async fn test_list_tools_handler() {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert("test".to_string(), Arc::new(TestTool));
        
        let handler = ListToolsHandler::new(tools);
        let result = handler.handle("tools/list", None).await.unwrap();
        
        let response: ListToolsResponse = serde_json::from_value(result).unwrap();
        assert_eq!(response.tools.len(), 1);
        assert_eq!(response.tools[0].name, "test");
    }

    #[tokio::test]
    async fn test_tool_handler() {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert("test".to_string(), Arc::new(TestTool));
        
        let session_manager = Arc::new(SessionManager::new(ServerCapabilities::default()));
        let handler = SessionAwareToolHandler::new(tools, session_manager);
        let params = json_rpc_server::RequestParams::Object(
            [("name".to_string(), serde_json::json!("test")),
             ("arguments".to_string(), serde_json::json!({}))]
            .into_iter().collect()
        );
        
        let result = handler.handle("tools/call", Some(params)).await.unwrap();
        let response: CallToolResponse = serde_json::from_value(result).unwrap();
        
        assert_eq!(response.content.len(), 1);
        if let ToolResult::Text { text } = &response.content[0] {
            assert_eq!(text, "test result");
        }
    }
}