//! Unified error types for server state storage operations.

/// Unified error type for server state storage operations.
///
/// Mirrors the pattern used in `turul-mcp-session-storage` and
/// `turul-mcp-task-storage` for consistency across storage crates.
#[derive(Debug, thiserror::Error)]
pub enum ServerStateError {
    /// No state exists for the requested entity.
    #[error("Entity not found: {entity_type}/{entity_id}")]
    EntityNotFound {
        /// Entity type namespace, such as `tools`.
        entity_type: String,
        /// Identifier of the entity within its type.
        entity_id: String,
    },

    /// No fingerprint is stored for the entity type named in the payload.
    #[error("Fingerprint not found for entity type: {0}")]
    FingerprintNotFound(String),

    /// The backing database reported a failure.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// A value could not be serialized or deserialized.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Another writer changed the state while this operation was in progress.
    #[error("Concurrent modification: {0}")]
    ConcurrentModification(String),

    /// The storage backend is misconfigured.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// A storage failure that fits no other variant.
    #[error("Generic storage error: {0}")]
    Generic(String),
}

impl From<serde_json::Error> for ServerStateError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}
