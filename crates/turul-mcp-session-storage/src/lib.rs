//! Session storage backends for MCP servers on the opt-in 2025-11-25 lane; a
//! 2026-07-28 server is stateless and needs none of this crate.
//!
//! The [`SessionStorage`] trait persists sessions, per-session state and
//! replayable SSE events. [`InMemorySessionStorage`] is the default backend;
//! SQLite, PostgreSQL and DynamoDB backends sit behind Cargo features.
//!
//! To use sessions, build [`turul-mcp-server`](https://docs.rs/turul-mcp-server)
//! with `--no-default-features --features http,sse,protocol-2025-11-25` and pass a
//! backend to `McpServerBuilder::with_session_storage`.
//!
//! # Example
//!
//! The storage API on its own:
//!
//! ```toml
//! [dependencies]
//! turul-mcp-session-storage = "0.4"
//! turul-mcp-protocol = "0.4"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ```rust
//! use turul_mcp_protocol::ServerCapabilities;
//! use turul_mcp_session_storage::prelude::*;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemorySessionStorage::new();
//! let session = storage.create_session(ServerCapabilities::default()).await?;
//! storage
//!     .set_session_state(&session.session_id, "greeting", serde_json::json!("hi"))
//!     .await?;
//! assert_eq!(storage.session_count().await?, 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! | Feature | Default | Backend |
//! |---|---|---|
//! | `in-memory` | yes | [`InMemorySessionStorage`]: one process, lost on restart |
//! | `sqlite` | no | `SqliteSessionStorage`: a local file |
//! | `postgres` | no | `PostgresSessionStorage`: shared across instances |
//! | `dynamodb` | no | `DynamoDbSessionStorage`: AWS serverless |
//! | `protocol-2026-07-28` | yes | Build against MCP 2026-07-28 types |
//! | `protocol-2025-11-25` | no | Build against MCP 2025-11-25 types |
//!
//! # Where next
//!
//! - [`SessionStorage`] for the trait a custom backend implements
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server) for the server builder
//! - [Examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)

// Core trait and types
mod traits;
/// Core session storage traits and types for pluggable backend implementations
pub use traits::*;

mod session_view;
pub use session_view::SessionView;

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
