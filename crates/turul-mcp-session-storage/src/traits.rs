//! Session Storage Trait and Implementations
//!
//! This module provides the core SessionStorage trait abstraction that enables
//! pluggable session backends for different deployment scenarios:
//! - InMemory: Development and testing
//! - SQLite: Single-instance production
//! - PostgreSQL: Multi-instance production
//! - NATS: Distributed with JetStream
//! - AWS: DynamoDB + SNS for Lambda/serverless

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use turul_mcp_protocol::{ClientCapabilities, ServerCapabilities};

// Note: SessionEvent removed to avoid circular dependency

/// Comprehensive session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier (UUID v7 for temporal ordering)
    pub session_id: String,
    /// Client capabilities negotiated during initialization
    pub client_capabilities: Option<ClientCapabilities>,
    /// Server capabilities provided during initialization
    pub server_capabilities: Option<ServerCapabilities>,
    /// Session state key-value store
    pub state: HashMap<String, Value>,
    /// Session creation timestamp (Unix millis)
    pub created_at: u64,
    /// Last activity timestamp (Unix millis)
    pub last_activity: u64,
    /// Whether session has completed MCP initialization
    pub is_initialized: bool,
    /// Session metadata (connection info, user agent, etc.)
    pub metadata: HashMap<String, Value>,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionInfo {
    /// Create a new session with UUID v7 for temporal ordering
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            session_id: Uuid::now_v7().to_string(),
            client_capabilities: None,
            server_capabilities: None,
            state: HashMap::new(),
            created_at: now,
            last_activity: now,
            is_initialized: false,
            metadata: HashMap::new(),
        }
    }

    /// Create session with specific ID (for testing)
    pub fn with_id(session_id: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            session_id,
            client_capabilities: None,
            server_capabilities: None,
            state: HashMap::new(),
            created_at: now,
            last_activity: now,
            is_initialized: false,
            metadata: HashMap::new(),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = chrono::Utc::now().timestamp_millis() as u64;
    }

    /// Check if session is expired based on timeout
    pub fn is_expired(&self, timeout_minutes: u64) -> bool {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let timeout_millis = timeout_minutes * 60 * 1000;
        now - self.last_activity > timeout_millis
    }
}

/// SSE event with proper metadata for resumability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    /// Monotonic event ID for ordering and resumability
    pub id: u64,
    /// Event timestamp (Unix millis)
    pub timestamp: u64,
    /// Event type for client-side filtering
    pub event_type: String,
    /// Event data payload
    pub data: Value,
    /// Retry timeout in milliseconds (optional)
    pub retry: Option<u32>,
}

impl SseEvent {
    /// Create new event with auto-generated ID
    pub fn new(event_type: String, data: Value) -> Self {
        Self {
            id: 0, // Will be set by SessionStorage when storing
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            event_type,
            data,
            retry: None,
        }
    }

    /// Format as SSE message for HTTP response
    ///
    /// MCP Inspector and the official TypeScript SDK only process SSE events
    /// with no event name or "message". Custom event names are discarded.
    /// We use "message" for all JSON-RPC notifications to ensure compatibility.
    pub fn format(&self) -> String {
        let mut result = String::new();

        // Event ID for resumability
        result.push_str(&format!("id: {}\n", self.id));

        // Event type - MCP Inspector only processes "message" or empty event names
        // Use "message" for JSON-RPC notifications, empty for keepalives
        if self.event_type == "ping" || self.event_type == "keepalive" {
            // Omit event line for keepalives (default event type)
        } else {
            // Use "message" for all JSON-RPC notifications (MCP Inspector compatible)
            result.push_str("event: message\n");
        }

        // Event data (JSON)
        if let Ok(data_str) = serde_json::to_string(&self.data) {
            result.push_str(&format!("data: {}\n", data_str));
        } else {
            result.push_str("data: {}\n");
        }

        // Retry timeout if specified
        if let Some(retry) = self.retry {
            result.push_str(&format!("retry: {}\n", retry));
        }

        // End of event
        result.push('\n');

        result
    }
}

/// Core trait for session storage backends
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Error type for storage operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get the backend name for logging and debugging
    fn backend_name(&self) -> &'static str;

    // ============================================================================
    // Session Management
    // ============================================================================

    /// Create a new session with automatically generated UUID v7
    ///
    /// **USE THIS METHOD** for:
    /// - Production code
    /// - Normal server operations
    /// - Tests that don't need specific session IDs
    ///
    /// The session ID is generated using `Uuid::now_v7()` which provides:
    /// - Temporal ordering (sessions created later have higher IDs)
    /// - Better database performance vs UUID v4
    /// - Collision resistance
    async fn create_session(
        &self,
        capabilities: ServerCapabilities,
    ) -> Result<SessionInfo, Self::Error>;

    /// Create session with a specific session ID
    ///
    /// **ONLY USE THIS METHOD** for:
    /// - Unit tests that need predictable session IDs
    /// - Integration tests that need to correlate sessions
    /// - Migration scenarios from other session systems
    ///
    /// **DO NOT USE** for:
    /// - Production code (use `create_session()` instead)
    /// - Normal server operations
    /// - Tests that don't specifically need custom session IDs
    ///
    /// # Example
    /// ```rust,no_run
    /// use turul_mcp_session_storage::{SessionStorage, SessionStorageError};
    /// use turul_mcp_protocol::ServerCapabilities;
    ///
    /// # async fn example(storage: &dyn SessionStorage<Error = SessionStorageError>) -> Result<(), Box<dyn std::error::Error>> {
    /// let caps = ServerCapabilities::default();
    ///
    /// // ✅ CORRECT - Let the storage assign an ID
    /// let _session = storage.create_session(caps.clone()).await?;
    ///
    /// // ❌ WRONG - Never generate synthetic IDs outside tests
    /// // storage.create_session_with_id(Uuid::now_v7().to_string(), caps).await?;
    ///
    /// // Only use create_session_with_id for testing specific session behavior:
    /// // let _test_session = storage.create_session_with_id("test-session-123".to_string(), caps).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create_session_with_id(
        &self,
        session_id: String,
        capabilities: ServerCapabilities,
    ) -> Result<SessionInfo, Self::Error>;

    /// Get session by ID
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error>;

    /// Update entire session info
    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error>;

    /// Update session state value
    async fn set_session_state(
        &self,
        session_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Self::Error>;

    /// Get session state value
    async fn get_session_state(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<Value>, Self::Error>;

    /// Remove session state value
    async fn remove_session_state(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<Value>, Self::Error>;

    /// Delete session completely
    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error>;

    /// List all session IDs
    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error>;

    // ============================================================================
    // Event Management (for SSE resumability)
    // ============================================================================

    /// Store an event for a session (assigns unique event ID)
    async fn store_event(&self, session_id: &str, event: SseEvent)
    -> Result<SseEvent, Self::Error>;

    /// Get events after a specific event ID (for resumability)
    async fn get_events_after(
        &self,
        session_id: &str,
        after_event_id: u64,
    ) -> Result<Vec<SseEvent>, Self::Error>;

    /// Get recent events (for initial connection)
    async fn get_recent_events(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SseEvent>, Self::Error>;

    /// Delete old events (cleanup)
    async fn delete_events_before(
        &self,
        session_id: &str,
        before_event_id: u64,
    ) -> Result<u64, Self::Error>;

    // ============================================================================
    // Cleanup and Maintenance
    // ============================================================================

    /// Remove expired sessions (returns list of removed session IDs)
    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>, Self::Error>;

    /// Get session count for monitoring
    async fn session_count(&self) -> Result<usize, Self::Error>;

    /// Get total event count across all sessions
    async fn event_count(&self) -> Result<usize, Self::Error>;

    /// Perform maintenance tasks (compaction, cleanup, etc.)
    async fn maintenance(&self) -> Result<(), Self::Error>;
}

/// Result type for session storage operations
pub type SessionResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Unified error type for all session storage backends
#[derive(Debug, thiserror::Error)]
pub enum SessionStorageError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Maximum sessions limit reached: {0}")]
    MaxSessionsReached(usize),

    #[error("Maximum events limit reached: {0}")]
    MaxEventsReached(usize),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("AWS SDK error: {0}")]
    AwsError(String),

    #[error("AWS configuration error: {0}")]
    AwsConfigurationError(String),

    #[error("DynamoDB table does not exist: {0}")]
    TableNotFound(String),

    #[error("Invalid session data: {0}")]
    InvalidData(String),

    #[error("Concurrent modification error: {0}")]
    ConcurrentModification(String),

    #[error("Generic storage error: {0}")]
    Generic(String),
}

// Direct conversions from common error types
impl From<serde_json::Error> for SessionStorageError {
    fn from(err: serde_json::Error) -> Self {
        SessionStorageError::SerializationError(err.to_string())
    }
}

#[cfg(feature = "sqlite")]
impl From<sqlx::Error> for SessionStorageError {
    fn from(err: sqlx::Error) -> Self {
        SessionStorageError::DatabaseError(err.to_string())
    }
}

// Conversion implementations for backend-specific errors
impl From<crate::in_memory::InMemoryError> for SessionStorageError {
    fn from(err: crate::in_memory::InMemoryError) -> Self {
        match err {
            crate::in_memory::InMemoryError::SessionNotFound(id) => {
                SessionStorageError::SessionNotFound(id)
            }
            crate::in_memory::InMemoryError::MaxSessionsReached(limit) => {
                SessionStorageError::MaxSessionsReached(limit)
            }
            crate::in_memory::InMemoryError::MaxEventsReached(limit) => {
                SessionStorageError::MaxEventsReached(limit)
            }
            crate::in_memory::InMemoryError::SerializationError(e) => {
                SessionStorageError::SerializationError(e.to_string())
            }
        }
    }
}

#[cfg(feature = "sqlite")]
impl From<crate::sqlite::SqliteError> for SessionStorageError {
    fn from(err: crate::sqlite::SqliteError) -> Self {
        match err {
            crate::sqlite::SqliteError::Database(e) => {
                SessionStorageError::DatabaseError(e.to_string())
            }
            crate::sqlite::SqliteError::Serialization(e) => {
                SessionStorageError::SerializationError(e.to_string())
            }
            crate::sqlite::SqliteError::SessionNotFound(id) => {
                SessionStorageError::SessionNotFound(id)
            }
            crate::sqlite::SqliteError::Connection(e) => SessionStorageError::ConnectionError(e),
            crate::sqlite::SqliteError::Migration(e) => SessionStorageError::MigrationError(e),
        }
    }
}

#[cfg(feature = "postgres")]
impl From<crate::postgres::PostgresError> for SessionStorageError {
    fn from(err: crate::postgres::PostgresError) -> Self {
        match err {
            crate::postgres::PostgresError::Database(e) => {
                SessionStorageError::DatabaseError(e.to_string())
            }
            crate::postgres::PostgresError::Serialization(e) => {
                SessionStorageError::SerializationError(e.to_string())
            }
            crate::postgres::PostgresError::SessionNotFound(id) => {
                SessionStorageError::SessionNotFound(id)
            }
            crate::postgres::PostgresError::Connection(e) => {
                SessionStorageError::ConnectionError(e)
            }
            crate::postgres::PostgresError::Migration(e) => SessionStorageError::MigrationError(e),
            crate::postgres::PostgresError::ConcurrentModification(e) => {
                SessionStorageError::ConcurrentModification(e)
            }
        }
    }
}

#[cfg(feature = "dynamodb")]
impl From<crate::dynamodb::DynamoDbError> for SessionStorageError {
    fn from(err: crate::dynamodb::DynamoDbError) -> Self {
        match err {
            crate::dynamodb::DynamoDbError::AwsError(e) => SessionStorageError::AwsError(e),
            crate::dynamodb::DynamoDbError::SerializationError(e) => {
                SessionStorageError::SerializationError(e.to_string())
            }
            crate::dynamodb::DynamoDbError::SessionNotFound(id) => {
                SessionStorageError::SessionNotFound(id)
            }
            crate::dynamodb::DynamoDbError::InvalidSessionData(e) => {
                SessionStorageError::InvalidData(e)
            }
            crate::dynamodb::DynamoDbError::TableNotFound(table) => {
                SessionStorageError::TableNotFound(table)
            }
            crate::dynamodb::DynamoDbError::ConfigError(e) => {
                SessionStorageError::AwsConfigurationError(e)
            }
        }
    }
}

/// Type alias for boxed session storage trait object with unified error type
pub type BoxedSessionStorage = dyn SessionStorage<Error = SessionStorageError>;

/// Convenience trait for creating session storage instances
pub trait SessionStorageBuilder {
    type Storage: SessionStorage;
    type Config;
    type Error: std::error::Error + Send + Sync + 'static;

    fn build(config: Self::Config) -> Result<Self::Storage, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_creation() {
        let session = SessionInfo::new();
        assert!(!session.session_id.is_empty());
        assert!(!session.is_initialized);
        assert!(session.state.is_empty());
    }

    #[test]
    fn test_session_expiration() {
        let mut session = SessionInfo::new();
        assert!(!session.is_expired(30)); // 30 minute timeout

        // Simulate old session
        session.last_activity = chrono::Utc::now().timestamp_millis() as u64 - (31 * 60 * 1000);
        assert!(session.is_expired(30));
    }

    #[test]
    fn test_sse_event_formatting() {
        let mut event = SseEvent {
            id: 123,
            timestamp: 1234567890,
            event_type: "data".to_string(),
            data: serde_json::json!({"message": "test"}),
            retry: Some(1000),
        };
        event.id = 123; // Set ID directly

        let formatted = event.format();
        assert!(formatted.contains("id: 123"));
        assert!(formatted.contains("event: data"));
        assert!(formatted.contains("retry: 1000"));
        assert!(formatted.contains("data: {\"message\":\"test\"}"));
    }
}
