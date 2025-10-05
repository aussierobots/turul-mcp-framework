// SessionView trait for middleware access to sessions
//!
//! Minimal interface for middleware to access session state without depending on
//! the concrete SessionContext implementation in turul-mcp-server.

use async_trait::async_trait;
use serde_json::Value;

/// Minimal session interface for middleware access
///
/// This trait provides a read-write view of session state for middleware
/// without exposing the full SessionContext implementation details.
///
/// # Silent Write Failures
///
/// **IMPORTANT**: `set_state` and `set_metadata` currently always return `Ok(())`.
/// The underlying storage may log errors but does not propagate them to middleware.
/// This design prevents middleware from blocking request processing due to storage issues.
///
/// # Example
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
    /// Get the unique session identifier
    ///
    /// # Returns
    ///
    /// The session ID as a string reference. Typically a UUID v7 for temporal ordering.
    fn session_id(&self) -> &str;

    /// Get a state value from the session
    ///
    /// # Parameters
    ///
    /// - `key`: The state key to retrieve
    ///
    /// # Returns
    ///
    /// - `Ok(Some(value))`: State value exists
    /// - `Ok(None)`: State key not found
    /// - `Err(error)`: Storage error occurred
    async fn get_state(&self, key: &str) -> Result<Option<Value>, String>;

    /// Set a state value in the session
    ///
    /// # Parameters
    ///
    /// - `key`: The state key to set
    /// - `value`: The JSON value to store
    ///
    /// # Returns
    ///
    /// - `Ok(())`: State was successfully set (or error was logged silently)
    /// - `Err(error)`: Storage error occurred
    ///
    /// **Note**: Current implementation always returns `Ok(())` even if underlying
    /// storage fails. Errors are logged but not propagated.
    async fn set_state(&self, key: &str, value: Value) -> Result<(), String>;

    /// Get a metadata value from the session
    ///
    /// Metadata is typically set during session initialization and provides
    /// context about the session (client version, capabilities, etc.).
    ///
    /// # Parameters
    ///
    /// - `key`: The metadata key to retrieve
    ///
    /// # Returns
    ///
    /// - `Ok(Some(value))`: Metadata value exists
    /// - `Ok(None)`: Metadata key not found
    /// - `Err(error)`: Storage error occurred
    async fn get_metadata(&self, key: &str) -> Result<Option<Value>, String>;

    /// Set a metadata value in the session
    ///
    /// # Parameters
    ///
    /// - `key`: The metadata key to set
    /// - `value`: The JSON value to store
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Metadata was successfully set (or error was logged silently)
    /// - `Err(error)`: Storage error occurred
    ///
    /// **Note**: Current implementation always returns `Ok(())` even if underlying
    /// storage fails. Errors are logged but not propagated.
    async fn set_metadata(&self, key: &str, value: Value) -> Result<(), String>;
}
