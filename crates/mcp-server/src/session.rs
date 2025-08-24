//! Session Management for MCP Servers
//!
//! This module provides transparent session management for MCP tools and handlers.
//! Sessions are automatically created and managed by the framework.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use mcp_protocol::{ClientCapabilities, Implementation, McpVersion, ServerCapabilities};

/// Session context provided automatically to tools and handlers
#[derive(Clone)]
pub struct SessionContext {
    /// Unique session identifier
    pub session_id: String,
    /// Get session state value by key
    pub get_state: Arc<dyn Fn(&str) -> Option<Value> + Send + Sync>,
    /// Set session state value by key
    pub set_state: Arc<dyn Fn(&str, Value) + Send + Sync>,
    /// Remove session state value by key
    pub remove_state: Arc<dyn Fn(&str) -> Option<Value> + Send + Sync>,
    /// Check if session is initialized
    pub is_initialized: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Send notification to this session
    pub send_notification: Arc<dyn Fn(SessionEvent) + Send + Sync>,
}

impl SessionContext {
    /// Convenience method to get typed session state
    pub fn get_typed_state<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        (self.get_state)(key)
            .and_then(|v| serde_json::from_value(v).ok())
    }

    /// Convenience method to set typed session state
    pub fn set_typed_state<T>(&self, key: &str, value: T) -> Result<(), String>
    where
        T: serde::Serialize,
    {
        match serde_json::to_value(value) {
            Ok(json_value) => {
                (self.set_state)(key, json_value);
                Ok(())
            }
            Err(e) => Err(format!("Failed to serialize value: {}", e)),
        }
    }

    /// Send a custom notification to this session
    pub fn notify(&self, event: SessionEvent) {
        (self.send_notification)(event);
    }

    /// Send a progress notification
    pub fn notify_progress(&self, progress_token: impl Into<String>, progress: u64) {
        let mut other = std::collections::HashMap::new();
        other.insert("progressToken".to_string(), serde_json::json!(progress_token.into()));
        other.insert("progress".to_string(), serde_json::json!(progress));
        
        let params = mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = mcp_protocol::JsonRpcNotification::new(
            "notifications/progress".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a progress notification with total
    pub fn notify_progress_with_total(&self, progress_token: impl Into<String>, progress: u64, total: u64) {
        let mut other = std::collections::HashMap::new();
        other.insert("progressToken".to_string(), serde_json::json!(progress_token.into()));
        other.insert("progress".to_string(), serde_json::json!(progress));
        other.insert("total".to_string(), serde_json::json!(total));
        
        let params = mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = mcp_protocol::JsonRpcNotification::new(
            "notifications/progress".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a logging message notification
    pub fn notify_log(&self, level: &str, message: impl Into<String>) {
        let mut other = std::collections::HashMap::new();
        other.insert("level".to_string(), serde_json::json!(level));
        other.insert("message".to_string(), serde_json::json!(message.into()));
        
        let params = mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = mcp_protocol::JsonRpcNotification::new(
            "notifications/message".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a resource list changed notification
    pub fn notify_resources_changed(&self) {
        let notification = mcp_protocol::JsonRpcNotification::new(
            "notifications/resources/listChanged".to_string()
        );
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a resource updated notification
    pub fn notify_resource_updated(&self, uri: impl Into<String>) {
        let mut other = std::collections::HashMap::new();
        other.insert("uri".to_string(), serde_json::json!(uri.into()));
        
        let params = mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = mcp_protocol::JsonRpcNotification::new(
            "notifications/resources/updated".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a tools list changed notification
    pub fn notify_tools_changed(&self) {
        let notification = mcp_protocol::JsonRpcNotification::new(
            "notifications/tools/listChanged".to_string()
        );
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }
}

/// Events that can be sent to a session
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Notification to be sent to client
    Notification(Value),
    /// Keep-alive ping
    KeepAlive,
    /// Session termination
    Disconnect,
    /// Custom event with type and data
    Custom { event_type: String, data: Value },
}

/// Individual MCP session state
#[derive(Debug)]
pub struct McpSession {
    /// Unique session identifier
    pub id: String,
    /// When the session was created
    pub created: Instant,
    /// Last activity timestamp (for expiry)
    pub last_accessed: Instant,
    /// MCP protocol version for this session
    pub mcp_version: McpVersion,
    /// Client capabilities negotiated during initialization
    pub client_capabilities: Option<ClientCapabilities>,
    /// Server capabilities sent to client
    pub server_capabilities: ServerCapabilities,
    /// Client implementation info
    pub client_info: Option<Implementation>,
    /// Per-session state storage for tools/handlers
    pub state: HashMap<String, Value>,
    /// Broadcast sender for SSE notifications
    pub event_sender: broadcast::Sender<SessionEvent>,
    /// Session initialization status
    pub initialized: bool,
}

impl McpSession {
    /// Create a new session
    pub fn new(server_capabilities: ServerCapabilities) -> Self {
        let session_id = Uuid::now_v7().to_string();
        let (event_sender, _) = broadcast::channel(128);

        Self {
            id: session_id,
            created: Instant::now(),
            last_accessed: Instant::now(),
            mcp_version: McpVersion::CURRENT,
            client_capabilities: None,
            server_capabilities,
            client_info: None,
            state: HashMap::new(),
            event_sender,
            initialized: false,
        }
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Check if session has expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }

    /// Initialize session with client information
    pub fn initialize(
        &mut self,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
    ) {
        self.client_info = Some(client_info);
        self.client_capabilities = Some(client_capabilities);
        self.initialized = true;
        self.touch();
    }

    /// Initialize session with client information and negotiated protocol version
    pub fn initialize_with_version(
        &mut self,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
        mcp_version: McpVersion,
    ) {
        self.client_info = Some(client_info);
        self.client_capabilities = Some(client_capabilities);
        self.mcp_version = mcp_version;
        self.initialized = true;
        self.touch();
    }

    /// Get state value
    pub fn get_state(&self, key: &str) -> Option<Value> {
        self.state.get(key).cloned()
    }

    /// Set state value
    pub fn set_state(&mut self, key: &str, value: Value) {
        self.state.insert(key.to_string(), value);
        self.touch();
    }

    /// Remove state value
    pub fn remove_state(&mut self, key: &str) -> Option<Value> {
        let result = self.state.remove(key);
        if result.is_some() {
            self.touch();
        }
        result
    }

    /// Send event to this session
    pub fn send_event(&self, event: SessionEvent) -> Result<(), String> {
        self.event_sender
            .send(event)
            .map_err(|e| format!("Failed to send event: {}", e))?;
        Ok(())
    }

    /// Subscribe to session events (for SSE)
    pub fn subscribe_events(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_sender.subscribe()
    }
}

/// Session management errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session expired: {0}")]
    Expired(String),
    #[error("Session not initialized: {0}")]
    NotInitialized(String),
    #[error("Invalid session data: {0}")]
    InvalidData(String),
}

/// Global session manager for MCP servers
pub struct SessionManager {
    /// Active sessions
    sessions: RwLock<HashMap<String, McpSession>>,
    /// Default session expiry time
    session_timeout: Duration,
    /// Cleanup interval
    cleanup_interval: Duration,
    /// Default server capabilities for new sessions
    default_capabilities: ServerCapabilities,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(default_capabilities: ServerCapabilities) -> Self {
        Self::with_timeouts(
            default_capabilities,
            Duration::from_secs(30 * 60), // 30 minutes
            Duration::from_secs(60),      // 1 minute
        )
    }
    
    /// Create a new session manager with custom timeouts
    pub fn with_timeouts(
        default_capabilities: ServerCapabilities, 
        session_timeout: Duration,
        cleanup_interval: Duration,
    ) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            session_timeout,
            cleanup_interval,
            default_capabilities,
        }
    }

    /// Create a new session and return its ID
    pub async fn create_session(&self) -> String {
        let session = McpSession::new(self.default_capabilities.clone());
        let session_id = session.id.clone();

        debug!("Creating new session: {}", session_id);
        self.sessions.write().await.insert(session_id.clone(), session);
        session_id
    }

    /// Get session and update last accessed time
    pub async fn touch_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if session.is_expired(self.session_timeout) {
                sessions.remove(session_id);
                return Err(SessionError::Expired(session_id.to_string()));
            }
            session.touch();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Initialize a session with client information
    pub async fn initialize_session(
        &self,
        session_id: &str,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.initialize(client_info, client_capabilities);
            debug!("Session {} initialized", session_id);
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Initialize a session with client information and negotiated protocol version
    pub async fn initialize_session_with_version(
        &self,
        session_id: &str,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
        mcp_version: McpVersion,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.initialize_with_version(client_info, client_capabilities, mcp_version);
            debug!("Session {} initialized with protocol version {}", session_id, mcp_version);
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Check if session exists and is valid
    pub async fn session_exists(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| !s.is_expired(self.session_timeout))
            .unwrap_or(false)
    }

    /// Get session state value
    pub async fn get_session_state(&self, session_id: &str, key: &str) -> Option<Value> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id)?.get_state(key)
    }

    /// Set session state value
    pub async fn set_session_state(&self, session_id: &str, key: &str, value: Value) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.set_state(key, value);
        }
    }

    /// Remove session state value
    pub async fn remove_session_state(&self, session_id: &str, key: &str) -> Option<Value> {
        let mut sessions = self.sessions.write().await;
        sessions.get_mut(session_id)?.remove_state(key)
    }

    /// Check if session is initialized
    pub async fn is_session_initialized(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.initialized)
            .unwrap_or(false)
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            debug!("Session {} removed", session_id);
            // Send disconnect event
            let _ = session.send_event(SessionEvent::Disconnect);
            true
        } else {
            false
        }
    }

    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self) -> usize {
        let cutoff = Instant::now() - self.session_timeout;
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        sessions.retain(|id, session| {
            let keep = session.last_accessed >= cutoff;
            if !keep {
                info!("Session {} expired and removed", id);
                // Send disconnect event before removal
                let _ = session.send_event(SessionEvent::Disconnect);
            }
            keep
        });

        initial_count - sessions.len()
    }

    /// Send event to specific session
    pub async fn send_event_to_session(
        &self,
        session_id: &str,
        event: SessionEvent,
    ) -> Result<(), SessionError> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            session.send_event(event)
                .map_err(|e| SessionError::InvalidData(e))?;
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Broadcast event to all sessions
    pub async fn broadcast_event(&self, event: SessionEvent) {
        let sessions = self.sessions.read().await;
        for (session_id, session) in sessions.iter() {
            if let Err(e) = session.send_event(event.clone()) {
                warn!("Failed to send event to session {}: {}", session_id, e);
            }
        }
    }

    /// Get active session count
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Create session context for a session
    pub fn create_session_context(self: &Arc<Self>, session_id: &str) -> Option<SessionContext> {
        let session_id = session_id.to_string();
        let session_manager = Arc::clone(self);

        // Create closures for state management
        let get_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> Option<Value> {
                futures::executor::block_on(async {
                    session_manager.get_session_state(&session_id, key).await
                })
            })
        };

        let set_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str, value: Value| {
                futures::executor::block_on(async {
                    session_manager.set_session_state(&session_id, key, value).await
                });
            })
        };

        let remove_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> Option<Value> {
                futures::executor::block_on(async {
                    session_manager.remove_session_state(&session_id, key).await
                })
            })
        };

        let is_initialized = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move || -> bool {
                futures::executor::block_on(async {
                    session_manager.is_session_initialized(&session_id).await
                })
            })
        };

        let send_notification = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |event: SessionEvent| {
                futures::executor::block_on(async {
                    let _ = session_manager.send_event_to_session(&session_id, event).await;
                });
            })
        };

        Some(SessionContext {
            session_id,
            get_state,
            set_state,
            remove_state,
            is_initialized,
            send_notification,
        })
    }

    /// Start automatic cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(manager.cleanup_interval);
            loop {
                interval.tick().await;
                let cleaned = manager.cleanup_expired().await;
                if cleaned > 0 {
                    debug!("Cleaned up {} expired sessions", cleaned);
                }
            }
        })
    }

    /// Get a session's event receiver for SSE streaming
    pub async fn get_session_event_receiver(&self, session_id: &str) -> Option<broadcast::Receiver<SessionEvent>> {
        let sessions = self.sessions.read().await;
        Some(sessions.get(session_id)?.subscribe_events())
    }
}

/// Trait for session-aware components
#[async_trait]
pub trait SessionAware {
    /// Handle request with session context
    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> Result<Value, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_session_creation() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;
        assert!(!session_id.is_empty());
        assert!(manager.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_session_state() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Set state
        manager
            .set_session_state(&session_id, "test_key", json!("test_value"))
            .await;

        // Get state
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, Some(json!("test_value")));

        // Remove state
        let removed = manager.remove_session_state(&session_id, "test_key").await;
        assert_eq!(removed, Some(json!("test_value")));

        // Verify removed
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_session_context() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let ctx = manager.create_session_context(&session_id).unwrap();

        // Test state operations through context
        (ctx.set_state)("test", json!("value"));
        let value = (ctx.get_state)("test");
        assert_eq!(value, Some(json!("value")));

        let removed = (ctx.remove_state)("test");
        assert_eq!(removed, Some(json!("value")));

        // Test notification sending
        ctx.notify_log("info", "Test notification");
        ctx.notify_progress("test-token", 50);
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let capabilities = ServerCapabilities::default();
        let mut manager = SessionManager::new(capabilities);
        manager.session_timeout = Duration::from_millis(100); // Very short timeout

        let session_id = manager.create_session().await;
        assert!(manager.session_exists(&session_id).await);

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Session should be expired now
        let result = manager.touch_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::Expired(_))));
    }
}