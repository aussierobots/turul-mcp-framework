//! MCP Server Implementation
//!
//! This module provides the main MCP server implementation.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, error, info, warn};

use crate::handlers::McpHandler;
use crate::session::{SessionContext, SessionManager};
use crate::{McpServerBuilder, McpTool, Result, tool::tool_to_descriptor};
use turul_mcp_json_rpc_server::JsonRpcHandler;

use turul_mcp_protocol::*;
use turul_mcp_protocol::McpError;

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
    /// Session storage backend (shared between SessionManager and HTTP layer)
    session_storage: Option<Arc<turul_mcp_session_storage::BoxedSessionStorage>>,
    /// Optional client instructions
    instructions: Option<String>,
    /// Strict MCP lifecycle enforcement
    strict_lifecycle: bool,

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
        session_storage: Option<Arc<turul_mcp_session_storage::BoxedSessionStorage>>,
        strict_lifecycle: bool,
        #[cfg(feature = "http")] bind_address: SocketAddr,
        #[cfg(feature = "http")] mcp_path: String,
        #[cfg(feature = "http")] enable_cors: bool,
        #[cfg(feature = "http")] enable_sse: bool,
    ) -> Self {
        // Create session manager with server capabilities, custom timeouts, and storage
        let session_manager = match &session_storage {
            Some(storage) => {
                if let (Some(timeout_mins), Some(cleanup_secs)) =
                    (session_timeout_minutes, session_cleanup_interval_seconds)
                {
                    Arc::new(SessionManager::with_storage_and_timeouts(
                        Arc::clone(storage),
                        capabilities.clone(),
                        std::time::Duration::from_secs(timeout_mins * 60),
                        std::time::Duration::from_secs(cleanup_secs),
                    ))
                } else {
                    Arc::new(SessionManager::with_storage_and_timeouts(
                        Arc::clone(storage),
                        capabilities.clone(),
                        std::time::Duration::from_secs(30 * 60), // Default 30 minutes
                        std::time::Duration::from_secs(60),      // Default 1 minute
                    ))
                }
            }
            None => {
                // Default to InMemory storage
                if let (Some(timeout_mins), Some(cleanup_secs)) =
                    (session_timeout_minutes, session_cleanup_interval_seconds)
                {
                    Arc::new(SessionManager::with_timeouts(
                        capabilities.clone(),
                        std::time::Duration::from_secs(timeout_mins * 60),
                        std::time::Duration::from_secs(cleanup_secs),
                    ))
                } else {
                    Arc::new(SessionManager::new(capabilities.clone()))
                }
            }
        };

        // Debug: Log session storage configuration
        if let Some(storage) = &session_storage {
            debug!(
                "McpServer configured with session storage backend: {:p}",
                storage
            );
        } else {
            debug!("McpServer configured without session storage");
        }

        Self {
            implementation,
            capabilities,
            tools,
            handlers,
            session_manager,
            session_storage,
            instructions,
            strict_lifecycle,
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
    
    /// Get the server's configured capabilities
    pub fn capabilities(&self) -> &turul_mcp_protocol::ServerCapabilities {
        &self.capabilities
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
            Err(McpError::configuration(
                "No transport available. Enable the 'http' feature to use HTTP transport."
            ))
        }
    }

    /// Run the server with HTTP transport (requires "http" feature)
    #[cfg(feature = "http")]
    pub async fn run_http(&self) -> Result<()> {
        info!(
            "Starting MCP server: {} v{}",
            self.implementation.name, self.implementation.version
        );
        info!("Session management: enabled with automatic cleanup");

        if self.enable_sse {
            info!("SSE notifications: enabled at GET {}", self.mcp_path);
        }

        // Start session cleanup task
        let _cleanup_task = self.session_manager.clone().start_cleanup_task();

        // Create session-aware tool handler
        let tool_handler = SessionAwareToolHandler::new(
            self.tools.clone(),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );

        // Create session-aware initialize handler
        let init_handler = SessionAwareInitializeHandler::new(
            self.implementation.clone(),
            self.capabilities.clone(),
            self.instructions.clone(),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );

        // Build HTTP server with shared session storage from SessionManager
        let session_storage = self.session_manager.get_storage();
        debug!("Configuring HTTP MCP server with session storage backend");
        let mut builder =
            turul_http_mcp_server::HttpMcpServer::builder_with_storage(session_storage)
                .bind_address(self.bind_address)
                .mcp_path(&self.mcp_path)
                .cors(self.enable_cors)
                .get_sse(self.enable_sse) // GET SSE controlled by main server enable_sse flag
                // POST SSE remains at default (false) for compatibility
                .register_handler(vec!["initialize".to_string()], init_handler)
                .register_handler(
                    vec!["tools/list".to_string()],
                    ListToolsHandler::new(self.tools.clone()),
                )
                .register_handler(vec!["tools/call".to_string()], tool_handler);

        // Register all MCP handlers with session awareness
        for (method, handler) in &self.handlers {
            let bridge_handler =
                SessionAwareMcpHandlerBridge::new(handler.clone(), self.session_manager.clone(), self.strict_lifecycle);
            builder = builder.register_handler(vec![method.clone()], bridge_handler);
        }

        // Register special initialized notification handler that can mark sessions as initialized
        use crate::handlers::InitializedNotificationHandler;
        let initialized_handler = InitializedNotificationHandler::new(self.session_manager.clone());
        let initialized_bridge = SessionAwareMcpHandlerBridge::new(
            Arc::new(initialized_handler),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );
        builder = builder.register_handler(
            vec!["notifications/initialized".to_string()],
            initialized_bridge,
        );

        let http_server = builder.build();

        // SSE is now integrated directly into the session management
        if self.enable_sse {
            debug!("SSE support enabled with integrated session management");

            // Set up event forwarding bridge between SessionManager and StreamManager
            self.setup_sse_event_bridge(&http_server).await;
        }

        http_server.run().await.map_err(|http_err| {
            match http_err {
                turul_http_mcp_server::HttpMcpError::Mcp(mcp_err) => mcp_err,
                turul_http_mcp_server::HttpMcpError::Http(http_err) => McpError::transport(&http_err.to_string()),
                turul_http_mcp_server::HttpMcpError::JsonRpc(rpc_err) => McpError::json_rpc_protocol(&rpc_err.to_string()),
                turul_http_mcp_server::HttpMcpError::Serialization(ser_err) => McpError::SerializationError(ser_err),
                turul_http_mcp_server::HttpMcpError::Io(io_err) => McpError::IoError(io_err),
                turul_http_mcp_server::HttpMcpError::InvalidRequest(msg) => McpError::InvalidParameters(msg),
            }
        })?;
        Ok(())
    }

    /// Set up event forwarding bridge between SessionManager and StreamManager
    async fn setup_sse_event_bridge(&self, http_server: &turul_http_mcp_server::HttpMcpServer) {
        debug!("🌉 Setting up SSE event bridge between SessionManager and StreamManager");

        let stream_manager = http_server.get_stream_manager();
        let mut global_events = self.session_manager.subscribe_all_session_events();

        tokio::spawn(async move {
            debug!("🌐 SSE Event Bridge: Started listening for session events");

            while let Ok((session_id, event)) = global_events.recv().await {
                debug!(
                    "📡 SSE Bridge: Received event from session {}: {:?}",
                    session_id, event
                );

                // Convert SessionEvent to StreamManager event format
                match event {
                    crate::session::SessionEvent::Custom { event_type, data } => {
                        debug!(
                            "📤 SSE Bridge: Broadcasting custom event '{}' to StreamManager",
                            event_type
                        );

                        if let Err(e) = stream_manager
                            .broadcast_to_session(&session_id, event_type, data)
                            .await
                        {
                            error!(
                                "❌ SSE Bridge: Failed to broadcast to session {}: {}",
                                session_id, e
                            );
                        } else {
                            debug!(
                                "✅ SSE Bridge: Successfully broadcast to session {}",
                                session_id
                            );
                        }
                    }
                    other_event => {
                        debug!("⏭ SSE Bridge: Skipping non-custom event: {:?}", other_event);
                    }
                }
            }

            debug!("🚫 SSE Event Bridge: Global event receiver closed");
        });

        info!("✅ SSE event bridge established successfully");
    }

    /// Run the server and return the HTTP server handle for SSE access (requires "http" feature)
    #[cfg(feature = "http")]
    pub async fn run_with_sse_access(
        &self,
    ) -> Result<(
        turul_http_mcp_server::HttpMcpServer,
        tokio::task::JoinHandle<turul_http_mcp_server::Result<()>>,
    )> {
        info!(
            "Starting MCP server: {} v{}",
            self.implementation.name, self.implementation.version
        );
        info!("Session management: enabled with automatic cleanup");

        if self.enable_sse {
            info!("SSE notifications: enabled - SSE manager available for notifications");
        }

        // Start session cleanup task
        let _cleanup_task = self.session_manager.clone().start_cleanup_task();

        // Create session-aware tool handler
        let tool_handler = SessionAwareToolHandler::new(
            self.tools.clone(),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );

        // Create session-aware initialize handler
        let init_handler = SessionAwareInitializeHandler::new(
            self.implementation.clone(),
            self.capabilities.clone(),
            self.instructions.clone(),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );

        // Build HTTP server with shared session storage from SessionManager
        let session_storage = self.session_manager.get_storage();
        debug!("Configuring HTTP MCP server with session storage backend");
        let mut builder =
            turul_http_mcp_server::HttpMcpServer::builder_with_storage(session_storage)
                .bind_address(self.bind_address)
                .mcp_path(&self.mcp_path)
                .cors(self.enable_cors)
                .get_sse(self.enable_sse) // GET SSE controlled by main server enable_sse flag
                // POST SSE remains at default (false) for compatibility
                .register_handler(vec!["initialize".to_string()], init_handler)
                .register_handler(
                    vec!["tools/list".to_string()],
                    ListToolsHandler::new(self.tools.clone()),
                )
                .register_handler(vec!["tools/call".to_string()], tool_handler);

        // TODO investigate if this also adds the tools/list and tools/call handlers
        // Register all MCP handlers with session awareness
        for (method, handler) in &self.handlers {
            let bridge_handler =
                SessionAwareMcpHandlerBridge::new(handler.clone(), self.session_manager.clone(), self.strict_lifecycle);
            builder = builder.register_handler(vec![method.clone()], bridge_handler);
        }

        // Register special initialized notification handler that can mark sessions as initialized
        use crate::handlers::InitializedNotificationHandler;
        let initialized_handler = InitializedNotificationHandler::new(self.session_manager.clone());
        let initialized_bridge = SessionAwareMcpHandlerBridge::new(
            Arc::new(initialized_handler),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );
        builder = builder.register_handler(
            vec!["notifications/initialized".to_string()],
            initialized_bridge,
        );

        let http_server = builder.build();

        // Run server in background task
        let server_task = {
            let server = http_server.clone();
            tokio::spawn(async move { server.run().await })
        };

        Ok((http_server, server_task))
    }

    /// Get session storage configuration info
    pub fn session_storage_info(&self) -> &str {
        if let Some(storage) = &self.session_storage {
            debug!(
                "Accessing session storage for info - backend is configured: {:p}",
                storage
            );
            "Backend configured"
        } else {
            "No backend configured"
        }
    }
}

/// Session-aware bridge handler to adapt McpHandler to JsonRpcHandler
pub struct SessionAwareMcpHandlerBridge {
    handler: Arc<dyn McpHandler>,
    session_manager: Arc<SessionManager>,
    strict_lifecycle: bool,
}

impl SessionAwareMcpHandlerBridge {
    pub fn new(handler: Arc<dyn McpHandler>, session_manager: Arc<SessionManager>, strict_lifecycle: bool) -> Self {
        Self {
            handler,
            session_manager,
            strict_lifecycle,
        }
    }
}

#[async_trait]
impl JsonRpcHandler for SessionAwareMcpHandlerBridge {
    type Error = McpError;

    async fn handle(
        &self,
        method: &str,
        params: Option<turul_mcp_json_rpc_server::RequestParams>,
        session_context: Option<turul_mcp_json_rpc_server::r#async::SessionContext>,
    ) -> std::result::Result<serde_json::Value, McpError> {
        debug!("Handling {} request via session-aware bridge", method);

        // Convert JSON-RPC SessionContext to MCP SessionContext
        let mcp_session_context = if let Some(json_rpc_ctx) = session_context {
            debug!(
                "Converting JSON-RPC session context: session_id={}",
                json_rpc_ctx.session_id
            );
            Some(SessionContext::from_json_rpc_with_broadcaster(
                json_rpc_ctx, 
                self.session_manager.get_storage()
            ))
        } else {
            // Fallback: extract session ID from params (legacy behavior)
            let session_id = extract_session_id_from_params(&params);
            if let Some(sid) = session_id {
                debug!("Fallback: extracted session_id from params: {}", sid);
                self.session_manager.create_session_context(&sid)
            } else {
                None
            }
        };

        // MCP Lifecycle Guard: Ensure session is initialized before allowing operations (if strict mode enabled)
        if self.strict_lifecycle && method != "initialize" && method != "notifications/initialized"
            && let Some(ref session_ctx) = mcp_session_context {
                let session_initialized = self.session_manager.is_session_initialized(&session_ctx.session_id).await;
                if !session_initialized {
                    debug!(
                        "🚫 STRICT MODE: Rejecting {} request for session {} - session not yet initialized (waiting for notifications/initialized)",
                        method, session_ctx.session_id
                    );
                    return Err(McpError::SessionError(
                        "Session not initialized - client must send notifications/initialized first (strict lifecycle mode)".to_string()
                    ));
                }
            }

        // Convert JSON-RPC params to Value
        let mcp_params = params.map(|p| p.to_value());

        // Call the MCP handler with session context - propagate errors directly
        match self
            .handler
            .handle_with_session(mcp_params, mcp_session_context)
            .await
        {
            Ok(result) => Ok(result),
            Err(error) => {
                error!("MCP handler error: {}", error);
                Err(error) // Propagate McpError directly, no double-wrapping!
            }
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        self.handler.supported_methods()
    }
}

/// Extract session ID from request parameters (placeholder implementation)
fn extract_session_id_from_params(
    _params: &Option<turul_mcp_json_rpc_server::RequestParams>,
) -> Option<String> {
    // In a real implementation, this would extract session ID from HTTP headers
    // For now, return None as we'll implement proper session extraction later
    None
}

/// Session-aware handler for initialize requests
pub struct SessionAwareInitializeHandler {
    implementation: Implementation,
    capabilities: ServerCapabilities,
    instructions: Option<String>,
    session_manager: Arc<SessionManager>,
    strict_lifecycle: bool,
}

impl SessionAwareInitializeHandler {
    pub fn new(
        implementation: Implementation,
        capabilities: ServerCapabilities,
        instructions: Option<String>,
        session_manager: Arc<SessionManager>,
        strict_lifecycle: bool,
    ) -> Self {
        Self {
            implementation,
            capabilities,
            instructions,
            session_manager,
            strict_lifecycle,
        }
    }

    /// Negotiate protocol version with client
    ///
    /// Server supports backward compatibility with older protocol versions.
    /// The negotiation follows this priority:
    /// 1. Use client's requested version if server supports it
    /// 2. Use the highest version both client and server support
    /// 3. Fall back to minimum compatible version
    fn negotiate_version(&self, client_version: &str) -> std::result::Result<McpVersion, String> {
        use turul_mcp_protocol::version::McpVersion;

        // Parse client's requested version
        let requested_version = client_version.parse::<McpVersion>()
            .map_err(|_| format!("Unsupported protocol version: {}", client_version))?;

        // Define server's supported versions (all versions from 2024-11-05 to current)
        let supported_versions = [McpVersion::V2024_11_05,
            McpVersion::V2025_03_26,
            McpVersion::V2025_06_18];

        // Strategy 1: If server supports client's requested version, use it
        if supported_versions.contains(&requested_version) {
            return Ok(requested_version);
        }

        // Strategy 2: Use the highest version the server supports that's <= client version
        // This allows clients to request newer versions while falling back gracefully
        let negotiated = requested_version;

        // Verify the negotiated version is in our supported list
        if supported_versions.contains(&negotiated) {
            Ok(negotiated)
        } else {
            // Strategy 3: Fall back to minimum supported version
            Err(format!(
                "Cannot negotiate compatible version with client version {}",
                client_version
            ))
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

        info!(
            "Server capabilities adjusted for protocol version {}",
            version
        );
        debug!(
            "Capabilities: logging={}, tools={}, resources={}, prompts={}",
            adjusted.logging.is_some(),
            adjusted.tools.is_some(),
            adjusted.resources.is_some(),
            adjusted.prompts.is_some()
        );

        adjusted
    }
}

#[async_trait]
impl JsonRpcHandler for SessionAwareInitializeHandler {
    type Error = McpError;

    async fn handle(
        &self,
        method: &str,
        params: Option<turul_mcp_json_rpc_server::RequestParams>,
        session_context: Option<turul_mcp_json_rpc_server::r#async::SessionContext>,
    ) -> std::result::Result<serde_json::Value, McpError> {
        debug!("Handling {} request with session support", method);

        if method != "initialize" {
            return Err(McpError::InvalidParameters(format!(
                "Method not supported: {}",
                method
            )));
        }

        // Parse initialize request
        let request = if let Some(params) = params {
            let params_value = params.to_value();
            serde_json::from_value::<InitializeRequest>(params_value).map_err(|e| {
                McpError::InvalidParameters(format!(
                    "Invalid initialize request: {}",
                    e
                ))
            })?
        } else {
            return Err(McpError::MissingParameter(
                "Missing parameters for initialize".to_string(),
            ));
        };

        // Perform protocol version negotiation
        let negotiated_version = match self.negotiate_version(&request.protocol_version) {
            Ok(version) => {
                info!(
                    "Protocol version negotiated: {} (client requested: {})",
                    version, request.protocol_version
                );
                version
            }
            Err(e) => {
                error!("Protocol version negotiation failed: {}", e);
                return Err(McpError::ConfigurationError(
                    format!("Version negotiation failed: {}", e),
                ));
            }
        };

        // Use session ID provided by HTTP layer, or create new one if not provided
        let session_id = if let Some(ctx) = &session_context {
            debug!("Using session from context: {}", ctx.session_id);
            
            // Add session to cache if it doesn't exist there
            // This handles sessions created directly in storage by session_handler
            let cache_exists = self.session_manager.session_exists_in_cache(&ctx.session_id).await;
            debug!("Session {} exists in cache: {}", ctx.session_id, cache_exists);
            
            if !cache_exists {
                debug!("Session {} not in cache, checking storage", ctx.session_id);
                
                // Try to load session from storage with its actual capabilities
                match self.session_manager.load_session_from_storage(&ctx.session_id).await {
                    Ok(true) => {
                        debug!("Session {} loaded from storage with preserved capabilities", ctx.session_id);
                    }
                    Ok(false) => {
                        // Session doesn't exist in storage either - this shouldn't happen
                        // in normal flow but handle gracefully
                        warn!("Session {} not found in storage, creating with defaults", ctx.session_id);
                        self.session_manager.add_session_to_cache(
                            ctx.session_id.clone(),
                            self.session_manager.get_default_capabilities()
                        ).await;
                    }
                    Err(e) => {
                        error!("Failed to load session {} from storage: {}", ctx.session_id, e);
                        // Fallback to defaults only on error
                        self.session_manager.add_session_to_cache(
                            ctx.session_id.clone(),
                            self.session_manager.get_default_capabilities()
                        ).await;
                    }
                }
            } else {
                debug!("Session {} already exists in cache", ctx.session_id);
            }
            
            ctx.session_id.clone()
        } else {
            debug!("No session context provided, creating new session");
            self.session_manager.create_session().await
        };

        // Store client info and capabilities in session state for later initialization
        // Per MCP spec, session is NOT initialized until client sends notifications/initialized
        self.session_manager
            .set_session_state(
                &session_id,
                "client_info",
                serde_json::to_value(&request.client_info).map_err(|e| {
                    McpError::SerializationError(e)
                })?,
            )
            .await;

        self.session_manager
            .set_session_state(
                &session_id,
                "client_capabilities",
                serde_json::to_value(&request.capabilities).map_err(|e| {
                    McpError::SerializationError(e)
                })?,
            )
            .await;

        self.session_manager
            .set_session_state(
                &session_id,
                "negotiated_version",
                serde_json::to_value(negotiated_version).map_err(|e| {
                    McpError::SerializationError(e)
                })?,
            )
            .await;

        // Store negotiated version before initialization (differs by mode)

        // Store the negotiated version in session state for tools to access
        self.session_manager
            .set_session_state(
                &session_id,
                "mcp_version",
                serde_json::json!(negotiated_version.as_str()),
            )
            .await;

        // In lenient mode, immediately mark session as initialized
        // In strict mode, wait for notifications/initialized from client
        if !self.strict_lifecycle {
            debug!(
                "📝 LENIENT MODE: Immediately initializing session {} (strict_lifecycle=false)",
                session_id
            );
            if let Err(e) = self
                .session_manager
                .initialize_session_with_version(
                    &session_id,
                    request.client_info,
                    request.capabilities,
                    negotiated_version,
                )
                .await
            {
                error!("❌ Failed to initialize session {}: {}", session_id, e);
                return Err(McpError::SessionError(
                    format!("Failed to initialize session: {}", e),
                ));
            }
            info!(
                "✅ Session {} created and immediately initialized with protocol version {} (lenient mode)",
                session_id, negotiated_version
            );
        } else {
            info!(
                "⏳ Session {} created and ready for client with protocol version {} (strict mode - waiting for notifications/initialized)",
                session_id, negotiated_version
            );
        }

        // Create response with negotiated version and adjusted capabilities
        let adjusted_capabilities = self.adjust_capabilities_for_version(negotiated_version);
        let mut response = InitializeResult::new(
            negotiated_version,
            adjusted_capabilities,
            self.implementation.clone(),
        );

        if let Some(instructions) = &self.instructions {
            response = response.with_instructions(instructions.clone());
        }

        // Session ID is communicated to HTTP layer via session manager

        serde_json::to_value(response).map_err(|e| {
            McpError::SerializationError(e)
        })
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["initialize".to_string()]
    }
}

/// Handler for tools/list requests
pub struct ListToolsHandler {
    tools: HashMap<String, Arc<dyn McpTool>>,
}

impl ListToolsHandler {
    pub fn new(tools: HashMap<String, Arc<dyn McpTool>>) -> Self {
        Self { tools }
    }
}

#[async_trait]
impl JsonRpcHandler for ListToolsHandler {
    type Error = McpError;

    async fn handle(
        &self,
        method: &str,
        params: Option<turul_mcp_json_rpc_server::RequestParams>,
        _session_context: Option<turul_mcp_json_rpc_server::r#async::SessionContext>,
    ) -> std::result::Result<serde_json::Value, McpError> {
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};

        debug!("Handling {} request", method);

        if method != "tools/list" {
            return Err(McpError::InvalidParameters(format!(
                "Method '{}' not supported by tools/list handler", method
            )));
        }

        // Parse typed parameters for cursor and meta propagation
        use turul_mcp_protocol::tools::{ListToolsParams, ListToolsResult};
        let list_params = if let Some(params_value) = params {
            serde_json::from_value::<ListToolsParams>(params_value.to_value()).map_err(|e| {
                McpError::InvalidParameters(format!("Invalid parameters for tools/list: {}", e))
            })?
        } else {
            ListToolsParams::new()
        };

        let cursor = list_params.cursor;
        debug!("Listing tools with cursor: {:?}", cursor);

        // Convert tools to descriptors and sort by name for stable pagination
        let mut tools: Vec<Tool> = self
            .tools
            .values()
            .map(|tool| tool_to_descriptor(tool.as_ref()))
            .collect();

        // Sort by tool name to ensure stable ordering for pagination
        tools.sort_by(|a, b| a.name.cmp(&b.name));

        let mut base_response = ListToolsResult::new(tools.clone());

        // For simplicity, showing all tools in one page
        // In a real implementation with many tools, implement proper pagination like other handlers
        let has_more = false;
        let total = Some(tools.len() as u64);
        let next_cursor: Option<Cursor> = None; // No next page in this simple implementation

        // Set top-level nextCursor field on the result before wrapping
        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        let mut paginated_response = PaginatedResponse::with_pagination(
            base_response,
            next_cursor,
            total,
            has_more,
        );

        // Propagate optional _meta from request to response (MCP 2025-06-18 compliance)
        if let Some(meta) = list_params.meta {
            let mut meta_obj = turul_mcp_protocol::meta::Meta::new();
            meta_obj.extra = meta;
            paginated_response = paginated_response.with_meta(meta_obj);
        }

        serde_json::to_value(paginated_response).map_err(|e| {
            McpError::SerializationError(e)
        })
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tools/list".to_string()]
    }
}

/// Session-aware handler for tool execution
pub struct SessionAwareToolHandler {
    tools: HashMap<String, Arc<dyn McpTool>>,
    session_manager: Arc<SessionManager>,
    strict_lifecycle: bool,
}

impl SessionAwareToolHandler {
    pub fn new(
        tools: HashMap<String, Arc<dyn McpTool>>,
        session_manager: Arc<SessionManager>,
        strict_lifecycle: bool,
    ) -> Self {
        Self {
            tools,
            session_manager,
            strict_lifecycle,
        }
    }
}

#[async_trait]
impl JsonRpcHandler for SessionAwareToolHandler {
    type Error = McpError;

    async fn handle(
        &self,
        method: &str,
        params: Option<turul_mcp_json_rpc_server::RequestParams>,
        session_context: Option<turul_mcp_json_rpc_server::r#async::SessionContext>,
    ) -> std::result::Result<serde_json::Value, McpError> {
        debug!("Handling {} request with session support", method);

        if method != "tools/call" {
            return Err(McpError::InvalidParameters(format!(
                "Method '{}' not supported by tools/call handler", method
            )));
        }

        // MCP Lifecycle Guard: Ensure session is initialized before allowing tool operations (if strict mode enabled)
        if self.strict_lifecycle {
            if let Some(ref session_ctx) = session_context {
                let session_initialized = self.session_manager.is_session_initialized(&session_ctx.session_id).await;
                if !session_initialized {
                    debug!(
                        "🚫 STRICT MODE: Rejecting {} request for session {} - session not yet initialized (waiting for notifications/initialized)",
                        method, session_ctx.session_id
                    );
                    return Err(McpError::configuration(
                        "Session not initialized - client must send notifications/initialized first (strict lifecycle mode)",
                    ));
                }
                debug!(
                    "✅ STRICT MODE: Session {} is initialized - allowing {} request",
                    session_ctx.session_id, method
                );
            }
        } else {
            debug!(
                "📝 LENIENT MODE: Allowing {} request without lifecycle check (strict_lifecycle=false)",
                method
            );
        }

        let params = params.ok_or_else(|| {
            McpError::MissingParameter("CallToolRequest".to_string())
        })?;

        // Use the parameter extraction pattern from the other project
        use turul_mcp_protocol::param_extraction::extract_params;

        let call_params: turul_mcp_protocol::tools::CallToolParams = extract_params(params)?;

        // Find the tool
        let tool = self.tools.get(&call_params.name).ok_or_else(|| {
            McpError::ToolNotFound(call_params.name.clone())
        })?;

        // Convert JSON-RPC SessionContext to MCP SessionContext for tool execution
        let mcp_session_context = if let Some(json_rpc_ctx) = session_context {
            debug!(
                "Converting JSON-RPC session context for tool call: session_id={}",
                json_rpc_ctx.session_id
            );
            Some(SessionContext::from_json_rpc_with_broadcaster(
                json_rpc_ctx, 
                self.session_manager.get_storage()
            ))
        } else {
            debug!("No session context provided for tool call");
            None
        };

        // Execute the tool with session context
        let args = call_params
            .arguments
            .map(|hashmap| {
                serde_json::to_value(hashmap)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
            })
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        match tool.call(args, mcp_session_context).await {
            Ok(response) => serde_json::to_value(response).map_err(|e| {
                McpError::SerializationError(e)
            }),
            Err(error_msg) => {
                error!("Tool execution error: {}", error_msg);
                let error_content = vec![ToolResult::text(format!("Error: {}", error_msg))];
                let response = CallToolResult::error(error_content);
                serde_json::to_value(response).map_err(|e| {
                    McpError::SerializationError(e)
                })
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
    use serde_json::Value;
    use std::collections::HashMap;
    use turul_mcp_protocol::ToolSchema;
    use turul_mcp_protocol::tools::{
        CallToolResult, HasAnnotations, HasBaseMetadata, HasDescription, HasInputSchema,
        HasOutputSchema, HasToolMeta, ToolResult,
    };

    struct TestTool {
        input_schema: ToolSchema,
    }

    impl TestTool {
        fn new() -> Self {
            Self {
                input_schema: ToolSchema::object(),
            }
        }
    }

    impl HasBaseMetadata for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn title(&self) -> Option<&str> {
            Some("Test Tool")
        }
    }

    impl HasDescription for TestTool {
        fn description(&self) -> Option<&str> {
            Some("Test tool for unit tests")
        }
    }

    impl HasInputSchema for TestTool {
        fn input_schema(&self) -> &ToolSchema {
            &self.input_schema
        }
    }

    impl HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            None
        }
    }

    impl HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }

    impl HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    #[async_trait]
    impl McpTool for TestTool {
        async fn call(
            &self,
            _args: Value,
            _session: Option<crate::SessionContext>,
        ) -> crate::McpResult<CallToolResult> {
            Ok(CallToolResult::success(vec![ToolResult::text(
                "test result",
            )]))
        }
    }

    #[test]
    fn test_server_creation() {
        let server = McpServer::builder()
            .name("test-server")
            .version("1.0.0")
            .tool(TestTool::new())
            .build()
            .unwrap();

        assert_eq!(server.implementation.name, "test-server");
        assert_eq!(server.implementation.version, "1.0.0");
        assert_eq!(server.tools.len(), 1);
    }

    #[tokio::test]
    async fn test_list_tools_handler() {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert("test".to_string(), Arc::new(TestTool::new()));

        let handler = ListToolsHandler::new(tools);
        let result = handler.handle("tools/list", None, None).await.unwrap();

        let response: ListToolsResult = serde_json::from_value(result).unwrap();
        assert_eq!(response.tools.len(), 1);
        assert_eq!(response.tools[0].name, "test");
    }

    #[tokio::test]
    async fn test_tool_handler() {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert("test".to_string(), Arc::new(TestTool::new()));

        let session_manager = Arc::new(SessionManager::new(ServerCapabilities::default()));
        let handler = SessionAwareToolHandler::new(tools, session_manager, false);
        // Create params matching the CallToolParams structure
        let params = turul_mcp_json_rpc_server::RequestParams::Object(
            [
                ("name".to_string(), serde_json::json!("test")),
                ("arguments".to_string(), serde_json::json!({})),
            ]
            .into_iter()
            .collect(),
        );

        let result = handler
            .handle("tools/call", Some(params), None)
            .await
            .unwrap();
        let response: CallToolResult = serde_json::from_value(result).unwrap();

        assert_eq!(response.content.len(), 1);
        if let ToolResult::Text { text } = &response.content[0] {
            assert_eq!(text, "test result");
        }
    }
}
