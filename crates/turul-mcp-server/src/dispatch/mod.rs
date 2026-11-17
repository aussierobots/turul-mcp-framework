use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, instrument};

use crate::handlers::McpHandler;
use crate::session::SessionContext;
use turul_mcp_protocol::{McpError, McpResult};

/// Dispatches MCP requests through middleware to handlers matched by exact method, pattern, or default.
pub struct McpDispatcher {
    /// Route handlers mapped by method pattern
    route_handlers: HashMap<String, Arc<dyn McpHandler>>,
    /// Wildcard handlers for method patterns (e.g., "tools/*")
    pattern_handlers: Vec<(String, Arc<dyn McpHandler>)>,
    /// Middleware stack for request processing
    middleware: Vec<Arc<dyn DispatchMiddleware>>,
    /// Default fallback handler
    default_handler: Option<Arc<dyn McpHandler>>,
}

/// Middleware run before and after a request is routed to its handler.
#[async_trait]
pub trait DispatchMiddleware: Send + Sync {
    /// Processes a request before routing.
    ///
    /// Returns `None` to continue, or `Some(result)` to short-circuit with that result.
    async fn before_dispatch(
        &self,
        method: &str,
        params: Option<&Value>,
        session: Option<&SessionContext>,
    ) -> Option<McpResult<Value>>;

    /// Processes the handler's result after routing and returns the result to pass on.
    async fn after_dispatch(
        &self,
        method: &str,
        result: &McpResult<Value>,
        session: Option<&SessionContext>,
    ) -> McpResult<Value>;
}

/// Request context for dispatch processing.
pub struct DispatchContext {
    /// JSON-RPC method name being dispatched.
    pub method: String,
    /// Request parameters, if any.
    pub params: Option<Value>,
    /// Session the request belongs to, if any.
    pub session: Option<SessionContext>,
    /// Free-form values shared between dispatch stages.
    pub metadata: HashMap<String, Value>,
}

impl McpDispatcher {
    /// Creates a dispatcher with no handlers or middleware.
    pub fn new() -> Self {
        Self {
            route_handlers: HashMap::new(),
            pattern_handlers: Vec::new(),
            middleware: Vec::new(),
            default_handler: None,
        }
    }

    /// Registers a handler for exact method matching.
    pub fn register_exact_handler(mut self, method: String, handler: Arc<dyn McpHandler>) -> Self {
        self.route_handlers.insert(method, handler);
        self
    }

    /// Registers a handler for pattern matching (e.g., `tools/*`).
    pub fn register_pattern_handler(
        mut self,
        pattern: String,
        handler: Arc<dyn McpHandler>,
    ) -> Self {
        self.pattern_handlers.push((pattern, handler));
        self
    }

    /// Registers middleware.
    ///
    /// `before_dispatch` runs in registration order; `after_dispatch` runs in reverse order.
    pub fn register_middleware(mut self, middleware: Arc<dyn DispatchMiddleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Sets the default fallback handler.
    pub fn set_default_handler(mut self, handler: Arc<dyn McpHandler>) -> Self {
        self.default_handler = Some(handler);
        self
    }

    /// Dispatches a request through the routing and middleware pipeline.
    ///
    /// # Errors
    ///
    /// Returns `McpError::InvalidParameters` when no exact, pattern, or default handler
    /// matches `method`; otherwise returns the handler's or middleware's result.
    #[instrument(skip(self, params, session))]
    pub async fn dispatch(
        &self,
        method: &str,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        debug!("Dispatching request: method={}", method);

        for middleware in &self.middleware {
            if let Some(result) = middleware
                .before_dispatch(method, params.as_ref(), session.as_ref())
                .await
            {
                debug!("Request short-circuited by middleware");
                return result;
            }
        }

        let handler = self.find_handler(method)?;

        let mut result = handler.handle_with_session(params, session.clone()).await;

        for middleware in self.middleware.iter().rev() {
            result = middleware
                .after_dispatch(method, &result, session.as_ref())
                .await;
        }

        result
    }

    /// Finds the handler for a method: exact match, then patterns in registration order, then the default.
    fn find_handler(&self, method: &str) -> McpResult<&Arc<dyn McpHandler>> {
        if let Some(handler) = self.route_handlers.get(method) {
            debug!("Found exact handler for method: {}", method);
            return Ok(handler);
        }

        for (pattern, handler) in &self.pattern_handlers {
            if self.matches_pattern(method, pattern) {
                debug!("Found pattern handler '{}' for method: {}", pattern, method);
                return Ok(handler);
            }
        }

        if let Some(ref handler) = self.default_handler {
            debug!("Using default handler for method: {}", method);
            return Ok(handler);
        }

        error!("No handler found for method: {}", method);
        Err(McpError::InvalidParameters(format!(
            "Method not found: {}",
            method
        )))
    }

    /// Returns whether a method matches a pattern.
    ///
    /// A trailing `/*` matches any non-empty suffix after the prefix; any other `*` never matches;
    /// a pattern without `*` must equal the method.
    fn matches_pattern(&self, method: &str, pattern: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            method.starts_with(prefix) && method.len() > prefix.len()
        } else if pattern.contains('*') {
            false
        } else {
            method == pattern
        }
    }

    /// Returns all registered exact methods and patterns, sorted.
    pub fn get_supported_methods(&self) -> Vec<String> {
        let mut methods = Vec::new();

        methods.extend(self.route_handlers.keys().cloned());

        methods.extend(
            self.pattern_handlers
                .iter()
                .map(|(pattern, _)| pattern.clone()),
        );

        methods.sort();
        methods
    }
}

impl Default for McpDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware that logs requests and responses at debug level.
///
/// `after_dispatch` returns `Ok` values unchanged and re-wraps any error as
/// `McpError::InvalidParameters` carrying the original error's message.
pub struct LoggingMiddleware;

#[async_trait]
impl DispatchMiddleware for LoggingMiddleware {
    async fn before_dispatch(
        &self,
        method: &str,
        params: Option<&Value>,
        session: Option<&SessionContext>,
    ) -> Option<McpResult<Value>> {
        let none_string = "none".to_string();
        let session_id = session
            .as_ref()
            .map(|s| s.session_id.as_str())
            .unwrap_or(&none_string);
        debug!(
            "Request: method={}, session={}, params={}",
            method,
            session_id,
            params
                .map(|p| p.to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        None
    }

    async fn after_dispatch(
        &self,
        method: &str,
        result: &McpResult<Value>,
        session: Option<&SessionContext>,
    ) -> McpResult<Value> {
        let none_string = "none".to_string();
        let session_id = session
            .as_ref()
            .map(|s| s.session_id.as_str())
            .unwrap_or(&none_string);
        match result {
            Ok(value) => {
                debug!(
                    "Response: method={}, session={}, success=true, result_keys={:?}",
                    method,
                    session_id,
                    value.as_object().map(|o| o.keys().collect::<Vec<_>>())
                );
            }
            Err(error) => {
                debug!(
                    "Response: method={}, session={}, error={}",
                    method, session_id, error
                );
            }
        }
        match result {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(McpError::InvalidParameters(error.to_string())),
        }
    }
}

/// Placeholder middleware that does not limit requests.
///
/// `before_dispatch` always continues; `after_dispatch` returns `Ok` values unchanged and
/// re-wraps any error as `McpError::InvalidParameters`.
pub struct RateLimitingMiddleware {}

#[async_trait]
impl DispatchMiddleware for RateLimitingMiddleware {
    async fn before_dispatch(
        &self,
        _method: &str,
        _params: Option<&Value>,
        _session: Option<&SessionContext>,
    ) -> Option<McpResult<Value>> {
        None
    }

    async fn after_dispatch(
        &self,
        _method: &str,
        result: &McpResult<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Value> {
        match result {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(McpError::InvalidParameters(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::McpHandler;

    struct TestHandler {
        response: Value,
    }

    #[async_trait]
    impl McpHandler for TestHandler {
        async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
            Ok(self.response.clone())
        }

        fn supported_methods(&self) -> Vec<String> {
            vec!["test".to_string()]
        }
    }

    #[tokio::test]
    async fn test_exact_routing() {
        let handler = Arc::new(TestHandler {
            response: Value::String("test_response".to_string()),
        });

        let dispatcher =
            McpDispatcher::new().register_exact_handler("test/method".to_string(), handler);

        let result = dispatcher
            .dispatch("test/method", None, None)
            .await
            .unwrap();
        assert_eq!(result, Value::String("test_response".to_string()));
    }

    #[tokio::test]
    async fn test_pattern_routing() {
        let handler = Arc::new(TestHandler {
            response: Value::String("pattern_response".to_string()),
        });

        let dispatcher =
            McpDispatcher::new().register_pattern_handler("tools/*".to_string(), handler);

        let result = dispatcher.dispatch("tools/list", None, None).await.unwrap();
        assert_eq!(result, Value::String("pattern_response".to_string()));
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let dispatcher = McpDispatcher::new();

        let result = dispatcher.dispatch("unknown/method", None, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            McpError::InvalidParameters(_)
        ));
    }
}
