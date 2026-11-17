//! Lambda MCP handler that delegates to the framework's HTTP handlers.
//!
//! Provides [`LambdaMcpHandler`], which processes Lambda HTTP requests by
//! delegating to `SessionMcpHandler` or `StreamableHttpHandler`.

use std::sync::Arc;

use lambda_http::{Body as LambdaBody, Request as LambdaRequest, Response as LambdaResponse};
use tracing::{debug, info};

use turul_http_mcp_server::{
    ServerConfig, SessionMcpHandler, StreamConfig, StreamManager, StreamableHttpHandler,
};
use turul_mcp_protocol::{McpError, ServerCapabilities};
use turul_mcp_session_storage::BoxedSessionStorage;
use turul_rpc::JsonRpcDispatcher;

use crate::error::Result;

#[cfg(feature = "cors")]
use crate::cors::{CorsConfig, create_preflight_response, inject_cors_headers};

/// Main handler for Lambda MCP requests.
///
/// Processes MCP requests in Lambda by delegating to the framework's HTTP handlers.
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

    /// StreamableHttpHandler for the streamable HTTP transport
    streamable_handler: StreamableHttpHandler,

    /// Whether SSE is enabled. Only read by in-crate tests.
    #[cfg_attr(not(test), allow(dead_code))]
    sse_enabled: bool,

    /// Custom route registry (e.g., .well-known endpoints)
    route_registry: Arc<turul_http_mcp_server::RouteRegistry>,

    /// Same dispatcher Arc held by session_handler/streamable_handler, kept
    /// for method-registration introspection. Only read by in-crate tests.
    #[cfg_attr(not(test), allow(dead_code))]
    dispatcher: Arc<JsonRpcDispatcher<McpError>>,

    /// Dynamic tool registry for request-time change detection
    #[cfg(feature = "dynamic-tools")]
    tool_registry: Option<Arc<turul_mcp_server::ToolRegistry>>,

    /// CORS configuration (if enabled)
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
}

impl LambdaMcpHandler {
    /// Sets the identity reported in each 2026-07-28 result's `_meta.serverInfo`.
    ///
    /// Needed as a setter because the production path builds through
    /// `with_middleware_and_fingerprint`, which takes no `Implementation`.
    pub fn with_server_info(mut self, info: turul_mcp_protocol::Implementation) -> Self {
        self.streamable_handler = self.streamable_handler.with_server_info(info);
        self
    }

    /// Creates a new Lambda MCP handler from the framework components.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dispatcher: JsonRpcDispatcher<McpError>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        config: ServerConfig,
        stream_config: StreamConfig,
        implementation: turul_mcp_protocol::Implementation,
        capabilities: ServerCapabilities,
        sse_enabled: bool,
        #[cfg(feature = "cors")] cors_config: Option<CorsConfig>,
    ) -> Self {
        let dispatcher = Arc::new(dispatcher);

        // Empty middleware stack, shared by both handlers.
        let middleware_stack = Arc::new(turul_http_mcp_server::middleware::MiddlewareStack::new());

        let session_handler = SessionMcpHandler::with_shared_stream_manager(
            config.clone(),
            dispatcher.clone(),
            session_storage.clone(),
            stream_config.clone(),
            stream_manager.clone(),
            middleware_stack.clone(),
        );

        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(config.clone()),
            dispatcher.clone(),
            session_storage.clone(),
            stream_manager.clone(),
            capabilities.clone(),
            middleware_stack,
            None, // no tool fingerprint
        )
        .with_server_info(implementation)
        .without_subscriptions_listen();

        Self {
            session_handler,
            streamable_handler,
            sse_enabled,
            route_registry: Arc::new(turul_http_mcp_server::RouteRegistry::new()),
            dispatcher,
            #[cfg(feature = "dynamic-tools")]
            tool_registry: None,
            #[cfg(feature = "cors")]
            cors_config,
        }
    }

    /// Creates a handler that shares the given stream manager (for advanced use cases).
    #[allow(clippy::too_many_arguments)]
    pub fn with_shared_stream_manager(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        stream_config: StreamConfig,
        implementation: turul_mcp_protocol::Implementation,
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

        let dispatcher_for_introspection = dispatcher.clone();

        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(config),
            dispatcher,
            session_storage,
            stream_manager,
            capabilities,
            middleware_stack,
            None, // no tool fingerprint
        )
        .with_server_info(implementation)
        .without_subscriptions_listen();

        Self {
            session_handler,
            streamable_handler,
            sse_enabled,
            route_registry: Arc::new(turul_http_mcp_server::RouteRegistry::new()),
            dispatcher: dispatcher_for_introspection,
            #[cfg(feature = "dynamic-tools")]
            tool_registry: None,
            #[cfg(feature = "cors")]
            cors_config: None,
        }
    }

    /// Creates a handler with a custom middleware stack (for testing and examples).
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
        Self::with_middleware_and_fingerprint(
            config,
            dispatcher,
            session_storage,
            stream_manager,
            stream_config,
            capabilities,
            middleware_stack,
            sse_enabled,
            route_registry,
            None,
        )
    }

    /// Creates a handler with a custom middleware stack and a tool fingerprint for session versioning.
    #[allow(clippy::too_many_arguments)]
    pub fn with_middleware_and_fingerprint(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        stream_config: StreamConfig,
        capabilities: ServerCapabilities,
        middleware_stack: Arc<turul_http_mcp_server::middleware::MiddlewareStack>,
        sse_enabled: bool,
        route_registry: Arc<turul_http_mcp_server::RouteRegistry>,
        tool_fingerprint: Option<String>,
    ) -> Self {
        // Create SessionMcpHandler with custom middleware and fingerprint
        let session_handler = SessionMcpHandler::with_shared_stream_manager(
            config.clone(),
            dispatcher.clone(),
            session_storage.clone(),
            stream_config.clone(),
            stream_manager.clone(),
            middleware_stack.clone(),
        )
        .with_tool_fingerprint(tool_fingerprint.clone());

        let dispatcher_for_introspection = dispatcher.clone();

        // Create StreamableHttpHandler with custom middleware and fingerprint
        let streamable_handler = StreamableHttpHandler::new(
            Arc::new(config),
            dispatcher,
            session_storage,
            stream_manager,
            capabilities,
            middleware_stack,
            tool_fingerprint,
        )
        .without_subscriptions_listen();

        Self {
            session_handler,
            streamable_handler,
            sse_enabled,
            route_registry,
            dispatcher: dispatcher_for_introspection,
            #[cfg(feature = "dynamic-tools")]
            tool_registry: None,
            #[cfg(feature = "cors")]
            cors_config: None,
        }
    }

    /// Sets the tool change notifier for restart/redeploy fingerprint mismatch notifications.
    pub fn with_tool_notifier(
        mut self,
        notifier: Arc<dyn turul_http_mcp_server::ToolChangeNotifier>,
    ) -> Self {
        self.session_handler = self
            .session_handler
            .with_tool_notifier(Arc::clone(&notifier));
        self.streamable_handler = self.streamable_handler.with_tool_notifier(notifier);
        self
    }

    /// Sets a dynamic tool registry for request-time change detection.
    #[cfg(feature = "dynamic-tools")]
    pub fn with_tool_registry(mut self, registry: Arc<turul_mcp_server::ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Sets the CORS configuration.
    #[cfg(feature = "cors")]
    pub fn with_cors(mut self, cors_config: CorsConfig) -> Self {
        self.cors_config = Some(cors_config);
        self
    }

    /// Returns the JSON-RPC method names registered on this handler's dispatcher.
    ///
    /// Mirrors `McpServer::registered_methods` so a cross-builder parity test can
    /// assert the Lambda and local builders register an identical set per protocol lane.
    pub fn registered_methods(&self) -> Vec<String> {
        self.dispatcher.registered_methods()
    }

    /// Returns the underlying stream manager used for notifications.
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
    /// The custom route for `path`: an exact match, or — when lambda_http
    /// prefixed an API Gateway `stage` onto the path — an exact match with that
    /// one prefix removed. The registry itself stays exact-match.
    fn match_route(
        &self,
        path: &str,
        stage: Option<&str>,
    ) -> std::result::Result<
        Option<&Arc<dyn turul_http_mcp_server::RouteHandler>>,
        turul_http_mcp_server::RouteValidationError,
    > {
        if let Some(handler) = self.route_registry.match_route(path)? {
            return Ok(Some(handler));
        }
        let unstaged = stage.and_then(|stage| {
            path.strip_prefix('/')
                .and_then(|p| p.strip_prefix(stage))
                .filter(|rest| rest.starts_with('/'))
        });
        match unstaged {
            Some(rest) => self.route_registry.match_route(rest),
            None => Ok(None),
        }
    }

    /// Pull tool changes from shared state before an MCP request; a failure is
    /// logged and the request proceeds on the current tool set.
    #[cfg(feature = "dynamic-tools")]
    async fn refresh_dynamic_tools(&self) {
        if let Some(ref registry) = self.tool_registry
            && let Err(e) = registry.check_for_changes().await
        {
            tracing::warn!(error = %e, "Failed to check for tool changes");
        }
    }

    /// Handles one Lambda HTTP request and returns the response to send back.
    ///
    /// Answers CORS preflights and custom routes itself; every other request is delegated to the MCP HTTP handler.
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

        // The API Gateway stage lambda_http prefixed onto the path, if any.
        let stage = crate::adapter::api_gateway_stage_prefix(&req);

        // Handle CORS preflight requests first (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if method == http::Method::OPTIONS
            && let Some(ref cors_config) = self.cors_config
            // A custom route answers its own preflight.
            && !matches!(self.match_route(req.uri().path(), stage.as_deref()), Ok(Some(_)))
        {
            debug!("Handling CORS preflight request");
            return create_preflight_response(cors_config, request_origin.as_deref());
        }

        // 🚀 DELEGATION: Convert Lambda request to hyper request
        let hyper_req = crate::adapter::lambda_to_hyper_request(req)?;

        // Check custom routes (e.g., .well-known) before MCP delegation
        let path = hyper_req.uri().path().to_string();
        if !self.route_registry.is_empty() {
            match self.match_route(&path, stage.as_deref()) {
                Ok(Some(route_handler)) => {
                    debug!("Custom route matched: {}", path);
                    use http_body_util::BodyExt;
                    let (parts, body) = hyper_req.into_parts();
                    let boxed_req = hyper::Request::from_parts(parts, body.boxed_unsync());
                    let route_resp = route_handler.handle(boxed_req).await;
                    // `mut` is only needed when the `cors` feature injects headers below.
                    #[cfg_attr(not(feature = "cors"), allow(unused_mut))]
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
                    // `mut` is only needed when the `cors` feature injects headers below.
                    #[cfg_attr(not(feature = "cors"), allow(unused_mut))]
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

        // Check for remote tool changes (Dynamic mode with coordination). Only
        // requests bound for MCP pay for it; a custom route never does.
        #[cfg(feature = "dynamic-tools")]
        self.refresh_dynamic_tools().await;

        // A protocol-2026-07-28 build serves a single spec: every request goes
        // to the streamable handler, which mints the stateless core's
        // per-request session (so middleware-injected state reaches tool
        // handlers) and enforces Server Validation. SessionMcpHandler would
        // dispatch without a session and bypass both contracts.
        #[cfg(feature = "protocol-2026-07-28")]
        let hyper_resp = self.streamable_handler.handle_request(hyper_req).await;

        // protocol-2025-11-25 buffered lane: SessionMcpHandler owns the
        // Mcp-Session-Id lifecycle and returns buffered JSON bodies suited to
        // non-streaming Lambda responses.
        #[cfg(feature = "protocol-2025-11-25")]
        let hyper_resp = self
            .session_handler
            .handle_mcp_request(hyper_req)
            .await
            .map_err(|e| crate::error::LambdaError::McpFramework(e.to_string()))?;

        // 🚀 DELEGATION: Convert hyper response back to Lambda response
        // `mut` is only needed when the `cors` feature injects headers below.
        #[cfg_attr(not(feature = "cors"), allow(unused_mut))]
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

        // The API Gateway stage lambda_http prefixed onto the path, if any.
        let stage = crate::adapter::api_gateway_stage_prefix(&req);

        // Handle CORS preflight requests first (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if method == http::Method::OPTIONS
            && let Some(ref cors_config) = self.cors_config
            // A custom route answers its own preflight.
            && !matches!(self.match_route(req.uri().path(), stage.as_deref()), Ok(Some(_)))
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
            match self.match_route(&path, stage.as_deref()) {
                Ok(Some(route_handler)) => {
                    debug!("Custom route matched (streaming): {}", path);
                    use http_body_util::BodyExt;
                    let (parts, body) = hyper_req.into_parts();
                    let boxed_req = hyper::Request::from_parts(parts, body.boxed_unsync());
                    // `mut` is only needed when the `cors` feature injects headers below.
                    #[cfg_attr(not(feature = "cors"), allow(unused_mut))]
                    let mut route_resp = route_handler.handle(boxed_req).await;
                    #[cfg(feature = "cors")]
                    if let Some(ref cors_config) = self.cors_config {
                        inject_cors_headers(
                            &mut route_resp,
                            cors_config,
                            request_origin.as_deref(),
                        )
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    }
                    return Ok(route_resp);
                }
                Ok(None) => {} // No match, continue to MCP handler
                Err(e) => {
                    debug!("Route validation error (streaming): {}", e);
                    // `mut` is only needed when the `cors` feature injects headers below.
                    #[cfg_attr(not(feature = "cors"), allow(unused_mut))]
                    let mut err_resp = e.into_response();
                    #[cfg(feature = "cors")]
                    if let Some(ref cors_config) = self.cors_config {
                        inject_cors_headers(&mut err_resp, cors_config, request_origin.as_deref())
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    }
                    return Ok(err_resp);
                }
            }
        }

        // Check for remote tool changes (Dynamic mode with coordination). Only
        // requests bound for MCP pay for it; a custom route never does.
        #[cfg(feature = "dynamic-tools")]
        self.refresh_dynamic_tools().await;

        // 🚀 PROTOCOL ROUTING: Check protocol version and route to appropriate handler
        use turul_http_mcp_server::protocol::McpProtocolVersion;
        // Absent and unrecognised are distinct: absent falls back to this build's
        // lane, unrecognised must NOT be silently downgraded to a superseded spec
        // — it goes to the streamable handler, which answers with the spec's
        // UnsupportedProtocolVersion error rather than serving the request.
        let raw_version = hyper_req
            .headers()
            .get("MCP-Protocol-Version")
            .and_then(|h| h.to_str().ok());
        let parsed_version = raw_version.and_then(McpProtocolVersion::parse_version);
        let unrecognised_version = raw_version.is_some() && parsed_version.is_none();
        let protocol_version = parsed_version.unwrap_or(McpProtocolVersion::LATEST);

        // A protocol-2026-07-28 build serves a single spec: route everything
        // to the streamable handler, whose Server Validation rejects
        // unsupported MCP-Protocol-Version values. Routing legacy version
        // headers to SessionMcpHandler would bypass that contract and dispatch
        // without a per-request session.
        #[cfg(feature = "protocol-2026-07-28")]
        let route_streamable = true;
        #[cfg(feature = "protocol-2025-11-25")]
        let route_streamable = unrecognised_version || protocol_version.supports_streamable_http();

        // Route based on protocol version
        let hyper_resp = if route_streamable {
            debug!(
                "Using StreamableHttpHandler for protocol {} (header={:?}, unrecognised={})",
                protocol_version.as_str(),
                raw_version,
                unrecognised_version
            );
            self.streamable_handler.handle_request(hyper_req).await
        } else {
            // Legacy protocol: use SessionMcpHandler
            debug!(
                "Using SessionMcpHandler for legacy protocol {}",
                protocol_version.as_str()
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
        // `mut` is only needed when the `cors` feature injects headers below.
        #[cfg_attr(not(feature = "cors"), allow(unused_mut))]
        let mut lambda_resp = crate::adapter::hyper_to_lambda_streaming(hyper_resp);

        // Apply CORS headers if configured (Lambda-specific logic)
        #[cfg(feature = "cors")]
        if let Some(ref cors_config) = self.cors_config {
            inject_cors_headers(&mut lambda_resp, cors_config, request_origin.as_deref())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }

        Ok(lambda_resp)
    }

    /// Convert Lambda response to streaming format (helper for CORS preflight).
    /// Its only caller is the preflight branch, which is `cors`-gated.
    #[cfg(feature = "cors")]
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
    #[cfg(feature = "protocol-2025-11-25")]
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
    #[cfg(feature = "protocol-2025-11-25")]
    #[derive(Clone, Default)]
    struct LifecycleTestTool;

    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasBaseMetadata for LifecycleTestTool {
        fn name(&self) -> &str {
            "ping_tool"
        }
    }
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasDescription for LifecycleTestTool {
        fn description(&self) -> Option<&str> {
            Some("test tool")
        }
    }
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasInputSchema for LifecycleTestTool {
        fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
            static SCHEMA: std::sync::OnceLock<turul_mcp_protocol::ToolSchema> =
                std::sync::OnceLock::new();
            SCHEMA.get_or_init(turul_mcp_protocol::ToolSchema::object)
        }
    }
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasOutputSchema for LifecycleTestTool {
        fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
            None
        }
    }
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasAnnotations for LifecycleTestTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasToolMeta for LifecycleTestTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasIcons for LifecycleTestTool {}
    #[cfg(feature = "protocol-2025-11-25")]
    impl turul_mcp_builders::traits::HasExecution for LifecycleTestTool {}

    #[async_trait::async_trait]
    #[cfg(feature = "protocol-2025-11-25")]
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
    #[cfg(feature = "protocol-2025-11-25")]
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
    #[cfg(feature = "protocol-2025-11-25")]
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
    #[cfg(feature = "protocol-2025-11-25")]
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
    #[cfg(feature = "protocol-2025-11-25")]
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

    /// Full strict lifecycle handshake succeeds on handle_streaming()
    // The initialize/initialized handshake and strict lifecycle exist only in
    // 2025-11-25; the 2026-07-28 stateless core has neither.
    #[cfg(feature = "protocol-2025-11-25")]
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

    /// Strict lifecycle rejects both tools/list and tools/call before notifications/initialized
    #[cfg(feature = "protocol-2025-11-25")]
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

    /// tools/list succeeds immediately after notifications/initialized (race fix proof)
    // The initialize/initialized handshake is 2025-11-25 only; the 2026-07-28 stateless
    // core has no handshake and requires a per-request _meta on tools/list.
    #[cfg(feature = "protocol-2025-11-25")]
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

    /// Lenient mode allows operations without notifications/initialized
    #[cfg(feature = "protocol-2025-11-25")]
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

    // ── Streaming custom-route CORS regression tests ──
    //
    // Guards the parity between the buffered `handle()` path and the
    // streaming `handle_streaming()` path: both must apply configured
    // CORS to custom-route responses (matched and validation-error)
    // before returning.

    #[cfg(feature = "cors")]
    mod cors_streaming_routes {
        use super::*;
        use async_trait::async_trait;
        use bytes::Bytes;
        use http_body_util::Full;
        use hyper::{Request as HyperRequest, Response as HyperResponse, StatusCode};
        use turul_http_mcp_server::middleware::MiddlewareStack;
        use turul_http_mcp_server::{
            RouteBody, RouteHandler, RouteRegistry, StreamConfig, StreamManager,
        };

        struct StubRoute {
            status: StatusCode,
            body: &'static str,
        }

        #[async_trait]
        impl RouteHandler for StubRoute {
            async fn handle(&self, _req: HyperRequest<RouteBody>) -> HyperResponse<RouteBody> {
                use http_body_util::BodyExt;
                HyperResponse::builder()
                    .status(self.status)
                    .header("Content-Type", "application/json")
                    .body(
                        Full::new(Bytes::from(self.body))
                            .map_err(|never| match never {})
                            .boxed_unsync(),
                    )
                    .unwrap()
            }
        }

        fn handler_with_route_and_cors(
            registry: Arc<RouteRegistry>,
            cors: Option<CorsConfig>,
        ) -> LambdaMcpHandler {
            let session_storage = Arc::new(InMemorySessionStorage::new());
            let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
            let dispatcher = Arc::new(JsonRpcDispatcher::new());
            let config = ServerConfig::default();
            let capabilities = ServerCapabilities::default();
            let middleware_stack = Arc::new(MiddlewareStack::new());

            let handler = LambdaMcpHandler::with_middleware(
                config,
                dispatcher,
                session_storage,
                stream_manager,
                StreamConfig::default(),
                capabilities,
                middleware_stack,
                false,
                registry,
            );
            match cors {
                Some(cfg) => handler.with_cors(cfg),
                None => handler,
            }
        }

        fn get_request(path: &str, origin: &str) -> LambdaRequest {
            Request::builder()
                .method("GET")
                .uri(path)
                .header("Origin", origin)
                .body(LambdaBody::Empty)
                .unwrap()
        }

        /// A route that owns its CORS and answers its own preflight, as a Server
        /// Card does.
        struct CorsOwningRoute;

        #[async_trait]
        impl RouteHandler for CorsOwningRoute {
            async fn handle(&self, req: HyperRequest<RouteBody>) -> HyperResponse<RouteBody> {
                use http_body_util::BodyExt;
                let status = if req.method() == hyper::Method::OPTIONS {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::OK
                };
                HyperResponse::builder()
                    .status(status)
                    .header("Access-Control-Allow-Origin", "*")
                    .header(
                        "Access-Control-Allow-Headers",
                        "Content-Type, If-None-Match",
                    )
                    .header("Access-Control-Expose-Headers", "ETag")
                    .body(
                        Full::new(Bytes::new())
                            .map_err(|never| match never {})
                            .boxed_unsync(),
                    )
                    .unwrap()
            }
        }

        fn cors_owning_handler() -> LambdaMcpHandler {
            let mut registry = RouteRegistry::new();
            registry.add_route("/mcp/server-card", Arc::new(CorsOwningRoute));
            handler_with_route_and_cors(Arc::new(registry), Some(CorsConfig::default()))
        }

        fn request(method: &str, path: &str) -> LambdaRequest {
            Request::builder()
                .method(method)
                .uri(path)
                .header("Origin", "https://browser.example")
                .header("Access-Control-Request-Method", "GET")
                .body(LambdaBody::Empty)
                .unwrap()
        }

        fn assert_route_cors_kept(headers: &http::HeaderMap, what: &str) {
            assert_eq!(headers["access-control-allow-origin"], "*", "{what}");
            assert_eq!(headers["access-control-expose-headers"], "ETag", "{what}");
            assert_eq!(
                headers["access-control-allow-headers"], "Content-Type, If-None-Match",
                "{what}"
            );
        }

        /// An API Gateway event parsed by lambda_http itself, so the path carries
        /// exactly the stage prefix a deployed Lambda sees.
        fn apigw_event(version: &str, method: &str, path: &str, stage: &str) -> LambdaRequest {
            let event = if version == "rest" {
                serde_json::json!({
                    "resource": "/{proxy+}", "path": path, "httpMethod": method,
                    "headers": { "Origin": "https://browser.example" },
                    "isBase64Encoded": false,
                    "requestContext": { "resourcePath": "/{proxy+}", "httpMethod": method,
                        "requestId": "r", "accountId": "1", "stage": stage, "apiId": "a",
                        "identity": {} }
                })
            } else {
                // HTTP API puts a named stage in rawPath itself.
                let raw = if stage == "$default" {
                    path.to_string()
                } else {
                    format!("/{stage}{path}")
                };
                serde_json::json!({
                    "version": "2.0", "routeKey": "$default", "rawPath": raw,
                    "rawQueryString": "", "headers": { "origin": "https://browser.example" },
                    "isBase64Encoded": false,
                    "requestContext": { "accountId": "1", "apiId": "a",
                        "domainName": "a.execute-api.example.com",
                        "http": { "method": method, "path": raw, "protocol": "HTTP/1.1",
                                  "sourceIp": "1.2.3.4", "userAgent": "t" },
                        "requestId": "r", "stage": stage, "time": "t", "timeEpoch": 0 }
                })
            };
            lambda_http::request::from_str(&event.to_string())
                .expect("lambda_http parses the event")
        }

        fn card_route_handler() -> LambdaMcpHandler {
            let mut registry = RouteRegistry::new();
            registry.add_route(
                "/mcp/server-card",
                Arc::new(StubRoute {
                    status: StatusCode::OK,
                    body: "card",
                }),
            );
            handler_with_route_and_cors(Arc::new(registry), Some(CorsConfig::default()))
        }

        #[tokio::test]
        async fn route_matches_behind_an_api_gateway_stage_rest_and_http_api() {
            let handler = card_route_handler();
            for version in ["rest", "http"] {
                for stage in ["prod", "$default"] {
                    let req = apigw_event(version, "GET", "/mcp/server-card", stage);
                    let what = format!("{version} stage {stage} path {}", req.uri().path());
                    let resp = handler.handle(req).await.unwrap();
                    assert_eq!(resp.status(), StatusCode::OK, "buffered {what}");
                    let req = apigw_event(version, "GET", "/mcp/server-card", stage);
                    let resp = handler.handle_streaming(req).await.unwrap();
                    assert_eq!(resp.status(), StatusCode::OK, "streaming {what}");
                }
            }
        }

        #[tokio::test]
        async fn route_matches_behind_an_api_gateway_stage_only_for_that_stage() {
            let handler = card_route_handler();
            // A different first segment is not the stage: no route matches, and
            // the request falls through to the MCP handler instead of the card.
            let req = apigw_event("rest", "GET", "/other/mcp/server-card", "prod");
            assert_eq!(req.uri().path(), "/prod/other/mcp/server-card");
            let resp = handler.handle(req).await.unwrap();
            assert_ne!(resp.status(), StatusCode::OK);
            assert_ne!(resp.body().as_ref(), b"card");
            // The stage alone is not a route either.
            let req = apigw_event("rest", "GET", "/server-card", "mcp");
            let resp = handler.handle(req).await.unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "/mcp + /server-card is the route itself"
            );
        }

        /// Counts fingerprint reads, the storage call `check_for_changes` makes.
        #[cfg(feature = "dynamic-tools")]
        struct CountingState {
            inner: turul_mcp_server_state_storage::InMemoryServerStateStorage,
            fingerprint_reads: std::sync::atomic::AtomicUsize,
        }

        #[cfg(feature = "dynamic-tools")]
        #[async_trait]
        impl turul_mcp_server_state_storage::ServerStateStorage for CountingState {
            fn backend_name(&self) -> &'static str {
                "counting"
            }
            async fn get_entity_state(
                &self,
                t: &str,
                id: &str,
            ) -> std::result::Result<
                Option<turul_mcp_server_state_storage::EntityState>,
                turul_mcp_server_state_storage::ServerStateError,
            > {
                self.inner.get_entity_state(t, id).await
            }
            async fn set_entity_state(
                &self,
                t: &str,
                id: &str,
                st: turul_mcp_server_state_storage::EntityState,
            ) -> std::result::Result<(), turul_mcp_server_state_storage::ServerStateError>
            {
                self.inner.set_entity_state(t, id, st).await
            }
            async fn delete_entity_state(
                &self,
                t: &str,
                id: &str,
            ) -> std::result::Result<(), turul_mcp_server_state_storage::ServerStateError>
            {
                self.inner.delete_entity_state(t, id).await
            }
            async fn get_active_entities(
                &self,
                t: &str,
            ) -> std::result::Result<Vec<String>, turul_mcp_server_state_storage::ServerStateError>
            {
                self.inner.get_active_entities(t).await
            }
            async fn get_fingerprint(
                &self,
                t: &str,
            ) -> std::result::Result<Option<String>, turul_mcp_server_state_storage::ServerStateError>
            {
                self.fingerprint_reads
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.inner.get_fingerprint(t).await
            }
            async fn set_fingerprint(
                &self,
                t: &str,
                fp: String,
            ) -> std::result::Result<(), turul_mcp_server_state_storage::ServerStateError>
            {
                self.inner.set_fingerprint(t, fp).await
            }
            async fn get_registry_snapshot(
                &self,
                t: &str,
            ) -> std::result::Result<
                Option<turul_mcp_server_state_storage::RegistrySnapshot>,
                turul_mcp_server_state_storage::ServerStateError,
            > {
                self.inner.get_registry_snapshot(t).await
            }
            async fn maintenance(
                &self,
            ) -> std::result::Result<(), turul_mcp_server_state_storage::ServerStateError>
            {
                self.inner.maintenance().await
            }
        }

        /// A custom-route request never reads shared tool state;
        /// a request bound for MCP still does.
        #[cfg(feature = "dynamic-tools")]
        #[tokio::test]
        async fn a_route_request_skips_the_dynamic_tools_check() {
            let state = Arc::new(CountingState {
                inner: turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
                fingerprint_reads: std::sync::atomic::AtomicUsize::new(0),
            });
            let registry = Arc::new(turul_mcp_server::ToolRegistry::new(
                std::collections::HashMap::new(),
                Arc::new(turul_mcp_server::SessionManager::new(
                    ServerCapabilities::default(),
                )),
                state.clone(),
            ));
            let handler = card_route_handler().with_tool_registry(registry);
            let reads = || {
                state
                    .fingerprint_reads
                    .load(std::sync::atomic::Ordering::SeqCst)
            };

            for method in ["GET", "OPTIONS"] {
                handler
                    .handle(apigw_event("rest", method, "/mcp/server-card", "prod"))
                    .await
                    .unwrap();
                handler
                    .handle_streaming(apigw_event("rest", method, "/mcp/server-card", "prod"))
                    .await
                    .unwrap();
            }
            assert_eq!(reads(), 0, "a card request read tool state");

            handler
                .handle(apigw_event("rest", "POST", "/mcp", "prod"))
                .await
                .unwrap();
            assert_eq!(
                reads(),
                1,
                "an MCP request must still check for tool changes"
            );
        }

        #[tokio::test]
        async fn a_route_that_sets_cors_keeps_it_and_answers_its_preflight() {
            let handler = cors_owning_handler();
            for method in ["GET", "OPTIONS"] {
                let buffered = handler
                    .handle(request(method, "/mcp/server-card"))
                    .await
                    .unwrap();
                assert_route_cors_kept(buffered.headers(), &format!("buffered {method}"));
                let streaming = handler
                    .handle_streaming(request(method, "/mcp/server-card"))
                    .await
                    .unwrap();
                assert_route_cors_kept(streaming.headers(), &format!("streaming {method}"));
                if method == "OPTIONS" {
                    assert_eq!(
                        buffered.status(),
                        StatusCode::NO_CONTENT,
                        "route answers preflight"
                    );
                    assert_eq!(
                        streaming.status(),
                        StatusCode::NO_CONTENT,
                        "route answers preflight"
                    );
                }
            }
            // Any other path still gets the transport's own preflight.
            let other = handler.handle(request("OPTIONS", "/mcp")).await.unwrap();
            assert_ne!(other.headers()["access-control-expose-headers"], "ETag");
        }

        #[tokio::test]
        async fn streaming_custom_route_match_injects_cors() {
            let mut registry = RouteRegistry::new();
            registry.add_route(
                "/.well-known/oauth-protected-resource",
                Arc::new(StubRoute {
                    status: StatusCode::OK,
                    body: r#"{"resource":"https://example.test/mcp"}"#,
                }),
            );
            let handler =
                handler_with_route_and_cors(Arc::new(registry), Some(CorsConfig::default()));

            let req = get_request(
                "/.well-known/oauth-protected-resource",
                "https://client.example.test",
            );
            let resp = handler.handle_streaming(req).await.unwrap();

            assert_eq!(resp.status(), StatusCode::OK);
            assert!(
                resp.headers().contains_key("access-control-allow-origin"),
                "matched streaming route must carry CORS headers",
            );
            assert!(
                resp.headers().contains_key("access-control-expose-headers"),
                "matched streaming route must expose configured headers",
            );
        }

        #[tokio::test]
        async fn streaming_route_validation_error_injects_cors() {
            // Empty registry + path-traversal path → validation error branch.
            let registry = Arc::new({
                let mut r = RouteRegistry::new();
                r.add_route(
                    "/.well-known/oauth-protected-resource",
                    Arc::new(StubRoute {
                        status: StatusCode::OK,
                        body: "{}",
                    }),
                );
                r
            });
            let handler = handler_with_route_and_cors(registry, Some(CorsConfig::default()));

            let req = get_request("/../etc/passwd", "https://client.example.test");
            let resp = handler.handle_streaming(req).await.unwrap();

            assert!(
                resp.status().is_client_error(),
                "path-traversal must be a 4xx, got {}",
                resp.status(),
            );
            assert!(
                resp.headers().contains_key("access-control-allow-origin"),
                "validation-error streaming route must carry CORS headers",
            );
        }

        #[tokio::test]
        async fn streaming_custom_route_without_cors_config_returns_untouched() {
            // Sanity: without `.with_cors()`, the route response must NOT
            // gain CORS headers (regression guard so we never inject
            // default CORS for consumers who deliberately opted out).
            let mut registry = RouteRegistry::new();
            registry.add_route(
                "/.well-known/oauth-protected-resource",
                Arc::new(StubRoute {
                    status: StatusCode::OK,
                    body: "{}",
                }),
            );
            let handler = handler_with_route_and_cors(Arc::new(registry), None);

            let req = get_request(
                "/.well-known/oauth-protected-resource",
                "https://client.example.test",
            );
            let resp = handler.handle_streaming(req).await.unwrap();

            assert_eq!(resp.status(), StatusCode::OK);
            assert!(
                !resp.headers().contains_key("access-control-allow-origin"),
                "no CORS config → no CORS headers (got {:?})",
                resp.headers(),
            );
        }
    }

    // ── OAuth-style 401 challenge through streaming + CORS ──
    //
    // Verifies the transport contract: a middleware that returns
    // `MiddlewareError::http_challenge(401, ...)` produces a response
    // that (a) keeps the WWW-Authenticate header, (b) carries
    // configured CORS, and (c) exposes WWW-Authenticate so browser
    // OAuth clients can read it for RFC 9728 discovery.

    #[cfg(feature = "cors")]
    mod cors_streaming_oauth {
        use super::*;
        use async_trait::async_trait;
        use turul_http_mcp_server::middleware::{
            DispatcherResult, McpMiddleware, MiddlewareError, MiddlewareStack, RequestContext,
            SessionInjection,
        };
        use turul_http_mcp_server::{StreamConfig, StreamManager};
        use turul_mcp_session_storage::SessionView;

        struct ForceChallenge;

        #[async_trait]
        impl McpMiddleware for ForceChallenge {
            fn runs_before_session(&self) -> bool {
                true
            }

            async fn before_dispatch(
                &self,
                _ctx: &mut RequestContext<'_>,
                _session: Option<&dyn SessionView>,
                _injection: &mut SessionInjection,
            ) -> std::result::Result<(), MiddlewareError> {
                Err(MiddlewareError::http_challenge(
                    401,
                    "Bearer realm=\"mcp\", resource_metadata=\"https://example.test/.well-known/oauth-protected-resource\"",
                ))
            }

            async fn after_dispatch(
                &self,
                _ctx: &RequestContext<'_>,
                _result: &mut DispatcherResult,
            ) -> std::result::Result<(), MiddlewareError> {
                Ok(())
            }
        }

        #[tokio::test]
        async fn streaming_401_challenge_has_cors_and_exposes_www_authenticate() {
            let session_storage = Arc::new(InMemorySessionStorage::new());
            let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
            let dispatcher = Arc::new(JsonRpcDispatcher::new());
            // Direct construction bypasses the builder's CORS→origin-policy
            // derivation; allow-all CORS pairs with Disabled.
            let config = ServerConfig {
                origin_policy: turul_http_mcp_server::OriginPolicy::Disabled,
                ..Default::default()
            };
            let capabilities = ServerCapabilities::default();

            let mut middleware = MiddlewareStack::new();
            middleware.push(Arc::new(ForceChallenge));
            let middleware = Arc::new(middleware);

            let route_registry = Arc::new(turul_http_mcp_server::RouteRegistry::new());

            let handler = LambdaMcpHandler::with_middleware(
                config,
                dispatcher,
                session_storage,
                stream_manager,
                StreamConfig::default(),
                capabilities,
                middleware,
                false,
                route_registry,
            )
            .with_cors(CorsConfig::default());

            let req = Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("MCP-Protocol-Version", "2025-11-25")
                .header("Origin", "https://client.example.test")
                .body(LambdaBody::Text(
                    r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
                ))
                .unwrap();

            let resp = handler.handle_streaming(req).await.unwrap();
            let headers = resp.headers();

            assert_eq!(resp.status(), 401, "challenge must be 401");
            assert!(
                headers.contains_key("www-authenticate"),
                "WWW-Authenticate must be preserved through streaming transport",
            );
            assert!(
                headers.contains_key("access-control-allow-origin"),
                "401 response must carry Access-Control-Allow-Origin",
            );
            let expose = headers
                .get("access-control-expose-headers")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                expose
                    .split(',')
                    .map(str::trim)
                    .any(|h| h.eq_ignore_ascii_case("WWW-Authenticate")),
                "expose-headers must include WWW-Authenticate; got {expose:?}",
            );
        }
    }
}
