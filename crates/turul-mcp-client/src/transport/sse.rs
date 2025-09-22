//! SSE (Server-Sent Events) transport implementation for MCP client

use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::{Client, Response};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::error::{McpClientResult, TransportError};
use crate::transport::{
    ConnectionInfo, EventReceiver, ServerEvent, Transport, TransportCapabilities,
    TransportStatistics, TransportType,
};

/// SSE transport for MCP client (HTTP+SSE 2024-11-05)
#[derive(Debug)]
pub struct SseTransport {
    /// HTTP client
    client: Client,
    /// Server endpoint URL
    endpoint: Url,
    /// SSE endpoint URL
    sse_endpoint: Url,
    /// Connection state
    connected: AtomicBool,
    /// Request counter
    request_counter: AtomicU64,
    /// Statistics
    stats: Arc<parking_lot::Mutex<TransportStatistics>>,
    /// Event sender for server events
    event_sender: Option<mpsc::UnboundedSender<ServerEvent>>,
    /// SSE stream handle
    sse_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SseTransport {
    /// Create a new SSE transport
    pub fn new(endpoint: &str) -> McpClientResult<Self> {
        let url = Url::parse(endpoint)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

        // Validate URL scheme
        if !matches!(url.scheme(), "http" | "https") {
            return Err(TransportError::ConnectionFailed(format!(
                "Invalid scheme for SSE transport: {}",
                url.scheme()
            ))
            .into());
        }

        // Create SSE endpoint by adding /sse to the path
        let mut sse_url = url.clone();
        let sse_path = format!("{}/sse", url.path().trim_end_matches('/'));
        sse_url.set_path(&sse_path);

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("mcp-client/0.1.0")
            .build()
            .map_err(|e| TransportError::Sse(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            endpoint: url,
            sse_endpoint: sse_url,
            connected: AtomicBool::new(false),
            request_counter: AtomicU64::new(0),
            stats: Arc::new(parking_lot::Mutex::new(TransportStatistics::default())),
            event_sender: None,
            sse_handle: None,
        })
    }

    /// Create SSE transport with custom endpoints
    pub fn with_endpoints(endpoint: &str, sse_endpoint: &str) -> McpClientResult<Self> {
        let url = Url::parse(endpoint)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

        let sse_url = Url::parse(sse_endpoint)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid SSE URL: {}", e)))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("mcp-client/0.1.0")
            .build()
            .map_err(|e| TransportError::Sse(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            endpoint: url,
            sse_endpoint: sse_url,
            connected: AtomicBool::new(false),
            request_counter: AtomicU64::new(0),
            stats: Arc::new(parking_lot::Mutex::new(TransportStatistics::default())),
            event_sender: None,
            sse_handle: None,
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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            self.update_stats(|stats| {
                stats.errors += 1;
                stats.last_error = Some(format!("HTTP {}: {}", status, error_text));
            });

            return Err(
                TransportError::Sse(format!("HTTP error {}: {}", status, error_text)).into(),
            );
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| TransportError::Sse(format!("Failed to read response: {}", e)))?;

        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| TransportError::Sse(format!("Invalid JSON response: {}", e)))?;

        self.update_stats(|stats| stats.responses_received += 1);

        Ok(response_json)
    }

    /// Start SSE event stream
    async fn start_sse_stream(
        &mut self,
        sender: mpsc::UnboundedSender<ServerEvent>,
    ) -> McpClientResult<()> {
        debug!(sse_endpoint = %self.sse_endpoint, "Starting SSE stream");

        let client = self.client.clone();
        let sse_endpoint = self.sse_endpoint.clone();
        let stats = Arc::clone(&self.stats);

        let handle = tokio::spawn(async move {
            loop {
                debug!("Connecting to SSE endpoint");

                let response = match client
                    .get(sse_endpoint.clone())
                    .header("Accept", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("MCP-Protocol-Version", "2025-06-18")
                    .send()
                    .await
                {
                    Ok(response) => response,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to SSE endpoint");
                        sender
                            .send(ServerEvent::Error(format!("SSE connection failed: {}", e)))
                            .ok();
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                if !response.status().is_success() {
                    error!(status = %response.status(), "SSE endpoint returned error");
                    sender
                        .send(ServerEvent::Error(format!(
                            "SSE error: {}",
                            response.status()
                        )))
                        .ok();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                info!("SSE stream connected");

                let mut stream = response.bytes_stream();
                let mut buffer = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            // Process complete SSE events
                            while let Some(pos) = buffer.find("\n\n") {
                                let event_text = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                if let Some(event) = Self::parse_sse_event(&event_text) {
                                    stats.lock().events_received += 1;

                                    if sender.send(event).is_err() {
                                        debug!("Event receiver closed, stopping SSE stream");
                                        return;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "SSE stream error");
                            sender
                                .send(ServerEvent::Error(format!("SSE stream error: {}", e)))
                                .ok();
                            break;
                        }
                    }
                }

                warn!("SSE stream disconnected, reconnecting in 5 seconds");
                sender.send(ServerEvent::ConnectionLost).ok();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        self.sse_handle = Some(handle);
        Ok(())
    }

    /// Parse SSE event from text
    fn parse_sse_event(event_text: &str) -> Option<ServerEvent> {
        let mut event_type = None;
        let mut data = String::new();

        for line in event_text.lines() {
            if let Some(stripped) = line.strip_prefix("event: ") {
                event_type = Some(stripped.to_string());
            } else if let Some(stripped) = line.strip_prefix("data: ") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(stripped);
            }
        }

        if data.is_empty() {
            return None;
        }

        match event_type.as_deref() {
            Some("notification") => match serde_json::from_str::<Value>(&data) {
                Ok(json) => Some(ServerEvent::Notification(json)),
                Err(e) => {
                    warn!(error = %e, data = %data, "Failed to parse notification JSON");
                    None
                }
            },
            Some("request") => match serde_json::from_str::<Value>(&data) {
                Ok(json) => Some(ServerEvent::Request(json)),
                Err(e) => {
                    warn!(error = %e, data = %data, "Failed to parse request JSON");
                    None
                }
            },
            Some("heartbeat") | Some("ping") => Some(ServerEvent::Heartbeat),
            Some(event_name) => {
                debug!(event_type = event_name, data = %data, "Unknown SSE event type");
                None
            }
            None => {
                // Assume it's a notification if no event type is specified
                match serde_json::from_str::<Value>(&data) {
                    Ok(json) => Some(ServerEvent::Notification(json)),
                    Err(_) => None,
                }
            }
        }
    }
}

#[async_trait]
impl Transport for SseTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Sse
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            streaming: true,
            bidirectional: true,
            server_events: true,
            max_message_size: None,
            persistent: true,
        }
    }

    async fn connect(&mut self) -> McpClientResult<()> {
        debug!(
            endpoint = %self.endpoint,
            sse_endpoint = %self.sse_endpoint,
            "Connecting to SSE endpoints"
        );

        // Test main endpoint
        let response = self
            .client
            .request(reqwest::Method::OPTIONS, self.endpoint.clone())
            .send()
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Main endpoint test failed: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(TransportError::ConnectionFailed(format!(
                "Main endpoint returned status: {}",
                response.status()
            ))
            .into());
        }

        // Test SSE endpoint
        let sse_response = self
            .client
            .head(self.sse_endpoint.clone())
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("SSE endpoint test failed: {}", e))
            })?;

        if !sse_response.status().is_success() {
            return Err(TransportError::ConnectionFailed(format!(
                "SSE endpoint returned status: {}",
                sse_response.status()
            ))
            .into());
        }

        self.connected.store(true, Ordering::SeqCst);
        info!("SSE transport connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> McpClientResult<()> {
        debug!("Disconnecting SSE transport");
        self.connected.store(false, Ordering::SeqCst);

        // Stop SSE stream
        if let Some(handle) = self.sse_handle.take() {
            handle.abort();
        }

        // Close event sender
        if let Some(sender) = self.event_sender.take() {
            sender.send(ServerEvent::ConnectionLost).ok();
        }

        info!("SSE transport disconnected");
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
            "Sending SSE request"
        );

        self.update_stats(|stats| stats.requests_sent += 1);

        let response = self
            .client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
            .await
            .map_err(|e| TransportError::Sse(format!("Failed to send request: {}", e)))?;

        let result = self.handle_response(response).await?;

        let elapsed = start_time.elapsed();
        self.update_stats(|stats| {
            let new_avg = if stats.responses_received > 0 {
                (stats.avg_response_time_ms * (stats.responses_received - 1) as f64
                    + elapsed.as_millis() as f64)
                    / stats.responses_received as f64
            } else {
                elapsed.as_millis() as f64
            };
            stats.avg_response_time_ms = new_avg;
        });

        debug!(elapsed_ms = elapsed.as_millis(), "SSE request completed");

        Ok(result)
    }

    async fn send_request_with_headers(
        &mut self,
        request: Value,
    ) -> McpClientResult<crate::transport::TransportResponse> {
        // For SSE transport, we can delegate to the HTTP-style request but with header extraction
        // This is used primarily for the initial initialize request
        if !self.is_connected() {
            return Err(TransportError::ConnectionFailed("Not connected".to_string()).into());
        }

        debug!(
            method = request.get("method").and_then(|v| v.as_str()),
            "Sending SSE request with header extraction"
        );

        let response = self
            .client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to send request: {}", e)))?;

        // Extract headers
        let mut headers = std::collections::HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to read response: {}", e)))?;

        let result: Value = serde_json::from_str(&response_text)
            .map_err(|e| TransportError::Http(format!("Invalid JSON response: {}", e)))?;

        Ok(crate::transport::TransportResponse::new(result, headers))
    }

    async fn send_notification(&mut self, notification: Value) -> McpClientResult<()> {
        if !self.is_connected() {
            return Err(TransportError::ConnectionFailed("Not connected".to_string()).into());
        }

        debug!(
            method = notification.get("method").and_then(|v| v.as_str()),
            "Sending SSE notification"
        );

        self.update_stats(|stats| stats.notifications_sent += 1);

        let response = self
            .client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&notification)
            .send()
            .await
            .map_err(|e| TransportError::Sse(format!("Failed to send notification: {}", e)))?;

        if response.status().is_success() {
            debug!("SSE notification sent successfully");
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(TransportError::Sse(format!(
                "Notification failed with status {}: {}",
                status, error_text
            ))
            .into())
        }
    }

    async fn send_delete(&mut self, session_id: &str) -> McpClientResult<()> {
        if !self.is_connected() {
            return Err(TransportError::ConnectionFailed("Not connected".to_string()).into());
        }

        info!(
            endpoint = %self.endpoint,
            session_id = session_id,
            "Sending DELETE request for session termination (SSE transport)"
        );

        self.update_stats(|stats| stats.requests_sent += 1);
        let start_time = Instant::now();

        let response = self
            .client
            .delete(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .map_err(|e| TransportError::Sse(format!("Failed to send DELETE request: {}", e)))?;

        let elapsed = start_time.elapsed();
        self.update_stats(|stats| {
            stats.responses_received += 1;
            // Update average response time
            let elapsed_ms = elapsed.as_millis() as f64;
            if stats.responses_received == 1 {
                stats.avg_response_time_ms = elapsed_ms;
            } else {
                stats.avg_response_time_ms = (stats.avg_response_time_ms
                    * (stats.responses_received - 1) as f64
                    + elapsed_ms)
                    / stats.responses_received as f64;
            }
        });

        // DELETE should return 2xx success status
        if response.status().is_success() {
            info!(
                session_id = session_id,
                status = %response.status(),
                elapsed_ms = elapsed.as_millis(),
                "Session DELETE request completed successfully (SSE)"
            );
            Ok(())
        } else {
            warn!(
                session_id = session_id,
                status = %response.status(),
                elapsed_ms = elapsed.as_millis(),
                "DELETE request failed but continuing with cleanup (SSE)"
            );
            // Don't fail on DELETE errors - session cleanup should continue locally
            Ok(())
        }
    }

    async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver> {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Start SSE stream
        self.start_sse_stream(sender.clone()).await?;
        self.event_sender = Some(sender);

        info!("SSE event listener started");
        Ok(receiver)
    }

    fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            transport_type: self.transport_type(),
            endpoint: self.endpoint.to_string(),
            connected: self.is_connected(),
            capabilities: self.capabilities(),
            metadata: serde_json::json!({
                "main_endpoint": self.endpoint.to_string(),
                "sse_endpoint": self.sse_endpoint.to_string(),
                "scheme": self.endpoint.scheme(),
                "host": self.endpoint.host_str(),
                "port": self.endpoint.port()
            }),
        }
    }

    fn statistics(&self) -> TransportStatistics {
        self.stats.lock().clone()
    }
}

impl Drop for SseTransport {
    fn drop(&mut self) {
        debug!("🔥 DROP: SseTransport (CLIENT) - SSE transport being cleaned up");
        if let Some(handle) = self.sse_handle.take() {
            debug!("🔥 Aborting SSE handle - this will close the SSE connection");
            handle.abort();
        } else {
            debug!("🔥 No SSE handle to abort - connection may already be closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_transport_creation() {
        let transport = SseTransport::new("http://localhost:8080/mcp").unwrap();
        assert_eq!(transport.transport_type(), TransportType::Sse);
        assert!(!transport.is_connected());
        assert_eq!(
            transport.sse_endpoint.to_string(),
            "http://localhost:8080/mcp/sse"
        );
    }

    #[test]
    fn test_sse_event_parsing() {
        // Test notification event
        let event_text = "event: notification\ndata: {\"method\":\"test\",\"params\":{}}";
        let event = SseTransport::parse_sse_event(event_text).unwrap();
        let ServerEvent::Notification(json) = event else {
            panic!("Expected notification event, got: {:?}", event);
        };
        assert_eq!(json["method"], "test");

        // Test heartbeat event
        let heartbeat_text = "event: heartbeat\ndata: ping";
        let heartbeat = SseTransport::parse_sse_event(heartbeat_text).unwrap();
        assert!(matches!(heartbeat, ServerEvent::Heartbeat));

        // Test multiline data
        let multiline_text = "event: notification\ndata: {\ndata:   \"method\": \"test\"\ndata: }";
        let multiline_event = SseTransport::parse_sse_event(multiline_text).unwrap();
        let ServerEvent::Notification(json) = multiline_event else {
            panic!("Expected notification event, got: {:?}", multiline_event);
        };
        assert_eq!(json["method"], "test");
    }

    #[test]
    fn test_custom_endpoints() {
        let transport = SseTransport::with_endpoints(
            "http://localhost:8080/mcp",
            "http://localhost:8080/events",
        )
        .unwrap();

        assert_eq!(transport.endpoint.to_string(), "http://localhost:8080/mcp");
        assert_eq!(
            transport.sse_endpoint.to_string(),
            "http://localhost:8080/events"
        );
    }
}
