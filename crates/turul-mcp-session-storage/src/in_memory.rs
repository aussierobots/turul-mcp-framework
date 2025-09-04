//! In-Memory Session Storage Implementation
//!
//! This implementation stores all session data in memory using Arc<RwLock<>>
//! for thread safety. Suitable for:
//! - Development and testing
//! - Single-instance deployments with session persistence not required
//! - High-performance scenarios where sessions are short-lived

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info};

use turul_mcp_protocol::ServerCapabilities;
use crate::{SessionStorage, SessionInfo, SseEvent, SessionStorageError};

/// In-memory storage for sessions and events (SSE compliant)
#[derive(Debug, Clone)]
pub struct InMemorySessionStorage {
    /// All sessions by session ID
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    /// All events by session_id -> Vec<SseEvent>
    events: Arc<RwLock<HashMap<String, Vec<SseEvent>>>>,
    /// Global event ID counter for ordering
    event_counter: Arc<AtomicU64>,
    /// Configuration
    config: InMemoryConfig,
}

/// Configuration for in-memory session storage
#[derive(Debug, Clone)]
pub struct InMemoryConfig {
    /// Maximum events to keep per session (for memory management)
    pub max_events_per_session: usize,
    /// Maximum sessions to keep (for memory management)
    pub max_sessions: usize,
}

impl Default for InMemoryConfig {
    fn default() -> Self {
        Self {
            max_events_per_session: 10_000,  // 10k events per session
            max_sessions: 100_000,           // 100k concurrent sessions
        }
    }
}

/// Error type for in-memory storage operations
#[derive(Debug, thiserror::Error)]
pub enum InMemoryError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Maximum sessions limit reached: {0}")]
    MaxSessionsReached(usize),
    #[error("Maximum events per session limit reached: {0}")]
    MaxEventsReached(usize),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl InMemorySessionStorage {
    /// Create new in-memory session storage with default configuration
    pub fn new() -> Self {
        Self::with_config(InMemoryConfig::default())
    }

    /// Create new in-memory session storage with custom configuration
    pub fn with_config(config: InMemoryConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(HashMap::new())),
            event_counter: Arc::new(AtomicU64::new(1)), // Start at 1 for SSE compatibility
            config,
        }
    }

    /// Get current statistics
    pub async fn stats(&self) -> InMemoryStats {
        let sessions = self.sessions.read().await;
        let events = self.events.read().await;

        let total_events = events.values().map(|v| v.len()).sum();

        InMemoryStats {
            session_count: sessions.len(),
            total_event_count: total_events,
            max_events_per_session: self.config.max_events_per_session,
            max_sessions: self.config.max_sessions,
        }
    }

    /// Cleanup old events to prevent memory bloat
    async fn cleanup_events(&self) -> Result<u64, InMemoryError> {
        let mut events = self.events.write().await;
        let mut total_removed = 0u64;

        for (session_id, event_list) in events.iter_mut() {
            if event_list.len() > self.config.max_events_per_session {
                let excess = event_list.len() - self.config.max_events_per_session;
                event_list.drain(0..excess); // Remove oldest events
                total_removed += excess as u64;
                debug!("Cleaned up {} old events for session {}", excess, session_id);
            }
        }

        if total_removed > 0 {
            info!("Cleaned up {} old events across all sessions", total_removed);
        }

        Ok(total_removed)
    }
}

/// Statistics for in-memory storage
#[derive(Debug, Clone)]
pub struct InMemoryStats {
    pub session_count: usize,
    pub total_event_count: usize,
    pub max_events_per_session: usize,
    pub max_sessions: usize,
}

#[async_trait]
impl SessionStorage for InMemorySessionStorage {
    type Error = SessionStorageError;

    fn backend_name(&self) -> &'static str {
        "InMemory"
    }

    // ============================================================================
    // Session Management
    // ============================================================================

    async fn create_session(&self, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut sessions = self.sessions.write().await;
        
        if sessions.len() >= self.config.max_sessions {
            return Err(SessionStorageError::MaxSessionsReached(self.config.max_sessions));
        }

        let mut session = SessionInfo::new();
        session.server_capabilities = Some(capabilities);
        
        let session_id = session.session_id.clone();
        sessions.insert(session_id.clone(), session.clone());
        
        debug!("Created session: {}", session_id);
        Ok(session)
    }

    async fn create_session_with_id(&self, session_id: String, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut sessions = self.sessions.write().await;
        
        if sessions.len() >= self.config.max_sessions {
            return Err(SessionStorageError::MaxSessionsReached(self.config.max_sessions));
        }

        let mut session = SessionInfo::with_id(session_id.clone());
        session.server_capabilities = Some(capabilities);
        
        sessions.insert(session_id.clone(), session.clone());
        
        debug!("Created session with ID: {}", session_id);
        Ok(session)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_info.session_id.clone(), session_info);
        Ok(())
    }

    async fn set_session_state(&self, session_id: &str, key: &str, value: serde_json::Value) -> Result<(), Self::Error> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.state.insert(key.to_string(), value);
            session.touch(); // Update last activity
            Ok(())
        } else {
            Err(SessionStorageError::SessionNotFound(session_id.to_string()))
        }
    }

    async fn get_session_state(&self, session_id: &str, key: &str) -> Result<Option<serde_json::Value>, Self::Error> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            Ok(session.state.get(key).cloned())
        } else {
            Err(SessionStorageError::SessionNotFound(session_id.to_string()))
        }
    }

    async fn remove_session_state(&self, session_id: &str, key: &str) -> Result<Option<serde_json::Value>, Self::Error> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            let removed = session.state.remove(key);
            session.touch(); // Update last activity
            Ok(removed)
        } else {
            Err(SessionStorageError::SessionNotFound(session_id.to_string()))
        }
    }

    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error> {
        let mut sessions = self.sessions.write().await;
        let mut events = self.events.write().await;
        
        // Remove the session
        let removed = sessions.remove(session_id).is_some();
        
        if removed {
            // Remove all events for this session
            events.remove(session_id);
            
            debug!("Deleted session and all associated data: {}", session_id);
        }
        
        Ok(removed)
    }

    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }


    // ============================================================================
    // Event Management
    // ============================================================================

    async fn store_event(&self, session_id: &str, mut event: SseEvent) -> Result<SseEvent, Self::Error> {
        let mut events = self.events.write().await;
        
        // Assign unique event ID
        event.id = self.event_counter.fetch_add(1, Ordering::SeqCst);
        
        let event_list = events.entry(session_id.to_string()).or_insert_with(Vec::new);
        
        // Check event limit
        if event_list.len() >= self.config.max_events_per_session {
            return Err(SessionStorageError::MaxEventsReached(self.config.max_events_per_session));
        }
        
        event_list.push(event.clone());
        
        debug!("Stored event: session={}, event_id={}", session_id, event.id);
        Ok(event)
    }

    async fn get_events_after(&self, session_id: &str, after_event_id: u64) -> Result<Vec<SseEvent>, Self::Error> {
        let events = self.events.read().await;
        
        if let Some(event_list) = events.get(session_id) {
            let filtered: Vec<SseEvent> = event_list
                .iter()
                .filter(|event| event.id > after_event_id)
                .cloned()
                .collect();
            Ok(filtered)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_recent_events(&self, session_id: &str, limit: usize) -> Result<Vec<SseEvent>, Self::Error> {
        let events = self.events.read().await;
        
        if let Some(event_list) = events.get(session_id) {
            let recent: Vec<SseEvent> = event_list
                .iter()
                .rev()
                .take(limit)
                .rev()
                .cloned()
                .collect();
            Ok(recent)
        } else {
            Ok(Vec::new())
        }
    }

    async fn delete_events_before(&self, session_id: &str, before_event_id: u64) -> Result<u64, Self::Error> {
        let mut events = self.events.write().await;
        
        if let Some(event_list) = events.get_mut(session_id) {
            let original_len = event_list.len();
            event_list.retain(|event| event.id >= before_event_id);
            let removed = original_len - event_list.len();
            Ok(removed as u64)
        } else {
            Ok(0)
        }
    }

    // ============================================================================
    // Cleanup and Maintenance
    // ============================================================================

    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>, Self::Error> {
        let mut sessions = self.sessions.write().await;
        let mut events = self.events.write().await;
        
        let cutoff_millis = older_than
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let mut expired_sessions = Vec::new();
        
        // Find expired sessions
        sessions.retain(|session_id, session_info| {
            if session_info.last_activity < cutoff_millis {
                expired_sessions.push(session_id.clone());
                false
            } else {
                true
            }
        });
        
        // Remove events for expired sessions
        for session_id in &expired_sessions {
            events.remove(session_id);
        }
        
        if !expired_sessions.is_empty() {
            info!("Expired {} sessions", expired_sessions.len());
        }
        
        Ok(expired_sessions)
    }

    async fn session_count(&self) -> Result<usize, Self::Error> {
        let sessions = self.sessions.read().await;
        Ok(sessions.len())
    }

    async fn event_count(&self) -> Result<usize, Self::Error> {
        let events = self.events.read().await;
        let total = events.values().map(|v| v.len()).sum();
        Ok(total)
    }

    async fn maintenance(&self) -> Result<(), Self::Error> {
        self.cleanup_events().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_protocol::ServerCapabilities;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let storage = InMemorySessionStorage::new();
        
        // Create session
        let session = storage.create_session(ServerCapabilities::default()).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Get session
        let retrieved = storage.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().session_id, session_id);
        
        // Delete session
        let deleted = storage.delete_session(&session_id).await.unwrap();
        assert!(deleted);
        
        // Verify deletion
        let not_found = storage.get_session(&session_id).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_session_state() {
        let storage = InMemorySessionStorage::new();
        let session = storage.create_session(ServerCapabilities::default()).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Set state
        let value = serde_json::json!({"test": "value"});
        storage.set_session_state(&session_id, "test_key", value.clone()).await.unwrap();
        
        // Get state
        let retrieved = storage.get_session_state(&session_id, "test_key").await.unwrap();
        assert_eq!(retrieved, Some(value));
        
        // Remove state
        let removed = storage.remove_session_state(&session_id, "test_key").await.unwrap();
        assert_eq!(removed, Some(serde_json::json!({"test": "value"})));
        
        // Verify removal
        let not_found = storage.get_session_state(&session_id, "test_key").await.unwrap();
        assert_eq!(not_found, None);
    }

    #[tokio::test]
    async fn test_event_storage_and_retrieval() {
        let storage = InMemorySessionStorage::new();
        let session = storage.create_session(ServerCapabilities::default()).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Store events (no stream_id needed)
        let event1 = SseEvent::new("data".to_string(), serde_json::json!({"message": "test1"}));
        let event2 = SseEvent::new("data".to_string(), serde_json::json!({"message": "test2"}));
        
        let stored1 = storage.store_event(&session_id, event1).await.unwrap();
        let stored2 = storage.store_event(&session_id, event2).await.unwrap();
        
        assert!(stored1.id < stored2.id); // Event IDs should be ordered
        
        // Get events after first event
        let events_after = storage.get_events_after(&session_id, stored1.id).await.unwrap();
        assert_eq!(events_after.len(), 1);
        assert_eq!(events_after[0].id, stored2.id);
        
        // Get recent events
        let recent = storage.get_recent_events(&session_id, 10).await.unwrap();
        assert_eq!(recent.len(), 2);
    }
}