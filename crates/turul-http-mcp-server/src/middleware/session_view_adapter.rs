//! SessionView adapter for BoxedSessionStorage
//!
//! Provides a SessionView implementation that bridges between middleware
//! and the session storage backend. This allows middleware to read/write
//! session state without needing to know about storage implementation details.

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use turul_mcp_session_storage::{BoxedSessionStorage, SessionView};

/// SessionView adapter backed by BoxedSessionStorage
///
/// This adapter implements the SessionView trait by delegating to the underlying
/// session storage backend. It handles:
/// - Reading/writing session state
/// - Reading/writing session metadata (with `__meta__:` prefix)
/// - Error conversion from storage errors to String
///
/// # Architecture
///
/// ```text
/// Middleware → SessionView → StorageBackedSessionView → BoxedSessionStorage
/// ```
///
/// # Example
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use turul_mcp_session_storage::InMemorySessionStorage;
/// use turul_http_mcp_server::middleware::StorageBackedSessionView;
///
/// # async fn example() {
/// let storage = Arc::new(InMemorySessionStorage::new());
/// let session_id = "test-session-123".to_string();
///
/// // Create adapter
/// let session_view = StorageBackedSessionView::new(session_id, storage);
///
/// // Middleware can now use SessionView trait methods
/// // session_view.get_state("key").await
/// # }
/// ```
pub struct StorageBackedSessionView {
    session_id: String,
    storage: Arc<BoxedSessionStorage>,
}

impl StorageBackedSessionView {
    /// Create a new session view adapter
    ///
    /// # Parameters
    ///
    /// - `session_id`: The session ID to operate on
    /// - `storage`: The storage backend to delegate to
    pub fn new(session_id: String, storage: Arc<BoxedSessionStorage>) -> Self {
        Self {
            session_id,
            storage,
        }
    }
}

#[async_trait]
impl SessionView for StorageBackedSessionView {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    async fn get_state(&self, key: &str) -> Result<Option<Value>, String> {
        // Get session from storage
        let session = self
            .storage
            .get_session(&self.session_id)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        // Extract state value
        Ok(session.and_then(|s| s.state.get(key).cloned()))
    }

    async fn set_state(&self, key: &str, value: Value) -> Result<(), String> {
        // Get current session
        let mut session = self
            .storage
            .get_session(&self.session_id)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?
            .ok_or_else(|| format!("Session '{}' not found", self.session_id))?;

        // Update state
        session.state.insert(key.to_string(), value);

        // Update last activity timestamp
        session.last_activity = chrono::Utc::now().timestamp_millis() as u64;

        // Write back to storage
        self.storage
            .update_session(session)
            .await
            .map_err(|e| format!("Failed to update session: {}", e))
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<Value>, String> {
        // Metadata is stored in session.metadata with __meta__: prefix for consistency
        // with turul-mcp-server's SessionContext pattern
        let prefixed_key = format!("__meta__:{}", key);

        let session = self
            .storage
            .get_session(&self.session_id)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        Ok(session.and_then(|s| s.metadata.get(&prefixed_key).cloned()))
    }

    async fn set_metadata(&self, key: &str, value: Value) -> Result<(), String> {
        // Metadata is stored in session.metadata with __meta__: prefix
        let prefixed_key = format!("__meta__:{}", key);

        let mut session = self
            .storage
            .get_session(&self.session_id)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?
            .ok_or_else(|| format!("Session '{}' not found", self.session_id))?;

        // Update metadata
        session.metadata.insert(prefixed_key, value);

        // Update last activity timestamp
        session.last_activity = chrono::Utc::now().timestamp_millis() as u64;

        // Write back to storage
        self.storage
            .update_session(session)
            .await
            .map_err(|e| format!("Failed to update session: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turul_mcp_protocol::ServerCapabilities;
    use turul_mcp_session_storage::{BoxedSessionStorage, InMemorySessionStorage};

    #[tokio::test]
    async fn test_session_view_state() {
        let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());

        // Create a session
        let session_info = storage
            .create_session(ServerCapabilities::default())
            .await
            .unwrap();

        let session_id = session_info.session_id.clone();

        // Create adapter
        let view = StorageBackedSessionView::new(session_id.clone(), Arc::clone(&storage));

        // Test state operations
        assert_eq!(view.get_state("key1").await.unwrap(), None);

        view.set_state("key1", json!("value1")).await.unwrap();
        assert_eq!(view.get_state("key1").await.unwrap(), Some(json!("value1")));

        view.set_state("key2", json!({"nested": "object"}))
            .await
            .unwrap();
        assert_eq!(
            view.get_state("key2").await.unwrap(),
            Some(json!({"nested": "object"}))
        );
    }

    #[tokio::test]
    async fn test_session_view_metadata() {
        let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());

        let session_info = storage
            .create_session(ServerCapabilities::default())
            .await
            .unwrap();

        let session_id = session_info.session_id.clone();
        let view = StorageBackedSessionView::new(session_id.clone(), Arc::clone(&storage));

        // Test metadata operations
        assert_eq!(view.get_metadata("meta1").await.unwrap(), None);

        view.set_metadata("meta1", json!("metadata_value"))
            .await
            .unwrap();
        assert_eq!(
            view.get_metadata("meta1").await.unwrap(),
            Some(json!("metadata_value"))
        );

        // Verify metadata is stored with __meta__: prefix in underlying storage
        let session = storage.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(
            session.metadata.get("__meta__:meta1"),
            Some(&json!("metadata_value"))
        );
    }

    #[tokio::test]
    async fn test_session_view_session_id() {
        let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());
        let session_info = storage
            .create_session(ServerCapabilities::default())
            .await
            .unwrap();

        let view =
            StorageBackedSessionView::new(session_info.session_id.clone(), Arc::clone(&storage));

        assert_eq!(view.session_id(), &session_info.session_id);
    }

    #[tokio::test]
    async fn test_session_view_nonexistent_session() {
        let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());
        let view = StorageBackedSessionView::new("nonexistent".to_string(), Arc::clone(&storage));

        // Getting state from nonexistent session returns None
        assert_eq!(view.get_state("key").await.unwrap(), None);

        // Setting state on nonexistent session fails
        let result = view.set_state("key", json!("value")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
