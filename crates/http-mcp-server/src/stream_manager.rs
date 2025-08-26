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
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, error, warn};

use mcp_session_storage::{SessionStorage, SseEvent};

/// Enhanced stream manager with resumability support
pub struct StreamManager<S: SessionStorage> {
    /// Session storage backend for persistence
    storage: Arc<S>,
    /// Per-session broadcasters for real-time events
    broadcasters: Arc<RwLock<HashMap<String, broadcast::Sender<SseEvent>>>>,
    /// Configuration
    config: StreamConfig,
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

/// SSE stream wrapper that formats events properly (one per session)
pub struct SseStream {
    /// Underlying event stream
    stream: Pin<Box<dyn Stream<Item = SseEvent> + Send>>,
    /// Session metadata
    session_id: String,
}

impl SseStream {
    /// Get the session ID this stream belongs to
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    
    /// Get stream identifier for logging (same as session_id)
    pub fn stream_identifier(&self) -> String {
        self.session_id.clone()
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
    #[error("Broadcast error: {0}")]
    BroadcastError(#[from] broadcast::error::SendError<SseEvent>),
}

impl<S: SessionStorage + 'static> StreamManager<S> {
    /// Create new stream manager with session storage backend
    pub fn new(storage: Arc<S>) -> Self {
        Self::with_config(storage, StreamConfig::default())
    }

    /// Create stream manager with custom configuration
    pub fn with_config(storage: Arc<S>, config: StreamConfig) -> Self {
        Self {
            storage,
            broadcasters: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Handle SSE connection request with proper resumability
    pub async fn handle_sse_connection(
        &self,
        session_id: String,
        last_event_id: Option<u64>,
    ) -> Result<Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>, StreamError> {
        // Verify session exists
        if self.storage.get_session(&session_id).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?
            .is_none() 
        {
            return Err(StreamError::SessionNotFound(session_id));
        }

        // Create the SSE stream (one per session, as per SSE standard)
        let sse_stream = self.create_sse_stream(session_id.clone(), last_event_id).await?;

        // Convert to HTTP response
        let response = self.stream_to_response(sse_stream).await;

        info!("Created SSE connection: session={}, last_event_id={:?}", 
              session_id, last_event_id);

        Ok(response)
    }

    /// Create SSE stream with resumability support
    async fn create_sse_stream(
        &self,
        session_id: String,
        last_event_id: Option<u64>,
    ) -> Result<SseStream, StreamError> {
        // Get or create broadcaster for this session
        let broadcaster = self.get_or_create_broadcaster(&session_id).await;

        // Create the combined stream
        let storage = self.storage.clone();
        let session_id_clone = session_id.clone();
        let config = self.config.clone();

        let combined_stream = async_stream::stream! {
            // 1. First, yield any historical events (resumability)
            if let Some(after_event_id) = last_event_id {
                debug!("Replaying events after ID {} for session={}", 
                       after_event_id, session_id_clone);
                
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

            // 2. Then, stream real-time events from broadcaster
            let mut receiver = broadcaster.subscribe();
            let mut keepalive_interval = tokio::time::interval(
                tokio::time::Duration::from_secs(config.keepalive_interval_seconds)
            );

            loop {
                tokio::select! {
                    // Real-time events
                    event = receiver.recv() => {
                        match event {
                            Ok(event) => {
                                // All events for this session (no stream filtering needed)
                                yield event;
                            },
                            Err(broadcast::error::RecvError::Closed) => {
                                debug!("Broadcaster closed for session={}", session_id_clone);
                                break;
                            },
                            Err(broadcast::error::RecvError::Lagged(count)) => {
                                warn!("SSE stream lagged by {} events for session={}", count, session_id_clone);
                                // Continue streaming - client can reconnect with Last-Event-ID if needed
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
        };

        Ok(SseStream {
            stream: Box::pin(combined_stream),
            session_id,
        })
    }

    /// Get or create broadcaster for a session
    async fn get_or_create_broadcaster(&self, session_id: &str) -> broadcast::Sender<SseEvent> {
        let mut broadcasters = self.broadcasters.write().await;
        
        if let Some(sender) = broadcasters.get(session_id) {
            sender.clone()
        } else {
            let (sender, _) = broadcast::channel(self.config.channel_buffer_size);
            broadcasters.insert(session_id.to_string(), sender.clone());
            debug!("Created broadcaster for session: {}", session_id);
            sender
        }
    }

    /// Convert SSE stream to HTTP response with proper headers
    async fn stream_to_response(&self, sse_stream: SseStream) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>> {
        // Extract session info before moving the stream
        let session_id = sse_stream.session_id().to_string();
        let stream_identifier = sse_stream.stream_identifier();
        
        // Log stream creation with session identifier
        info!("Converting SSE stream to HTTP response: {}", stream_identifier);
        debug!("Stream details: session_id={}", session_id);
        
        // Transform events to SSE format and create proper HTTP frames
        let formatted_stream = sse_stream.stream.map(|event| {
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

    /// Broadcast event to specific session with persistence
    pub async fn broadcast_to_session(
        &self,
        session_id: &str,
        event_type: String,
        data: Value,
    ) -> Result<u64, StreamError> {
        // Create the event (no stream_id needed)
        let event = SseEvent::new(event_type, data);

        // Store event for resumability
        let stored_event = self.storage.store_event(session_id, event).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?;

        // Get or create broadcaster for this session (ensure it exists for all events)
        let broadcaster = self.get_or_create_broadcaster(session_id).await;
        
        // Broadcast to real-time subscribers
        if let Err(e) = broadcaster.send(stored_event.clone()) {
            match e {
                broadcast::error::SendError(_) => {
                    debug!("No active SSE subscribers for session: {} (event will be available for reconnection)", session_id);
                    // DON'T remove broadcaster - keep it for reconnections
                    // Events are stored in session storage and will be replayed on reconnect
                }
            }
        }

        debug!("Broadcast event to session={}, event_id={}", 
               session_id, stored_event.id);

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

    /// Clean up closed broadcasters
    pub async fn cleanup_broadcasters(&self) -> usize {
        let mut broadcasters = self.broadcasters.write().await;
        let initial_count = broadcasters.len();
        
        // Remove broadcasters with no active receivers
        broadcasters.retain(|session_id, sender| {
            let has_receivers = sender.receiver_count() > 0;
            if !has_receivers {
                debug!("Cleaned up broadcaster for session: {}", session_id);
            }
            has_receivers
        });
        
        let cleaned_count = initial_count - broadcasters.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} inactive broadcasters", cleaned_count);
        }
        
        cleaned_count
    }

    /// Create SSE stream for POST requests (MCP Streamable HTTP)
    pub async fn create_post_sse_stream(
        &self,
        session_id: String,
        response: mcp_json_rpc_server::JsonRpcResponse,
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
        let broadcasters = self.broadcasters.read().await;
        let session_count = self.storage.session_count().await.unwrap_or(0);
        let event_count = self.storage.event_count().await.unwrap_or(0);
        
        StreamStats {
            active_broadcasters: broadcasters.len(),
            total_sessions: session_count,
            total_events: event_count,
            channel_buffer_size: self.config.channel_buffer_size,
        }
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
    use mcp_session_storage::InMemorySessionStorage;
    use mcp_protocol::ServerCapabilities;

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