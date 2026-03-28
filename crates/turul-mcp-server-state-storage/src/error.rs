//! Unified error types for server state storage operations.

/// Unified error type for server state storage operations.
///
/// Mirrors the pattern used in `turul-mcp-session-storage` and
/// `turul-mcp-task-storage` for consistency across storage crates.
#[derive(Debug, thiserror::Error)]
pub enum ServerStateError {
    #[error("Entity not found: {entity_type}/{entity_id}")]
    EntityNotFound {
        entity_type: String,
        entity_id: String,
    },

    #[error("Fingerprint not found for entity type: {0}")]
    FingerprintNotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Concurrent modification: {0}")]
    ConcurrentModification(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Generic storage error: {0}")]
    Generic(String),
}

impl From<serde_json::Error> for ServerStateError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}
