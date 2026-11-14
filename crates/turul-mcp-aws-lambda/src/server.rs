//! Lambda MCP server
//!
//! A [`LambdaMcpServer`] wraps the [`McpServer`] its builder produced (ADR-035)
//! and adds only Lambda transport: the request adapter, streaming, CORS and the
//! Lambda `ServerConfig`. Handlers, capabilities and the resource-security
//! policy are the core server's.

use std::sync::Arc;

use tracing::{debug, info};

use turul_http_mcp_server::{ServerConfig, StreamConfig, StreamManager};
use turul_mcp_protocol::{Implementation, ServerCapabilities};
use turul_mcp_server::McpServer;

use crate::error::Result;
use crate::handler::LambdaMcpHandler;

#[cfg(feature = "cors")]
use crate::cors::CorsConfig;

/// Main Lambda MCP server.
pub struct LambdaMcpServer {
    /// Server implementation information, as built.
    pub implementation: Implementation,
    /// Server capabilities, as built. Read-only: capabilities are configured
    /// through the builder (ADR-035), and `server/discover` reports those.
    pub capabilities: ServerCapabilities,
    /// The core server: the single owner of handlers and capabilities.
    core: McpServer,
    /// Lambda HTTP-layer configuration.
    server_config: ServerConfig,
    /// Enable SSE responses.
    enable_sse: bool,
    /// Streaming configuration.
    stream_config: StreamConfig,
    /// CORS configuration (if enabled).
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
}

impl LambdaMcpServer {
    /// Wrap a built core server with Lambda transport settings (use the builder).
    pub(crate) fn new(
        core: McpServer,
        server_config: ServerConfig,
        enable_sse: bool,
        stream_config: StreamConfig,
        #[cfg(feature = "cors")] cors_config: Option<CorsConfig>,
    ) -> Self {
        Self {
            implementation: core.implementation.clone(),
            capabilities: core.capabilities.clone(),
            core,
            server_config,
            enable_sse,
            stream_config,
            #[cfg(feature = "cors")]
            cors_config,
        }
    }

    /// Get a reference to the server capabilities.
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    /// Create a Lambda handler ready for use with the Lambda runtime.
    ///
    /// The Lambda counterpart of `McpServer::run_http()`: it serves the core
    /// server's dispatcher through the Lambda adapter. Call once per cold start.
    pub async fn handler(&self) -> Result<LambdaMcpHandler> {
        info!(
            "Creating Lambda MCP handler: {} v{}",
            self.implementation.name, self.implementation.version
        );

        if self.enable_sse {
            info!("SSE notifications: enabled for Lambda responses");

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

        let parts = self.core.transport_parts();

        // Start session cleanup task (same as the HTTP server)
        let _cleanup_task = parts.session_manager.clone().start_cleanup_task();

        // Sync tool registry with shared storage on cold start (coordination mode only)
        #[cfg(feature = "dynamic-tools")]
        if parts.coordination_enabled
            && let Some(ref registry) = parts.tool_registry
        {
            use tracing::warn;
            match registry.sync_from_storage().await {
                Ok(_) => info!("Dynamic: synced tool registry with shared storage"),
                Err(e) => {
                    warn!(error = %e, "Dynamic: failed to sync with shared storage on cold start")
                }
            }
        }

        // Cold-start recovery: handler() runs once per Lambda cold start; the
        // returned LambdaMcpHandler is cloned per request.
        #[cfg(feature = "protocol-2025-11-25")]
        if let Some(ref runtime) = parts.task_runtime {
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

        let stream_manager = Arc::new(StreamManager::with_config(
            parts.session_storage.clone(),
            self.stream_config.clone(),
        ));

        // Awaited event dispatcher: custom events persist through the
        // StreamManager on the request path (guaranteed persistence).
        {
            use turul_mcp_server::SessionEventDispatcher;

            struct LambdaEventDispatcher {
                stream_manager: Arc<StreamManager>,
            }

            #[async_trait::async_trait]
            impl SessionEventDispatcher for LambdaEventDispatcher {
                async fn dispatch_to_session(
                    &self,
                    session_id: &str,
                    event_type: String,
                    data: serde_json::Value,
                ) -> std::result::Result<(), String> {
                    self.stream_manager
                        .broadcast_to_session(session_id, event_type, data)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                }
            }

            parts
                .session_manager
                .set_event_dispatcher(Arc::new(LambdaEventDispatcher {
                    stream_manager: Arc::clone(&stream_manager),
                }))
                .await;
            debug!("Lambda event dispatcher installed (guaranteed persistence for Custom events)");
        }

        let handler = LambdaMcpHandler::with_middleware_and_fingerprint(
            self.server_config.clone(),
            Arc::new(parts.dispatcher),
            parts.session_storage,
            stream_manager,
            self.stream_config.clone(),
            parts.capabilities,
            Arc::new(parts.middleware_stack),
            self.enable_sse,
            parts.route_registry,
            Some(parts.tool_fingerprint),
        )
        .with_server_info(parts.implementation)
        .with_tool_notifier(parts.tool_notifier);

        #[cfg(feature = "dynamic-tools")]
        let handler = match parts.tool_registry {
            Some(registry) => handler.with_tool_registry(registry),
            None => handler,
        };

        // CORS must be applied last: every CORS branch inside LambdaMcpHandler
        // reads the config set here.
        #[cfg(feature = "cors")]
        let handler = match self.cors_config.clone() {
            Some(cors) => handler.with_cors(cors),
            None => handler,
        };

        Ok(handler)
    }

    /// Get information about the session storage backend
    pub fn session_storage_info(&self) -> &str {
        "Session storage configured"
    }
}
