//! HTTP transport implementation for MCP client

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use async_trait::async_trait;
use reqwest::{Client, Response};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use url::Url;

use crate::error::{McpClientResult, TransportError};
use crate::transport::{
    Transport, TransportType, TransportCapabilities, ConnectionInfo, 
    EventReceiver, ServerEvent, TransportStatistics
};

/// HTTP transport for MCP client (Streamable HTTP 2025-03-26+)
#[derive(Debug)]
pub struct HttpTransport {
    /// HTTP client
    client: Client,
    /// Server endpoint URL
    endpoint: Url,
    /// Connection state
    connected: AtomicBool,
    /// Request counter
    request_counter: AtomicU64,
    /// Statistics
    stats: Arc<parking_lot::Mutex<TransportStatistics>>,
    /// Event sender for server events
    event_sender: Option<mpsc::UnboundedSender<ServerEvent>>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(endpoint: &str) -> McpClientResult<Self> {
        let url = Url::parse(endpoint)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;
        
        // Validate URL scheme
        if !matches!(url.scheme(), "http" | "https") {
            return Err(TransportError::ConnectionFailed(
                format!("Invalid scheme for HTTP transport: {}", url.scheme())
            ).into());
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("mcp-client/0.1.0")
            .build()
            .map_err(|e| TransportError::Http(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            client,
            endpoint: url,
            connected: AtomicBool::new(false),
            request_counter: AtomicU64::new(0),
            stats: Arc::new(parking_lot::Mutex::new(TransportStatistics::default())),
            event_sender: None,
        })
    }
    
    /// Create HTTP transport with custom client
    pub fn with_client(endpoint: &str, client: Client) -> McpClientResult<Self> {
        let url = Url::parse(endpoint)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;
        
        Ok(Self {
            client,
            endpoint: url,
            connected: AtomicBool::new(false),
            request_counter: AtomicU64::new(0),
            stats: Arc::new(parking_lot::Mutex::new(TransportStatistics::default())),
            event_sender: None,
        })
    }
    
    /// Generate unique request ID
    fn next_request_id(&self) -> String {
        let counter = self.request_counter.fetch_add(1, Ordering::SeqCst);
        format!("req_{}", counter)
    }
    
    /// Update statistics
    fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut TransportStatistics),
    {
        let mut stats = self.stats.lock();
        update_fn(&mut stats);
    }
    
    /// Handle HTTP response
    async fn handle_response(&self, response: Response) -> McpClientResult<Value> {
        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            self.update_stats(|stats| {
                stats.errors += 1;
                stats.last_error = Some(format!("HTTP {}: {}", status, error_text));
            });
            
            return Err(TransportError::Http(
                format!("HTTP error {}: {}", status, error_text)
            ).into());
        }
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        
        if !content_type.contains("application/json") {
            warn!(
                content_type = content_type,
                "Unexpected content type from server"
            );
        }
        
        let response_text = response.text().await
            .map_err(|e| TransportError::Http(format!("Failed to read response: {}", e)))?;
        
        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| TransportError::Http(format!("Invalid JSON response: {}", e)))?;
        
        self.update_stats(|stats| stats.responses_received += 1);
        
        Ok(response_json)
    }
}

#[async_trait]
impl Transport for HttpTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
    
    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            streaming: true,
            bidirectional: false,
            server_events: false,
            max_message_size: None,
            persistent: false,
        }
    }
    
    async fn connect(&mut self) -> McpClientResult<()> {
        debug!(endpoint = %self.endpoint, "Connecting to HTTP endpoint");
        
        // Test connection with a simple OPTIONS request
        let response = self.client
            .request(reqwest::Method::OPTIONS, self.endpoint.clone())
            .send()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Connection test failed: {}", e)))?;
        
        if response.status().is_success() {
            self.connected.store(true, Ordering::SeqCst);
            info!(endpoint = %self.endpoint, "HTTP transport connected");
            Ok(())
        } else {
            Err(TransportError::ConnectionFailed(
                format!("Server returned status: {}", response.status())
            ).into())
        }
    }
    
    async fn disconnect(&mut self) -> McpClientResult<()> {
        debug!("Disconnecting HTTP transport");
        self.connected.store(false, Ordering::SeqCst);
        
        // Close event sender if exists
        if let Some(sender) = self.event_sender.take() {
            sender.send(ServerEvent::ConnectionLost).ok();
        }
        
        info!("HTTP transport disconnected");
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
    
    async fn send_request(&mut self, request: Value) -> McpClientResult<Value> {
        if !self.is_connected() {
            return Err(TransportError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        let start_time = Instant::now();
        
        // Ensure request has an ID
        let mut request = request;
        if request.get("id").is_none() {
            request["id"] = Value::String(self.next_request_id());
        }
        
        debug!(
            method = request.get("method").and_then(|v| v.as_str()),
            id = request.get("id").and_then(|v| v.as_str()),
            "Sending HTTP request"
        );
        
        self.update_stats(|stats| stats.requests_sent += 1);
        
        let response = self.client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to send request: {}", e)))?;
        
        let result = self.handle_response(response).await?;
        
        let elapsed = start_time.elapsed();
        self.update_stats(|stats| {
            let new_avg = if stats.responses_received > 0 {
                (stats.avg_response_time_ms * (stats.responses_received - 1) as f64 + elapsed.as_millis() as f64) / stats.responses_received as f64
            } else {
                elapsed.as_millis() as f64
            };
            stats.avg_response_time_ms = new_avg;
        });
        
        debug!(
            elapsed_ms = elapsed.as_millis(),
            "HTTP request completed"
        );
        
        Ok(result)
    }
    
    async fn send_notification(&mut self, notification: Value) -> McpClientResult<()> {
        if !self.is_connected() {
            return Err(TransportError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        debug!(
            method = notification.get("method").and_then(|v| v.as_str()),
            "Sending HTTP notification"
        );
        
        self.update_stats(|stats| stats.notifications_sent += 1);
        
        let response = self.client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&notification)
            .send()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to send notification: {}", e)))?;
        
        // For notifications, we expect a 204 No Content or similar
        if response.status().is_success() {
            debug!("HTTP notification sent successfully");
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(TransportError::Http(
                format!("Notification failed with status {}: {}", status, error_text)
            ).into())
        }
    }
    
    async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver> {
        // HTTP transport doesn't support server-initiated events
        // Return a channel that will never receive messages
        let (sender, receiver) = mpsc::unbounded_channel();
        self.event_sender = Some(sender);
        
        warn!("HTTP transport does not support server events - event listener will be inactive");
        Ok(receiver)
    }
    
    fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            transport_type: self.transport_type(),
            endpoint: self.endpoint.to_string(),
            connected: self.is_connected(),
            capabilities: self.capabilities(),
            metadata: serde_json::json!({
                "scheme": self.endpoint.scheme(),
                "host": self.endpoint.host_str(),
                "port": self.endpoint.port(),
                "path": self.endpoint.path()
            }),
        }
    }
    
    async fn health_check(&mut self) -> McpClientResult<bool> {
        if !self.is_connected() {
            return Ok(false);
        }
        
        // Send a ping request
        let ping_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "health_check",
            "method": "ping",
            "params": {}
        });
        
        match self.send_request(ping_request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!(error = %e, "Health check failed");
                Ok(false)
            }
        }
    }
    
    fn statistics(&self) -> TransportStatistics {
        self.stats.lock().clone()
    }
}

// Use parking_lot for better performance
use parking_lot;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[test]
    fn test_http_transport_creation() {
        let transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();
        assert_eq!(transport.transport_type(), TransportType::Http);
        assert!(!transport.is_connected());
    }
    
    #[test]
    fn test_invalid_url() {
        let result = HttpTransport::new("invalid-url");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_scheme() {
        let result = HttpTransport::new("ws://localhost:8080/mcp");
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_connection_info() {
        let transport = HttpTransport::new("https://example.com:8443/mcp/endpoint").unwrap();
        let info = transport.connection_info();
        
        assert_eq!(info.transport_type, TransportType::Http);
        assert_eq!(info.endpoint, "https://example.com:8443/mcp/endpoint");
        assert!(!info.connected);
        
        let metadata = info.metadata.as_object().unwrap();
        assert_eq!(metadata["scheme"], "https");
        assert_eq!(metadata["host"], "example.com");
        assert_eq!(metadata["port"], 8443);
        assert_eq!(metadata["path"], "/mcp/endpoint");
    }
    
    #[test]
    fn test_request_id_generation() {
        let transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();
        
        let id1 = transport.next_request_id();
        let id2 = transport.next_request_id();
        
        assert_ne!(id1, id2);
        assert!(id1.starts_with("req_"));
        assert!(id2.starts_with("req_"));
    }
}