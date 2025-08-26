//! Session Storage Abstraction and Implementations
//!
//! This module provides the core SessionStorage trait that enables pluggable
//! session backends for different deployment scenarios.

// Core trait and types
mod traits;
pub use traits::*;

// Implementations
pub mod in_memory;

// Re-export for convenience
pub use in_memory::{InMemorySessionStorage, InMemoryConfig, InMemoryError, InMemoryStats};

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
        
        // Stream operations
        let stream = storage.create_stream(&session_id, "stream1".to_string()).await.unwrap();
        assert_eq!(stream.stream_id, "stream1");
        
        // Event operations
        let event = crate::session_storage::SseEvent::new(
            "stream1".to_string(),
            "test".to_string(), 
            serde_json::json!({"data": "test"})
        );
        let stored = storage.store_event(&session_id, "stream1", event).await.unwrap();
        assert!(stored.id > 0);
        
        // Cleanup
        let deleted = storage.delete_session(&session_id).await.unwrap();
        assert!(deleted);
        assert_eq!(storage.session_count().await.unwrap(), 0);
    }
}