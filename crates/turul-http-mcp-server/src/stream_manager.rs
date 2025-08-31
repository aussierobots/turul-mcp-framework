//! Enhanced Stream Manager with MCP 2025-06-18 Resumability
//!
//! This module provides proper SSE stream management with:
//! - Event IDs for resumability
//! - Last-Event-ID header support
//! - Per-session event targeting (not broadcast to all)
//! - Event persistence and replay
//! - Proper HTTP status codes and headers

use std::sync::Arc;
use std::collections::HashMap;
use std::pin::Pin;
use futures::{Stream, StreamExt};
use hyper::{Response, StatusCode};
use http_body_util::{StreamBody, BodyExt};
use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, CACHE_CONTROL, ACCESS_CONTROL_ALLOW_ORIGIN};
use serde_json::Value;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, error, warn};

use turul_mcp_session_storage::SseEvent;

/// Connection ID for tracking individual SSE streams
pub type ConnectionId = String;

/// Enhanced stream manager with resumability support (MCP spec compliant)
pub struct StreamManager {
    /// Session storage backend for persistence
    storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    /// Per-session connections for real-time events (MCP compliant - no broadcasting)
    connections: Arc<RwLock<HashMap<String, HashMap<ConnectionId, mpsc::Sender<SseEvent>>>>>,
    /// Configuration
    config: StreamConfig,
    /// Unique instance ID for debugging
    instance_id: String,
}

/// Configuration for stream management
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Channel buffer size for real-time broadcasting
    pub channel_buffer_size: usize,
    /// Maximum events to replay on reconnection
    pub max_replay_events: usize,
    /// Keep-alive interval in seconds
    pub keepalive_interval_seconds: u64,
    /// CORS configuration
    pub cors_origin: String,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            channel_buffer_size: 1000,
            max_replay_events: 100,
            keepalive_interval_seconds: 30,
            cors_origin: "*".to_string(),
        }
    }
}

/// SSE stream wrapper that formats events properly (MCP compliant - one connection per stream)
pub struct SseStream {
    /// Underlying event stream
    stream: Option<Pin<Box<dyn Stream<Item = SseEvent> + Send>>>,
    /// Session metadata
    session_id: String,
    /// Connection identifier (for MCP spec compliance)
    connection_id: ConnectionId,
}

impl SseStream {
    /// Get the session ID this stream belongs to
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the connection ID for this stream
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// Get stream identifier for logging (session + connection)
    pub fn stream_identifier(&self) -> String {
        format!("{}:{}", self.session_id, self.connection_id)
    }
}

impl Drop for SseStream {
    fn drop(&mut self) {
        debug!("🔥 DROP: SseStream - session={}, connection={}",
               self.session_id, self.connection_id);
        if self.stream.is_some() {
            debug!("🔥 Stream still present during drop - this indicates early cleanup");
        } else {
            debug!("🔥 Stream was properly extracted before drop");
        }
    }
}

/// Error type for stream management
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Stream not found: session={0}, stream={1}")]
    StreamNotFound(String, String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

impl StreamManager {
    /// Create new stream manager with session storage backend
    pub fn new(storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>) -> Self {
        Self::with_config(storage, StreamConfig::default())
    }

    /// Create stream manager with custom configuration
    pub fn with_config(storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>, config: StreamConfig) -> Self {
        use uuid::Uuid;
        let instance_id = Uuid::now_v7().to_string();
        debug!("🔧 Creating StreamManager instance: {}", instance_id);
        Self {
            storage,
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
            instance_id,
        }
    }

    /// Handle SSE connection request with proper resumability
    pub async fn handle_sse_connection(
        &self,
        session_id: String,
        connection_id: ConnectionId,
        last_event_id: Option<u64>,
    ) -> Result<Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>, StreamError> {
        // Verify session exists
        if self.storage.get_session(&session_id).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?
            .is_none()
        {
            return Err(StreamError::SessionNotFound(session_id));
        }

        // Create the SSE stream (one per connection, MCP compliant)
        let sse_stream = self.create_sse_stream(session_id.clone(), connection_id.clone(), last_event_id).await?;

        // Convert to HTTP response
        let response = self.stream_to_response(sse_stream).await;

        info!("Created SSE connection: session={}, connection={}, last_event_id={:?}",
              session_id, connection_id, last_event_id);

        Ok(response)
    }

    /// Create SSE stream with resumability support (MCP compliant - no broadcast)
    async fn create_sse_stream(
        &self,
        session_id: String,
        connection_id: ConnectionId,
        last_event_id: Option<u64>,
    ) -> Result<SseStream, StreamError> {
        // Create mpsc channel for this specific connection (MCP compliant)
        let (sender, mut receiver) = mpsc::channel(self.config.channel_buffer_size);

        // Register this connection with the session
        self.register_connection(&session_id, connection_id.clone(), sender).await;

        // Create the combined stream
        let storage = self.storage.clone();
        let session_id_clone = session_id.clone();
        let connection_id_clone = connection_id.clone();
        let config = self.config.clone();

        let combined_stream = async_stream::stream! {
            // 1. First, yield any historical events (resumability)
            if let Some(after_event_id) = last_event_id {
                debug!("Replaying events after ID {} for session={}, connection={}",
                       after_event_id, session_id_clone, connection_id_clone);

                match storage.get_events_after(&session_id_clone, after_event_id).await {
                    Ok(events) => {
                        for event in events.into_iter().take(config.max_replay_events) {
                            yield event;
                        }
                    },
                    Err(e) => {
                        error!("Failed to get historical events: {}", e);
                        // Continue with real-time events even if historical replay fails
                    }
                }
            }

            // 2. Then, stream real-time events from dedicated channel
            let mut keepalive_interval = tokio::time::interval(
                tokio::time::Duration::from_secs(config.keepalive_interval_seconds)
            );

            loop {
                tokio::select! {
                    // Real-time events from this connection's channel
                    event = receiver.recv() => {
                        match event {
                            Some(event) => {
                                debug!("📨 Received event for connection {}: {}", connection_id_clone, event.event_type);
                                yield event;
                            },
                            None => {
                                debug!("Connection channel closed for session={}, connection={}", session_id_clone, connection_id_clone);
                                break;
                            }
                        }
                    },

                    // Keep-alive pings
                    _ = keepalive_interval.tick() => {
                        let keepalive_event = SseEvent {
                            id: 0, // Keep-alive events don't need persistent IDs
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            event_type: "ping".to_string(),
                            data: serde_json::json!({"type": "keepalive"}),
                            retry: None,
                        };
                        yield keepalive_event;
                    }
                }
            }

            // Clean up connection when stream ends
            debug!("🧹 Cleaning up connection: session={}, connection={}", session_id_clone, connection_id_clone);
        };

        Ok(SseStream {
            stream: Some(Box::pin(combined_stream)),
            session_id,
            connection_id,
        })
    }

    /// Register a new connection for a session (MCP compliant)
    async fn register_connection(
        &self,
        session_id: &str,
        connection_id: ConnectionId,
        sender: mpsc::Sender<SseEvent>
    ) {
        let mut connections = self.connections.write().await;

        debug!("[{}] 🔍 BEFORE registration: HashMap has {} sessions", self.instance_id, connections.len());
        for (sid, conns) in connections.iter() {
            debug!("[{}] 🔍 Existing session before: {} with {} connections", self.instance_id, sid, conns.len());
        }

        // Get or create session entry
        let session_connections = connections.entry(session_id.to_string())
            .or_insert_with(HashMap::new);

        // Add this connection
        session_connections.insert(connection_id.clone(), sender);

        debug!("[{}] 🔗 Registered connection: session={}, connection={}, total_connections={}",
               self.instance_id, session_id, connection_id, session_connections.len());

        debug!("[{}] 🔍 AFTER registration: HashMap has {} sessions", self.instance_id, connections.len());
        for (sid, conns) in connections.iter() {
            debug!("[{}] 🔍 Session after: {} with {} connections", self.instance_id, sid, conns.len());
        }
    }

    /// Remove a connection when it's closed
    pub async fn unregister_connection(&self, session_id: &str, connection_id: &ConnectionId) {
        debug!("🔴 UNREGISTER called for session={}, connection={}", session_id, connection_id);
        let mut connections = self.connections.write().await;

        debug!("🔍 BEFORE unregister: HashMap has {} sessions", connections.len());

        if let Some(session_connections) = connections.get_mut(session_id) {
            if session_connections.remove(connection_id).is_some() {
                debug!("🔌 Unregistered connection: session={}, connection={}", session_id, connection_id);

                // Clean up empty sessions
                if session_connections.is_empty() {
                    connections.remove(session_id);
                    debug!("🧹 Removed empty session: {}", session_id);
                }
            }
        }

        debug!("🔍 AFTER unregister: HashMap has {} sessions", connections.len());
    }

    /// Convert SSE stream to HTTP response with proper headers
    async fn stream_to_response(&self, mut sse_stream: SseStream) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>> {
        // Extract session info before moving the stream
        let session_id = sse_stream.session_id().to_string();
        let stream_identifier = sse_stream.stream_identifier();

        // Log stream creation with session identifier
        info!("Converting SSE stream to HTTP response: {}", stream_identifier);
        debug!("Stream details: session_id={}", session_id);

        // Transform events to SSE format and create proper HTTP frames
        // Extract stream from Option wrapper
        let stream = sse_stream.stream.take().expect("Stream should be present in SseStream");

        let formatted_stream = stream.map(|event| {
            let sse_formatted = event.format();
            debug!("📡 Streaming SSE event: id={}, event_type={}", event.id, event.event_type);
            Ok(hyper::body::Frame::data(Bytes::from(sse_formatted)))
        });

        // Create streaming body from the actual event stream and box it
        let body = StreamBody::new(formatted_stream).boxed_unsync();

        // Build response with proper SSE headers for streaming
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/event-stream")
            .header(CACHE_CONTROL, "no-cache")
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, &self.config.cors_origin)
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap()
    }

    /// Send event to specific session (MCP compliant - ONE connection only)
    pub async fn broadcast_to_session(
        &self,
        session_id: &str,
        event_type: String,
        data: Value,
    ) -> Result<u64, StreamError> {
        // Create the event
        let event = SseEvent::new(event_type, data);

        // Store event for resumability
        let stored_event = self.storage.store_event(session_id, event).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?;

        // DEBUG: Check connection state more thoroughly
        let connections = self.connections.read().await;
        debug!("[{}] 🔍 Checking connections for session {}: connections hashmap has {} sessions",
               self.instance_id, session_id, connections.len());

        if let Some(session_connections) = connections.get(session_id) {
            debug!("🔍 Session {} found with {} connections", session_id, session_connections.len());

            if !session_connections.is_empty() {
                // Pick the FIRST available connection (MCP compliant)
                let (selected_connection_id, selected_sender) = session_connections.iter().next().unwrap();

                // Check if sender is closed
                if selected_sender.is_closed() {
                    warn!("🔌 Sender is closed for connection: session={}, connection={}",
                          session_id, selected_connection_id);
                    debug!("📭 Connection sender was closed, event stored for reconnection");
                } else {
                    debug!("✅ Sender is open, attempting to send to connection: session={}, connection={}",
                           session_id, selected_connection_id);

                    match selected_sender.try_send(stored_event.clone()) {
                        Ok(()) => {
                            info!("✅ Sent notification to ONE connection: session={}, connection={}, event_id={}, method={}",
                                  session_id, selected_connection_id, stored_event.id, stored_event.event_type);
                        },
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            warn!("⚠️ Connection buffer full: session={}, connection={}", session_id, selected_connection_id);
                            // Event is still stored for reconnection
                        },
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            warn!("🔌 Connection closed during send: session={}, connection={}", session_id, selected_connection_id);
                            // Event is still stored for reconnection
                        }
                    }
                }
            } else {
                debug!("📭 No active connections for session: {} (event stored for reconnection)", session_id);
            }
        } else {
            debug!("📭 No connections registered for session: {} (event stored for reconnection)", session_id);

            // DEBUG: List all sessions in connections
            for (sid, conns) in connections.iter() {
                debug!("🔍 Available session: {} with {} connections", sid, conns.len());
            }
        }

        Ok(stored_event.id)
    }

    /// Broadcast to all sessions (for server-wide notifications)
    pub async fn broadcast_to_all_sessions(
        &self,
        event_type: String,
        data: Value,
    ) -> Result<Vec<String>, StreamError> {
        // Get all session IDs
        let session_ids = self.storage.list_sessions().await
            .map_err(|e| StreamError::StorageError(e.to_string()))?;

        let mut failed_sessions = Vec::new();

        for session_id in session_ids {
            if let Err(e) = self.broadcast_to_session(&session_id, event_type.clone(), data.clone()).await {
                error!("Failed to broadcast to session {}: {}", session_id, e);
                failed_sessions.push(session_id);
            }
        }

        Ok(failed_sessions)
    }

    /// Clean up closed connections
    pub async fn cleanup_connections(&self) -> usize {
        debug!("🧹 CLEANUP_CONNECTIONS called");
        let mut connections = self.connections.write().await;
        let mut total_cleaned = 0;

        debug!("🔍 BEFORE cleanup: HashMap has {} sessions", connections.len());

        // Clean up closed connections
        connections.retain(|session_id, session_connections| {
            let initial_count = session_connections.len();

            // Remove closed connections
            session_connections.retain(|connection_id, sender| {
                if sender.is_closed() {
                    debug!("🧹 Cleaned up closed connection: session={}, connection={}", session_id, connection_id);
                    false
                } else {
                    true
                }
            });

            let cleaned_count = initial_count - session_connections.len();
            total_cleaned += cleaned_count;

            // Keep session if it has active connections
            !session_connections.is_empty()
        });

        if total_cleaned > 0 {
            info!("Cleaned up {} inactive connections", total_cleaned);
        }

        total_cleaned
    }

    /// Create SSE stream for POST requests (MCP Streamable HTTP)
    pub async fn create_post_sse_stream(
        &self,
        session_id: String,
        response: turul_mcp_json_rpc_server::JsonRpcResponse,
    ) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, StreamError> {
        // Verify session exists
        if self.storage.get_session(&session_id).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?
            .is_none()
        {
            return Err(StreamError::SessionNotFound(session_id));
        }

        info!("Creating POST SSE stream for session: {}", session_id);

        // Create the SSE response body
        let response_json = serde_json::to_string(&response)
            .map_err(|e| StreamError::StorageError(format!("Failed to serialize response: {}", e)))?;

        // For MCP Streamable HTTP, we create an SSE stream that:
        // 1. Sends any pending notifications for this session
        // 2. Sends the JSON-RPC response as an SSE event
        // 3. Closes the stream

        let mut sse_content = String::new();

        // 1. Include recent notifications that may have been generated during tool execution
        // Note: Since tool notifications are processed asynchronously, we need to wait a moment
        // and then check for recent events to include in the POST SSE response
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        if let Ok(events) = self.storage.get_recent_events(&session_id, 10).await {
            for event in events {
                // Convert stored SSE event to notification JSON-RPC format
                if event.event_type != "ping" { // Skip keepalive events
                    let notification_sse = format!(
                        "event: data\ndata: {}\n\n",
                        event.data
                    );
                    sse_content.push_str(&notification_sse);
                    debug!("📤 Including notification in POST SSE stream: event_type={}", event.event_type);
                }
            }
        }

        // 2. Add the JSON-RPC tool response
        let response_sse = format!(
            "event: data\ndata: {}\n\n",
            response_json
        );
        sse_content.push_str(&response_sse);

        debug!("📡 POST SSE response created: session={}, content_length={}", session_id, sse_content.len());

        // Build response with proper SSE headers including MCP session ID
        Ok(hyper::Response::builder()
            .status(hyper::StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, "text/event-stream")
            .header(hyper::header::CACHE_CONTROL, "no-cache")
            .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, &self.config.cors_origin)
            .header("Connection", "keep-alive")
            .header("Mcp-Session-Id", &session_id)
            .body(http_body_util::Full::new(bytes::Bytes::from(sse_content)))
            .unwrap())
    }

    /// Get statistics about active streams
    pub async fn get_stats(&self) -> StreamStats {
        let connections = self.connections.read().await;
        let session_count = self.storage.session_count().await.unwrap_or(0);
        let event_count = self.storage.event_count().await.unwrap_or(0);

        // Count total active connections
        let total_connections: usize = connections.values()
            .map(|session_connections| session_connections.len())
            .sum();

        StreamStats {
            active_broadcasters: total_connections, // Now tracks active connections
            total_sessions: session_count,
            total_events: event_count,
            channel_buffer_size: self.config.channel_buffer_size,
        }
    }
}

impl Drop for StreamManager {
    fn drop(&mut self) {
        debug!("🔥 DROP: StreamManager instance {} - this may cause connection loss!",
               self.instance_id);
        debug!("🔥 If this appears during request processing, it indicates architecture problem");
    }
}

/// Stream manager statistics
#[derive(Debug, Clone)]
pub struct StreamStats {
    pub active_broadcasters: usize,
    pub total_sessions: usize,
    pub total_events: usize,
    pub channel_buffer_size: usize,
}

// Helper to create async stream
#[cfg(not(test))]
use async_stream;

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_session_storage::InMemorySessionStorage;
    use turul_mcp_protocol::ServerCapabilities;

    #[tokio::test]
    async fn test_stream_manager_creation() {
        let storage = Arc::new(InMemorySessionStorage::new());
        let manager = StreamManager::new(storage);

        let stats = manager.get_stats().await;
        assert_eq!(stats.active_broadcasters, 0);
        assert_eq!(stats.total_sessions, 0);
    }

    #[tokio::test]
    async fn test_broadcast_to_session() {
        let storage = Arc::new(InMemorySessionStorage::new());
        let manager = StreamManager::new(storage.clone());

        // Create a session
        let session = storage.create_session(ServerCapabilities::default()).await.unwrap();
        let session_id = session.session_id.clone();

        // Broadcast an event
        let event_id = manager.broadcast_to_session(
            &session_id,
            "test".to_string(),
            serde_json::json!({"message": "test"})
        ).await.unwrap();

        assert!(event_id > 0);

        // Verify event was stored
        let events = storage.get_events_after(&session_id, 0).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event_id);
    }
}
