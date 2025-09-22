use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

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
