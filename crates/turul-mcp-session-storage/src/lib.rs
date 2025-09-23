//! # Session Storage Abstractions and Implementations
//!
//! **Pluggable session storage backends for MCP servers across deployment scenarios.**
//!
//! Provides the core SessionStorage trait with implementations for InMemory, SQLite,
//! PostgreSQL, and DynamoDB, enabling seamless scaling from development to production.
//!
//! [![Crates.io](https://img.shields.io/crates/v/turul-mcp-session-storage.svg)](https://crates.io/crates/turul-mcp-session-storage)
//! [![Documentation](https://docs.rs/turul-mcp-session-storage/badge.svg)](https://docs.rs/turul-mcp-session-storage)
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! turul-mcp-session-storage = "0.2"
//!
//! # Optional features for different backends
//! turul-mcp-session-storage = { version = "0.2", features = ["sqlite"] }
//! turul-mcp-session-storage = { version = "0.2", features = ["postgres"] }
//! turul-mcp-session-storage = { version = "0.2", features = ["dynamodb"] }
//! ```

// Core trait and types
mod traits;
/// Core session storage traits and types for pluggable backend implementations
pub use traits::*;

// Implementations
pub mod in_memory;
pub mod prelude;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "dynamodb")]
pub mod dynamodb;

// Re-export for convenience
/// In-memory session storage implementation for development and testing
pub use in_memory::{InMemoryConfig, InMemoryError, InMemorySessionStorage, InMemoryStats};

#[cfg(feature = "sqlite")]
/// SQLite-backed session storage for file-based persistence
pub use sqlite::{SqliteConfig, SqliteError, SqliteSessionStorage};

#[cfg(feature = "postgres")]
/// PostgreSQL-backed session storage for production deployments
pub use postgres::{PostgresConfig, PostgresError, PostgresSessionStorage};

#[cfg(feature = "dynamodb")]
/// DynamoDB-backed session storage for AWS serverless deployments
pub use dynamodb::{DynamoDbConfig, DynamoDbError, DynamoDbSessionStorage};

/// Convenience type alias for session storage results
pub type StorageResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Create a default in-memory session storage instance for development and testing
pub fn create_default_storage() -> InMemorySessionStorage {
    InMemorySessionStorage::new()
}

/// Create an in-memory session storage with custom configuration and cleanup settings
pub fn create_memory_storage(config: InMemoryConfig) -> InMemorySessionStorage {
    InMemorySessionStorage::with_config(config)
}

/// Create a SQLite session storage with default configuration and automatic schema setup
#[cfg(feature = "sqlite")]
pub async fn create_sqlite_storage() -> Result<SqliteSessionStorage, SqliteError> {
    SqliteSessionStorage::new().await
}

/// Create a SQLite session storage with custom database path and connection settings
#[cfg(feature = "sqlite")]
pub async fn create_sqlite_storage_with_config(
    config: SqliteConfig,
) -> Result<SqliteSessionStorage, SqliteError> {
    SqliteSessionStorage::with_config(config).await
}

/// Create a PostgreSQL session storage with default connection parameters from environment
#[cfg(feature = "postgres")]
pub async fn create_postgres_storage() -> Result<PostgresSessionStorage, PostgresError> {
    PostgresSessionStorage::new().await
}

/// Create a PostgreSQL session storage with custom database URL and connection pool settings
#[cfg(feature = "postgres")]
pub async fn create_postgres_storage_with_config(
    config: PostgresConfig,
) -> Result<PostgresSessionStorage, PostgresError> {
    PostgresSessionStorage::with_config(config).await
}

/// Create a DynamoDB session storage with default AWS configuration and table names
#[cfg(feature = "dynamodb")]
pub async fn create_dynamodb_storage() -> Result<DynamoDbSessionStorage, DynamoDbError> {
    DynamoDbSessionStorage::new().await
}

/// Create a DynamoDB session storage with custom AWS region and table configuration
#[cfg(feature = "dynamodb")]
pub async fn create_dynamodb_storage_with_config(
    config: DynamoDbConfig,
) -> Result<DynamoDbSessionStorage, DynamoDbError> {
    DynamoDbSessionStorage::with_config(config).await
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use turul_mcp_protocol::ServerCapabilities;

    #[tokio::test]
    async fn test_storage_trait_compliance() {
        let storage = create_default_storage();

        // Test that our storage implements all trait methods
        let session = storage
            .create_session(ServerCapabilities::default())
            .await
            .unwrap();
        let session_id = session.session_id.clone();

        // Session operations
        assert!(storage.get_session(&session_id).await.unwrap().is_some());
        assert_eq!(storage.session_count().await.unwrap(), 1);

        // State operations
        storage
            .set_session_state(&session_id, "test", serde_json::json!("value"))
            .await
            .unwrap();
        let value = storage
            .get_session_state(&session_id, "test")
            .await
            .unwrap();
        assert_eq!(value, Some(serde_json::json!("value")));

        // Event operations
        let event = crate::SseEvent::new("test".to_string(), serde_json::json!({"data": "test"}));
        let stored = storage.store_event(&session_id, event).await.unwrap();
        assert!(stored.id > 0);

        // Cleanup
        let deleted = storage.delete_session(&session_id).await.unwrap();
        assert!(deleted);
        assert_eq!(storage.session_count().await.unwrap(), 0);
    }
}
