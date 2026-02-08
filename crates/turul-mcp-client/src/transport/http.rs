//! HTTP transport implementation for MCP client

use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, Response};
use serde_json::{Deserializer, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use url::Url;

use crate::error::{McpClientResult, TransportError};
use crate::transport::{
    ConnectionInfo, EventReceiver, ServerEvent, Transport, TransportCapabilities,
    TransportStatistics, TransportType,
};

/// HTTP transport for MCP client (Streamable HTTP)
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
    /// Queue for progress notifications when no listener exists
    queued_events: Arc<parking_lot::Mutex<Vec<ServerEvent>>>,
    /// Session ID from server (set after initialization) - shared between transport and SSE task
    session_id: Arc<parking_lot::Mutex<Option<String>>>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(endpoint: &str) -> McpClientResult<Self> {
        let url = Url::parse(endpoint)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

        // Validate URL scheme
        if !matches!(url.scheme(), "http" | "https") {
            return Err(TransportError::ConnectionFailed(format!(
                "Invalid scheme for HTTP transport: {}",
                url.scheme()
            ))
            .into());
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
            queued_events: Arc::new(parking_lot::Mutex::new(Vec::new())),
            session_id: Arc::new(parking_lot::Mutex::new(None)),
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
            queued_events: Arc::new(parking_lot::Mutex::new(Vec::new())),
            session_id: Arc::new(parking_lot::Mutex::new(None)),
        })
    }

    /// Set the session ID to use for subsequent requests
    pub fn set_session_id(&mut self, session_id: String) {
        debug!("Setting session ID: {}", session_id);
        *self.session_id.lock() = Some(session_id);
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

    /// Test helper: Process an in-memory byte stream (for testing without network)
    #[doc(hidden)]
    pub async fn test_handle_byte_stream<S, B, E>(&mut self, stream: S) -> McpClientResult<Value>
    where
        S: Stream<Item = Result<B, E>> + Unpin,
        B: AsRef<[u8]>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.handle_byte_stream(stream).await
    }

    /// Handle HTTP response
    async fn handle_response(&mut self, response: Response) -> McpClientResult<Value> {
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
                TransportError::Http(format!("HTTP error {}: {}", status, error_text)).into(),
            );
        }

        // Capture session ID from response headers if present
        if let Some(session_header) = response.headers().get("mcp-session-id")
            && let Ok(session_str) = session_header.to_str()
        {
            debug!("Captured session ID from response: {}", session_str);
            *self.session_id.lock() = Some(session_str.to_owned());
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Always use streaming approach for JSON responses (handles both single and chunked)
        if content_type.contains("application/json") {
            self.handle_json_stream(response).await
        } else if content_type.contains("text/event-stream") {
            // Should not happen in send_request, but handle gracefully
            Err(TransportError::Http("Unexpected SSE response in send_request".to_string()).into())
        } else {
            Err(TransportError::Http(format!("Unsupported content type: {}", content_type)).into())
        }
    }

    /// Handle JSON stream (works for both single responses and chunked streaming)
    async fn handle_json_stream(&mut self, response: Response) -> McpClientResult<Value> {
        use futures::StreamExt;

        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other));

        self.handle_byte_stream(stream).await
    }

    /// Handle a generic byte stream (for both HTTP responses and in-memory testing)
    async fn handle_byte_stream<S, B, E>(&mut self, mut stream: S) -> McpClientResult<Value>
    where
        S: Stream<Item = Result<B, E>> + Unpin,
        B: AsRef<[u8]>,
        E: std::error::Error + Send + Sync + 'static,
    {
        use futures::StreamExt;

        // Ensure event channel exists (lazy creation)
        if self.event_sender.is_none() {
            let (tx, _rx) = mpsc::unbounded_channel();
            self.event_sender = Some(tx);
        }

        let mut buffer = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| TransportError::Http(format!("Stream error: {}", e)))?;
            buffer.extend_from_slice(chunk.as_ref());

            // Use proper incremental JSON parsing that tracks exact bytes consumed
            let mut processed_bytes = 0;
            loop {
                if processed_bytes >= buffer.len() {
                    break;
                }

                // Try to parse a single JSON value from the remaining buffer
                let remaining_buffer = &buffer[processed_bytes..];
                let mut stream = Deserializer::from_slice(remaining_buffer).into_iter::<Value>();

                match stream.next() {
                    Some(Ok(json)) => {
                        // Calculate how many bytes were consumed by this JSON object
                        let consumed_bytes = stream.byte_offset();

                        debug!(
                            "Parsed JSON frame ({} bytes): {}",
                            consumed_bytes,
                            serde_json::to_string(&json).unwrap_or_default()
                        );

                        // Check if this is a final frame (has id + result/error)
                        if let Some(_id) = json.get("id") {
                            if json.get("result").is_some() {
                                self.update_stats(|stats| stats.responses_received += 1);
                                return Ok(json); // Success response
                            } else if let Some(error) = json.get("error") {
                                self.update_stats(|stats| {
                                    stats.errors += 1;
                                    stats.last_error = Some(format!("JSON-RPC error: {}", error));
                                });
                                return Err(TransportError::Http(format!(
                                    "Server error: {}",
                                    error
                                ))
                                .into());
                            }
                        }

                        // Progress notification - send via event channel or queue for later
                        if json.get("method").is_some() {
                            let event = ServerEvent::Notification(json.clone());
                            if let Some(sender) = &self.event_sender {
                                if sender.send(event.clone()).is_err() {
                                    // Channel closed, queue the event for future listeners
                                    debug!("Event channel closed, queuing notification");
                                    self.queued_events.lock().push(event);
                                } else {
                                    debug!(
                                        "Forwarded progress notification via active event channel"
                                    );
                                }
                            } else {
                                // No listener yet, queue the event
                                debug!("No event listener active, queuing progress notification");
                                self.queued_events.lock().push(event);
                            }
                        }

                        // Update processed bytes counter
                        processed_bytes += consumed_bytes;
                    }
                    Some(Err(e)) => {
                        // If we can't parse a complete JSON object, we have an incomplete frame
                        // Wait for more data - the byte_offset approach handles partial JSON correctly
                        debug!(
                            "Incomplete JSON in buffer, waiting for more data. Parse error: {}",
                            e
                        );
                        break;
                    }
                    None => {
                        // No more complete JSON objects to parse, wait for more data
                        break;
                    }
                }
            }

            // Remove processed data from buffer
            if processed_bytes > 0 {
                buffer.drain(..processed_bytes);
            }
        }

        // Stream ended - check if we have a complete response in buffer
        if !buffer.is_empty()
            && let Ok(json) = serde_json::from_slice::<Value>(&buffer)
            && json.get("id").is_some()
            && (json.get("result").is_some() || json.get("error").is_some())
        {
            self.update_stats(|stats| stats.responses_received += 1);
            return Ok(json);
        }

        // If we get here, no final frame was found
        Err(TransportError::Http("Stream ended without final result".to_string()).into())
    }

    /// Handle HTTP response with headers
    async fn handle_response_with_headers(
        &mut self,
        response: Response,
    ) -> McpClientResult<crate::transport::TransportResponse> {
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
                TransportError::Http(format!("HTTP error {}: {}", status, error_text)).into(),
            );
        }

        // Capture session ID from response headers if present
        if let Some(session_header) = response.headers().get("mcp-session-id")
            && let Ok(session_str) = session_header.to_str()
        {
            debug!("Captured session ID from response headers: {}", session_str);
            *self.session_id.lock() = Some(session_str.to_owned());
        }

        // Extract headers
        let mut headers = std::collections::HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Use streaming approach for JSON responses (handles both single and chunked)
        let response_json = if content_type.contains("application/json") {
            self.handle_json_stream(response).await?
        } else if content_type.contains("text/event-stream") {
            // Should not happen in send_request_with_headers, but handle gracefully
            return Err(TransportError::Http(
                "Unexpected SSE response in send_request_with_headers".to_string(),
            )
            .into());
        } else {
            return Err(TransportError::Http(format!(
                "Unsupported content type: {}",
                content_type
            ))
            .into());
        };

        Ok(crate::transport::TransportResponse::new(
            response_json,
            headers,
        ))
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
        let response = self
            .client
            .request(reqwest::Method::OPTIONS, self.endpoint.clone())
            .send()
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Connection test failed: {}", e))
            })?;

        if response.status().is_success() {
            self.connected.store(true, Ordering::SeqCst);
            info!(endpoint = %self.endpoint, "HTTP transport connected");
            Ok(())
        } else {
            Err(TransportError::ConnectionFailed(format!(
                "Server returned status: {}",
                response.status()
            ))
            .into())
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

        let mut req_builder = self
            .client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25");

        // Include session ID if we have one
        if let Some(ref session_id) = *self.session_id.lock() {
            debug!("HTTP request using session ID: {}", session_id);
            req_builder = req_builder.header("Mcp-Session-Id", session_id);
        } else {
            warn!(
                "HTTP request attempted without session ID - server may reject for non-initialize methods"
            );
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to send request: {}", e)))?;

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

        debug!(elapsed_ms = elapsed.as_millis(), "HTTP request completed");

        Ok(result)
    }

    async fn send_request_with_headers(
        &mut self,
        request: Value,
    ) -> McpClientResult<crate::transport::TransportResponse> {
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
            "Sending HTTP request with header extraction"
        );

        self.update_stats(|stats| stats.requests_sent += 1);

        let mut req_builder = self
            .client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25");

        // Include session ID if we have one
        if let Some(ref session_id) = *self.session_id.lock() {
            debug!("HTTP request with headers using session ID: {}", session_id);
            req_builder = req_builder.header("Mcp-Session-Id", session_id);
        } else {
            warn!("HTTP request with headers attempted without session ID - server may reject");
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to send request: {}", e)))?;

        let result = self.handle_response_with_headers(response).await?;

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

        debug!(
            elapsed_ms = elapsed.as_millis(),
            "HTTP request with headers completed"
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

        let mut req_builder = self
            .client
            .post(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25");

        // Include session ID if we have one
        if let Some(ref session_id) = *self.session_id.lock() {
            debug!("HTTP notification using session ID: {}", session_id);
            req_builder = req_builder.header("Mcp-Session-Id", session_id);
        } else {
            warn!("HTTP notification attempted without session ID - server may reject");
        }

        let response = req_builder
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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(TransportError::Http(format!(
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
            "Sending DELETE request for session termination"
        );

        self.update_stats(|stats| stats.requests_sent += 1);
        let start_time = Instant::now();

        let response = self
            .client
            .delete(self.endpoint.clone())
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .map_err(|e| TransportError::Http(format!("Failed to send DELETE request: {}", e)))?;

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
                "Session DELETE request completed successfully"
            );
            Ok(())
        } else {
            warn!(
                session_id = session_id,
                status = %response.status(),
                elapsed_ms = elapsed.as_millis(),
                "DELETE request failed but continuing with cleanup"
            );
            // Don't fail on DELETE errors - session cleanup should continue locally
            Ok(())
        }
    }

    async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver> {
        use futures::StreamExt;

        let (tx, rx) = mpsc::unbounded_channel();
        self.event_sender = Some(tx.clone());

        // Replay any queued events to the new listener
        let queued_events = {
            let mut queue = self.queued_events.lock();
            let events = queue.clone();
            queue.clear(); // Clear the queue after replaying
            events
        };

        for event in &queued_events {
            if tx.send(event.clone()).is_err() {
                warn!("Failed to replay queued event - channel already closed");
                break;
            }
        }

        if !queued_events.is_empty() {
            debug!(
                "Replayed {} queued events to new listener",
                queued_events.len()
            );
        }

        if !self.is_connected() {
            warn!("Not connected - event listener will be inactive");
            return Ok(rx);
        }

        // Start SSE connection task for GET requests
        let client = self.client.clone();
        let url = self.endpoint.clone();
        let session_id = self.session_id.clone();

        info!("Starting SSE event listener for GET requests at: {}", url);

        tokio::spawn(async move {
            loop {
                // GET request with SSE accept header
                let mut request_builder = client
                    .get(url.as_str())
                    .header("Accept", "text/event-stream")
                    .header("MCP-Protocol-Version", "2025-11-25");

                // Include session ID if available (read fresh value each time)
                if let Some(ref current_session_id) = *session_id.lock() {
                    debug!("SSE request using session ID: {}", current_session_id);
                    request_builder = request_builder.header("Mcp-Session-Id", current_session_id);
                } else {
                    warn!("SSE request attempted without session ID - server may reject");
                }

                let response = match request_builder.send().await {
                    Ok(resp) if resp.status().is_success() => {
                        debug!("SSE connection established, status: {}", resp.status());

                        let content_type = resp
                            .headers()
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");

                        if !content_type.contains("text/event-stream") {
                            warn!(
                                "Server returned content-type '{}' instead of 'text/event-stream'",
                                content_type
                            );
                        }

                        resp
                    }
                    Ok(resp) => {
                        warn!("SSE connection failed with status: {}", resp.status());
                        if tx
                            .send(ServerEvent::Error(format!("HTTP {}", resp.status())))
                            .is_err()
                        {
                            debug!("Event channel closed, stopping SSE listener");
                            return;
                        }
                        // Wait before retrying
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    Err(e) => {
                        warn!("SSE connection error: {}", e);
                        if tx.send(ServerEvent::Error(e.to_string())).is_err() {
                            debug!("Event channel closed, stopping SSE listener");
                            return;
                        }
                        // Wait before retrying
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };

                let mut stream = response.bytes_stream();
                let mut buffer = String::new();

                while let Some(chunk) = stream.next().await {
                    if tx.is_closed() {
                        debug!("Event receiver dropped, stopping SSE listener");
                        return; // Receiver dropped, exit cleanly
                    }

                    match chunk {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));

                            // Parse SSE events (data: {json}\n\n)
                            while let Some(end) = buffer.find("\n\n") {
                                let event_text = buffer[..end].to_string();
                                buffer.drain(..end + 2);

                                // Parse SSE event fields
                                let mut event_type = None;
                                let mut data = String::new();
                                let mut id = None;

                                for line in event_text.lines() {
                                    if let Some(event_value) = line.strip_prefix("event: ") {
                                        event_type = Some(event_value.to_string());
                                    } else if let Some(data_value) = line.strip_prefix("data: ") {
                                        if !data.is_empty() {
                                            data.push('\n');
                                        }
                                        data.push_str(data_value);
                                    } else if let Some(id_value) = line.strip_prefix("id: ") {
                                        id = Some(id_value.to_string());
                                    }
                                }

                                if !data.is_empty() {
                                    // Try to parse as JSON
                                    match serde_json::from_str::<Value>(&data) {
                                        Ok(json) => {
                                            debug!(
                                                "Received SSE event: type={:?}, id={:?}, data={}",
                                                event_type, id, data
                                            );

                                            // Determine event type based on JSON content
                                            if json.get("method").is_some() {
                                                // Notification
                                                if tx.send(ServerEvent::Notification(json)).is_err()
                                                {
                                                    debug!("Event channel closed during send");
                                                    return;
                                                }
                                            } else if json.get("id").is_some() {
                                                // Request requiring response
                                                if tx.send(ServerEvent::Request(json)).is_err() {
                                                    debug!("Event channel closed during send");
                                                    return;
                                                }
                                            } else {
                                                // Other data
                                                if tx.send(ServerEvent::Notification(json)).is_err()
                                                {
                                                    debug!("Event channel closed during send");
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to parse SSE data as JSON: {} - data: {}",
                                                e, data
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("SSE stream error: {}", e);
                            break;
                        }
                    }
                }

                // Connection lost, send event and retry
                info!("SSE connection lost, attempting to reconnect...");
                if tx.send(ServerEvent::ConnectionLost).is_err() {
                    debug!("Event channel closed, stopping SSE listener");
                    return;
                }

                // Wait before retrying
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });

        info!("SSE event listener started successfully");
        Ok(rx)
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

    fn set_session_id(&mut self, session_id: String) {
        debug!("HttpTransport: Setting session ID: {}", session_id);
        *self.session_id.lock() = Some(session_id);
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

    /// Test helper to create an in-memory byte stream for testing
    fn create_test_stream(
        data: Vec<Vec<u8>>,
    ) -> impl Stream<Item = Result<Vec<u8>, std::io::Error>> + Unpin {
        futures::stream::iter(data.into_iter().map(Ok))
    }

    #[tokio::test]
    async fn test_handle_byte_stream_single_response() {
        let mut transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();

        // Create a single JSON response
        let response_json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": []
            }
        });
        let response_bytes = serde_json::to_vec(&response_json).unwrap();

        // Create stream with single chunk
        let stream = create_test_stream(vec![response_bytes]);

        // Process the stream
        let result = transport.handle_byte_stream(stream).await.unwrap();

        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 1);
        assert!(result.get("result").is_some());
    }

    #[tokio::test]
    async fn test_handle_byte_stream_chunked_response() {
        let mut transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();

        // Create a response that will be split across chunks
        let response_json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    {"name": "calculator", "description": "A calculator tool"}
                ]
            }
        });
        let response_bytes = serde_json::to_vec(&response_json).unwrap();

        // Split the response into multiple chunks to test streaming
        let chunk1 = response_bytes[..20].to_vec();
        let chunk2 = response_bytes[20..40].to_vec();
        let chunk3 = response_bytes[40..].to_vec();

        let stream = create_test_stream(vec![chunk1, chunk2, chunk3]);

        // Process the stream
        let result = transport.handle_byte_stream(stream).await.unwrap();

        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 2);
        assert!(result.get("result").is_some());
    }

    #[tokio::test]
    async fn test_handle_byte_stream_with_progress_notifications() {
        let mut transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();

        // Create progress notification followed by final response
        let progress1 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "test_progress",
                "progress": 50,
                "total": 100
            }
        });

        let progress2 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "test_progress",
                "progress": 100,
                "total": 100
            }
        });

        let final_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "success": true
            }
        });

        // Create separate chunks for each frame
        let progress1_bytes = serde_json::to_vec(&progress1).unwrap();
        let progress2_bytes = serde_json::to_vec(&progress2).unwrap();
        let final_bytes = serde_json::to_vec(&final_response).unwrap();

        let stream = create_test_stream(vec![progress1_bytes, progress2_bytes, final_bytes]);

        // Start event listener to capture progress notifications
        let mut events = transport.start_event_listener().await.unwrap();

        // Process the stream (should return final response)
        let result = transport.handle_byte_stream(stream).await.unwrap();

        // Verify final response
        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 3);
        assert_eq!(result["result"]["success"], true);

        // Verify progress notifications were captured (with timeout to avoid hanging)
        use tokio::time::{Duration, timeout};

        let event1 = timeout(Duration::from_millis(100), events.recv()).await;
        assert!(event1.is_ok() && event1.unwrap().is_some());

        let event2 = timeout(Duration::from_millis(100), events.recv()).await;
        assert!(event2.is_ok() && event2.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_progress_event_queue_replay() {
        let mut transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();

        // Create progress notifications that arrive BEFORE starting event listener
        let progress1 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "early_progress",
                "progress": 25,
                "total": 100
            }
        });

        let progress2 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "early_progress",
                "progress": 75,
                "total": 100
            }
        });

        let final_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "result": {
                "queued_events_test": true
            }
        });

        // Create separate chunks for each frame
        let progress1_bytes = serde_json::to_vec(&progress1).unwrap();
        let progress2_bytes = serde_json::to_vec(&progress2).unwrap();
        let final_bytes = serde_json::to_vec(&final_response).unwrap();

        let stream = create_test_stream(vec![progress1_bytes, progress2_bytes, final_bytes]);

        // Process stream WITHOUT starting event listener first
        // This should queue the progress notifications
        let result = transport.handle_byte_stream(stream).await.unwrap();

        // Verify final response
        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 4);
        assert_eq!(result["result"]["queued_events_test"], true);

        // NOW start event listener - should replay queued events
        let mut events = transport.start_event_listener().await.unwrap();

        // Verify queued events are replayed (with timeout to avoid hanging)
        use tokio::time::{Duration, timeout};

        let event1 = timeout(Duration::from_millis(100), events.recv()).await;
        assert!(
            event1.is_ok() && event1.unwrap().is_some(),
            "Should replay first queued event"
        );

        let event2 = timeout(Duration::from_millis(100), events.recv()).await;
        assert!(
            event2.is_ok() && event2.unwrap().is_some(),
            "Should replay second queued event"
        );

        // Verify no more events (queue should be empty after replay)
        let no_more_events = timeout(Duration::from_millis(50), events.recv()).await;
        assert!(
            no_more_events.is_err(),
            "Should not have any more events after replay"
        );
    }
}
