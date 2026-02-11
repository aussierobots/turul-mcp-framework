//! Core middleware trait definitions

use super::{DispatcherResult, MiddlewareError, RequestContext, SessionInjection};
use async_trait::async_trait;
use turul_mcp_session_storage::SessionView;

/// Core middleware trait for intercepting MCP requests and responses
///
/// Middleware can inspect and modify requests before they reach the dispatcher,
/// and inspect/modify responses before they're sent to the client.
///
/// # Lifecycle
///
/// 1. **Before Dispatch**: Called before the MCP method handler executes
///    - Access to request method, parameters, and metadata
///    - Can inject state into session via `SessionInjection`
///    - Can short-circuit request by returning error
///
/// 2. **After Dispatch**: Called after the MCP method handler completes
///    - Access to the result (success or error)
///    - Can modify the response
///    - Can log, audit, or transform results
///
/// # Transport Agnostic
///
/// Middleware works across all transports (HTTP, Lambda) via normalized `RequestContext`.
///
/// # Examples
///
/// ```rust,no_run
/// use turul_http_mcp_server::middleware::{McpMiddleware, RequestContext, SessionInjection, MiddlewareError};
/// use turul_mcp_session_storage::SessionView;
/// use async_trait::async_trait;
///
/// struct AuthMiddleware {
///     api_key: String,
/// }
///
/// #[async_trait]
/// impl McpMiddleware for AuthMiddleware {
///     async fn before_dispatch(
///         &self,
///         ctx: &mut RequestContext<'_>,
///         session: Option<&dyn SessionView>,
///         injection: &mut SessionInjection,
///     ) -> Result<(), MiddlewareError> {
///         // Extract API key from metadata
///         let provided_key = ctx.metadata()
///             .get("api-key")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| MiddlewareError::Unauthorized("Missing API key".into()))?;
///
///         // Validate
///         if provided_key != self.api_key {
///             return Err(MiddlewareError::Unauthorized("Invalid API key".into()));
///         }
///
///         // Inject auth metadata into session (if session exists)
///         injection.set_metadata("authenticated", serde_json::json!(true));
///
///         // For initialize (session is None), injection will be applied when session is created
///         // For other methods (session is Some), can also read existing state if needed
///         if let Some(sess) = session {
///             // Can check existing session state for rate limiting, etc.
///         }
///
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait McpMiddleware: Send + Sync {
    /// Called before the MCP method handler executes
    ///
    /// # Parameters
    ///
    /// - `ctx`: Mutable request context (method, params, metadata)
    /// - `session`: Optional read-only access to session view
    ///   - `None` for `initialize` (session doesn't exist yet)
    ///   - `Some(session)` for all other methods (session already created)
    /// - `injection`: Write-only mechanism to populate session state/metadata
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Continue to next middleware or dispatcher
    /// - `Err(MiddlewareError)`: Short-circuit and return error to client
    ///
    /// # Notes
    ///
    /// - Middleware executes in registration order
    /// - First error stops the chain
    /// - Session injection is applied after all middleware succeed
    /// - For `initialize`, session is `None` but middleware can still validate headers/rate-limit
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError>;

    /// Called after the MCP method handler completes (optional)
    ///
    /// # Parameters
    ///
    /// - `ctx`: Read-only request context
    /// - `result`: Mutable dispatcher result (can modify response/error)
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Continue to next middleware
    /// - `Err(MiddlewareError)`: Replace result with error
    ///
    /// # Notes
    ///
    /// - Middleware executes in reverse registration order
    /// - Default implementation is a no-op
    /// - Can transform successful responses or errors
    #[allow(unused_variables)]
    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(()) // Default: no-op
    }
}
