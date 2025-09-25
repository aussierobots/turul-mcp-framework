//! HTTP MCP Server with SessionStorage integration
//!
//! This server provides MCP 2025-06-18 compliant HTTP transport with
//! pluggable session storage backends and proper SSE resumability.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

use turul_mcp_json_rpc_server::{JsonRpcDispatcher, JsonRpcHandler};
use turul_mcp_protocol::McpError;
use turul_mcp_session_storage::InMemorySessionStorage;

use crate::{CorsLayer, Result, SessionMcpHandler, StreamConfig, StreamManager};
use crate::streamable_http::{StreamableHttpHandler, McpProtocolVersion};

/// Configuration for the HTTP MCP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to
    pub bind_address: SocketAddr,
    /// Path for MCP endpoint
    pub mcp_path: String,
    /// Enable CORS
    pub enable_cors: bool,
    /// Maximum request body size
    pub max_body_size: usize,
    /// Enable GET SSE support (persistent event streams)
    pub enable_get_sse: bool,
    /// Enable POST SSE support (streaming tool call responses) - disabled by default for compatibility
    pub enable_post_sse: bool,
    /// Session expiry time in minutes (default: 30 minutes)
    pub session_expiry_minutes: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            mcp_path: "/mcp".to_string(),
            enable_cors: true,
            max_body_size: 1024 * 1024,            // 1MB
            enable_get_sse: cfg!(feature = "sse"), // GET SSE enabled if "sse" feature is compiled
            enable_post_sse: false, // Disabled by default for better client compatibility (e.g., MCP Inspector)
            session_expiry_minutes: 30, // 30 minutes default
        }
    }
}

/// Builder for HTTP MCP server with pluggable storage
pub struct HttpMcpServerBuilder {
    config: ServerConfig,
    dispatcher: JsonRpcDispatcher<McpError>,
    session_storage: Option<Arc<turul_mcp_session_storage::BoxedSessionStorage>>,
    stream_config: StreamConfig,
}

impl HttpMcpServerBuilder {
    /// Create a new builder with in-memory storage (zero-configuration)
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            dispatcher: JsonRpcDispatcher::<McpError>::new(),
            session_storage: Some(Arc::new(InMemorySessionStorage::new())),
            stream_config: StreamConfig::default(),
        }
    }
}

impl HttpMcpServerBuilder {
    /// Create a new builder with specific session storage
    pub fn with_storage(
        session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    ) -> Self {
        Self {
            config: ServerConfig::default(),
            dispatcher: JsonRpcDispatcher::<McpError>::new(),
            session_storage: Some(session_storage),
            stream_config: StreamConfig::default(),
        }
    }

    /// Set the bind address
    pub fn bind_address(mut self, addr: SocketAddr) -> Self {
        self.config.bind_address = addr;
        self
    }

    /// Set the MCP endpoint path
    pub fn mcp_path(mut self, path: impl Into<String>) -> Self {
        self.config.mcp_path = path.into();
        self
    }

    /// Enable or disable CORS
    pub fn cors(mut self, enable: bool) -> Self {
        self.config.enable_cors = enable;
        self
    }

    /// Set maximum request body size
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.config.max_body_size = size;
        self
    }

    /// Enable or disable GET SSE for persistent event streams
    pub fn get_sse(mut self, enable: bool) -> Self {
        self.config.enable_get_sse = enable;
        self
    }

    /// Enable or disable POST SSE for streaming tool call responses (disabled by default for compatibility)
    pub fn post_sse(mut self, enable: bool) -> Self {
        self.config.enable_post_sse = enable;
        self
    }

    /// Enable or disable both GET and POST SSE (convenience method)
    pub fn sse(mut self, enable: bool) -> Self {
        self.config.enable_get_sse = enable;
        self.config.enable_post_sse = enable;
        self
    }

    /// Set session expiry time in minutes
    pub fn session_expiry_minutes(mut self, minutes: u64) -> Self {
        self.config.session_expiry_minutes = minutes;
        self
    }

    /// Configure SSE streaming settings
    pub fn stream_config(mut self, config: StreamConfig) -> Self {
        self.stream_config = config;
        self
    }

    /// Register a JSON-RPC handler for specific methods
    pub fn register_handler<H>(mut self, methods: Vec<String>, handler: H) -> Self
    where
        H: JsonRpcHandler<Error = McpError> + 'static,
    {
        self.dispatcher.register_methods(methods, handler);
        self
    }

    /// Register a default handler for unhandled methods
    pub fn default_handler<H>(mut self, handler: H) -> Self
    where
        H: JsonRpcHandler<Error = McpError> + 'static,
    {
        self.dispatcher.set_default_handler(handler);
        self
    }

    /// Build the HTTP MCP server
    pub fn build(self) -> HttpMcpServer {
        let session_storage = self
            .session_storage
            .expect("Session storage must be provided");

        // ✅ CORRECTED ARCHITECTURE: Create single shared StreamManager instance
        let stream_manager = Arc::new(StreamManager::with_config(
            Arc::clone(&session_storage),
            self.stream_config.clone(),
        ));

        // Create shared dispatcher Arc
        let dispatcher = Arc::new(self.dispatcher);

        // Create StreamableHttpHandler for MCP 2025-06-18 support
        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(self.config.clone()),
            Arc::clone(&dispatcher),
            Arc::clone(&session_storage),
            Arc::clone(&stream_manager),
        );

        HttpMcpServer {
            config: self.config,
            dispatcher,
            session_storage,
            stream_config: self.stream_config,
            stream_manager,
            streamable_handler,
        }
    }
}

impl Default for HttpMcpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP MCP Server with SessionStorage integration
#[derive(Clone)]
pub struct HttpMcpServer {
    config: ServerConfig,
    dispatcher: Arc<JsonRpcDispatcher<McpError>>,
    session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    stream_config: StreamConfig,
    // ✅ CORRECTED ARCHITECTURE: Single shared StreamManager instance
    stream_manager: Arc<StreamManager>,
    // StreamableHttpHandler for MCP 2025-06-18 clients
    streamable_handler: StreamableHttpHandler,
}

impl HttpMcpServer {
    /// Create a new builder with default in-memory storage
    pub fn builder() -> HttpMcpServerBuilder {
        HttpMcpServerBuilder::new()
    }
}

impl HttpMcpServer {
    /// Create a new builder with specific session storage
    pub fn builder_with_storage(
        session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    ) -> HttpMcpServerBuilder {
        HttpMcpServerBuilder::with_storage(session_storage)
    }

    /// Get the shared StreamManager instance for event forwarding bridge
    /// Returns reference to the same StreamManager used by HTTP server
    pub fn get_stream_manager(&self) -> Arc<crate::StreamManager> {
        Arc::clone(&self.stream_manager)
    }

    /// Run the server with session management
    pub async fn run(&self) -> Result<()> {
        // Start session cleanup task
        self.start_session_cleanup().await;

        let listener = TcpListener::bind(&self.config.bind_address).await?;
        info!("HTTP MCP server listening on {}", self.config.bind_address);
        info!("MCP endpoint available at: {}", self.config.mcp_path);
        info!("Session storage: {}", self.session_storage.backend_name());

        // ✅ CORRECTED ARCHITECTURE: Create single SessionMcpHandler instance outside the loop
        let session_handler = SessionMcpHandler::with_shared_stream_manager(
            self.config.clone(),
            Arc::clone(&self.dispatcher),
            Arc::clone(&self.session_storage),
            self.stream_config.clone(),
            Arc::clone(&self.stream_manager),
        );

        // Create combined handler that routes based on protocol version
        let handler = McpRequestHandler {
            session_handler,
            streamable_handler: self.streamable_handler.clone(),
        };

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            debug!("New connection from {}", peer_addr);

            let handler_clone = handler.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(move |req| handle_request(req, handler_clone.clone()));

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    // Filter out common client disconnection errors that aren't actual problems
                    let err_str = err.to_string();
                    if err_str.contains("connection closed before message completed") {
                        debug!("Client disconnected (normal): {}", err);
                    } else {
                        error!("Error serving connection: {}", err);
                    }
                }
            });
        }
    }

    /// Start background session cleanup task
    async fn start_session_cleanup(&self) {
        let storage = Arc::clone(&self.session_storage);
        let session_expiry_minutes = self.config.session_expiry_minutes;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;

                let expire_time = std::time::SystemTime::now()
                    - std::time::Duration::from_secs(session_expiry_minutes * 60);
                match storage.expire_sessions(expire_time).await {
                    Ok(expired) => {
                        if !expired.is_empty() {
                            info!("Expired {} sessions", expired.len());
                            for session_id in expired {
                                debug!("Expired session: {}", session_id);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Session cleanup error: {}", err);
                    }
                }
            }
        });
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> ServerStats {
        let session_count = self.session_storage.session_count().await.unwrap_or(0);
        let event_count = self.session_storage.event_count().await.unwrap_or(0);

        ServerStats {
            sessions: session_count,
            events: event_count,
            storage_type: self.session_storage.backend_name().to_string(),
        }
    }
}

/// Handle requests with MCP 2025-06-18 compliance
/// Combined handler that routes based on MCP protocol version
#[derive(Clone)]
struct McpRequestHandler {
    session_handler: SessionMcpHandler,
    streamable_handler: StreamableHttpHandler,
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    handler: McpRequestHandler,
) -> std::result::Result<
    Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>,
    hyper::Error,
> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();

    debug!("Handling {} {}", method, path);

    // Route the request
    let response = if path == handler.session_handler.config.mcp_path {
        // Extract MCP protocol version from headers
        let protocol_version_str = req
            .headers()
            .get("MCP-Protocol-Version")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("2025-06-18"); // Default to latest version (we only support the latest protocol)

        let protocol_version = McpProtocolVersion::parse_version(protocol_version_str)
            .unwrap_or(McpProtocolVersion::V2025_06_18);

        debug!(
            "MCP request: protocol_version={}, method={}",
            protocol_version.as_str(),
            method
        );

        // Route based on protocol version - MCP 2025-06-18 uses Streamable HTTP, older versions use SessionMcpHandler
        debug!(
            "Routing MCP request: protocol_version={}, method={}, handler={}",
            protocol_version.as_str(),
            method,
            if protocol_version.supports_streamable_http() { "StreamableHttpHandler" } else { "SessionMcpHandler" }
        );

        if protocol_version.supports_streamable_http() {
            // Use StreamableHttpHandler for MCP 2025-06-18 clients
            let streamable_response = handler.streamable_handler.handle_request(req).await;
            Ok(streamable_response)
        } else {
            // Use SessionMcpHandler for legacy clients (MCP 2024-11-05 and earlier)
            match handler.session_handler.handle_mcp_request(req).await {
                Ok(mcp_response) => Ok(mcp_response),
                Err(err) => {
                    error!("Request handling error: {}", err);
                    Ok(Response::builder()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(
                            Full::new(Bytes::from(format!("Internal Server Error: {}", err)))
                                .map_err(|never| match never {})
                                .boxed_unsync(),
                        )
                        .unwrap())
                }
            }
        }
    } else {
        // 404 for other paths
        Ok(Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(
                Full::new(Bytes::from("Not Found"))
                    .map_err(|never| match never {})
                    .boxed_unsync(),
            )
            .unwrap())
    };

    // Apply CORS if enabled
    match response {
        Ok(mut final_response) => {
            if handler.session_handler.config.enable_cors {
                CorsLayer::apply_cors_headers(final_response.headers_mut());
            }
            Ok(final_response)
        }
        Err(e) => Err(e),
    }
}

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub sessions: usize,
    pub events: usize,
    pub storage_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use turul_mcp_session_storage::InMemorySessionStorage;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.mcp_path, "/mcp");
        assert!(config.enable_cors);
        assert_eq!(config.max_body_size, 1024 * 1024);
    }

    #[test]
    fn test_builder() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000);
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let server = HttpMcpServer::builder_with_storage(session_storage)
            .bind_address(addr)
            .mcp_path("/api/mcp")
            .cors(false)
            .max_body_size(2048)
            .build();

        assert_eq!(server.config.bind_address, addr);
        assert_eq!(server.config.mcp_path, "/api/mcp");
        assert!(!server.config.enable_cors);
        assert_eq!(server.config.max_body_size, 2048);
    }

    #[tokio::test]
    async fn test_server_stats() {
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let server = HttpMcpServer::builder_with_storage(session_storage).build();

        let stats = server.get_stats().await;
        assert_eq!(stats.sessions, 0);
        assert_eq!(stats.events, 0);
        assert_eq!(stats.storage_type, "InMemory");
    }
}
