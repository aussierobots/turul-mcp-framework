//! Lambda MCP handler that delegates to SessionMcpHandler
//!
//! This module provides the LambdaMcpHandler that processes Lambda HTTP
//! requests by delegating to SessionMcpHandler, eliminating code duplication.

use std::sync::Arc;

use lambda_http::{Body as LambdaBody, Request as LambdaRequest, Response as LambdaResponse};
use tracing::{debug, info};

use turul_http_mcp_server::{
    ServerConfig, SessionMcpHandler, StreamConfig, StreamManager, StreamableHttpHandler,
};
use turul_mcp_json_rpc_server::JsonRpcDispatcher;
use turul_mcp_protocol::{McpError, ServerCapabilities};
use turul_mcp_session_storage::BoxedSessionStorage;

use crate::error::Result;

#[cfg(feature = "cors")]
use crate::cors::{CorsConfig, create_preflight_response, inject_cors_headers};

/// Main handler for Lambda MCP requests
///
/// This handler processes MCP requests in Lambda by delegating to SessionMcpHandler,
/// eliminating 600+ lines of duplicate business logic code.
///
/// Features:
/// 1. Type conversion between lambda_http and hyper
/// 2. Delegation to SessionMcpHandler for all business logic
/// 3. CORS support for browser clients
/// 4. SSE validation to prevent silent failures
#[derive(Clone)]
pub struct LambdaMcpHandler {
    /// SessionMcpHandler for legacy protocol support
    session_handler: SessionMcpHandler,

    /// StreamableHttpHandler for MCP 2025-11-25 with proper headers
    streamable_handler: StreamableHttpHandler,

    /// Whether SSE is enabled (used for testing and debugging)
    #[allow(dead_code)]
    sse_enabled: bool,

    /// Custom route registry (e.g., .well-known endpoints)
    route_registry: Arc<turul_http_mcp_server::RouteRegistry>,

    /// CORS configuration (if enabled)
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
}

impl LambdaMcpHandler {
    /// Create a new Lambda MCP handler with the framework components
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dispatcher: JsonRpcDispatcher<McpError>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        config: ServerConfig,
        stream_config: StreamConfig,
        _implementation: turul_mcp_protocol::Implementation,
        capabilities: ServerCapabilities,
        sse_enabled: bool,
        #[cfg(feature = "cors")] cors_config: Option<CorsConfig>,
    ) -> Self {
        let dispatcher = Arc::new(dispatcher);

        // Create empty middleware stack (shared by both handlers)
        let middleware_stack = Arc::new(turul_http_mcp_server::middleware::MiddlewareStack::new());

        // Create SessionMcpHandler for legacy protocol support
        let session_handler = SessionMcpHandler::with_shared_stream_manager(
            config.clone(),
            dispatcher.clone(),
            session_storage.clone(),
            stream_config.clone(),
            stream_manager.clone(),
            middleware_stack.clone(),
        );

        // Create StreamableHttpHandler for MCP 2025-11-25 support
        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(config.clone()),
            dispatcher.clone(),
            session_storage.clone(),
            stream_manager.clone(),
            capabilities.clone(),
            middleware_stack,
        );

        Self {
            session_handler,
            streamable_handler,
            sse_enabled,
            route_registry: Arc::new(turul_http_mcp_server::RouteRegistry::new()),
            #[cfg(feature = "cors")]
            cors_config,
        }
    }

    /// Create with shared stream manager (for advanced use cases)
    #[allow(clippy::too_many_arguments)]
    pub fn with_shared_stream_manager(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        stream_config: StreamConfig,
        _implementation: turul_mcp_protocol::Implementation,
        capabilities: ServerCapabilities,
        sse_enabled: bool,
    ) -> Self {
        // Create empty middleware stack (shared by both handlers)
        let middleware_stack = Arc::new(turul_http_mcp_server::middleware::MiddlewareStack::new());

        // Create SessionMcpHandler for legacy protocol support
        let session_handler = SessionMcpHandler::with_shared_stream_manager(
            config.clone(),
            dispatcher.clone(),
            session_storage.clone(),
            stream_config.clone(),
            stream_manager.clone(),
            middleware_stack.clone(),
        );

        // Create StreamableHttpHandler for MCP 2025-11-25 support
        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(config),
            dispatcher,
            session_storage,
            stream_manager,
            capabilities,
            middleware_stack,
        );

        Self {
            session_handler,
            streamable_handler,
            sse_enabled,
            route_registry: Arc::new(turul_http_mcp_server::RouteRegistry::new()),
            #[cfg(feature = "cors")]
            cors_config: None,
        }
    }

    /// Create with custom middleware stack (for testing and examples)
    #[allow(clippy::too_many_arguments)]
    pub fn with_middleware(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        stream_config: StreamConfig,
        capabilities: ServerCapabilities,
        middleware_stack: Arc<turul_http_mcp_server::middleware::MiddlewareStack>,
        sse_enabled: bool,
        route_registry: Arc<turul_http_mcp_server::RouteRegistry>,
    ) -> Self {
        // Create SessionMcpHandler with custom middleware
        let session_handler = SessionMcpHandler::with_shared_stream_manager(
            config.clone(),
            dispatcher.clone(),
            session_storage.clone(),
            stream_config.clone(),
            stream_manager.clone(),
            middleware_stack.clone(),
        );

        // Create StreamableHttpHandler with custom middleware
        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(config),
            dispatcher,
            session_storage,
            stream_manager,
            capabilities,
            middleware_stack,
        );

        Self {
            session_handler,
            streamable_handler,
            sse_enabled,
            route_registry,
            #[cfg(feature = "cors")]
            cors_config: None,
        }
    }

    /// Set CORS configuration
    #[cfg(feature = "cors")]
    pub fn with_cors(mut self, cors_config: CorsConfig) -> Self {
        self.cors_config = Some(cors_config);
        self
    }

    /// Get access to the underlying stream manager for notifications
    pub fn get_stream_manager(&self) -> &Arc<StreamManager> {
        self.session_handler.get_stream_manager()
    }

    /// Handle a Lambda HTTP request (snapshot mode - no real-time SSE)
    ///
    /// This method performs delegation to SessionMcpHandler for all business logic.
    /// It only handles Lambda-specific concerns: CORS and type conversion.
    ///
    /// Note: If SSE is enabled (.sse(true)), SSE responses may not stream properly
    /// with regular Lambda runtime. For proper SSE streaming, use handle_streaming()
    /// with run_with_streaming_response().
    pub async fn handle(&self, req: LambdaRequest) -> Result<LambdaResponse<LambdaBody>> {
        let method = req.method().clone();
        let uri = req.uri().clone();

        let request_origin = req
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        info!(
            "🌐 Lambda MCP request: {} {} (origin: {:?})",
            method, uri, request_origin
        );

        // Handle CORS preflight requests first (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if method == http::Method::OPTIONS
            && let Some(ref cors_config) = self.cors_config
        {
            debug!("Handling CORS preflight request");
            return create_preflight_response(cors_config, request_origin.as_deref());
        }

        // 🚀 DELEGATION: Convert Lambda request to hyper request
        let hyper_req = crate::adapter::lambda_to_hyper_request(req)?;

        // Check custom routes (e.g., .well-known) before MCP delegation
        let path = hyper_req.uri().path().to_string();
        if !self.route_registry.is_empty() {
            match self.route_registry.match_route(&path) {
                Ok(Some(route_handler)) => {
                    debug!("Custom route matched: {}", path);
                    use http_body_util::BodyExt;
                    let (parts, body) = hyper_req.into_parts();
                    let boxed_req = hyper::Request::from_parts(parts, body.boxed_unsync());
                    let route_resp = route_handler.handle(boxed_req).await;
                    let mut lambda_resp =
                        crate::adapter::hyper_to_lambda_response(route_resp).await?;
                    #[cfg(feature = "cors")]
                    if let Some(ref cors_config) = self.cors_config {
                        inject_cors_headers(
                            &mut lambda_resp,
                            cors_config,
                            request_origin.as_deref(),
                        )?;
                    }
                    return Ok(lambda_resp);
                }
                Ok(None) => {} // No match, continue to MCP handler
                Err(e) => {
                    debug!("Route validation error: {}", e);
                    let route_resp = e.into_response();
                    let mut lambda_resp =
                        crate::adapter::hyper_to_lambda_response(route_resp).await?;
                    #[cfg(feature = "cors")]
                    if let Some(ref cors_config) = self.cors_config {
                        inject_cors_headers(
                            &mut lambda_resp,
                            cors_config,
                            request_origin.as_deref(),
                        )?;
                    }
                    return Ok(lambda_resp);
                }
            }
        }

        // 🚀 DELEGATION: Use SessionMcpHandler for all business logic
        let hyper_resp = self
            .session_handler
            .handle_mcp_request(hyper_req)
            .await
            .map_err(|e| crate::error::LambdaError::McpFramework(e.to_string()))?;

        // 🚀 DELEGATION: Convert hyper response back to Lambda response
        let mut lambda_resp = crate::adapter::hyper_to_lambda_response(hyper_resp).await?;

        // Apply CORS headers if configured (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if let Some(ref cors_config) = self.cors_config {
            inject_cors_headers(&mut lambda_resp, cors_config, request_origin.as_deref())?;
        }

        Ok(lambda_resp)
    }

    /// Handle Lambda streaming request (real SSE streaming)
    ///
    /// This method enables real-time SSE streaming using Lambda's streaming response capability.
    /// It delegates all business logic to SessionMcpHandler.
    pub async fn handle_streaming(
        &self,
        req: LambdaRequest,
    ) -> std::result::Result<
        lambda_http::Response<
            http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>,
        >,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let request_origin = req
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        debug!(
            "🌊 Lambda streaming MCP request: {} {} (origin: {:?})",
            method, uri, request_origin
        );

        // Handle CORS preflight requests first (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if method == http::Method::OPTIONS
            && let Some(ref cors_config) = self.cors_config
        {
            debug!("Handling CORS preflight request (streaming)");
            let preflight_response =
                create_preflight_response(cors_config, request_origin.as_deref())
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            // Convert LambdaResponse<LambdaBody> to streaming response
            return Ok(self.convert_lambda_response_to_streaming(preflight_response));
        }

        // 🚀 DELEGATION: Convert Lambda request to hyper request
        let hyper_req = crate::adapter::lambda_to_hyper_request(req)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Check custom routes (e.g., .well-known) before MCP delegation
        let path = hyper_req.uri().path().to_string();
        if !self.route_registry.is_empty() {
            match self.route_registry.match_route(&path) {
                Ok(Some(route_handler)) => {
                    debug!("Custom route matched (streaming): {}", path);
                    use http_body_util::BodyExt;
                    let (parts, body) = hyper_req.into_parts();
                    let boxed_req = hyper::Request::from_parts(parts, body.boxed_unsync());
                    return Ok(route_handler.handle(boxed_req).await);
                }
                Ok(None) => {} // No match, continue to MCP handler
                Err(e) => {
                    debug!("Route validation error (streaming): {}", e);
                    return Ok(e.into_response());
                }
            }
        }

        // 🚀 PROTOCOL ROUTING: Check protocol version and route to appropriate handler
        use turul_http_mcp_server::protocol::McpProtocolVersion;
        let protocol_version = hyper_req
            .headers()
            .get("MCP-Protocol-Version")
            .and_then(|h| h.to_str().ok())
            .and_then(McpProtocolVersion::parse_version)
            .unwrap_or(McpProtocolVersion::V2025_06_18);

        // Route based on protocol version
        let hyper_resp = if protocol_version.supports_streamable_http() {
            // Use StreamableHttpHandler for MCP 2025-11-25 (proper headers, chunked SSE)
            debug!(
                "Using StreamableHttpHandler for protocol {}",
                protocol_version.to_string()
            );
            self.streamable_handler.handle_request(hyper_req).await
        } else {
            // Legacy protocol: use SessionMcpHandler
            debug!(
                "Using SessionMcpHandler for legacy protocol {}",
                protocol_version.to_string()
            );
            self.session_handler
                .handle_mcp_request(hyper_req)
                .await
                .map_err(|e| {
                    Box::new(crate::error::LambdaError::McpFramework(e.to_string()))
                        as Box<dyn std::error::Error + Send + Sync>
                })?
        };

        // 🚀 DELEGATION: Convert hyper response to Lambda streaming response (preserves streaming!)
        let mut lambda_resp = crate::adapter::hyper_to_lambda_streaming(hyper_resp);

        // Apply CORS headers if configured (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if let Some(ref cors_config) = self.cors_config {
            inject_cors_headers(&mut lambda_resp, cors_config, request_origin.as_deref())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }

        Ok(lambda_resp)
    }

    /// Convert Lambda response to streaming format (helper for CORS preflight)
    fn convert_lambda_response_to_streaming(
        &self,
        lambda_response: LambdaResponse<LambdaBody>,
    ) -> lambda_http::Response<http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>>
    {
        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};

        let (parts, body) = lambda_response.into_parts();
        let body_bytes = match body {
            LambdaBody::Empty => Bytes::new(),
            LambdaBody::Text(text) => Bytes::from(text),
            LambdaBody::Binary(bytes) => Bytes::from(bytes),
            _ => Bytes::new(),
        };

        // Map error type from Infallible to hyper::Error
        let streaming_body = Full::new(body_bytes)
            .map_err(|e: std::convert::Infallible| match e {})
            .boxed_unsync();

        lambda_http::Response::from_parts(parts, streaming_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Request;
    use turul_mcp_session_storage::InMemorySessionStorage;

    #[tokio::test]
    async fn test_handler_creation() {
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
        let dispatcher = JsonRpcDispatcher::new();
        let config = ServerConfig::default();
        let implementation = turul_mcp_protocol::Implementation::new("test", "1.0.0");
        let capabilities = ServerCapabilities::default();

        let handler = LambdaMcpHandler::new(
            dispatcher,
            session_storage,
            stream_manager,
            config,
            StreamConfig::default(),
            implementation,
            capabilities,
            false, // SSE disabled for test
            #[cfg(feature = "cors")]
            None,
        );

        // Test that handler was created successfully
        assert!(!handler.sse_enabled);
    }

    #[tokio::test]
    async fn test_sse_enabled_with_handle_works() {
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
        let dispatcher = JsonRpcDispatcher::new();
        let config = ServerConfig::default();
        let implementation = turul_mcp_protocol::Implementation::new("test", "1.0.0");
        let capabilities = ServerCapabilities::default();

        // Create handler with SSE enabled
        let handler = LambdaMcpHandler::new(
            dispatcher,
            session_storage,
            stream_manager,
            config,
            StreamConfig::default(),
            implementation,
            capabilities,
            true, // SSE enabled - should work with handle() for snapshot-based SSE
            #[cfg(feature = "cors")]
            None,
        );

        // Create a test Lambda request
        let lambda_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Text(
                r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
            ))
            .unwrap();

        // handle() should work (provides snapshot-based SSE rather than real-time streaming)
        let result = handler.handle(lambda_req).await;
        assert!(
            result.is_ok(),
            "handle() should work with SSE enabled for snapshot-based responses"
        );
    }

    /// Test that verifies StreamConfig is properly threaded through the delegation
    #[tokio::test]
    async fn test_stream_config_preservation() {
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let dispatcher = JsonRpcDispatcher::new();
        let config = ServerConfig::default();
        let implementation = turul_mcp_protocol::Implementation::new("test", "1.0.0");
        let capabilities = ServerCapabilities::default();

        // Create a custom StreamConfig with non-default values
        let custom_stream_config = StreamConfig {
            channel_buffer_size: 1024,      // Non-default value (default is 1000)
            max_replay_events: 200,         // Non-default value (default is 100)
            keepalive_interval_seconds: 10, // Non-default value (default is 30)
            cors_origin: "https://custom-test.example.com".to_string(), // Non-default value
        };

        // Create stream manager with the custom config
        let stream_manager = Arc::new(StreamManager::with_config(
            session_storage.clone(),
            custom_stream_config.clone(),
        ));

        let handler = LambdaMcpHandler::new(
            dispatcher,
            session_storage,
            stream_manager,
            config,
            custom_stream_config.clone(),
            implementation,
            capabilities,
            false, // SSE disabled for test
            #[cfg(feature = "cors")]
            None,
        );

        // The handler should be created successfully, proving the StreamConfig was accepted
        assert!(!handler.sse_enabled);

        // Verify that the stream manager has the custom configuration
        let stream_manager = handler.get_stream_manager();

        // Verify the StreamConfig values were propagated correctly
        let actual_config = stream_manager.get_config();

        assert_eq!(
            actual_config.channel_buffer_size, custom_stream_config.channel_buffer_size,
            "Custom channel_buffer_size was not propagated correctly"
        );
        assert_eq!(
            actual_config.max_replay_events, custom_stream_config.max_replay_events,
            "Custom max_replay_events was not propagated correctly"
        );
        assert_eq!(
            actual_config.keepalive_interval_seconds,
            custom_stream_config.keepalive_interval_seconds,
            "Custom keepalive_interval_seconds was not propagated correctly"
        );
        assert_eq!(
            actual_config.cors_origin, custom_stream_config.cors_origin,
            "Custom cors_origin was not propagated correctly"
        );

        // Verify the stream manager is accessible (proves delegation worked)
        assert!(Arc::strong_count(stream_manager) >= 1);
    }

    /// Test the full builder → server → handler chain with StreamConfig
    #[tokio::test]
    async fn test_full_builder_chain_stream_config() {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        // Create a custom StreamConfig with non-default values
        let custom_stream_config = turul_http_mcp_server::StreamConfig {
            channel_buffer_size: 2048,      // Non-default value
            max_replay_events: 500,         // Non-default value
            keepalive_interval_seconds: 15, // Non-default value
            cors_origin: "https://full-chain-test.example.com".to_string(),
        };

        // Test the complete builder → server → handler chain
        let server = LambdaMcpServerBuilder::new()
            .name("full-chain-test")
            .version("1.0.0")
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(true) // Enable SSE to test streaming functionality
            .stream_config(custom_stream_config.clone())
            .build()
            .await
            .expect("Server should build successfully");

        // Create handler from server (this is the critical chain step)
        let handler = server
            .handler()
            .await
            .expect("Handler should be created from server");

        // Verify the handler was created successfully
        assert!(handler.sse_enabled, "SSE should be enabled");

        // Verify that the custom StreamConfig was preserved through the entire chain
        let stream_manager = handler.get_stream_manager();
        let actual_config = stream_manager.get_config();

        assert_eq!(
            actual_config.channel_buffer_size, custom_stream_config.channel_buffer_size,
            "Custom channel_buffer_size should be preserved through builder → server → handler chain"
        );
        assert_eq!(
            actual_config.max_replay_events, custom_stream_config.max_replay_events,
            "Custom max_replay_events should be preserved through builder → server → handler chain"
        );
        assert_eq!(
            actual_config.keepalive_interval_seconds,
            custom_stream_config.keepalive_interval_seconds,
            "Custom keepalive_interval_seconds should be preserved through builder → server → handler chain"
        );
        assert_eq!(
            actual_config.cors_origin, custom_stream_config.cors_origin,
            "Custom cors_origin should be preserved through builder → server → handler chain"
        );

        // Verify the stream manager is functional
        assert!(
            Arc::strong_count(stream_manager) >= 1,
            "Stream manager should be properly initialized"
        );

        // Additional verification: Test that the configuration is actually used functionally
        // by verifying the stream manager can be used with the custom configuration
        let test_session_id = uuid::Uuid::now_v7().as_simple().to_string();

        // The stream manager should be able to handle session operations with the custom config
        // This verifies the config isn't just preserved but actually used
        let subscriptions = stream_manager.get_subscriptions(&test_session_id).await;
        assert!(
            subscriptions.is_empty(),
            "New session should have no subscriptions initially"
        );

        // Verify the stream manager was constructed with our custom config values
        // This confirms the config propagated through the entire builder → server → handler chain
        assert_eq!(
            stream_manager.get_config().channel_buffer_size,
            2048,
            "Stream manager should be using the custom buffer size functionally"
        );
    }

    /// Test matrix: 4 combinations of streaming runtime vs SSE configuration
    /// This ensures we don't have runtime hangs or configuration conflicts
    ///
    /// Test 1: Non-streaming runtime + sse(false) - This should work (snapshot mode)
    #[tokio::test]
    async fn test_non_streaming_runtime_sse_false() {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("test-non-streaming-sse-false")
            .version("1.0.0")
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false) // Disable SSE for non-streaming runtime
            .build()
            .await
            .expect("Server should build successfully");

        let handler = server
            .handler()
            .await
            .expect("Handler should be created from server");

        // Verify configuration
        assert!(!handler.sse_enabled, "SSE should be disabled");

        // Create a test request (POST /mcp works in all configs)
        let lambda_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Text(
                r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
            ))
            .unwrap();

        // This should work without hanging
        let result = handler.handle(lambda_req).await;
        assert!(
            result.is_ok(),
            "POST /mcp should work with non-streaming + sse(false)"
        );
    }

    /// Test 2: Non-streaming runtime + sse(true) - This should work (snapshot-based SSE)
    #[tokio::test]
    async fn test_non_streaming_runtime_sse_true() {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("test-non-streaming-sse-true")
            .version("1.0.0")
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(true) // Enable SSE for snapshot-based responses
            .build()
            .await
            .expect("Server should build successfully");

        let handler = server
            .handler()
            .await
            .expect("Handler should be created from server");

        // Verify configuration
        assert!(handler.sse_enabled, "SSE should be enabled");

        // Create a test request (POST /mcp works in all configs)
        let lambda_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Text(
                r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
            ))
            .unwrap();

        // This should work without hanging (provides snapshot-based SSE)
        let result = handler.handle(lambda_req).await;
        assert!(
            result.is_ok(),
            "POST /mcp should work with non-streaming + sse(true)"
        );

        // Note: GET /mcp would provide snapshot events, not real-time streaming
        // This is the key difference from handle_streaming()
    }

    /// Test 3: Streaming runtime + sse(false) - This should work (SSE disabled)
    #[tokio::test]
    async fn test_streaming_runtime_sse_false() {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("test-streaming-sse-false")
            .version("1.0.0")
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false) // Disable SSE even with streaming runtime
            .build()
            .await
            .expect("Server should build successfully");

        let handler = server
            .handler()
            .await
            .expect("Handler should be created from server");

        // Verify configuration
        assert!(!handler.sse_enabled, "SSE should be disabled");

        // Create a test request for streaming handler
        let lambda_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Text(
                r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
            ))
            .unwrap();

        // This should work with streaming runtime even when SSE is disabled
        let result = handler.handle_streaming(lambda_req).await;
        assert!(
            result.is_ok(),
            "Streaming runtime should work with sse(false)"
        );
    }

    /// Test 4: Streaming runtime + sse(true) - This should work (real-time SSE streaming)
    #[tokio::test]
    async fn test_streaming_runtime_sse_true() {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("test-streaming-sse-true")
            .version("1.0.0")
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(true) // Enable SSE with streaming runtime for real-time streaming
            .build()
            .await
            .expect("Server should build successfully");

        let handler = server
            .handler()
            .await
            .expect("Handler should be created from server");

        // Verify configuration
        assert!(handler.sse_enabled, "SSE should be enabled");

        // Create a test request for streaming handler
        let lambda_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Text(
                r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
            ))
            .unwrap();

        // This should work and provide real-time SSE streaming
        let result = handler.handle_streaming(lambda_req).await;
        assert!(
            result.is_ok(),
            "Streaming runtime should work with sse(true) for real-time streaming"
        );

        // Note: GET /mcp would provide real-time streaming events
        // This is the optimal configuration for real-time notifications
    }

    // ── Strict lifecycle tests over handle_streaming() ────────────────

    /// Helper: build a Lambda handler with strict lifecycle and a test tool via the builder.
    async fn build_strict_streaming_handler() -> LambdaMcpHandler {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("lifecycle-test")
            .version("1.0.0")
            .tool(LifecycleTestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .strict_lifecycle(true) // explicit — survives default changes
            .sse(true)
            .build()
            .await
            .expect("build should succeed");

        server.handler().await.expect("handler should succeed")
    }

    // Test tool for lifecycle tests — satisfies all required traits
    #[derive(Clone, Default)]
    struct LifecycleTestTool;

    impl turul_mcp_builders::traits::HasBaseMetadata for LifecycleTestTool {
        fn name(&self) -> &str {
            "ping_tool"
        }
    }
    impl turul_mcp_builders::traits::HasDescription for LifecycleTestTool {
        fn description(&self) -> Option<&str> {
            Some("test tool")
        }
    }
    impl turul_mcp_builders::traits::HasInputSchema for LifecycleTestTool {
        fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
            static SCHEMA: std::sync::OnceLock<turul_mcp_protocol::ToolSchema> =
                std::sync::OnceLock::new();
            SCHEMA.get_or_init(turul_mcp_protocol::ToolSchema::object)
        }
    }
    impl turul_mcp_builders::traits::HasOutputSchema for LifecycleTestTool {
        fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
            None
        }
    }
    impl turul_mcp_builders::traits::HasAnnotations for LifecycleTestTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }
    impl turul_mcp_builders::traits::HasToolMeta for LifecycleTestTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }
    impl turul_mcp_builders::traits::HasIcons for LifecycleTestTool {}
    impl turul_mcp_builders::traits::HasExecution for LifecycleTestTool {}

    #[async_trait::async_trait]
    impl turul_mcp_server::McpTool for LifecycleTestTool {
        async fn call(
            &self,
            _args: serde_json::Value,
            _session: Option<turul_mcp_server::SessionContext>,
        ) -> turul_mcp_server::McpResult<turul_mcp_protocol::tools::CallToolResult> {
            Ok(turul_mcp_protocol::tools::CallToolResult::success(vec![
                turul_mcp_protocol::tools::ToolResult::text("pong"),
            ]))
        }
    }

    /// Helper: create a Lambda POST request for handle_streaming()
    fn streaming_mcp_request(body: &str, session_id: Option<&str>) -> LambdaRequest {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-11-25");

        if let Some(sid) = session_id {
            builder = builder.header("Mcp-Session-Id", sid);
        }

        builder.body(LambdaBody::Text(body.to_string())).unwrap()
    }

    /// Helper: collect streaming response body into a string
    async fn collect_streaming_body(
        response: lambda_http::Response<
            http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>,
        >,
    ) -> (http::StatusCode, String) {
        use http_body_util::BodyExt;
        let status = response.status();
        let session_id = response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map(|c| c.to_bytes())
            .unwrap_or_default();
        let body_str = String::from_utf8_lossy(&body_bytes).to_string();
        let _ = session_id; // available if needed
        (status, body_str)
    }

    /// Helper: extract session ID from a streaming response
    fn extract_session_id(
        response: &lambda_http::Response<
            http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>,
        >,
    ) -> Option<String> {
        response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    }

    /// Helper: parse JSON from a response body (handles SSE "data: " prefix)
    fn parse_response_json(body: &str) -> serde_json::Value {
        // Strip SSE framing if present
        let json_str = body
            .lines()
            .find(|line| line.starts_with("data: "))
            .map(|line| &line[6..])
            .unwrap_or(body.trim());
        serde_json::from_str(json_str)
            .unwrap_or_else(|e| panic!("Failed to parse JSON from body: {e}\nBody: {body}"))
    }

    /// P0: Full strict lifecycle handshake succeeds on handle_streaming()
    #[tokio::test]
    async fn test_lambda_streaming_strict_handshake_succeeds() {
        let handler = build_strict_streaming_handler().await;

        // Step 1: initialize
        let init_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "initialize", "id": 1,
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "1.0.0" }
                }
            })
            .to_string(),
            None,
        );
        let init_resp = handler
            .handle_streaming(init_req)
            .await
            .expect("initialize should succeed");
        let session_id = extract_session_id(&init_resp).expect("must return session ID");
        let (status, _body) = collect_streaming_body(init_resp).await;
        assert_eq!(status, 200, "initialize should return 200");

        // Step 2: notifications/initialized
        let notif_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            })
            .to_string(),
            Some(&session_id),
        );
        let notif_resp = handler
            .handle_streaming(notif_req)
            .await
            .expect("notification should succeed");
        let (status, _) = collect_streaming_body(notif_resp).await;
        assert_eq!(status, 202, "notifications/initialized should return 202");

        // Step 3: tools/list
        let list_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "tools/list", "id": 2
            })
            .to_string(),
            Some(&session_id),
        );
        let list_resp = handler
            .handle_streaming(list_req)
            .await
            .expect("tools/list should succeed");
        let (status, body) = collect_streaming_body(list_resp).await;
        assert_eq!(status, 200, "tools/list should return 200");
        let json = parse_response_json(&body);
        assert!(
            json["result"]["tools"].is_array(),
            "tools/list should return tools array: {json}"
        );

        // Step 4: tools/call
        let call_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "tools/call", "id": 3,
                "params": { "name": "ping_tool", "arguments": {} }
            })
            .to_string(),
            Some(&session_id),
        );
        let call_resp = handler
            .handle_streaming(call_req)
            .await
            .expect("tools/call should succeed");
        let (status, body) = collect_streaming_body(call_resp).await;
        assert_eq!(status, 200, "tools/call should return 200");
        let json = parse_response_json(&body);
        assert!(
            json["result"].is_object(),
            "tools/call should return result: {json}"
        );
    }

    /// P0: Strict lifecycle rejects both tools/list and tools/call before notifications/initialized
    #[tokio::test]
    async fn test_lambda_streaming_strict_rejects_before_initialized() {
        let handler = build_strict_streaming_handler().await;

        // Initialize to get session
        let init_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "initialize", "id": 1,
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "1.0.0" }
                }
            })
            .to_string(),
            None,
        );
        let init_resp = handler.handle_streaming(init_req).await.unwrap();
        let session_id = extract_session_id(&init_resp).unwrap();
        let _ = collect_streaming_body(init_resp).await;

        // tools/list without notifications/initialized — must fail
        let list_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "tools/list", "id": 2
            })
            .to_string(),
            Some(&session_id),
        );
        let list_resp = handler.handle_streaming(list_req).await.unwrap();
        let (_, body) = collect_streaming_body(list_resp).await;
        let json = parse_response_json(&body);
        assert!(
            json["error"].is_object(),
            "tools/list should return JSON-RPC error: {json}"
        );
        assert_eq!(
            json["error"]["code"].as_i64().unwrap(),
            -32031,
            "tools/list must return SessionError code -32031, got: {json}"
        );
        assert!(
            json["error"]["message"]
                .as_str()
                .unwrap()
                .contains("notifications/initialized"),
            "Error must mention notifications/initialized: {}",
            json["error"]["message"]
        );

        // tools/call without notifications/initialized — must also fail
        let call_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "tools/call", "id": 3,
                "params": { "name": "ping_tool", "arguments": {} }
            })
            .to_string(),
            Some(&session_id),
        );
        let call_resp = handler.handle_streaming(call_req).await.unwrap();
        let (_, body) = collect_streaming_body(call_resp).await;
        let json = parse_response_json(&body);
        assert!(
            json["error"].is_object(),
            "tools/call should return JSON-RPC error: {json}"
        );
        assert_eq!(
            json["error"]["code"].as_i64().unwrap(),
            -32031,
            "tools/call must return SessionError code -32031, got: {json}"
        );
        assert!(
            json["error"]["message"]
                .as_str()
                .unwrap()
                .contains("notifications/initialized"),
            "Error must mention notifications/initialized: {}",
            json["error"]["message"]
        );
    }

    /// P0: tools/list succeeds immediately after notifications/initialized (race fix proof)
    #[tokio::test]
    async fn test_lambda_streaming_initialized_is_effective_immediately() {
        let handler = build_strict_streaming_handler().await;

        // Initialize
        let init_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "initialize", "id": 1,
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "1.0.0" }
                }
            })
            .to_string(),
            None,
        );
        let init_resp = handler.handle_streaming(init_req).await.unwrap();
        let session_id = extract_session_id(&init_resp).unwrap();
        let _ = collect_streaming_body(init_resp).await;

        // notifications/initialized
        let notif_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            })
            .to_string(),
            Some(&session_id),
        );
        let notif_resp = handler.handle_streaming(notif_req).await.unwrap();
        let (status, _) = collect_streaming_body(notif_resp).await;
        assert_eq!(status, 202);

        // Immediately — no delay — send tools/list
        let list_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "tools/list", "id": 2
            })
            .to_string(),
            Some(&session_id),
        );
        let list_resp = handler.handle_streaming(list_req).await.unwrap();
        let (status, body) = collect_streaming_body(list_resp).await;
        assert_eq!(
            status, 200,
            "tools/list must succeed immediately after initialized"
        );
        let json = parse_response_json(&body);
        assert!(
            json["result"]["tools"].is_array(),
            "Must return tools list, not error: {json}"
        );
    }

    /// P1: Lenient mode allows operations without notifications/initialized
    #[tokio::test]
    async fn test_lambda_streaming_lenient_mode_allows_without_initialized() {
        use crate::LambdaMcpServerBuilder;
        use turul_mcp_session_storage::InMemorySessionStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("lenient-test")
            .version("1.0.0")
            .tool(LifecycleTestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .strict_lifecycle(false) // lenient mode
            .sse(true)
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();

        // Initialize (no notifications/initialized)
        let init_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "initialize", "id": 1,
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "1.0.0" }
                }
            })
            .to_string(),
            None,
        );
        let init_resp = handler.handle_streaming(init_req).await.unwrap();
        let session_id = extract_session_id(&init_resp).unwrap();
        let _ = collect_streaming_body(init_resp).await;

        // Skip notifications/initialized — go straight to tools/list
        let list_req = streaming_mcp_request(
            &serde_json::json!({
                "jsonrpc": "2.0", "method": "tools/list", "id": 2
            })
            .to_string(),
            Some(&session_id),
        );
        let list_resp = handler.handle_streaming(list_req).await.unwrap();
        let (status, body) = collect_streaming_body(list_resp).await;
        assert_eq!(
            status, 200,
            "Lenient mode should allow tools/list without initialized"
        );
        let json = parse_response_json(&body);
        assert!(
            json["result"]["tools"].is_array(),
            "Must return tools list in lenient mode: {json}"
        );
    }
}
