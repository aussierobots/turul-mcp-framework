//! Integration tests for middleware system
//!
//! These tests verify that middleware can:
//! - Read and write session state
//! - Inject metadata during initialize
//! - Return semantic error codes
//! - Work across both HTTP transports and Lambda

use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};

use crate::middleware::{
    DispatcherResult, McpMiddleware, MiddlewareError, MiddlewareStack, RequestContext,
    SessionInjection, StorageBackedSessionView,
};
use turul_mcp_session_storage::{BoxedSessionStorage, InMemorySessionStorage, SessionView};

/// Test middleware that reads and writes session state
struct SessionAccessMiddleware {
    /// Track what we read from session
    reads: Arc<Mutex<Vec<String>>>,
    /// Track what we wrote to session
    writes: Arc<Mutex<Vec<(String, String)>>>,
}

impl SessionAccessMiddleware {
    #[allow(clippy::type_complexity)]
    fn new() -> (
        Self,
        Arc<Mutex<Vec<String>>>,
        Arc<Mutex<Vec<(String, String)>>>,
    ) {
        let reads = Arc::new(Mutex::new(Vec::new()));
        let writes = Arc::new(Mutex::new(Vec::new()));

        let middleware = Self {
            reads: Arc::clone(&reads),
            writes: Arc::clone(&writes),
        };

        (middleware, reads, writes)
    }
}

#[async_trait]
impl McpMiddleware for SessionAccessMiddleware {
    async fn before_dispatch(
        &self,
        _ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        if let Some(session_view) = session {
            // Try to read existing state
            if let Ok(Some(value)) = session_view.get_state("existing_key").await
                && let Some(s) = value.as_str()
            {
                self.reads.lock().unwrap().push(s.to_string());
            }

            // Write new state via injection
            injection.set_state("middleware_wrote", json!("test_value"));
            self.writes
                .lock()
                .unwrap()
                .push(("middleware_wrote".to_string(), "test_value".to_string()));

            // Write metadata via injection
            injection.set_metadata("request_id", json!("req-123"));
            self.writes
                .lock()
                .unwrap()
                .push(("request_id".to_string(), "req-123".to_string()));
        }

        Ok(())
    }

    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        // No-op for this test
        Ok(())
    }
}

/// Test middleware that returns errors with semantic codes
struct ErrorMiddleware {
    error_type: String,
}

impl ErrorMiddleware {
    fn unauthenticated() -> Self {
        Self {
            error_type: "unauthenticated".to_string(),
        }
    }

    fn unauthorized() -> Self {
        Self {
            error_type: "unauthorized".to_string(),
        }
    }

    fn rate_limit() -> Self {
        Self {
            error_type: "rate_limit".to_string(),
        }
    }
}

#[async_trait]
impl McpMiddleware for ErrorMiddleware {
    async fn before_dispatch(
        &self,
        _ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        match self.error_type.as_str() {
            "unauthenticated" => Err(MiddlewareError::Unauthenticated(
                "Missing API key".to_string(),
            )),
            "unauthorized" => Err(MiddlewareError::Unauthorized(
                "Insufficient permissions".to_string(),
            )),
            "rate_limit" => Err(MiddlewareError::RateLimitExceeded {
                message: "Too many requests".to_string(),
                retry_after: Some(60),
            }),
            _ => Ok(()),
        }
    }

    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_middleware_can_read_write_session_state() {
    // Create storage and session
    let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());
    let session_info = storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .unwrap();
    let session_id = session_info.session_id.clone();

    // Pre-populate session with some state
    let session_view = StorageBackedSessionView::new(session_id.clone(), Arc::clone(&storage));
    session_view
        .set_state("existing_key", json!("existing_value"))
        .await
        .unwrap();

    // Create middleware and stack
    let (middleware, reads, writes) = SessionAccessMiddleware::new();
    let mut stack = MiddlewareStack::new();
    stack.push(Arc::new(middleware));

    // Execute middleware
    let mut ctx = RequestContext::new("test/method", None);
    let injection = stack
        .execute_before(&mut ctx, Some(&session_view))
        .await
        .unwrap();

    // Verify middleware read the existing state
    {
        let read_values = reads.lock().unwrap();
        assert_eq!(read_values.len(), 1);
        assert_eq!(read_values[0], "existing_value");
    }

    // Verify middleware wrote via injection
    {
        let write_values = writes.lock().unwrap();
        assert_eq!(write_values.len(), 2);
        assert!(
            write_values
                .iter()
                .any(|(k, v)| k == "middleware_wrote" && v == "test_value")
        );
        assert!(
            write_values
                .iter()
                .any(|(k, v)| k == "request_id" && v == "req-123")
        );
    }

    // Verify injection contains the writes
    assert_eq!(
        injection.state().get("middleware_wrote"),
        Some(&json!("test_value"))
    );
    assert_eq!(
        injection.metadata().get("request_id"),
        Some(&json!("req-123"))
    );

    // Apply injection to storage (simulating what run_middleware_and_dispatch does)
    for (key, value) in injection.state() {
        session_view.set_state(key, value.clone()).await.unwrap();
    }
    for (key, value) in injection.metadata() {
        session_view.set_metadata(key, value.clone()).await.unwrap();
    }

    // Verify state persisted to storage
    assert_eq!(
        session_view.get_state("middleware_wrote").await.unwrap(),
        Some(json!("test_value"))
    );
    assert_eq!(
        session_view.get_metadata("request_id").await.unwrap(),
        Some(json!("req-123"))
    );
}

#[tokio::test]
async fn test_middleware_can_inject_during_initialize() {
    // Simulate initialize flow: session created, then middleware runs with it
    let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());

    // 1. Create session (like initialize does)
    let session_info = storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .unwrap();
    let session_id = session_info.session_id.clone();

    // 2. Create session view for middleware
    let session_view = StorageBackedSessionView::new(session_id.clone(), Arc::clone(&storage));

    // 3. Run middleware with the new session
    let (middleware, _reads, writes) = SessionAccessMiddleware::new();
    let mut stack = MiddlewareStack::new();
    stack.push(Arc::new(middleware));

    let mut ctx = RequestContext::new("initialize", None);
    let injection = stack
        .execute_before(&mut ctx, Some(&session_view))
        .await
        .unwrap();

    // 4. Apply injection (like run_middleware_and_dispatch does)
    for (key, value) in injection.state() {
        session_view.set_state(key, value.clone()).await.unwrap();
    }
    for (key, value) in injection.metadata() {
        session_view.set_metadata(key, value.clone()).await.unwrap();
    }

    // 5. Verify middleware wrote to the session
    {
        let write_values = writes.lock().unwrap();
        assert!(write_values.len() >= 2); // At least state + metadata
    }

    // 6. Verify data persisted to storage
    assert_eq!(
        session_view.get_state("middleware_wrote").await.unwrap(),
        Some(json!("test_value"))
    );
    assert_eq!(
        session_view.get_metadata("request_id").await.unwrap(),
        Some(json!("req-123"))
    );

    // 7. Verify we can read it back from storage
    let session = storage.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(
        session.state.get("middleware_wrote"),
        Some(&json!("test_value"))
    );

    // Metadata stored with __meta__: prefix
    assert_eq!(
        session.metadata.get("__meta__:request_id"),
        Some(&json!("req-123"))
    );
}

#[tokio::test]
async fn test_middleware_error_unauthenticated() {
    let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());
    let session_info = storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .unwrap();

    let session_view =
        StorageBackedSessionView::new(session_info.session_id.clone(), Arc::clone(&storage));

    let mut stack = MiddlewareStack::new();
    stack.push(Arc::new(ErrorMiddleware::unauthenticated()));

    let mut ctx = RequestContext::new("test/method", None);
    let result = stack.execute_before(&mut ctx, Some(&session_view)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MiddlewareError::Unauthenticated(msg) => {
            assert_eq!(msg, "Missing API key");
        }
        other => panic!("Expected Unauthenticated error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_middleware_error_unauthorized() {
    let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());
    let session_info = storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .unwrap();

    let session_view =
        StorageBackedSessionView::new(session_info.session_id.clone(), Arc::clone(&storage));

    let mut stack = MiddlewareStack::new();
    stack.push(Arc::new(ErrorMiddleware::unauthorized()));

    let mut ctx = RequestContext::new("test/method", None);
    let result = stack.execute_before(&mut ctx, Some(&session_view)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MiddlewareError::Unauthorized(msg) => {
            assert_eq!(msg, "Insufficient permissions");
        }
        other => panic!("Expected Unauthorized error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_middleware_error_rate_limit_with_retry_after() {
    let storage: Arc<BoxedSessionStorage> = Arc::new(InMemorySessionStorage::new());
    let session_info = storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .unwrap();

    let session_view =
        StorageBackedSessionView::new(session_info.session_id.clone(), Arc::clone(&storage));

    let mut stack = MiddlewareStack::new();
    stack.push(Arc::new(ErrorMiddleware::rate_limit()));

    let mut ctx = RequestContext::new("test/method", None);
    let result = stack.execute_before(&mut ctx, Some(&session_view)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MiddlewareError::RateLimitExceeded {
            message,
            retry_after,
        } => {
            assert_eq!(message, "Too many requests");
            assert_eq!(retry_after, Some(60));
        }
        other => panic!("Expected RateLimitExceeded error, got {:?}", other),
    }
}
