use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

#[cfg(feature = "streams")]
use futures::{Stream, StreamExt};
#[cfg(feature = "streams")]
use std::pin::Pin;

use crate::{
    error::JsonRpcError,
    notification::JsonRpcNotification,
    request::{JsonRpcRequest, RequestParams},
    response::{JsonRpcMessage, ResponseResult},
};

/// Minimal session context for JSON-RPC handlers
/// This provides basic session information without circular dependencies
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// Unique session identifier
    pub session_id: String,
    /// Session metadata
    pub metadata: HashMap<String, Value>,
    /// Optional broadcaster for session notifications
    pub broadcaster: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Session timestamp (Unix milliseconds)
    pub timestamp: u64,
}

/// Trait for handling JSON-RPC method calls
#[async_trait]
pub trait JsonRpcHandler: Send + Sync {
    /// The error type returned by this handler
    type Error: std::error::Error + Send + Sync + 'static;

    /// Handle a JSON-RPC method call with optional session context
    /// Returns domain errors only - dispatcher handles conversion to JSON-RPC errors
    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<Value, Self::Error>;

    /// Handle a JSON-RPC notification with optional session context (optional - default does nothing)
    async fn handle_notification(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<(), Self::Error> {
        // Default implementation - ignore notifications
        let _ = (method, params, session_context);
        Ok(())
    }

    /// List supported methods (optional - used for introspection)
    fn supported_methods(&self) -> Vec<String> {
        vec![]
    }
}

/// A simple function-based handler
pub struct FunctionHandler<F, N, E>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<Value, E>>
        + Send
        + Sync,
    N: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<(), E>>
        + Send
        + Sync,
{
    handler_fn: F,
    notification_fn: Option<N>,
    methods: Vec<String>,
}

impl<F, N, E> FunctionHandler<F, N, E>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<Value, E>>
        + Send
        + Sync,
    N: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<(), E>>
        + Send
        + Sync,
{
    pub fn new(handler_fn: F) -> Self {
        Self {
            handler_fn,
            notification_fn: None,
            methods: vec![],
        }
    }

    pub fn with_notification_handler(mut self, notification_fn: N) -> Self {
        self.notification_fn = Some(notification_fn);
        self
    }

    pub fn with_methods(mut self, methods: Vec<String>) -> Self {
        self.methods = methods;
        self
    }
}

#[async_trait]
impl<F, N, E> JsonRpcHandler for FunctionHandler<F, N, E>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<Value, E>>
        + Send
        + Sync,
    N: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<(), E>>
        + Send
        + Sync,
{
    type Error = E;

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<Value, Self::Error> {
        (self.handler_fn)(method, params, session_context).await
    }

    async fn handle_notification(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<(), Self::Error> {
        if let Some(ref notification_fn) = self.notification_fn {
            (notification_fn)(method, params, session_context).await
        } else {
            Ok(())
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        self.methods.clone()
    }
}

/// Trait for errors that can be converted to JSON-RPC error objects
pub trait ToJsonRpcError: std::error::Error + Send + Sync + 'static {
    /// Convert this error to a JSON-RPC error object
    fn to_error_object(&self) -> crate::error::JsonRpcErrorObject;
}

/// JSON-RPC method dispatcher with specific error type
pub struct JsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    pub handlers: HashMap<String, Arc<dyn JsonRpcHandler<Error = E>>>,
    pub default_handler: Option<Arc<dyn JsonRpcHandler<Error = E>>>,
}

impl<E> JsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            default_handler: None,
        }
    }

    /// Register a handler for a specific method
    pub fn register_method<H>(&mut self, method: String, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        self.handlers.insert(method, Arc::new(handler));
    }

    /// Register a handler for multiple methods
    pub fn register_methods<H>(&mut self, methods: Vec<String>, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        let handler_arc = Arc::new(handler);
        for method in methods {
            self.handlers.insert(method, handler_arc.clone());
        }
    }

    /// Set a default handler for unregistered methods
    pub fn set_default_handler<H>(&mut self, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        self.default_handler = Some(Arc::new(handler));
    }

    /// Process a JSON-RPC request with session context and return a response
    pub async fn handle_request_with_context(
        &self,
        request: JsonRpcRequest,
        session_context: SessionContext,
    ) -> JsonRpcMessage {
        let handler = self
            .handlers
            .get(&request.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                match handler
                    .handle(&request.method, request.params, Some(session_context))
                    .await
                {
                    Ok(result) => {
                        JsonRpcMessage::success(request.id, ResponseResult::Success(result))
                    }
                    Err(domain_error) => {
                        // Convert domain error to JSON-RPC error using type-safe conversion
                        let error_object = domain_error.to_error_object();
                        let rpc_error = JsonRpcError::new(Some(request.id.clone()), error_object);
                        JsonRpcMessage::error(rpc_error)
                    }
                }
            }
            None => {
                let error = JsonRpcError::method_not_found(request.id.clone(), &request.method);
                JsonRpcMessage::error(error)
            }
        }
    }

    /// Process a JSON-RPC request and return a response (backward compatibility - no session context)
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcMessage {
        let handler = self
            .handlers
            .get(&request.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                match handler.handle(&request.method, request.params, None).await {
                    Ok(result) => {
                        JsonRpcMessage::success(request.id, ResponseResult::Success(result))
                    }
                    Err(domain_error) => {
                        // Convert domain error to JSON-RPC error using type-safe conversion
                        let error_object = domain_error.to_error_object();
                        let rpc_error = JsonRpcError::new(Some(request.id.clone()), error_object);
                        JsonRpcMessage::error(rpc_error)
                    }
                }
            }
            None => {
                let error = JsonRpcError::method_not_found(request.id.clone(), &request.method);
                JsonRpcMessage::error(error)
            }
        }
    }

    /// Process a JSON-RPC notification
    pub async fn handle_notification(&self, notification: JsonRpcNotification) -> Result<(), E> {
        let handler = self
            .handlers
            .get(&notification.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                handler
                    .handle_notification(&notification.method, notification.params, None)
                    .await
            }
            None => {
                // Notifications don't return errors, just ignore unknown methods
                Ok(())
            }
        }
    }

    /// Process a JSON-RPC notification with session context
    pub async fn handle_notification_with_context(
        &self,
        notification: JsonRpcNotification,
        session_context: Option<SessionContext>,
    ) -> Result<(), E> {
        let handler = self
            .handlers
            .get(&notification.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                handler
                    .handle_notification(&notification.method, notification.params, session_context)
                    .await
            }
            None => {
                // Notifications don't return errors, just ignore unknown methods
                Ok(())
            }
        }
    }

    /// Get all registered methods
    pub fn registered_methods(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

impl<E> Default for JsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JsonRpcRequest, RequestId};
    use serde_json::json;

    #[derive(thiserror::Error, Debug)]
    enum TestError {
        #[error("Test error: {0}")]
        TestError(String),
        #[error("Unknown method: {0}")]
        UnknownMethod(String),
    }

    impl ToJsonRpcError for TestError {
        fn to_error_object(&self) -> crate::error::JsonRpcErrorObject {
            use crate::error::JsonRpcErrorObject;
            match self {
                TestError::TestError(msg) => JsonRpcErrorObject::internal_error(Some(msg.clone())),
                TestError::UnknownMethod(method) => JsonRpcErrorObject::method_not_found(method),
            }
        }
    }

    struct TestHandler;

    #[async_trait]
    impl JsonRpcHandler for TestHandler {
        type Error = TestError;

        async fn handle(
            &self,
            method: &str,
            _params: Option<RequestParams>,
            _session_context: Option<SessionContext>,
        ) -> Result<Value, Self::Error> {
            match method {
                "add" => Ok(json!({"result": "addition"})),
                "error" => Err(TestError::TestError("test error".to_string())),
                _ => Err(TestError::UnknownMethod(method.to_string())),
            }
        }

        fn supported_methods(&self) -> Vec<String> {
            vec!["add".to_string(), "error".to_string()]
        }
    }

    #[tokio::test]
    async fn test_dispatcher_success() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let request = JsonRpcRequest::new_no_params(RequestId::Number(1), "add".to_string());

        let response = dispatcher.handle_request(request).await;
        assert_eq!(response.id(), Some(&RequestId::Number(1)));
        assert!(!response.is_error());
    }

    #[tokio::test]
    async fn test_dispatcher_method_not_found() {
        let dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();

        let request = JsonRpcRequest::new_no_params(RequestId::Number(1), "unknown".to_string());

        let response = dispatcher.handle_request(request).await;
        assert_eq!(response.id(), Some(&RequestId::Number(1)));
        assert!(response.is_error());
    }

    #[tokio::test]
    async fn test_function_handler() {
        // Test JsonRpcHandler directly
        let handler = TestHandler;
        let result = handler.handle("add", None, None).await.unwrap();
        assert_eq!(result["result"], "addition");
    }
}

// ============================================================================
// 🚀 STREAMING DISPATCHER - MCP 2025-06-18 Support
// ============================================================================

#[cfg(feature = "streams")]
pub mod streaming {
    use super::*;

    /// JSON-RPC frame for streaming responses
    /// Represents individual chunks in a progressive response stream
    #[derive(Debug, Clone)]
    pub enum JsonRpcFrame {
        /// Progress update with optional token for cancellation
        Progress {
            request_id: crate::types::RequestId,
            progress: Value,
            progress_token: Option<String>,
        },
        /// Partial result chunk
        PartialResult {
            request_id: crate::types::RequestId,
            data: Value,
        },
        /// Final result (ends the stream)
        FinalResult {
            request_id: crate::types::RequestId,
            result: Value,
        },
        /// Error result (ends the stream)
        Error {
            request_id: crate::types::RequestId,
            error: crate::error::JsonRpcErrorObject,
        },
        /// Notification frame (doesn't end stream)
        Notification {
            method: String,
            params: Option<Value>,
        },
    }

    impl JsonRpcFrame {
        /// Convert frame to JSON-RPC message format
        pub fn to_json(&self) -> Value {
            match self {
                JsonRpcFrame::Progress {
                    request_id,
                    progress,
                    progress_token,
                } => {
                    let mut obj = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request_id,
                        "_meta": {
                            "progress": progress
                        }
                    });

                    if let Some(token) = progress_token {
                        obj["_meta"]["progressToken"] = Value::String(token.clone());
                    }

                    obj
                }
                JsonRpcFrame::PartialResult { request_id, data } => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request_id,
                        "_meta": {
                            "partial": true
                        },
                        "result": data
                    })
                }
                JsonRpcFrame::FinalResult { request_id, result } => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request_id,
                        "result": result
                    })
                }
                JsonRpcFrame::Error { request_id, error } => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request_id,
                        "error": {
                            "code": error.code,
                            "message": &error.message,
                            "data": &error.data
                        }
                    })
                }
                JsonRpcFrame::Notification { method, params } => {
                    let mut obj = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": method
                    });

                    if let Some(params) = params {
                        obj["params"] = params.clone();
                    }

                    obj
                }
            }
        }

        /// Check if this frame ends the stream
        pub fn is_terminal(&self) -> bool {
            matches!(
                self,
                JsonRpcFrame::FinalResult { .. } | JsonRpcFrame::Error { .. }
            )
        }
    }

    /// Trait for handlers that support streaming responses
    #[async_trait]
    pub trait StreamingJsonRpcHandler: Send + Sync {
        /// The error type returned by this handler
        type Error: std::error::Error + Send + Sync + 'static;

        /// Handle a request with streaming response support
        /// Returns a stream of frames for progressive responses
        async fn handle_streaming(
            &self,
            method: &str,
            params: Option<crate::request::RequestParams>,
            session_context: Option<SessionContext>,
            request_id: crate::types::RequestId,
        ) -> Pin<Box<dyn Stream<Item = Result<JsonRpcFrame, Self::Error>> + Send>>;

        /// Handle a notification (non-streaming, same as regular handler)
        async fn handle_notification(
            &self,
            method: &str,
            params: Option<crate::request::RequestParams>,
            session_context: Option<SessionContext>,
        ) -> Result<(), Self::Error> {
            // Default implementation - ignore notifications
            let _ = (method, params, session_context);
            Ok(())
        }

        /// List supported methods (optional - used for introspection)
        fn supported_methods(&self) -> Vec<String> {
            vec![]
        }
    }

    /// Streaming JSON-RPC method dispatcher
    pub struct StreamingJsonRpcDispatcher<E>
    where
        E: ToJsonRpcError,
    {
        streaming_handlers: HashMap<String, Arc<dyn StreamingJsonRpcHandler<Error = E>>>,
        fallback_handlers: HashMap<String, Arc<dyn JsonRpcHandler<Error = E>>>,
        default_handler: Option<Arc<dyn JsonRpcHandler<Error = E>>>,
    }

    impl<E> StreamingJsonRpcDispatcher<E>
    where
        E: ToJsonRpcError,
    {
        pub fn new() -> Self {
            Self {
                streaming_handlers: HashMap::new(),
                fallback_handlers: HashMap::new(),
                default_handler: None,
            }
        }

        /// Register a streaming handler for a specific method
        pub fn register_streaming_method<H>(&mut self, method: String, handler: H)
        where
            H: StreamingJsonRpcHandler<Error = E> + 'static,
        {
            self.streaming_handlers.insert(method, Arc::new(handler));
        }

        /// Register a fallback (non-streaming) handler for a specific method
        pub fn register_fallback_method<H>(&mut self, method: String, handler: H)
        where
            H: JsonRpcHandler<Error = E> + 'static,
        {
            self.fallback_handlers.insert(method, Arc::new(handler));
        }

        /// Set a default handler for unregistered methods
        pub fn set_default_handler<H>(&mut self, handler: H)
        where
            H: JsonRpcHandler<Error = E> + 'static,
        {
            self.default_handler = Some(Arc::new(handler));
        }

        /// Process a JSON-RPC request with streaming support
        pub async fn handle_request_streaming(
            &self,
            request: crate::request::JsonRpcRequest,
            session_context: SessionContext,
        ) -> Pin<Box<dyn Stream<Item = JsonRpcFrame> + Send>> {
            // First try streaming handler
            if let Some(streaming_handler) = self.streaming_handlers.get(&request.method) {
                let request_id_clone = request.id.clone();
                let stream = streaming_handler
                    .handle_streaming(
                        &request.method,
                        request.params,
                        Some(session_context),
                        request.id.clone(),
                    )
                    .await;

                return Box::pin(stream.map(move |result| match result {
                    Ok(frame) => frame,
                    Err(domain_error) => JsonRpcFrame::Error {
                        request_id: request_id_clone.clone(),
                        error: domain_error.to_error_object(),
                    },
                }));
            }

            // Fall back to regular handler wrapped in streaming
            if let Some(fallback_handler) = self
                .fallback_handlers
                .get(&request.method)
                .or(self.default_handler.as_ref())
            {
                let method = request.method.clone();
                let params = request.params.clone();
                let request_id = request.id.clone();
                let handler = fallback_handler.clone();

                return Box::pin(futures::stream::once(async move {
                    match handler.handle(&method, params, Some(session_context)).await {
                        Ok(result) => JsonRpcFrame::FinalResult { request_id, result },
                        Err(domain_error) => JsonRpcFrame::Error {
                            request_id,
                            error: domain_error.to_error_object(),
                        },
                    }
                }));
            }

            // Method not found
            let error = crate::error::JsonRpcErrorObject {
                code: crate::error_codes::METHOD_NOT_FOUND,
                message: format!("Method '{}' not found", request.method),
                data: None,
            };

            Box::pin(futures::stream::once(async move {
                JsonRpcFrame::Error {
                    request_id: request.id,
                    error,
                }
            }))
        }

        /// Process a JSON-RPC notification
        pub async fn handle_notification(
            &self,
            notification: crate::notification::JsonRpcNotification,
        ) -> Result<(), E> {
            // Try streaming handler first
            if let Some(streaming_handler) = self.streaming_handlers.get(&notification.method) {
                return streaming_handler
                    .handle_notification(&notification.method, notification.params, None)
                    .await;
            }

            // Try fallback handler
            if let Some(fallback_handler) = self
                .fallback_handlers
                .get(&notification.method)
                .or(self.default_handler.as_ref())
            {
                return fallback_handler
                    .handle_notification(&notification.method, notification.params, None)
                    .await;
            }

            Ok(()) // Ignore unknown notifications
        }
    }

    impl<E> Default for StreamingJsonRpcDispatcher<E>
    where
        E: ToJsonRpcError,
    {
        fn default() -> Self {
            Self::new()
        }
    }
}
