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
