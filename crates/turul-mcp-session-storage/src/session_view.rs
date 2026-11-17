//! Defines [`SessionView`], the minimal interface for middleware to access session
//! state without depending on the concrete `SessionContext` implementation in
//! `turul-mcp-server`.

use async_trait::async_trait;
use serde_json::Value;

/// Provides middleware a read-write view of session state.
///
/// The view hides the full `SessionContext` implementation details.
///
/// Implementations used by the server may return `Ok(())` from `set_state` and
/// `set_metadata` even when the underlying storage write fails, logging the error
/// instead of propagating it, so storage problems do not block request processing.
///
/// # Examples
///
/// ```rust,no_run
/// use turul_mcp_session_storage::SessionView;
/// use serde_json::json;
///
/// async fn middleware_example(session: &dyn SessionView) {
///     // Read session state
///     if let Ok(Some(counter)) = session.get_state("request_count").await {
///         println!("Session {}: {} requests", session.session_id(), counter);
///     }
///
///     // Write session state
///     let _ = session.set_state("last_access", json!("2025-10-05")).await;
/// }
/// ```
#[async_trait]
pub trait SessionView: Send + Sync {
    /// Returns the unique session identifier, typically a UUID v7 for temporal ordering.
    fn session_id(&self) -> &str;

    /// Returns the state value stored under `key`.
    ///
    /// Yields `Ok(None)` when the key is absent and `Err` on a storage error.
    async fn get_state(&self, key: &str) -> Result<Option<Value>, String>;

    /// Stores the JSON `value` under the state `key`.
    ///
    /// Implementations may log a storage failure and still return `Ok(())`.
    async fn set_state(&self, key: &str, value: Value) -> Result<(), String>;

    /// Returns the metadata value stored under `key`.
    ///
    /// Metadata is typically set during session initialization and provides
    /// context about the session (client version, capabilities, etc.). Yields
    /// `Ok(None)` when the key is absent and `Err` on a storage error.
    async fn get_metadata(&self, key: &str) -> Result<Option<Value>, String>;

    /// Stores the JSON `value` under the metadata `key`.
    ///
    /// Implementations may log a storage failure and still return `Ok(())`.
    async fn set_metadata(&self, key: &str, value: Value) -> Result<(), String>;
}
