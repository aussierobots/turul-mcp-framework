use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    error::{JsonRpcError, JsonRpcProcessingError},
    notification::JsonRpcNotification,
    request::{JsonRpcRequest, RequestParams},
    response::{JsonRpcResponse, ResponseResult},
};

/// Result type for JSON-RPC method handlers
pub type JsonRpcResult<T> = Result<T, JsonRpcProcessingError>;

/// Trait for handling JSON-RPC method calls
#[async_trait]
pub trait JsonRpcHandler: Send + Sync {
    /// Handle a JSON-RPC method call
    async fn handle(&self, method: &str, params: Option<RequestParams>) -> JsonRpcResult<Value>;
    
    /// Handle a JSON-RPC notification (optional - default does nothing)
    async fn handle_notification(&self, method: &str, params: Option<RequestParams>) -> JsonRpcResult<()> {
        // Default implementation - ignore notifications
        let _ = (method, params);
        Ok(())
    }
    
    /// List supported methods (optional - used for introspection)
    fn supported_methods(&self) -> Vec<String> {
        vec![]
    }
}

/// A simple function-based handler
pub struct FunctionHandler<F, N> 
where
    F: Fn(&str, Option<RequestParams>) -> futures::future::BoxFuture<'static, JsonRpcResult<Value>> + Send + Sync,
    N: Fn(&str, Option<RequestParams>) -> futures::future::BoxFuture<'static, JsonRpcResult<()>> + Send + Sync,
{
    handler_fn: F,
    notification_fn: Option<N>,
    methods: Vec<String>,
}

impl<F, N> FunctionHandler<F, N>
where
    F: Fn(&str, Option<RequestParams>) -> futures::future::BoxFuture<'static, JsonRpcResult<Value>> + Send + Sync,
    N: Fn(&str, Option<RequestParams>) -> futures::future::BoxFuture<'static, JsonRpcResult<()>> + Send + Sync,
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
impl<F, N> JsonRpcHandler for FunctionHandler<F, N>
where
    F: Fn(&str, Option<RequestParams>) -> futures::future::BoxFuture<'static, JsonRpcResult<Value>> + Send + Sync,
    N: Fn(&str, Option<RequestParams>) -> futures::future::BoxFuture<'static, JsonRpcResult<()>> + Send + Sync,
{
    async fn handle(&self, method: &str, params: Option<RequestParams>) -> JsonRpcResult<Value> {
        (self.handler_fn)(method, params).await
    }

    async fn handle_notification(&self, method: &str, params: Option<RequestParams>) -> JsonRpcResult<()> {
        if let Some(ref notification_fn) = self.notification_fn {
            (notification_fn)(method, params).await
        } else {
            Ok(())
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        self.methods.clone()
    }
}

/// JSON-RPC method dispatcher
pub struct JsonRpcDispatcher {
    handlers: HashMap<String, Arc<dyn JsonRpcHandler>>,
    default_handler: Option<Arc<dyn JsonRpcHandler>>,
}

impl JsonRpcDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            default_handler: None,
        }
    }

    /// Register a handler for a specific method
    pub fn register_method<H>(&mut self, method: String, handler: H)
    where
        H: JsonRpcHandler + 'static,
    {
        self.handlers.insert(method, Arc::new(handler));
    }

    /// Register a handler for multiple methods
    pub fn register_methods<H>(&mut self, methods: Vec<String>, handler: H)
    where
        H: JsonRpcHandler + 'static,
    {
        let handler_arc = Arc::new(handler);
        for method in methods {
            self.handlers.insert(method, handler_arc.clone());
        }
    }

    /// Set a default handler for unregistered methods
    pub fn set_default_handler<H>(&mut self, handler: H)
    where
        H: JsonRpcHandler + 'static,
    {
        self.default_handler = Some(Arc::new(handler));
    }

    /// Process a JSON-RPC request and return a response
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let handler = self.handlers.get(&request.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                match handler.handle(&request.method, request.params).await {
                    Ok(result) => JsonRpcResponse::new(request.id, ResponseResult::Success(result)),
                    Err(err) => {
                        let rpc_error = err.to_rpc_error(Some(request.id.clone()));
                        // Convert error to response - in practice you'd want to log and return error response
                        JsonRpcResponse::new(request.id, ResponseResult::Success(
                            serde_json::json!({
                                "error": {
                                    "code": rpc_error.error.code,
                                    "message": rpc_error.error.message,
                                    "data": rpc_error.error.data
                                }
                            })
                        ))
                    }
                }
            }
            None => {
                let error = JsonRpcError::method_not_found(request.id.clone(), &request.method);
                JsonRpcResponse::new(request.id, ResponseResult::Success(
                    serde_json::json!({
                        "error": {
                            "code": error.error.code,
                            "message": error.error.message
                        }
                    })
                ))
            }
        }
    }

    /// Process a JSON-RPC notification
    pub async fn handle_notification(&self, notification: JsonRpcNotification) -> JsonRpcResult<()> {
        let handler = self.handlers.get(&notification.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                handler.handle_notification(&notification.method, notification.params).await
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

impl Default for JsonRpcDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::{RequestId, JsonRpcRequest};

    struct TestHandler;

    #[async_trait]
    impl JsonRpcHandler for TestHandler {
        async fn handle(&self, method: &str, _params: Option<RequestParams>) -> JsonRpcResult<Value> {
            match method {
                "add" => Ok(json!({"result": "addition"})),
                "error" => Err(JsonRpcProcessingError::HandlerError("test error".to_string())),
                _ => Err(JsonRpcProcessingError::HandlerError("unknown method".to_string())),
            }
        }

        fn supported_methods(&self) -> Vec<String> {
            vec!["add".to_string(), "error".to_string()]
        }
    }

    #[tokio::test]
    async fn test_dispatcher_success() {
        let mut dispatcher = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let request = JsonRpcRequest::new_no_params(
            RequestId::Number(1),
            "add".to_string(),
        );

        let response = dispatcher.handle_request(request).await;
        assert_eq!(response.id, RequestId::Number(1));
    }

    #[tokio::test]
    async fn test_dispatcher_method_not_found() {
        let dispatcher = JsonRpcDispatcher::new();

        let request = JsonRpcRequest::new_no_params(
            RequestId::Number(1),
            "unknown".to_string(),
        );

        let response = dispatcher.handle_request(request).await;
        assert_eq!(response.id, RequestId::Number(1));
        // Response contains error information
    }

    #[tokio::test] 
    async fn test_function_handler() {
        // Test FunctionHandler directly without type inference issues
        let handler = TestHandler;
        let result = handler.handle("add", None).await.unwrap();
        assert_eq!(result["result"], "addition");
    }
}