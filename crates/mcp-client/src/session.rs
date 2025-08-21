//! Session management for MCP client

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::error::{McpClientResult, SessionError};
use crate::config::ClientConfig;
use mcp_protocol_2025_06_18::{
    ClientCapabilities, ServerCapabilities, Implementation,
    InitializeRequest
};

/// Session state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Session is not initialized
    Uninitialized,
    /// Session is being initialized
    Initializing,
    /// Session is active and ready for operations
    Active,
    /// Session is reconnecting
    Reconnecting,
    /// Session has been terminated
    Terminated,
    /// Session encountered an error
    Error(String),
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Uninitialized => write!(f, "uninitialized"),
            SessionState::Initializing => write!(f, "initializing"),
            SessionState::Active => write!(f, "active"),
            SessionState::Reconnecting => write!(f, "reconnecting"),
            SessionState::Terminated => write!(f, "terminated"),
            SessionState::Error(err) => write!(f, "error: {}", err),
        }
    }
}

/// Session information and metadata
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: String,
    
    /// Session state
    pub state: SessionState,
    
    /// Client capabilities sent during initialization
    pub client_capabilities: Option<ClientCapabilities>,
    
    /// Server capabilities received during initialization
    pub server_capabilities: Option<ServerCapabilities>,
    
    /// Protocol version negotiated
    pub protocol_version: Option<String>,
    
    /// Session creation timestamp
    pub created_at: Instant,
    
    /// Last activity timestamp
    pub last_activity: Instant,
    
    /// Connection attempt count
    pub connection_attempts: u32,
    
    /// Session metadata
    pub metadata: Value,
}

impl SessionInfo {
    /// Create a new session info
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            session_id: Uuid::now_v7().to_string(),
            state: SessionState::Uninitialized,
            client_capabilities: None,
            server_capabilities: None,
            protocol_version: None,
            created_at: now,
            last_activity: now,
            connection_attempts: 0,
            metadata: Value::Null,
        }
    }
    
    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }
    
    /// Get session duration
    pub fn duration(&self) -> Duration {
        self.last_activity.duration_since(self.created_at)
    }
    
    /// Get time since last activity
    pub fn idle_time(&self) -> Duration {
        Instant::now().duration_since(self.last_activity)
    }
    
    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }
    
    /// Check if session can be used for operations
    pub fn is_ready(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
    
    /// Check if session needs initialization
    pub fn needs_initialization(&self) -> bool {
        matches!(self.state, SessionState::Uninitialized)
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Session manager handles session lifecycle and state
#[derive(Debug)]
pub struct SessionManager {
    /// Current session information
    session: Arc<RwLock<SessionInfo>>,
    
    /// Client configuration
    config: ClientConfig,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: ClientConfig) -> Self {
        Self {
            session: Arc::new(RwLock::new(SessionInfo::new())),
            config,
        }
    }
    
    /// Get current session information
    pub async fn session_info(&self) -> SessionInfo {
        self.session.read().await.clone()
    }
    
    /// Get session ID
    pub async fn session_id(&self) -> String {
        self.session.read().await.session_id.clone()
    }
    
    /// Get current session state
    pub async fn state(&self) -> SessionState {
        self.session.read().await.state.clone()
    }
    
    /// Update session state
    pub async fn set_state(&self, state: SessionState) {
        let mut session = self.session.write().await;
        debug!("Session state transition: {} -> {}", session.state, state);
        session.state = state;
        session.update_activity();
    }
    
    /// Check if session is ready for operations
    pub async fn is_ready(&self) -> bool {
        self.session.read().await.is_ready()
    }
    
    /// Initialize session with server capabilities
    pub async fn initialize(
        &self,
        client_capabilities: ClientCapabilities,
        server_capabilities: ServerCapabilities,
        protocol_version: String,
    ) -> McpClientResult<()> {
        let mut session = self.session.write().await;
        
        if !matches!(session.state, SessionState::Uninitialized | SessionState::Initializing) {
            return Err(SessionError::AlreadyInitialized.into());
        }
        
        session.client_capabilities = Some(client_capabilities);
        session.server_capabilities = Some(server_capabilities);
        session.protocol_version = Some(protocol_version.clone());
        session.state = SessionState::Active;
        session.update_activity();
        
        info!(
            session_id = %session.session_id,
            protocol_version = %protocol_version,
            "Session initialized successfully"
        );
        
        Ok(())
    }
    
    /// Mark session as initializing
    pub async fn mark_initializing(&self) -> McpClientResult<()> {
        let mut session = self.session.write().await;
        
        if !session.needs_initialization() {
            return Err(SessionError::AlreadyInitialized.into());
        }
        
        session.state = SessionState::Initializing;
        session.connection_attempts += 1;
        session.update_activity();
        
        debug!(
            session_id = %session.session_id,
            attempt = session.connection_attempts,
            "Session initialization started"
        );
        
        Ok(())
    }
    
    /// Terminate session
    pub async fn terminate(&self, reason: Option<String>) {
        let mut session = self.session.write().await;
        
        let previous_state = session.state.clone();
        session.state = SessionState::Terminated;
        session.update_activity();
        
        info!(
            session_id = %session.session_id,
            previous_state = %previous_state,
            reason = reason.as_deref().unwrap_or("user requested"),
            "Session terminated"
        );
    }
    
    /// Handle session error
    pub async fn handle_error(&self, error: String) {
        let mut session = self.session.write().await;
        
        let previous_state = session.state.clone();
        session.state = SessionState::Error(error.clone());
        session.update_activity();
        
        warn!(
            session_id = %session.session_id,
            previous_state = %previous_state,
            error = %error,
            "Session encountered error"
        );
    }
    
    /// Start reconnection process
    pub async fn start_reconnection(&self) {
        let mut session = self.session.write().await;
        
        if matches!(session.state, SessionState::Terminated) {
            return; // Cannot reconnect terminated sessions
        }
        
        session.state = SessionState::Reconnecting;
        session.connection_attempts += 1;
        session.update_activity();
        
        info!(
            session_id = %session.session_id,
            attempt = session.connection_attempts,
            "Session reconnection started"
        );
    }
    
    /// Reset session for new connection
    pub async fn reset(&self) {
        let mut session = self.session.write().await;
        *session = SessionInfo::new();
        
        debug!(
            session_id = %session.session_id,
            "Session reset for new connection"
        );
    }
    
    /// Update activity timestamp
    pub async fn update_activity(&self) {
        self.session.write().await.update_activity();
    }
    
    /// Get client capabilities for initialization
    pub fn create_client_capabilities(&self) -> ClientCapabilities {
        ClientCapabilities {
            experimental: None,
            sampling: None,
            elicitation: None,
            roots: None,
        }
    }
    
    /// Create initialization request
    pub async fn create_initialize_request(&self) -> InitializeRequest {
        let client_info = &self.config.client_info;
        
        InitializeRequest {
            protocol_version: "2025-06-18".to_string(),
            capabilities: self.create_client_capabilities(),
            client_info: Implementation {
                name: client_info.name.clone(),
                version: client_info.version.clone(),
                title: None,
            },
        }
    }
    
    /// Validate server capabilities
    pub async fn validate_server_capabilities(
        &self,
        server_capabilities: &ServerCapabilities,
    ) -> McpClientResult<()> {
        // Basic validation - ensure server supports required capabilities
        debug!(
            tools = ?server_capabilities.tools,
            resources = ?server_capabilities.resources,
            prompts = ?server_capabilities.prompts,
            "Validating server capabilities"
        );
        
        // For now, we accept any server capabilities
        // In the future, we might want to check for required features
        Ok(())
    }
    
    /// Get session statistics
    pub async fn statistics(&self) -> SessionStatistics {
        let session = self.session.read().await;
        
        SessionStatistics {
            session_id: session.session_id.clone(),
            state: session.state.clone(),
            duration: session.duration(),
            idle_time: session.idle_time(),
            connection_attempts: session.connection_attempts,
            protocol_version: session.protocol_version.clone(),
        }
    }
}

/// Session statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct SessionStatistics {
    pub session_id: String,
    pub state: SessionState,
    pub duration: Duration,
    pub idle_time: Duration,
    pub connection_attempts: u32,
    pub protocol_version: Option<String>,
}

impl SessionStatistics {
    /// Check if session is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.state, SessionState::Active) && self.idle_time < Duration::from_secs(300)
    }
    
    /// Get human-readable session status
    pub fn status_summary(&self) -> String {
        format!(
            "Session {} ({}) - Duration: {:?}, Idle: {:?}, Attempts: {}",
            &self.session_id[..8],
            self.state,
            self.duration,
            self.idle_time,
            self.connection_attempts
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClientConfig;
    
    #[tokio::test]
    async fn test_session_lifecycle() {
        let config = ClientConfig::default();
        let manager = SessionManager::new(config);
        
        // Initial state should be uninitialized
        assert_eq!(manager.state().await, SessionState::Uninitialized);
        assert!(!manager.is_ready().await);
        
        // Mark as initializing
        manager.mark_initializing().await.unwrap();
        assert_eq!(manager.state().await, SessionState::Initializing);
        
        // Initialize session
        let client_caps = manager.create_client_capabilities();
        let server_caps = ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tools: None,
            completions: None,
            elicitation: None,
        };
        
        manager.initialize(client_caps, server_caps, "2025-06-18".to_string()).await.unwrap();
        assert_eq!(manager.state().await, SessionState::Active);
        assert!(manager.is_ready().await);
        
        // Terminate session
        manager.terminate(Some("test completed".to_string())).await;
        assert_eq!(manager.state().await, SessionState::Terminated);
        assert!(!manager.is_ready().await);
    }
    
    #[tokio::test]
    async fn test_session_error_handling() {
        let config = ClientConfig::default();
        let manager = SessionManager::new(config);
        
        manager.handle_error("test error".to_string()).await;
        
        match manager.state().await {
            SessionState::Error(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected error state"),
        }
    }
    
    #[tokio::test]
    async fn test_session_reset() {
        let config = ClientConfig::default();
        let manager = SessionManager::new(config);
        
        let original_id = manager.session_id().await;
        
        manager.reset().await;
        
        let new_id = manager.session_id().await;
        assert_ne!(original_id, new_id);
        assert_eq!(manager.state().await, SessionState::Uninitialized);
    }
}