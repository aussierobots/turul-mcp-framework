//! Integration tests proving guaranteed event persistence via SessionEventDispatcher.
//!
//! These tests exercise the REAL path: SessionManager → dispatcher → StreamManager
//! → InMemorySessionStorage → read events back. They prove that notifications/tools/list_changed
//! is persisted to session event storage before the emitting function returns.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use turul_http_mcp_server::StreamManager;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult, ToolSchema};
use turul_mcp_protocol::McpResult;
use turul_mcp_server::session::{SessionContext, SessionEventDispatcher, SessionManager};
use turul_mcp_session_storage::InMemorySessionStorage;

// ---------------------------------------------------------------------------
// Test tool (minimal, for ToolRegistry)
// ---------------------------------------------------------------------------

struct SimpleTool {
    tool_name: &'static str,
}

impl HasBaseMetadata for SimpleTool {
    fn name(&self) -> &str { self.tool_name }
}
impl HasDescription for SimpleTool {
    fn description(&self) -> Option<&str> { Some("test") }
}
impl HasInputSchema for SimpleTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for SimpleTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}
impl HasAnnotations for SimpleTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> { None }
}
impl HasToolMeta for SimpleTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}
impl HasIcons for SimpleTool {}
impl HasExecution for SimpleTool {}

#[async_trait]
impl turul_mcp_server::McpTool for SimpleTool {
    async fn call(&self, _args: Value, _session: Option<SessionContext>) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
    }
}

// ---------------------------------------------------------------------------
// Local dispatcher (same as server.rs but defined here to avoid pub exposure)
// ---------------------------------------------------------------------------

struct TestEventDispatcher {
    stream_manager: Arc<StreamManager>,
}

#[async_trait]
impl SessionEventDispatcher for TestEventDispatcher {
    async fn dispatch_to_session(
        &self,
        session_id: &str,
        event_type: String,
        data: serde_json::Value,
    ) -> Result<(), String> {
        self.stream_manager
            .broadcast_to_session(session_id, event_type, data)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_tools() -> HashMap<String, Arc<dyn turul_mcp_server::McpTool>> {
    let mut tools: HashMap<String, Arc<dyn turul_mcp_server::McpTool>> = HashMap::new();
    tools.insert("alpha".to_string(), Arc::new(SimpleTool { tool_name: "alpha" }));
    tools.insert("beta".to_string(), Arc::new(SimpleTool { tool_name: "beta" }));
    tools
}

/// Build the full stack: SessionManager + StreamManager + dispatcher, backed by shared storage.
/// Returns (session_manager, session_storage) so tests can read events from storage.
async fn build_stack() -> (Arc<SessionManager>, Arc<InMemorySessionStorage>) {
    let session_storage = Arc::new(InMemorySessionStorage::new());

    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));

    let dispatcher = Arc::new(TestEventDispatcher { stream_manager });
    session_manager.set_event_dispatcher(dispatcher).await;

    (session_manager, session_storage)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Prove: deactivate_tool() persists exactly 1 event to session event storage
/// before the function returns.
#[tokio::test]
async fn test_deactivate_persists_to_event_storage() {
    let (session_manager, session_storage) = build_stack().await;
    let session_id = session_manager.create_session().await;

    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    // Deactivate a tool — this MUST persist the notification before returning
    registry.deactivate_tool("beta").await.unwrap();

    // Read events from session storage — the event MUST be present NOW
    use turul_mcp_session_storage::SessionStorage;
    let events = session_storage
        .get_recent_events(&session_id, 10)
        .await
        .expect("Should read events from storage");

    let tool_changed_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .collect();

    assert_eq!(
        tool_changed_events.len(),
        1,
        "Exactly 1 notifications/tools/list_changed event must be in storage, got {}",
        tool_changed_events.len()
    );
}

/// Prove: activate_tool() persists exactly 1 event to session event storage.
#[tokio::test]
async fn test_activate_persists_to_event_storage() {
    let (session_manager, session_storage) = build_stack().await;
    let session_id = session_manager.create_session().await;

    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    // Deactivate first, then count events after re-activate
    registry.deactivate_tool("alpha").await.unwrap();

    use turul_mcp_session_storage::SessionStorage;
    let count_before = session_storage
        .get_recent_events(&session_id, 100)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    registry.activate_tool("alpha").await.unwrap();

    let count_after = session_storage
        .get_recent_events(&session_id, 100)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    assert_eq!(
        count_after - count_before,
        1,
        "activate_tool must persist exactly 1 additional event"
    );
}

/// Prove: multiple sessions each get their own stored event.
#[tokio::test]
async fn test_notification_persisted_per_session() {
    let (session_manager, session_storage) = build_stack().await;
    let session_a = session_manager.create_session().await;
    let session_b = session_manager.create_session().await;

    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    registry.deactivate_tool("beta").await.unwrap();

    use turul_mcp_session_storage::SessionStorage;

    let events_a = session_storage
        .get_recent_events(&session_a, 10)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    let events_b = session_storage
        .get_recent_events(&session_b, 10)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    assert_eq!(events_a, 1, "Session A must have exactly 1 stored event");
    assert_eq!(events_b, 1, "Session B must have exactly 1 stored event");
}

/// Prove: the stored event contains valid JSON-RPC notification payload.
#[tokio::test]
async fn test_stored_event_payload_is_valid_jsonrpc() {
    let (session_manager, session_storage) = build_stack().await;
    let session_id = session_manager.create_session().await;

    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    registry.deactivate_tool("alpha").await.unwrap();

    use turul_mcp_session_storage::SessionStorage;
    let events = session_storage
        .get_recent_events(&session_id, 10)
        .await
        .unwrap();

    let event = events
        .iter()
        .find(|e| e.event_type == "notifications/tools/list_changed")
        .expect("Event must be stored");

    // Verify JSON-RPC envelope
    assert_eq!(event.data["jsonrpc"], "2.0", "Must have jsonrpc: 2.0");
    assert_eq!(
        event.data["method"], "notifications/tools/list_changed",
        "Must have correct method"
    );
}

/// Prove: exactly 1 stored event when both the awaited dispatcher AND the
/// global broadcast channel (observer bridge) are active simultaneously.
/// The bridge must NOT re-persist Custom events — only the dispatcher does.
#[tokio::test]
async fn test_no_duplicate_storage_with_bridge_running() {
    let session_storage = Arc::new(InMemorySessionStorage::new());

    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));

    // Install the awaited dispatcher (same as server.rs does)
    let dispatcher = Arc::new(TestEventDispatcher {
        stream_manager: Arc::clone(&stream_manager),
    });
    session_manager.set_event_dispatcher(dispatcher).await;

    // Start the observer bridge — simulates what server.rs/Lambda does.
    // With the dispatcher active, the bridge must NOT persist Custom events.
    {
        let mut global_events = session_manager.subscribe_all_session_events();
        tokio::spawn(async move {
            while let Ok((_session_id, event)) = global_events.recv().await {
                match event {
                    turul_mcp_server::SessionEvent::Custom { ref event_type, .. } => {
                        // Observer-only: do NOT call broadcast_to_session here.
                        let _ = event_type; // suppress unused warning
                    }
                    _ => {}
                }
            }
        });
    }

    let session_id = session_manager.create_session().await;

    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    registry.deactivate_tool("beta").await.unwrap();

    // Small yield to let the bridge task run (if it were going to persist, it would)
    tokio::task::yield_now().await;

    use turul_mcp_session_storage::SessionStorage;
    let events = session_storage
        .get_recent_events(&session_id, 100)
        .await
        .unwrap();

    let tool_changed_count = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    assert_eq!(
        tool_changed_count, 1,
        "Exactly 1 stored event expected (dispatcher only, bridge is observer-only), got {}",
        tool_changed_count
    );
}

/// Prove: the Lambda wiring path produces the same guaranteed persistence.
/// Simulates the Lambda server.rs pattern: create StreamManager, install
/// dispatcher, create ToolRegistry, emit notification, verify storage.
#[tokio::test]
async fn test_lambda_wiring_pattern_persists_events() {
    // Simulate Lambda cold start: create all components independently
    // (same construction order as LambdaMcpServer::handler())
    let session_storage = Arc::new(InMemorySessionStorage::new());

    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    // Lambda creates StreamManager (server.rs line 258)
    let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));

    // Lambda installs dispatcher (server.rs line ~270)
    struct LambdaDispatcher {
        stream_manager: Arc<StreamManager>,
    }
    #[async_trait]
    impl SessionEventDispatcher for LambdaDispatcher {
        async fn dispatch_to_session(
            &self,
            session_id: &str,
            event_type: String,
            data: serde_json::Value,
        ) -> Result<(), String> {
            self.stream_manager
                .broadcast_to_session(session_id, event_type, data)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }
    let dispatcher = Arc::new(LambdaDispatcher {
        stream_manager: Arc::clone(&stream_manager),
    });
    session_manager.set_event_dispatcher(dispatcher).await;

    // Lambda starts observer-only bridge (server.rs line ~300)
    {
        let mut global_events = session_manager.subscribe_all_session_events();
        tokio::spawn(async move {
            while let Ok((_session_id, event)) = global_events.recv().await {
                match event {
                    turul_mcp_server::SessionEvent::Custom { .. } => {
                        // Observer-only in Lambda
                    }
                    _ => {}
                }
            }
        });
    }

    // Simulate: session already exists from a previous Lambda invocation
    let session_id = session_manager.create_session().await;

    // Lambda creates ToolRegistry (same as production)
    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    // Simulate: check_for_changes() or activate/deactivate on the request path
    registry.deactivate_tool("alpha").await.unwrap();

    // MUST be in storage BEFORE this function returns (no async bridge needed)
    use turul_mcp_session_storage::SessionStorage;
    let events = session_storage
        .get_recent_events(&session_id, 10)
        .await
        .unwrap();

    let tool_changed = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    assert_eq!(
        tool_changed, 1,
        "Lambda path: exactly 1 event must be in storage before return, got {}",
        tool_changed
    );

    // Verify payload
    let event = events
        .iter()
        .find(|e| e.event_type == "notifications/tools/list_changed")
        .unwrap();
    assert_eq!(event.data["jsonrpc"], "2.0");
    assert_eq!(event.data["method"], "notifications/tools/list_changed");
}

/// Prove: check_for_changes() with real storage + real dispatcher persists
/// before returning. Simulates cross-instance fingerprint mismatch.
#[tokio::test]
async fn test_check_for_changes_real_persistence() {
    let session_storage = Arc::new(InMemorySessionStorage::new());
    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );

    // Instance A: write initial state to shared storage
    let sm_a = Arc::new(SessionManager::new(
        turul_mcp_protocol::ServerCapabilities::default(),
    ));
    let registry_a = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        sm_a,
        server_state.clone(),
    );
    registry_a.sync_from_storage().await.unwrap();

    // Instance A deactivates a tool → writes new fingerprint to storage
    registry_a.deactivate_tool("beta").await.unwrap();
    registry_a.sync_from_storage().await.unwrap();

    // Instance B: different session manager with dispatcher, same shared storage
    let sm_b = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));
    let stream_manager_b = Arc::new(StreamManager::new(session_storage.clone()));
    let dispatcher_b = Arc::new(TestEventDispatcher {
        stream_manager: stream_manager_b,
    });
    sm_b.set_event_dispatcher(dispatcher_b).await;

    let session_id = sm_b.create_session().await;

    let registry_b = turul_mcp_server::ToolRegistry::new(
        make_tools(), // all tools active — different from storage
        sm_b,
        server_state.clone(),
    );

    // check_for_changes() detects mismatch → emits notification → dispatcher persists
    let changed = registry_b.check_for_changes().await.unwrap();
    assert!(changed, "Should detect fingerprint mismatch");

    // Event MUST be in storage NOW — not deferred to a bridge task
    use turul_mcp_session_storage::SessionStorage;
    let events = session_storage
        .get_recent_events(&session_id, 10)
        .await
        .unwrap();

    let tool_changed = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();

    assert_eq!(
        tool_changed, 1,
        "check_for_changes must persist exactly 1 event via dispatcher before returning, got {}",
        tool_changed
    );
}

/// Prove: dispatcher failure propagates to the caller — no silent success
/// when mandatory persistence fails.
#[tokio::test]
async fn test_dispatcher_failure_propagates_to_caller() {
    // Create a dispatcher that always fails
    struct FailingDispatcher;

    #[async_trait]
    impl SessionEventDispatcher for FailingDispatcher {
        async fn dispatch_to_session(
            &self,
            _session_id: &str,
            _event_type: String,
            _data: serde_json::Value,
        ) -> Result<(), String> {
            Err("simulated storage failure".to_string())
        }
    }

    let session_storage = Arc::new(InMemorySessionStorage::new());
    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    // Install the failing dispatcher
    session_manager
        .set_event_dispatcher(Arc::new(FailingDispatcher))
        .await;

    let session_id = session_manager.create_session().await;

    let server_state = Arc::new(
        turul_mcp_server_state_storage::InMemoryServerStateStorage::new(),
    );
    let registry = turul_mcp_server::ToolRegistry::new(
        make_tools(),
        session_manager,
        server_state,
    );

    // deactivate_tool MUST return Err when dispatcher fails
    let result = registry.deactivate_tool("beta").await;
    assert!(
        result.is_err(),
        "deactivate_tool must fail when mandatory notification persistence fails"
    );

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("simulated storage failure"),
        "Error must surface the dispatcher failure, got: {}",
        err
    );

    // Verify: no events were persisted (dispatcher failed before storage)
    use turul_mcp_session_storage::SessionStorage;
    let events = session_storage
        .get_recent_events(&session_id, 100)
        .await
        .unwrap();
    let tool_changed = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();
    assert_eq!(
        tool_changed, 0,
        "No events should be persisted when dispatcher fails, got {}",
        tool_changed
    );
}

// ---------------------------------------------------------------------------
// Distributed session tests — sessions exist in storage but NOT in cache
// ---------------------------------------------------------------------------

/// Prove: a session that exists in storage but NOT in SessionManager's
/// in-memory cache still receives a per-session Custom event via dispatch_custom_event().
#[tokio::test]
async fn test_uncached_session_receives_per_session_custom_event() {
    let session_storage = Arc::new(InMemorySessionStorage::new());

    // Create session directly in storage — NOT through SessionManager
    use turul_mcp_session_storage::SessionStorage;
    let session_info = session_storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .expect("Should create session in storage");
    let session_id = session_info.session_id.clone();

    // SessionManager does NOT have this session in its cache
    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    // Install dispatcher backed by real StreamManager + same storage
    let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
    let dispatcher = Arc::new(TestEventDispatcher { stream_manager });
    session_manager.set_event_dispatcher(dispatcher).await;

    // dispatch_custom_event should bypass cache and persist via dispatcher
    let notification = turul_mcp_protocol::JsonRpcNotification::new(
        "notifications/tools/list_changed".to_string(),
    );
    let data = serde_json::to_value(&notification).unwrap();

    let result = session_manager
        .dispatch_custom_event(&session_id, "notifications/tools/list_changed".to_string(), data)
        .await;
    assert!(result.is_ok(), "dispatch_custom_event should succeed for uncached-but-real session");

    // Event MUST be in storage
    let events = session_storage
        .get_recent_events(&session_id, 10)
        .await
        .unwrap();
    let tool_changed = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();
    assert_eq!(tool_changed, 1, "Exactly 1 event for uncached session, got {}", tool_changed);
}

/// Prove: broadcast_event() reaches sessions that exist in storage but NOT in the cache.
#[tokio::test]
async fn test_broadcast_reaches_storage_backed_uncached_sessions() {
    let session_storage = Arc::new(InMemorySessionStorage::new());

    // Create session directly in storage — NOT through SessionManager
    use turul_mcp_session_storage::SessionStorage;
    let session_info = session_storage
        .create_session(turul_mcp_protocol::ServerCapabilities::default())
        .await
        .expect("Should create session in storage");
    let session_id = session_info.session_id.clone();

    // SessionManager with empty cache
    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
    let dispatcher = Arc::new(TestEventDispatcher { stream_manager });
    session_manager.set_event_dispatcher(dispatcher).await;

    // broadcast_event should enumerate from storage, not cache
    let notification = turul_mcp_protocol::JsonRpcNotification::new(
        "notifications/tools/list_changed".to_string(),
    );
    let data = serde_json::to_value(&notification).unwrap();

    let result = session_manager
        .broadcast_event(turul_mcp_server::SessionEvent::Custom {
            event_type: "notifications/tools/list_changed".to_string(),
            data,
        })
        .await;
    assert!(result.is_ok(), "broadcast_event should succeed with uncached storage sessions");

    // Event MUST be in storage for the uncached session
    let events = session_storage
        .get_recent_events(&session_id, 10)
        .await
        .unwrap();
    let tool_changed = events
        .iter()
        .filter(|e| e.event_type == "notifications/tools/list_changed")
        .count();
    assert_eq!(tool_changed, 1, "Broadcast must reach uncached session, got {}", tool_changed);
}

/// Prove: dispatch_custom_event for a nonexistent session succeeds (storage accepts any session_id).
#[tokio::test]
async fn test_dispatch_to_nonexistent_session_succeeds() {
    let session_storage = Arc::new(InMemorySessionStorage::new());
    let session_manager = Arc::new(SessionManager::with_storage(
        session_storage.clone(),
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    let stream_manager = Arc::new(StreamManager::new(session_storage.clone()));
    let dispatcher = Arc::new(TestEventDispatcher { stream_manager });
    session_manager.set_event_dispatcher(dispatcher).await;

    let notification = turul_mcp_protocol::JsonRpcNotification::new(
        "notifications/tools/list_changed".to_string(),
    );
    let data = serde_json::to_value(&notification).unwrap();

    // InMemory store_event() creates events list on the fly for any session_id
    let result = session_manager
        .dispatch_custom_event("does-not-exist-anywhere", "notifications/tools/list_changed".to_string(), data)
        .await;
    assert!(result.is_ok(), "dispatch_custom_event succeeds for nonexistent session (storage accepts any session_id)");
}

/// Prove: broadcast_event() propagates storage enumeration failure, not silent cache fallback.
#[tokio::test]
async fn test_broadcast_storage_enumeration_failure_propagates() {
    use turul_mcp_session_storage::{SessionStorageError, SseEvent};

    /// Minimal storage wrapper that fails on list_sessions() only.
    struct FailingListStorage {
        inner: Arc<InMemorySessionStorage>,
    }

    #[async_trait]
    impl turul_mcp_session_storage::SessionStorage for FailingListStorage {
        type Error = SessionStorageError;

        async fn create_session(&self, caps: turul_mcp_protocol::ServerCapabilities) -> std::result::Result<turul_mcp_session_storage::SessionInfo, Self::Error> {
            self.inner.create_session(caps).await
        }
        async fn create_session_with_id(&self, id: String, caps: turul_mcp_protocol::ServerCapabilities) -> std::result::Result<turul_mcp_session_storage::SessionInfo, Self::Error> {
            self.inner.create_session_with_id(id, caps).await
        }
        async fn get_session(&self, id: &str) -> std::result::Result<Option<turul_mcp_session_storage::SessionInfo>, Self::Error> {
            self.inner.get_session(id).await
        }
        async fn update_session(&self, info: turul_mcp_session_storage::SessionInfo) -> std::result::Result<(), Self::Error> {
            self.inner.update_session(info).await
        }
        async fn delete_session(&self, id: &str) -> std::result::Result<bool, Self::Error> {
            self.inner.delete_session(id).await
        }
        async fn list_sessions(&self) -> std::result::Result<Vec<String>, Self::Error> {
            Err(SessionStorageError::DatabaseError("simulated list_sessions failure".to_string()))
        }
        async fn session_count(&self) -> std::result::Result<usize, Self::Error> {
            self.inner.session_count().await
        }
        async fn set_session_state(&self, id: &str, key: &str, value: serde_json::Value) -> std::result::Result<(), Self::Error> {
            self.inner.set_session_state(id, key, value).await
        }
        async fn get_session_state(&self, id: &str, key: &str) -> std::result::Result<Option<serde_json::Value>, Self::Error> {
            self.inner.get_session_state(id, key).await
        }
        async fn remove_session_state(&self, id: &str, key: &str) -> std::result::Result<Option<serde_json::Value>, Self::Error> {
            self.inner.remove_session_state(id, key).await
        }
        async fn expire_sessions(&self, older_than: std::time::SystemTime) -> std::result::Result<Vec<String>, Self::Error> {
            self.inner.expire_sessions(older_than).await
        }
        async fn store_event(&self, session_id: &str, event: SseEvent) -> std::result::Result<SseEvent, Self::Error> {
            self.inner.store_event(session_id, event).await
        }
        async fn get_events_after(&self, session_id: &str, after_event_id: u64) -> std::result::Result<Vec<SseEvent>, Self::Error> {
            self.inner.get_events_after(session_id, after_event_id).await
        }
        async fn get_recent_events(&self, session_id: &str, limit: usize) -> std::result::Result<Vec<SseEvent>, Self::Error> {
            self.inner.get_recent_events(session_id, limit).await
        }
        async fn delete_events_before(&self, session_id: &str, before_event_id: u64) -> std::result::Result<u64, Self::Error> {
            self.inner.delete_events_before(session_id, before_event_id).await
        }
        async fn event_count(&self) -> std::result::Result<usize, Self::Error> {
            self.inner.event_count().await
        }
        async fn maintenance(&self) -> std::result::Result<(), Self::Error> {
            self.inner.maintenance().await
        }
        fn backend_name(&self) -> &'static str { "failing-test" }
    }

    let inner = Arc::new(InMemorySessionStorage::new());
    let failing_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = Arc::new(FailingListStorage { inner });

    let session_manager = Arc::new(SessionManager::with_storage(
        failing_storage,
        turul_mcp_protocol::ServerCapabilities::default(),
    ));

    // Dispatcher installed but list_sessions will fail
    let real_storage = Arc::new(InMemorySessionStorage::new());
    let stream_manager = Arc::new(StreamManager::new(real_storage));
    let dispatcher = Arc::new(TestEventDispatcher { stream_manager });
    session_manager.set_event_dispatcher(dispatcher).await;

    let notification = turul_mcp_protocol::JsonRpcNotification::new(
        "notifications/tools/list_changed".to_string(),
    );
    let data = serde_json::to_value(&notification).unwrap();

    let result = session_manager
        .broadcast_event(turul_mcp_server::SessionEvent::Custom {
            event_type: "notifications/tools/list_changed".to_string(),
            data,
        })
        .await;

    assert!(result.is_err(), "broadcast_event must propagate storage enumeration failure");
    assert!(
        result.unwrap_err().contains("simulated list_sessions failure"),
        "Error must surface the storage failure"
    );
}
