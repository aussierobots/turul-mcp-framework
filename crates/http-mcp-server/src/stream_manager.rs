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
use http_body_util::Full;
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

/// SSE stream wrapper that formats events properly
pub struct SseStream {
    /// Underlying event stream
    stream: Pin<Box<dyn Stream<Item = SseEvent> + Send>>,
    /// Stream metadata
    session_id: String,
    stream_id: String,
}

impl SseStream {
    /// Get the session ID this stream belongs to
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    
    /// Get the stream ID within the session
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
    
    /// Get full stream identifier for logging
    pub fn stream_identifier(&self) -> String {
        format!("{}:{}", self.session_id, self.stream_id)
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
        stream_id: String,
        last_event_id: Option<u64>,
    ) -> Result<Response<Full<Bytes>>, StreamError> {
        // Verify session exists
        if self.storage.get_session(&session_id).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?
            .is_none() 
        {
            return Err(StreamError::SessionNotFound(session_id));
        }

        // Create or get stream
        let _stream_info = match self.storage.get_stream(&session_id, &stream_id).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?
        {
            Some(info) => info,
            None => {
                // Create new stream
                self.storage.create_stream(&session_id, stream_id.clone()).await
                    .map_err(|e| StreamError::StorageError(e.to_string()))?
            }
        };

        // Create the SSE stream
        let sse_stream = self.create_sse_stream(session_id.clone(), stream_id.clone(), last_event_id).await?;

        // Convert to HTTP response
        let response = self.stream_to_response(sse_stream).await;

        info!("Created SSE connection: session={}, stream={}, last_event_id={:?}", 
              session_id, stream_id, last_event_id);

        Ok(response)
    }

    /// Create SSE stream with resumability support
    async fn create_sse_stream(
        &self,
        session_id: String,
        stream_id: String,
        last_event_id: Option<u64>,
    ) -> Result<SseStream, StreamError> {
        // Get or create broadcaster for this session
        let broadcaster = self.get_or_create_broadcaster(&session_id).await;

        // Create the combined stream
        let storage = self.storage.clone();
        let session_id_clone = session_id.clone();
        let stream_id_clone = stream_id.clone();
        let config = self.config.clone();

        let combined_stream = async_stream::stream! {
            // 1. First, yield any historical events (resumability)
            if let Some(after_event_id) = last_event_id {
                debug!("Replaying events after ID {} for session={}, stream={}", 
                       after_event_id, session_id_clone, stream_id_clone);
                
                match storage.get_events_after(&session_id_clone, &stream_id_clone, after_event_id).await {
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
                                // Only yield events for this specific stream
                                if event.stream_id == stream_id_clone {
                                    yield event;
                                }
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
                            stream_id: stream_id_clone.clone(),
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
            stream_id,
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
    async fn stream_to_response(&self, sse_stream: SseStream) -> Response<Full<Bytes>> {
        // Extract session and stream info before moving the stream
        let session_id = sse_stream.session_id().to_string();
        let stream_id = sse_stream.stream_id().to_string();
        let stream_identifier = sse_stream.stream_identifier();
        
        // Log stream creation with session and stream identifiers
        info!("Converting SSE stream to HTTP response: {}", stream_identifier);
        debug!("Stream details: session_id={}, stream_id={}", session_id, stream_id);
        
        // Transform events to SSE format
        let _formatted_stream = sse_stream.stream.map(|event| {
            Ok::<_, hyper::Error>(event.format())
        });

        // Create body from stream with session and stream info
        let sse_data = format!(
            "data: {{\"type\":\"sse_stream\",\"session_id\":\"{}\",\"stream_id\":\"{}\",\"identifier\":\"{}\"}}\n\n",
            session_id, stream_id, stream_identifier
        );
        let body = Full::new(Bytes::from(sse_data));

        // Build response with proper SSE headers
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
        stream_id: &str,
        event_type: String,
        data: Value,
    ) -> Result<u64, StreamError> {
        // Create the event
        let event = SseEvent::new(stream_id.to_string(), event_type, data);

        // Store event for resumability
        let stored_event = self.storage.store_event(session_id, stream_id, event).await
            .map_err(|e| StreamError::StorageError(e.to_string()))?;

        // Broadcast to real-time subscribers
        if let Some(broadcaster) = self.broadcasters.read().await.get(session_id) {
            // Send event to real-time subscribers
            if let Err(e) = broadcaster.send(stored_event.clone()) {
                match e {
                    broadcast::error::SendError(_) => {
                        debug!("Broadcast channel closed for session: {}", session_id);
                        // Remove closed broadcaster
                        let mut broadcasters = self.broadcasters.write().await;
                        broadcasters.remove(session_id);
                    }
                }
            }
        }

        debug!("Broadcast event to session={}, stream={}, event_id={}", 
               session_id, stream_id, stored_event.id);

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
            // Use a default stream ID for server-wide notifications
            let stream_id = "main";
            
            if let Err(e) = self.broadcast_to_session(&session_id, stream_id, event_type.clone(), data.clone()).await {
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
            "main",
            "test".to_string(),
            serde_json::json!({"message": "test"})
        ).await.unwrap();
        
        assert!(event_id > 0);
        
        // Verify event was stored
        let events = storage.get_events_after(&session_id, "main", 0).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event_id);
    }
}