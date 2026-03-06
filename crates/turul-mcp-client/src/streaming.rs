//! Streaming support for MCP client

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::error::{McpClientError, McpClientResult};
use crate::transport::ServerEvent;

/// Stream handler for processing server events
#[derive(Debug)]
pub struct StreamHandler {
    /// Event receiver from transport
    event_receiver: Option<mpsc::UnboundedReceiver<ServerEvent>>,
    /// Event callbacks
    callbacks: Arc<parking_lot::Mutex<StreamCallbacks>>,
    /// Channel for sending JSON-RPC responses back to the server
    response_sender: Option<mpsc::UnboundedSender<Value>>,
}

/// Type alias for request handler callback
type RequestHandler = Box<dyn Fn(Value) -> Result<Value, String> + Send + Sync>;

/// Callbacks for different types of server events
#[derive(Default)]
pub struct StreamCallbacks {
    /// Notification callback
    pub notification: Option<Box<dyn Fn(Value) + Send + Sync>>,
    /// Request callback (server asking client)
    pub request: Option<RequestHandler>,
    /// Connection lost callback
    pub connection_lost: Option<Box<dyn Fn() + Send + Sync>>,
    /// Error callback
    pub error: Option<Box<dyn Fn(String) + Send + Sync>>,
    /// Heartbeat callback
    pub heartbeat: Option<Box<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for StreamCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamCallbacks")
            .field(
                "notification",
                &self.notification.as_ref().map(|_| "function"),
            )
            .field("request", &self.request.as_ref().map(|_| "function"))
            .field(
                "connection_lost",
                &self.connection_lost.as_ref().map(|_| "function"),
            )
            .field("error", &self.error.as_ref().map(|_| "function"))
            .field("heartbeat", &self.heartbeat.as_ref().map(|_| "function"))
            .finish()
    }
}

impl StreamHandler {
    /// Create a new stream handler
    pub fn new() -> Self {
        Self {
            event_receiver: None,
            callbacks: Arc::new(parking_lot::Mutex::new(StreamCallbacks::default())),
            response_sender: None,
        }
    }

    /// Set event receiver from transport
    pub fn set_receiver(&mut self, receiver: mpsc::UnboundedReceiver<ServerEvent>) {
        self.event_receiver = Some(receiver);
    }

    /// Set channel for sending JSON-RPC responses back to the server
    pub fn set_response_sender(&mut self, sender: mpsc::UnboundedSender<Value>) {
        self.response_sender = Some(sender);
    }

    /// Set notification callback
    pub fn on_notification<F>(&self, callback: F)
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        self.callbacks.lock().notification = Some(Box::new(callback));
    }

    /// Set request callback
    pub fn on_request<F>(&self, callback: F)
    where
        F: Fn(Value) -> Result<Value, String> + Send + Sync + 'static,
    {
        self.callbacks.lock().request = Some(Box::new(callback));
    }

    /// Set connection lost callback
    pub fn on_connection_lost<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.lock().connection_lost = Some(Box::new(callback));
    }

    /// Set error callback
    pub fn on_error<F>(&self, callback: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.callbacks.lock().error = Some(Box::new(callback));
    }

    /// Set heartbeat callback
    pub fn on_heartbeat<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.lock().heartbeat = Some(Box::new(callback));
    }

    /// Start processing events
    pub async fn start(&mut self) -> McpClientResult<()> {
        let mut receiver = self
            .event_receiver
            .take()
            .ok_or_else(|| McpClientError::generic("No event receiver configured"))?;

        let callbacks = Arc::clone(&self.callbacks);
        let response_sender = self.response_sender.clone();

        tokio::spawn(async move {
            info!("Stream handler started");

            while let Some(event) = receiver.recv().await {
                debug!(event = ?event, "Received server event");

                let callbacks = callbacks.lock();

                match event {
                    ServerEvent::Notification(notification) => {
                        if let Some(ref callback) = callbacks.notification {
                            callback(notification);
                        }
                    }
                    ServerEvent::Request(request) => {
                        // Per JSON-RPC 2.0, only requests with a non-null id
                        // expect a response. Messages without id are notifications
                        // and MUST NOT receive a reply.
                        let request_id = request.get("id").filter(|id| !id.is_null()).cloned();

                        if request_id.is_none() {
                            warn!(
                                "Received server request without valid id, treating as notification"
                            );
                            if let Some(ref callback) = callbacks.request {
                                let _ = callback(request);
                            }
                            continue;
                        }

                        if let Some(ref callback) = callbacks.request {
                            let response_json = match callback(request) {
                                Ok(result) => {
                                    debug!("Request handled successfully");
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": request_id,
                                        "result": result
                                    })
                                }
                                Err(error) => {
                                    warn!(error = %error, "Request handler returned error");
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": request_id,
                                        "error": {
                                            "code": -32603,
                                            "message": error
                                        }
                                    })
                                }
                            };

                            if let Some(ref sender) = response_sender {
                                if let Err(e) = sender.send(response_json) {
                                    warn!("Failed to send response via channel: {}", e);
                                }
                            } else {
                                warn!("No response sender configured, response discarded");
                            }
                        } else {
                            warn!("Received server request but no request handler configured");
                            if let Some(ref sender) = response_sender {
                                let error_json = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": request_id,
                                    "error": {
                                        "code": -32601,
                                        "message": "Method not found: no request handler configured"
                                    }
                                });
                                if let Err(e) = sender.send(error_json) {
                                    warn!("Failed to send error response via channel: {}", e);
                                }
                            }
                        }
                    }
                    ServerEvent::Response(_) => {
                        // Response to a client-originated request received via SSE.
                        // Handled by the normal request/response matching path,
                        // not by the stream handler callback.
                        debug!("Received async response via event stream");
                    }
                    ServerEvent::ConnectionLost => {
                        warn!("Connection lost");
                        if let Some(ref callback) = callbacks.connection_lost {
                            callback();
                        }
                    }
                    ServerEvent::Error(error) => {
                        warn!(error = %error, "Server error");
                        if let Some(ref callback) = callbacks.error {
                            callback(error);
                        }
                    }
                    ServerEvent::Heartbeat => {
                        debug!("Heartbeat received");
                        if let Some(ref callback) = callbacks.heartbeat {
                            callback();
                        }
                    }
                }
            }

            info!("Stream handler stopped");
        });

        Ok(())
    }

    /// Check if handler is active
    pub fn is_active(&self) -> bool {
        self.event_receiver.is_some()
    }
}

impl Default for StreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress tracker for long-running operations
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    /// Operation ID
    pub operation_id: String,
    /// Total steps (if known)
    pub total: Option<u64>,
    /// Completed steps
    pub completed: u64,
    /// Progress message
    pub message: Option<String>,
    /// Progress metadata
    pub metadata: Value,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(operation_id: String) -> Self {
        Self {
            operation_id,
            total: None,
            completed: 0,
            message: None,
            metadata: Value::Null,
        }
    }

    /// Update progress
    pub fn update(&mut self, completed: u64, message: Option<String>) {
        self.completed = completed;
        self.message = message;
    }

    /// Set total steps
    pub fn set_total(&mut self, total: u64) {
        self.total = Some(total);
    }

    /// Get progress percentage (0.0 to 1.0)
    pub fn percentage(&self) -> Option<f64> {
        self.total.map(|total| {
            if total == 0 {
                1.0
            } else {
                (self.completed as f64) / (total as f64)
            }
        })
    }

    /// Check if operation is complete
    pub fn is_complete(&self) -> bool {
        if let Some(total) = self.total {
            self.completed >= total
        } else {
            false
        }
    }

    /// Get status summary
    pub fn status(&self) -> String {
        match (self.total, &self.message) {
            (Some(total), Some(msg)) => {
                format!(
                    "{}/{} ({}%) - {}",
                    self.completed,
                    total,
                    (self.percentage().unwrap_or(0.0) * 100.0) as u32,
                    msg
                )
            }
            (Some(total), None) => {
                format!(
                    "{}/{} ({}%)",
                    self.completed,
                    total,
                    (self.percentage().unwrap_or(0.0) * 100.0) as u32
                )
            }
            (None, Some(msg)) => {
                format!("{} steps - {}", self.completed, msg)
            }
            (None, None) => {
                format!("{} steps", self.completed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker() {
        let mut tracker = ProgressTracker::new("test-op".to_string());

        assert_eq!(tracker.completed, 0);
        assert_eq!(tracker.percentage(), None);

        tracker.set_total(100);
        assert_eq!(tracker.percentage(), Some(0.0));

        tracker.update(50, Some("halfway".to_string()));
        assert_eq!(tracker.percentage(), Some(0.5));
        assert_eq!(tracker.message, Some("halfway".to_string()));

        tracker.update(100, Some("complete".to_string()));
        assert_eq!(tracker.percentage(), Some(1.0));
        assert!(tracker.is_complete());
    }

    #[tokio::test]
    async fn test_stream_handler_callbacks() {
        let handler = StreamHandler::new();

        let notification_received = Arc::new(parking_lot::Mutex::new(false));
        let notification_received_clone = Arc::clone(&notification_received);

        handler.on_notification(move |_| {
            *notification_received_clone.lock() = true;
        });

        // Test that callback is registered
        assert!(handler.callbacks.lock().notification.is_some());
    }

    #[tokio::test]
    async fn test_stream_handler_sends_success_response() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let mut handler = StreamHandler::new();
        handler.set_receiver(event_rx);
        handler.set_response_sender(response_tx);

        handler.on_request(|_req| Ok(serde_json::json!({"status": "ok"})));

        handler.start().await.unwrap();

        // Send a server-initiated request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "srv-1",
            "method": "sampling/createMessage",
            "params": {}
        });
        event_tx.send(ServerEvent::Request(request)).unwrap();

        // Verify response format
        let response = tokio::time::timeout(std::time::Duration::from_secs(1), response_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], "srv-1");
        assert_eq!(response["result"]["status"], "ok");
        assert!(response.get("error").is_none());
    }

    #[tokio::test]
    async fn test_stream_handler_sends_error_response() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let mut handler = StreamHandler::new();
        handler.set_receiver(event_rx);
        handler.set_response_sender(response_tx);

        handler.on_request(|_req| Err("something went wrong".to_string()));

        handler.start().await.unwrap();

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "sampling/createMessage",
            "params": {}
        });
        event_tx.send(ServerEvent::Request(request)).unwrap();

        let response = tokio::time::timeout(std::time::Duration::from_secs(1), response_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 42);
        assert_eq!(response["error"]["code"], -32603);
        assert_eq!(response["error"]["message"], "something went wrong");
    }

    #[tokio::test]
    async fn test_stream_handler_no_callback_sends_method_not_found() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let mut handler = StreamHandler::new();
        handler.set_receiver(event_rx);
        handler.set_response_sender(response_tx);
        // No on_request callback registered

        handler.start().await.unwrap();

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "req-99",
            "method": "unknown/method",
            "params": {}
        });
        event_tx.send(ServerEvent::Request(request)).unwrap();

        let response = tokio::time::timeout(std::time::Duration::from_secs(1), response_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], "req-99");
        assert_eq!(response["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_stream_handler_no_id_skips_response() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let mut handler = StreamHandler::new();
        handler.set_receiver(event_rx);
        handler.set_response_sender(response_tx);

        let callback_called = Arc::new(parking_lot::Mutex::new(false));
        let callback_called_clone = Arc::clone(&callback_called);
        handler.on_request(move |_req| {
            *callback_called_clone.lock() = true;
            Ok(serde_json::json!({"handled": true}))
        });

        handler.start().await.unwrap();

        // Request without id — should invoke callback but NOT send response
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sampling/createMessage",
            "params": {}
        });
        event_tx.send(ServerEvent::Request(request)).unwrap();

        // Give handler time to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Callback should have been called
        assert!(
            *callback_called.lock(),
            "callback should be invoked even without id"
        );

        // But no response should be emitted
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(100), response_rx.recv()).await;
        assert!(
            result.is_err(),
            "no response should be sent for request without id"
        );
    }

    #[tokio::test]
    async fn test_stream_handler_null_id_skips_response() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let mut handler = StreamHandler::new();
        handler.set_receiver(event_rx);
        handler.set_response_sender(response_tx);

        handler.on_request(|_req| Ok(serde_json::json!({"handled": true})));

        handler.start().await.unwrap();

        // Request with explicit null id — also should NOT send response
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": null,
            "method": "sampling/createMessage",
            "params": {}
        });
        event_tx.send(ServerEvent::Request(request)).unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let result =
            tokio::time::timeout(std::time::Duration::from_millis(100), response_rx.recv()).await;
        assert!(
            result.is_err(),
            "no response should be sent for request with null id"
        );
    }
}
