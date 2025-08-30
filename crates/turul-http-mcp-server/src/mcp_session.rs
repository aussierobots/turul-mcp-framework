//! Streamable HTTP session management for MCP transport
//!
//! This module implements a lightweight session management system
//! for MCP streamable HTTP with SSE support.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

use crate::protocol::McpProtocolVersion;

/// Per-session state
pub struct Session {
    /// Broadcast sender for SSE notifications  
    pub sender: broadcast::Sender<String>,
    /// When the session was created (touched on access)
    pub created: Instant,
    /// MCP protocol version for this session
    pub version: McpProtocolVersion,
}

impl Session {
    /// Update the "last used" timestamp to now (touch mechanism)
    pub fn touch(&mut self) {
        self.created = Instant::now();
    }
}

/// A handle to a session's message stream
pub struct SessionHandle {
    pub session_id: String,
    pub receiver: broadcast::Receiver<String>,
}

pub type SessionMap = Mutex<HashMap<String, Session>>;

lazy_static::lazy_static! {
    /// Internal map: session_id -> Session (sender, creation time, MCP Version)
    static ref SESSIONS: SessionMap = Mutex::new(HashMap::new());
}

/// Create a brand new session.
/// Returns the `session_id` and a `Receiver<String>` you can use to drive an SSE stream.
pub async fn new_session(mcp_version: McpProtocolVersion) -> SessionHandle {
    let session_id = Uuid::now_v7().to_string();
    // 128-slot broadcast channel for JSON-RPC notifications
    let (sender, receiver) = broadcast::channel(128);
    let session = Session {
        sender: sender.clone(),
        created: Instant::now(),
        version: mcp_version,
    };
    SESSIONS.lock().await.insert(session_id.clone(), session);
    SessionHandle {
        session_id,
        receiver,
    }
}

/// Fetch and "touch" the sender for this session, extending its lifetime.
/// Returns `None` if the session does not exist or has already expired.
pub async fn get_sender(session_id: &str) -> Option<broadcast::Sender<String>> {
    let mut sessions = SESSIONS.lock().await;
    if let Some(session) = sessions.get_mut(session_id) {
        // Bump the creation time to now
        session.touch();
        // Return a clone of the sender
        return Some(session.sender.clone());
    }
    None
}

/// Get a session's receiver for SSE streaming
pub async fn get_receiver(session_id: &str) -> Option<broadcast::Receiver<String>> {
    let mut sessions = SESSIONS.lock().await;
    if let Some(session) = sessions.get_mut(session_id) {
        session.touch();
        return Some(session.sender.subscribe());
    }
    None
}

/// Check if a session exists and touch it
pub async fn session_exists(session_id: &str) -> bool {
    let mut sessions = SESSIONS.lock().await;
    if let Some(session) = sessions.get_mut(session_id) {
        session.touch();
        true
    } else {
        false
    }
}

/// Explicitly remove/terminate a session.
/// You can call this on client disconnect or after HTTP GET SSE finishes.
pub async fn remove_session(session_id: &str) -> bool {
    SESSIONS.lock().await.remove(session_id).is_some()
}

/// Expire any sessions older than `max_age`, dropping them from the map.
pub async fn expire_old(max_age: Duration) {
    let cutoff = Instant::now() - max_age;
    let mut sessions = SESSIONS.lock().await;
    sessions.retain(|sid, session| {
        let alive = session.created >= cutoff;
        if !alive {
            info!("Session {} expired", sid);
        }
        alive
    });
}

/// Send the given JSON-RPC message to a specific session
pub async fn send_to_session(session_id: &str, message: String) -> bool {
    if let Some(sender) = get_sender(session_id).await {
        sender.send(message).is_ok()
    } else {
        false
    }
}

/// Send the given JSON-RPC message to every active session.
pub async fn broadcast_to_all(message: String) {
    let sessions = SESSIONS.lock().await;
    for (sid, session) in sessions.iter() {
        warn!("Sending message: {} to session {}", message, sid);
        // Ignore errors (no subscribers, lag, etc.)
        let _ = session.sender.send(message.clone());
    }
}

/// Disconnect all sessions
pub async fn disconnect_all() {
    let mut sessions = SESSIONS.lock().await;
    // Just clear the map: dropping each Session.sender
    sessions.clear();
    info!("All sessions have been disconnected");
}

/// Get count of active sessions
pub async fn session_count() -> usize {
    let sessions = SESSIONS.lock().await;
    sessions.len()
}

/// Spawn session cleanup task for automatic session management
pub fn spawn_session_cleanup() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            expire_old(Duration::from_secs(30 * 60)).await; // 30 minutes
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let handle = new_session(McpProtocolVersion::V2025_06_18).await;
        let session_id = handle.session_id.clone();
        
        // Session should exist
        assert!(session_exists(&session_id).await);
        
        // Should be able to get sender
        assert!(get_sender(&session_id).await.is_some());
        
        // Should be able to get receiver  
        assert!(get_receiver(&session_id).await.is_some());
        
        // Remove session
        assert!(remove_session(&session_id).await);
        
        // Should no longer exist
        assert!(!session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_session_messaging() {
        let handle = new_session(McpProtocolVersion::V2025_06_18).await;
        let session_id = handle.session_id.clone();
        
        // Send message to session
        let message = r#"{"method":"test","params":{}}"#.to_string();
        assert!(send_to_session(&session_id, message.clone()).await);
        
        // Receive message
        let mut receiver = handle.receiver;
        let received = tokio::time::timeout(
            Duration::from_millis(100), 
            receiver.recv()
        ).await;
        
        assert!(received.is_ok());
        assert_eq!(received.unwrap().unwrap(), message);
    }
}