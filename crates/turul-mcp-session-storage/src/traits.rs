//! Core `SessionStorage` trait, its session and SSE event types, and the unified error type.
//!
//! The trait lets the server use pluggable session backends:
//! - InMemory: development and testing
//! - SQLite: single-instance deployments
//! - PostgreSQL: multi-instance deployments
//! - DynamoDB: Lambda and other serverless deployments

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use turul_mcp_protocol::{ClientCapabilities, ServerCapabilities};

/// Persisted state of one session: identity, capabilities, key-value state and timestamps.
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
    /// Creates a new session whose ID is a UUID v7 (no hyphens) for temporal ordering.
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            session_id: Uuid::now_v7().as_simple().to_string(),
            client_capabilities: None,
            server_capabilities: None,
            state: HashMap::new(),
            created_at: now,
            last_activity: now,
            is_initialized: false,
            metadata: HashMap::new(),
        }
    }

    /// Creates a session with the given ID instead of a generated one.
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

    /// Sets `last_activity` to the current time.
    pub fn touch(&mut self) {
        self.last_activity = chrono::Utc::now().timestamp_millis() as u64;
    }

    /// Returns whether the session's `terminated` state flag is set to `true`.
    pub fn is_terminated(&self) -> bool {
        self.state
            .get("terminated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Returns whether more than `timeout_minutes` have passed since `last_activity`.
    pub fn is_expired(&self, timeout_minutes: u64) -> bool {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let timeout_millis = timeout_minutes * 60 * 1000;
        now - self.last_activity > timeout_millis
    }
}

/// SSE event carrying the metadata needed for resumability.
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
    /// Creates an event with the current timestamp and no retry hint.
    ///
    /// The ID is `0` until a `SessionStorage` assigns one in `store_event`.
    pub fn new(event_type: String, data: Value) -> Self {
        Self {
            id: 0, // Will be set by SessionStorage when storing
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            event_type,
            data,
            retry: None,
        }
    }

    /// Formats the event as an SSE message for an HTTP response.
    ///
    /// MCP Inspector and the official TypeScript SDK only process SSE events
    /// with no event name or "message". Custom event names are discarded.
    /// We use "message" for all JSON-RPC notifications to ensure compatibility.
    ///
    /// Keepalive events use comment syntax (": keepalive\n\n") to preserve
    /// Last-Event-ID for MCP resumability. Including "id: 0" would reset the client's
    /// Last-Event-ID, causing full event replay on reconnection.
    pub fn format(&self) -> String {
        if self.event_type == "ping" || self.event_type == "keepalive" {
            return ": keepalive\n\n".to_string();
        }

        let mut result = String::new();

        result.push_str(&format!("id: {}\n", self.id));

        result.push_str("event: message\n");

        if let Ok(data_str) = serde_json::to_string(&self.data) {
            result.push_str(&format!("data: {}\n", data_str));
        } else {
            result.push_str("data: {}\n");
        }

        if let Some(retry) = self.retry {
            result.push_str(&format!("retry: {}\n", retry));
        }

        result.push('\n');

        result
    }
}

/// Core trait for session storage backends.
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Error type for storage operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Returns the backend name for logging and debugging.
    fn backend_name(&self) -> &'static str;

    // ============================================================================
    // Session Management
    // ============================================================================

    /// Creates a new session with an automatically generated UUID v7 ID.
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

    /// Creates a session with the given session ID.
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
    /// # Examples
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
    /// // storage.create_session_with_id(Uuid::now_v7().as_simple().to_string(), caps).await?;
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

    /// Returns the session with the given ID, or `None` if it does not exist.
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error>;

    /// Replaces the stored session info.
    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error>;

    /// Sets a session state value, creating or overwriting the key.
    async fn set_session_state(
        &self,
        session_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Self::Error>;

    /// Returns a session state value, or `None` if the key is not set.
    async fn get_session_state(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<Value>, Self::Error>;

    /// Removes a session state value and returns it, or `None` if the key was not set.
    async fn remove_session_state(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<Value>, Self::Error>;

    /// Deletes the session; returns whether it existed.
    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error>;

    /// Lists all session IDs.
    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error>;

    // ============================================================================
    // Event Management (for SSE resumability)
    // ============================================================================

    /// Stores an event for a session and returns it with its assigned event ID.
    async fn store_event(&self, session_id: &str, event: SseEvent)
    -> Result<SseEvent, Self::Error>;

    /// Returns events with an ID greater than `after_event_id`, for resumability.
    async fn get_events_after(
        &self,
        session_id: &str,
        after_event_id: u64,
    ) -> Result<Vec<SseEvent>, Self::Error>;

    /// Returns up to `limit` of the most recent events, for an initial connection.
    async fn get_recent_events(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SseEvent>, Self::Error>;

    /// Deletes events with an ID below `before_event_id`; returns the number deleted.
    async fn delete_events_before(
        &self,
        session_id: &str,
        before_event_id: u64,
    ) -> Result<u64, Self::Error>;

    // ============================================================================
    // Cleanup and Maintenance
    // ============================================================================

    /// Removes sessions whose last activity is before `older_than` and returns their IDs.
    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>, Self::Error>;

    /// Returns the number of sessions, for monitoring.
    async fn session_count(&self) -> Result<usize, Self::Error>;

    /// Returns the total number of events across all sessions.
    async fn event_count(&self) -> Result<usize, Self::Error>;

    /// Performs backend maintenance such as cleanup and compaction.
    async fn maintenance(&self) -> Result<(), Self::Error>;
}

/// Boxed-error result type for session storage operations.
pub type SessionResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Unified error type for all session storage backends.
#[derive(Debug, thiserror::Error)]
pub enum SessionStorageError {
    #[error("Session not found: {0}")]
    /// The requested session id does not exist in the backend.
    SessionNotFound(String),

    #[error("Maximum sessions limit reached: {0}")]
    /// Creating a session would exceed the backend's configured maximum number of sessions; carries the limit.
    MaxSessionsReached(usize),

    #[error("Maximum events limit reached: {0}")]
    /// Appending an event would exceed the configured maximum number of events per session; carries the limit.
    MaxEventsReached(usize),

    #[error("Database error: {0}")]
    /// A database or DynamoDB operation failed; carries the backend message.
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    /// A value could not be serialized or deserialized; carries the serializer message.
    SerializationError(String),

    #[error("Connection error: {0}")]
    /// The backend could not connect to its database or service; carries the failure message.
    ConnectionError(String),

    #[error("Migration error: {0}")]
    /// Applying a schema migration failed; carries the failure message.
    MigrationError(String),

    #[error("AWS SDK error: {0}")]
    /// An AWS SDK call failed; carries the SDK error message.
    AwsError(String),

    #[error("AWS configuration error: {0}")]
    /// The AWS client or backend configuration is invalid or incomplete; carries the failure message.
    AwsConfigurationError(String),

    #[error("DynamoDB table does not exist: {0}")]
    /// The configured DynamoDB table does not exist or its status could not be read; carries a message naming the table.
    TableNotFound(String),

    #[error("Invalid session data: {0}")]
    /// Stored session data is malformed or fails validation; carries a description of the problem.
    InvalidData(String),

    #[error("Concurrent modification error: {0}")]
    /// Another writer modified the same record concurrently; carries a description of the conflict.
    ConcurrentModification(String),

    #[error("Generic storage error: {0}")]
    /// An error that does not fit another variant; carries the message.
    Generic(String),
}

// Direct conversions from common error types
impl From<serde_json::Error> for SessionStorageError {
    fn from(err: serde_json::Error) -> Self {
        SessionStorageError::SerializationError(err.to_string())
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres"))]
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

/// Trait object for a session storage using the unified `SessionStorageError`.
pub type BoxedSessionStorage = dyn SessionStorage<Error = SessionStorageError>;

/// Builds a session storage from a backend-specific config.
pub trait SessionStorageBuilder {
    /// The storage type produced by `build`.
    type Storage: SessionStorage;
    /// The backend-specific configuration consumed by `build`.
    type Config;
    /// The error returned when `build` fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Builds a storage from `config`, returning the backend error when construction fails.
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
        // Test regular event formatting (should emit "event: message")
        let mut event = SseEvent {
            id: 123,
            timestamp: 1234567890,
            event_type: "notifications/progress".to_string(),
            data: serde_json::json!({"message": "test"}),
            retry: Some(1000),
        };
        event.id = 123; // Set ID directly

        let formatted = event.format();
        assert!(formatted.contains("id: 123"));
        assert!(formatted.contains("event: message")); // Always "message" for MCP Inspector compatibility
        assert!(formatted.contains("retry: 1000"));
        assert!(formatted.contains("data: {\"message\":\"test\"}"));

        // Test keepalive event formatting (should use comment syntax for MCP resumability)
        let keepalive = SseEvent {
            id: 0,
            timestamp: 1234567890,
            event_type: "ping".to_string(),
            data: serde_json::json!({"type": "keepalive"}),
            retry: None,
        };

        let keepalive_formatted = keepalive.format();
        // Keepalives use comment syntax to preserve Last-Event-ID
        assert_eq!(keepalive_formatted, ": keepalive\n\n");
        assert!(keepalive_formatted.starts_with(":")); // Comment-style
        assert!(!keepalive_formatted.contains("id:")); // No ID field
        assert!(!keepalive_formatted.contains("event:")); // No event field
        assert!(!keepalive_formatted.contains("data:")); // No data field
    }

    #[test]
    fn test_is_terminated_default_false() {
        let session = SessionInfo::new();
        assert!(!session.is_terminated());
    }

    #[test]
    fn test_is_terminated_true() {
        let mut session = SessionInfo::new();
        session
            .state
            .insert("terminated".to_string(), Value::Bool(true));
        assert!(session.is_terminated());
    }

    #[test]
    fn test_is_terminated_false_value() {
        let mut session = SessionInfo::new();
        session
            .state
            .insert("terminated".to_string(), Value::Bool(false));
        assert!(!session.is_terminated());
    }
}
