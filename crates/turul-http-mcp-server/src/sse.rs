//! Server-Sent Events (SSE) support for MCP

use futures::stream;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio_stream::Stream;
use tracing::{debug, error};

/// SSE event types
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// Connection established
    Connected,
    /// Data event with JSON payload
    Data(Value),
    /// Error event
    Error(String),
    /// Keep-alive ping
    KeepAlive,
}

impl SseEvent {
    /// Format as SSE message
    pub fn format(&self) -> String {
        match self {
            SseEvent::Connected => {
                // Use "message" event type for MCP Inspector compatibility
                "event: message\ndata: {\"type\":\"connected\",\"message\":\"SSE connection established\"}\n\n".to_string()
            }
            SseEvent::Data(data) => {
                // Use "message" event type for MCP Inspector compatibility
                format!(
                    "event: message\ndata: {}\n\n",
                    serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string())
                )
            }
            SseEvent::Error(msg) => {
                // Use "message" event type for MCP Inspector compatibility
                format!(
                    "event: message\ndata: {{\"error\":\"{}\"}}\n\n",
                    msg.replace('"', "\\\"")
                )
            }
            SseEvent::KeepAlive => {
                // Keepalive events omit the event line (per SSE spec for compatibility)
                ": keepalive\n\n".to_string()
            }
        }
    }
}

/// SSE connection manager
pub struct SseManager {
    /// Broadcast channel for sending events to all connections
    sender: broadcast::Sender<SseEvent>,
    /// Connection registry
    connections: Arc<RwLock<HashMap<String, SseConnection>>>,
}

/// Individual SSE connection
#[derive(Debug)]
pub struct SseConnection {
    /// Connection ID
    pub id: String,
    /// Receiver for events
    pub receiver: broadcast::Receiver<SseEvent>,
}

impl SseManager {
    /// Create a new SSE manager
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new SSE connection
    pub async fn create_connection(&self, connection_id: String) -> SseConnection {
        let receiver = self.sender.subscribe();
        let connection = SseConnection {
            id: connection_id.clone(),
            receiver,
        };

        // Register the connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(
                connection_id,
                SseConnection {
                    id: connection.id.clone(),
                    receiver: self.sender.subscribe(),
                },
            );
        }

        debug!("SSE connection created: {}", connection.id);

        // Send connected event
        let _ = self.sender.send(SseEvent::Connected);

        connection
    }

    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
        debug!("SSE connection removed: {}", connection_id);
    }

    /// Broadcast an event to all connections
    pub async fn broadcast(&self, event: SseEvent) {
        if let Err(err) = self.sender.send(event) {
            error!("Failed to broadcast SSE event: {}", err);
        }
    }

    /// Send data to all connections
    pub async fn send_data(&self, data: Value) {
        self.broadcast(SseEvent::Data(data)).await;
    }

    /// Send error to all connections
    pub async fn send_error(&self, message: String) {
        self.broadcast(SseEvent::Error(message)).await;
    }

    /// Send keep-alive ping
    pub async fn send_keep_alive(&self) {
        self.broadcast(SseEvent::KeepAlive).await;
    }

    /// Get number of active connections
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
}

impl Default for SseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SseConnection {
    /// Convert to a stream of SSE-formatted strings
    pub fn into_stream(self) -> impl Stream<Item = Result<String, broadcast::error::RecvError>> {
        stream::unfold(self, |mut connection| async move {
            match connection.receiver.recv().await {
                Ok(event) => {
                    let formatted = event.format();
                    Some((Ok(formatted), connection))
                }
                Err(err) => Some((Err(err), connection)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sse_event_format() {
        // All events except keepalive use "event: message" for MCP Inspector compatibility
        let connected = SseEvent::Connected;
        assert!(connected.format().contains("event: message"));

        let data = SseEvent::Data(json!({"message": "test"}));
        assert!(data.format().contains("event: message"));

        let error = SseEvent::Error("test error".to_string());
        assert!(error.format().contains("event: message"));

        // Keepalive uses SSE comment syntax (no event line)
        let ping = SseEvent::KeepAlive;
        assert!(!ping.format().contains("event:"));
        assert!(ping.format().starts_with(":"));
    }

    #[tokio::test]
    async fn test_sse_manager() {
        let manager = SseManager::new();
        assert_eq!(manager.connection_count().await, 0);

        let _conn = manager.create_connection("test-123".to_string()).await;
        assert_eq!(manager.connection_count().await, 1);

        manager.remove_connection("test-123").await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let manager = SseManager::new();
        let mut conn = manager.create_connection("test-456".to_string()).await;

        // First event should be Connected
        if let Ok(event) = conn.receiver.recv().await {
            assert!(matches!(event, SseEvent::Connected));
        }

        // Send a test event
        manager.send_data(json!({"test": "message"})).await;

        // The connection should receive the data event
        if let Ok(event) = conn.receiver.recv().await {
            match event {
                SseEvent::Data(data) => {
                    assert_eq!(data["test"], "message");
                }
                _ => panic!("Expected data event"),
            }
        }
    }
}
