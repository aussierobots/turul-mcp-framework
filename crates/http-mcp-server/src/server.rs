//! HTTP MCP Server implementation

use std::net::SocketAddr;
use std::sync::Arc;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use http_body_util::Full;
use bytes::Bytes;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::{info, error, debug};

use crate::{Result, CorsLayer, SessionMcpHandler, mcp_session};
use json_rpc_server::{JsonRpcHandler, JsonRpcDispatcher};

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
    /// Enable SSE support
    pub enable_sse: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            mcp_path: "/mcp".to_string(),
            enable_cors: true,
            max_body_size: 1024 * 1024, // 1MB
            enable_sse: cfg!(feature = "sse"),
        }
    }
}

/// Builder for HTTP MCP server
pub struct HttpMcpServerBuilder {
    config: ServerConfig,
    dispatcher: JsonRpcDispatcher,
}

impl HttpMcpServerBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            dispatcher: JsonRpcDispatcher::new(),
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

    /// Enable or disable SSE
    pub fn sse(mut self, enable: bool) -> Self {
        self.config.enable_sse = enable;
        self
    }

    /// Register a JSON-RPC handler for specific methods
    pub fn register_handler<H>(mut self, methods: Vec<String>, handler: H) -> Self
    where
        H: JsonRpcHandler + 'static,
    {
        self.dispatcher.register_methods(methods, handler);
        self
    }

    /// Register a default handler for unhandled methods
    pub fn default_handler<H>(mut self, handler: H) -> Self
    where
        H: JsonRpcHandler + 'static,
    {
        self.dispatcher.set_default_handler(handler);
        self
    }

    /// Build the HTTP MCP server
    pub fn build(self) -> HttpMcpServer {
        HttpMcpServer {
            config: self.config,
            dispatcher: Arc::new(self.dispatcher),
        }
    }
}

impl Default for HttpMcpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP MCP Server
#[derive(Clone)]
pub struct HttpMcpServer {
    config: ServerConfig,
    dispatcher: Arc<JsonRpcDispatcher>,
}

impl HttpMcpServer {
    /// Create a new builder
    pub fn builder() -> HttpMcpServerBuilder {
        HttpMcpServerBuilder::new()
    }

    /// Run the server
    pub async fn run(&self) -> Result<()> {
        // Start session cleanup task
        mcp_session::spawn_session_cleanup();
        
        let listener = TcpListener::bind(&self.config.bind_address).await?;
        info!("HTTP MCP server listening on {}", self.config.bind_address);
        info!("MCP endpoint available at: {}", self.config.mcp_path);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            debug!("New connection from {}", peer_addr);

            let config = self.config.clone();
            let dispatcher = Arc::clone(&self.dispatcher);

            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(move |req| {
                    handle_request(req, config.clone(), Arc::clone(&dispatcher))
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Error serving connection: {}", err);
                }
            });
        }
    }
}

/// Handle HTTP requests with session-integrated handler
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    config: ServerConfig,
    dispatcher: Arc<JsonRpcDispatcher>,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();

    debug!("Handling {} {}", method, path);

    // Create the JSON-RPC 2.0 over HTTP handler  
    let handler = SessionMcpHandler::new(config.clone(), dispatcher);

    // Route the request - handler now returns proper JSON-RPC responses
    let response = if path == &config.mcp_path {
        match handler.handle_mcp_request(req).await {
            Ok(json_rpc_response) => json_rpc_response,
            Err(err) => {
                error!("Request handling error: {}", err);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from(format!("Internal Server Error: {}", err))))
                    .unwrap()
            }
        }
    } else {
        // 404 for other paths
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap()
    };

    // Apply CORS if enabled
    let mut final_response = response;
    if config.enable_cors {
        CorsLayer::apply_cors_headers(final_response.headers_mut());
    }
    
    Ok(final_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

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
        let server = HttpMcpServer::builder()
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
}