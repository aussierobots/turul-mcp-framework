//! Middleware stack execution

use super::{DispatcherResult, McpMiddleware, MiddlewareError, RequestContext, SessionInjection};
use std::sync::Arc;
use turul_mcp_session_storage::SessionView;

/// Ordered collection of middleware with execution logic
///
/// The stack executes middleware in two phases:
///
/// 1. **Before dispatch**: Middleware execute in registration order
///    - First error stops the chain
///    - Session injections accumulate across all middleware
///
/// 2. **After dispatch**: Middleware execute in reverse registration order
///    - Allows proper cleanup/finalization
///    - Errors replace the result
///
/// # Examples
///
/// ```rust,no_run
/// use turul_http_mcp_server::middleware::{MiddlewareStack, McpMiddleware, RequestContext, SessionInjection, MiddlewareError};
/// use turul_mcp_session_storage::SessionView;
/// use async_trait::async_trait;
/// use std::sync::Arc;
///
/// struct LoggingMiddleware;
///
/// #[async_trait]
/// impl McpMiddleware for LoggingMiddleware {
///     async fn before_dispatch(
///         &self,
///         ctx: &mut RequestContext<'_>,
///         _session: Option<&dyn SessionView>,
///         _injection: &mut SessionInjection,
///     ) -> Result<(), MiddlewareError> {
///         println!("Request: {}", ctx.method());
///         Ok(())
///     }
/// }
///
/// # async fn example() {
/// let mut stack = MiddlewareStack::new();
/// stack.push(Arc::new(LoggingMiddleware));
///
/// assert_eq!(stack.len(), 1);
/// # }
/// ```
#[derive(Default, Clone)]
pub struct MiddlewareStack {
    middleware: Vec<Arc<dyn McpMiddleware>>,
}

impl MiddlewareStack {
    /// Create an empty middleware stack
    pub fn new() -> Self {
        Self::default()
    }

    /// Add middleware to the end of the stack
    ///
    /// # Parameters
    ///
    /// - `middleware`: Middleware implementation (must be Arc-wrapped for sharing)
    ///
    /// # Execution Order
    ///
    /// - Before dispatch: First added executes first
    /// - After dispatch: First added executes last (reverse order)
    pub fn push(&mut self, middleware: Arc<dyn McpMiddleware>) {
        self.middleware.push(middleware);
    }

    /// Get the number of middleware in the stack
    pub fn len(&self) -> usize {
        self.middleware.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }

    /// Execute all middleware before dispatch
    ///
    /// # Parameters
    ///
    /// - `ctx`: Mutable request context
    /// - `session`: Optional read-only session view
    ///   - `None` for `initialize` (session doesn't exist yet)
    ///   - `Some(session)` for all other methods
    ///
    /// # Returns
    ///
    /// - `Ok(SessionInjection)`: All middleware succeeded, contains accumulated injections
    /// - `Err(MiddlewareError)`: First middleware that failed
    ///
    /// # Execution
    ///
    /// 1. Execute each middleware in registration order
    /// 2. Accumulate session injections from all middleware
    /// 3. Stop on first error
    pub async fn execute_before(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
    ) -> Result<SessionInjection, MiddlewareError> {
        let mut combined_injection = SessionInjection::new();

        for middleware in &self.middleware {
            let mut injection = SessionInjection::new();
            middleware
                .before_dispatch(ctx, session, &mut injection)
                .await?;

            // Accumulate injections (later middleware can override earlier ones)
            for (key, value) in injection.state() {
                combined_injection.set_state(key.clone(), value.clone());
            }
            for (key, value) in injection.metadata() {
                combined_injection.set_metadata(key.clone(), value.clone());
            }
        }

        Ok(combined_injection)
    }

    /// Execute all middleware after dispatch
    ///
    /// # Parameters
    ///
    /// - `ctx`: Read-only request context
    /// - `result`: Mutable dispatcher result
    ///
    /// # Returns
    ///
    /// - `Ok(())`: All middleware succeeded
    /// - `Err(MiddlewareError)`: First middleware that failed
    ///
    /// # Execution
    ///
    /// 1. Execute each middleware in reverse registration order
    /// 2. Stop on first error
    /// 3. Allow middleware to modify result
    pub async fn execute_after(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        // Execute in reverse order
        for middleware in self.middleware.iter().rev() {
            middleware.after_dispatch(ctx, result).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct CountingMiddleware {
        id: String,
        counter: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl McpMiddleware for CountingMiddleware {
        async fn before_dispatch(
            &self,
            _ctx: &mut RequestContext<'_>,
            _session: Option<&dyn SessionView>,
            injection: &mut SessionInjection,
        ) -> Result<(), MiddlewareError> {
            self.counter
                .lock()
                .unwrap()
                .push(format!("before_{}", self.id));
            injection.set_state(&self.id, json!(true));
            Ok(())
        }

        async fn after_dispatch(
            &self,
            _ctx: &RequestContext<'_>,
            _result: &mut DispatcherResult,
        ) -> Result<(), MiddlewareError> {
            self.counter
                .lock()
                .unwrap()
                .push(format!("after_{}", self.id));
            Ok(())
        }
    }

    struct ErrorMiddleware {
        error_on_before: bool,
    }

    #[async_trait]
    impl McpMiddleware for ErrorMiddleware {
        async fn before_dispatch(
            &self,
            _ctx: &mut RequestContext<'_>,
            _session: Option<&dyn SessionView>,
            _injection: &mut SessionInjection,
        ) -> Result<(), MiddlewareError> {
            if self.error_on_before {
                Err(MiddlewareError::unauthorized("Test error"))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_middleware_execution_order() {
        let counter = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut stack = MiddlewareStack::new();

        stack.push(Arc::new(CountingMiddleware {
            id: "first".to_string(),
            counter: counter.clone(),
        }));
        stack.push(Arc::new(CountingMiddleware {
            id: "second".to_string(),
            counter: counter.clone(),
        }));

        let mut ctx = RequestContext::new("test/method", None);

        // Execute before (no session needed for this test)
        let injection = stack.execute_before(&mut ctx, None).await.unwrap();
        assert_eq!(injection.state().len(), 2);
        assert!(injection.state().contains_key("first"));
        assert!(injection.state().contains_key("second"));

        // Execute after
        let mut result = DispatcherResult::Success(json!({"ok": true}));
        stack.execute_after(&ctx, &mut result).await.unwrap();

        // Verify order: before in normal order, after in reverse
        let log = counter.lock().unwrap();
        assert_eq!(log[0], "before_first");
        assert_eq!(log[1], "before_second");
        assert_eq!(log[2], "after_second"); // Reverse order
        assert_eq!(log[3], "after_first");
    }

    #[tokio::test]
    async fn test_middleware_error_stops_chain() {
        let counter = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut stack = MiddlewareStack::new();

        stack.push(Arc::new(CountingMiddleware {
            id: "first".to_string(),
            counter: counter.clone(),
        }));
        stack.push(Arc::new(ErrorMiddleware {
            error_on_before: true,
        }));
        stack.push(Arc::new(CountingMiddleware {
            id: "third".to_string(),
            counter: counter.clone(),
        }));

        let mut ctx = RequestContext::new("test/method", None);

        // Execute before - should fail at second middleware (no session needed)
        let result = stack.execute_before(&mut ctx, None).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            MiddlewareError::unauthorized("Test error")
        );

        // Verify only first middleware executed
        let log = counter.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], "before_first");
    }

    #[tokio::test]
    async fn test_empty_stack() {
        let stack = MiddlewareStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);

        let mut ctx = RequestContext::new("test/method", None);

        let injection = stack.execute_before(&mut ctx, None).await.unwrap();
        assert!(injection.is_empty());

        let mut result = DispatcherResult::Success(json!({"ok": true}));
        stack.execute_after(&ctx, &mut result).await.unwrap();
    }
}
