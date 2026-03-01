//! Lambda MCP Server Implementation
//!
//! This module provides the main Lambda MCP server implementation that mirrors
//! the architecture of turul-mcp-server but adapted for AWS Lambda deployment.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::info;

use turul_http_mcp_server::{ServerConfig, StreamConfig, StreamManager};
use turul_mcp_protocol::{Implementation, ServerCapabilities};
use turul_mcp_server::{
    McpCompletion, McpElicitation, McpLogger, McpNotification, McpPrompt, McpResource, McpRoot,
    McpSampling, McpTool, handlers::McpHandler, session::SessionManager,
};
use turul_mcp_session_storage::BoxedSessionStorage;

use crate::error::Result;
use crate::handler::LambdaMcpHandler;

#[cfg(feature = "cors")]
use crate::cors::CorsConfig;

/// Main Lambda MCP server
///
/// This server stores all configuration and can create Lambda handlers when needed.
/// It mirrors the architecture of McpServer but is designed for Lambda deployment.
#[allow(dead_code)]
pub struct LambdaMcpServer {
    /// Server implementation information
    pub implementation: Implementation,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Registered tools
    tools: HashMap<String, Arc<dyn McpTool>>,
    /// Registered resources
    resources: HashMap<String, Arc<dyn McpResource>>,
    /// Registered prompts
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
    /// Registered elicitations
    elicitations: HashMap<String, Arc<dyn McpElicitation>>,
    /// Registered sampling providers
    sampling: HashMap<String, Arc<dyn McpSampling>>,
    /// Registered completion providers
    completions: HashMap<String, Arc<dyn McpCompletion>>,
    /// Registered loggers
    loggers: HashMap<String, Arc<dyn McpLogger>>,
    /// Registered root providers
    root_providers: HashMap<String, Arc<dyn McpRoot>>,
    /// Registered notification providers
    notifications: HashMap<String, Arc<dyn McpNotification>>,
    /// Registered handlers
    handlers: HashMap<String, Arc<dyn McpHandler>>,
    /// Configured roots
    roots: Vec<turul_mcp_protocol::roots::Root>,
    /// Optional client instructions
    instructions: Option<String>,
    /// Session manager for state persistence
    session_manager: Arc<SessionManager>,
    /// Session storage backend (shared between SessionManager and handler)
    session_storage: Arc<BoxedSessionStorage>,
    /// Strict MCP lifecycle enforcement
    strict_lifecycle: bool,
    /// Server configuration
    server_config: ServerConfig,
    /// Enable SSE streaming
    enable_sse: bool,
    /// Stream configuration
    stream_config: StreamConfig,
    /// CORS configuration (if enabled)
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
    /// Middleware stack for request/response interception
    middleware_stack: turul_http_mcp_server::middleware::MiddlewareStack,
    /// Optional task runtime for MCP task support
    task_runtime: Option<Arc<turul_mcp_server::TaskRuntime>>,
}

impl LambdaMcpServer {
    /// Create a new Lambda MCP server (use builder instead)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        implementation: Implementation,
        capabilities: ServerCapabilities,
        tools: HashMap<String, Arc<dyn McpTool>>,
        resources: HashMap<String, Arc<dyn McpResource>>,
        prompts: HashMap<String, Arc<dyn McpPrompt>>,
        elicitations: HashMap<String, Arc<dyn McpElicitation>>,
        sampling: HashMap<String, Arc<dyn McpSampling>>,
        completions: HashMap<String, Arc<dyn McpCompletion>>,
        loggers: HashMap<String, Arc<dyn McpLogger>>,
        root_providers: HashMap<String, Arc<dyn McpRoot>>,
        notifications: HashMap<String, Arc<dyn McpNotification>>,
        handlers: HashMap<String, Arc<dyn McpHandler>>,
        roots: Vec<turul_mcp_protocol::roots::Root>,
        instructions: Option<String>,
        session_storage: Arc<BoxedSessionStorage>,
        strict_lifecycle: bool,
        server_config: ServerConfig,
        enable_sse: bool,
        stream_config: StreamConfig,
        #[cfg(feature = "cors")] cors_config: Option<CorsConfig>,
        middleware_stack: turul_http_mcp_server::middleware::MiddlewareStack,
        task_runtime: Option<Arc<turul_mcp_server::TaskRuntime>>,
    ) -> Self {
        // Create session manager with server capabilities
        let session_manager = Arc::new(SessionManager::with_storage_and_timeouts(
            Arc::clone(&session_storage),
            capabilities.clone(),
            std::time::Duration::from_secs(30 * 60), // 30 minutes default
            std::time::Duration::from_secs(60),      // 1 minute cleanup interval
        ));

        Self {
            implementation,
            capabilities,
            tools,
            resources,
            prompts,
            elicitations,
            sampling,
            completions,
            loggers,
            root_providers,
            notifications,
            handlers,
            roots,
            instructions,
            session_manager,
            session_storage,
            strict_lifecycle,
            server_config,
            enable_sse,
            stream_config,
            #[cfg(feature = "cors")]
            cors_config,
            middleware_stack,
            task_runtime,
        }
    }

    /// Get a reference to the server capabilities.
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    /// Create a Lambda handler ready for use with Lambda runtime
    ///
    /// This is equivalent to McpServer::run_http() but creates a handler instead of running a server.
    pub async fn handler(&self) -> Result<LambdaMcpHandler> {
        info!(
            "Creating Lambda MCP handler: {} v{}",
            self.implementation.name, self.implementation.version
        );
        info!("Session management: enabled with automatic cleanup");

        if self.enable_sse {
            info!("SSE notifications: enabled for Lambda responses");

            // ⚠️ GUARDRAIL: SSE enabled without streaming feature
            #[cfg(not(feature = "streaming"))]
            {
                use tracing::warn;
                warn!("⚠️  SSE is enabled but 'streaming' feature is not available!");
                warn!(
                    "   For real SSE streaming, use handle_streaming() with run_with_streaming_response"
                );
                warn!(
                    "   Current handle() method will return SSE snapshots, not real-time streams"
                );
                warn!("   To enable streaming: add 'streaming' feature to turul-mcp-aws-lambda");
            }
        }

        // Start session cleanup task (same as MCP server)
        let _cleanup_task = self.session_manager.clone().start_cleanup_task();

        // Cold-start recovery: handler() is called once per Lambda cold start from main().
        // The returned LambdaMcpHandler is Clone'd for each request — recovery runs exactly once.
        if let Some(ref runtime) = self.task_runtime {
            match runtime.recover_stuck_tasks().await {
                Ok(recovered) if !recovered.is_empty() => {
                    info!(
                        count = recovered.len(),
                        "Recovered stuck tasks from previous invocations"
                    );
                }
                Err(e) => {
                    use tracing::warn;
                    warn!(error = %e, "Failed to recover stuck tasks on startup");
                }
                _ => {}
            }
        }

        // Create stream manager for SSE
        let stream_manager = Arc::new(StreamManager::with_config(
            self.session_storage.clone(),
            self.stream_config.clone(),
        ));

        // Create JSON-RPC dispatcher
        use turul_mcp_json_rpc_server::JsonRpcDispatcher;
        let mut dispatcher = JsonRpcDispatcher::new();

        // Create session-aware initialize handler (reuse MCP server handler)
        use turul_mcp_server::SessionAwareInitializeHandler;
        let init_handler = SessionAwareInitializeHandler::new(
            self.implementation.clone(),
            self.capabilities.clone(),
            self.instructions.clone(),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );
        dispatcher.register_method("initialize".to_string(), init_handler);

        // Create tools/list handler (reuse MCP server handler)
        use turul_mcp_server::ListToolsHandler;
        let list_handler = ListToolsHandler::new(self.tools.clone(), self.task_runtime.is_some());
        dispatcher.register_method("tools/list".to_string(), list_handler);

        // Create session-aware tool handler for tools/call (reuse MCP server handler)
        use turul_mcp_server::SessionAwareToolHandler;
        let mut tool_handler = SessionAwareToolHandler::new(
            self.tools.clone(),
            self.session_manager.clone(),
            self.strict_lifecycle,
        );
        if let Some(ref runtime) = self.task_runtime {
            tool_handler = tool_handler.with_task_runtime(Arc::clone(runtime));
        }
        dispatcher.register_method("tools/call".to_string(), tool_handler);

        // Register all MCP handlers with session awareness (reuse MCP server bridge)
        use turul_mcp_server::SessionAwareMcpHandlerBridge;
        for (method, handler) in &self.handlers {
            let bridge_handler = SessionAwareMcpHandlerBridge::new(
                handler.clone(),
                self.session_manager.clone(),
                self.strict_lifecycle,
            );
            dispatcher.register_method(method.clone(), bridge_handler);
        }

        // Create the Lambda handler with all components and middleware
        let middleware_stack = Arc::new(self.middleware_stack.clone());

        Ok(LambdaMcpHandler::with_middleware(
            self.server_config.clone(),
            Arc::new(dispatcher),
            self.session_storage.clone(),
            stream_manager,
            self.stream_config.clone(),
            self.capabilities.clone(),
            middleware_stack,
            self.enable_sse,
        ))
    }

    /// Get information about the session storage backend
    pub fn session_storage_info(&self) -> &str {
        "Session storage configured"
    }
}
