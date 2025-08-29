//! Session Storage Abstractions and Implementations
//!
//! This crate provides the core SessionStorage trait that enables pluggable
//! session backends for different deployment scenarios.

// Core trait and types
mod traits;
pub use traits::*;

// Implementations
pub mod in_memory;

#[cfg(feature = "sqlite")]
pub mod sqlite;

// Re-export for convenience
pub use in_memory::{InMemorySessionStorage, InMemoryConfig, InMemoryError, InMemoryStats};

#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteSessionStorage, SqliteConfig, SqliteError};

/// Convenience type alias for session storage results
pub type StorageResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Create a default in-memory session storage instance
pub fn create_default_storage() -> InMemorySessionStorage {
    InMemorySessionStorage::new()
}

/// Create an in-memory session storage with custom configuration
pub fn create_memory_storage(config: InMemoryConfig) -> InMemorySessionStorage {
    InMemorySessionStorage::with_config(config)
}

/// Create a SQLite session storage with default configuration
#[cfg(feature = "sqlite")]
pub async fn create_sqlite_storage() -> Result<SqliteSessionStorage, SqliteError> {
    SqliteSessionStorage::new().await
}

/// Create a SQLite session storage with custom configuration
#[cfg(feature = "sqlite")]
pub async fn create_sqlite_storage_with_config(config: SqliteConfig) -> Result<SqliteSessionStorage, SqliteError> {
    SqliteSessionStorage::with_config(config).await
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use mcp_protocol::ServerCapabilities;

    #[tokio::test]
    async fn test_storage_trait_compliance() {
        let storage = create_default_storage();
        
        // Test that our storage implements all trait methods
        let session = storage.create_session(ServerCapabilities::default()).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Session operations
        assert!(storage.get_session(&session_id).await.unwrap().is_some());
        assert_eq!(storage.session_count().await.unwrap(), 1);
        
        // State operations
        storage.set_session_state(&session_id, "test", serde_json::json!("value")).await.unwrap();
        let value = storage.get_session_state(&session_id, "test").await.unwrap();
        assert_eq!(value, Some(serde_json::json!("value")));
        
        // Event operations
        let event = crate::SseEvent::new(
            "test".to_string(), 
            serde_json::json!({"data": "test"})
        );
        let stored = storage.store_event(&session_id, event).await.unwrap();
        assert!(stored.id > 0);
        
        // Cleanup
        let deleted = storage.delete_session(&session_id).await.unwrap();
        assert!(deleted);
        assert_eq!(storage.session_count().await.unwrap(), 0);
    }
}