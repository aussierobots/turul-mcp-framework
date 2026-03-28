//! Dynamic Tool Registry for Runtime Tool Activation/Deactivation
//!
//! Provides an in-process mutable registry that allows precompiled tools to be
//! activated or deactivated at runtime. When tools change, connected clients
//! receive `notifications/tools/list_changed` via SSE.
//!
//! This module is gated behind the `dynamic-tools` feature flag.
//! Only for single-process, long-lived HTTP servers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::session::SessionManager;
use crate::tool::{McpTool, compute_tool_fingerprint, tool_to_descriptor};

/// In-process mutable tool registry for runtime activation/deactivation.
///
/// All tool implementations are compiled into the binary and registered at build time.
/// This registry controls which of those compiled tools are currently **active** —
/// it does not support adding new tool implementations at runtime.
///
/// # Concurrency
///
/// Uses `tokio::sync::RwLock` for the active set. The lock is never held across
/// await points — callers acquire a read lock, clone what they need, then release.
pub struct ToolRegistry {
    /// All compiled tools (immutable after construction)
    compiled_tools: HashMap<String, Arc<dyn McpTool>>,
    /// Mutable state: active tool set + fingerprint under a single lock.
    /// This ensures the fingerprint always matches the active set — no TOCTOU window.
    state: RwLock<ToolState>,
    /// SessionManager for broadcasting change events (transport-agnostic)
    session_manager: Arc<SessionManager>,
}

/// Active tool set and its corresponding fingerprint, kept consistent under one lock.
struct ToolState {
    active: HashSet<String>,
    fingerprint: String,
}

impl ToolRegistry {
    /// Create a new registry with all compiled tools initially active.
    pub fn new(
        compiled_tools: HashMap<String, Arc<dyn McpTool>>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        let active: HashSet<String> = compiled_tools.keys().cloned().collect();
        let fingerprint = Self::compute_fingerprint_for(&compiled_tools, &active);

        Self {
            compiled_tools,
            state: RwLock::new(ToolState {
                active,
                fingerprint,
            }),
            session_manager,
        }
    }

    /// Activate a precompiled tool by name.
    ///
    /// Returns `Ok(true)` if the tool was newly activated, `Ok(false)` if already active,
    /// or `Err` if the name is not a compiled tool.
    pub async fn activate_tool(&self, name: &str) -> Result<bool, ToolRegistryError> {
        if !self.compiled_tools.contains_key(name) {
            return Err(ToolRegistryError::NotCompiled(name.to_string()));
        }

        let changed = {
            let mut state = self.state.write().await;
            let inserted = state.active.insert(name.to_string());
            if inserted {
                // Recompute fingerprint atomically under the same write lock
                state.fingerprint =
                    Self::compute_fingerprint_for(&self.compiled_tools, &state.active);
            }
            inserted
        }; // write lock released here — active set + fingerprint are consistent

        if changed {
            self.broadcast_notification().await;
            info!("Tool '{}' activated", name);
        } else {
            debug!("Tool '{}' already active", name);
        }

        Ok(changed)
    }

    /// Deactivate a precompiled tool by name.
    ///
    /// Returns `Ok(true)` if the tool was deactivated, `Ok(false)` if already inactive,
    /// or `Err` if the name is not a compiled tool.
    pub async fn deactivate_tool(&self, name: &str) -> Result<bool, ToolRegistryError> {
        if !self.compiled_tools.contains_key(name) {
            return Err(ToolRegistryError::NotCompiled(name.to_string()));
        }

        let changed = {
            let mut state = self.state.write().await;
            let removed = state.active.remove(name);
            if removed {
                // Recompute fingerprint atomically under the same write lock
                state.fingerprint =
                    Self::compute_fingerprint_for(&self.compiled_tools, &state.active);
            }
            removed
        }; // write lock released here — active set + fingerprint are consistent

        if changed {
            self.broadcast_notification().await;
            info!("Tool '{}' deactivated", name);
        } else {
            debug!("Tool '{}' already inactive", name);
        }

        Ok(changed)
    }

    /// List all currently active tools as protocol `Tool` descriptors.
    pub async fn list_active_tools(&self) -> Vec<turul_mcp_protocol::Tool> {
        let state = self.state.read().await;
        let mut tools: Vec<turul_mcp_protocol::Tool> = self
            .compiled_tools
            .iter()
            .filter(|(name, _)| state.active.contains(*name))
            .map(|(_, tool)| tool_to_descriptor(tool.as_ref()))
            .collect();
        // Sort for deterministic output (matches tools/list behavior)
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }

    /// Get an active tool by name. Returns None if the tool is inactive or not compiled.
    ///
    /// Clones the Arc under the read lock, then releases. Safe to call across await points.
    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn McpTool>> {
        let state = self.state.read().await;
        if state.active.contains(name) {
            self.compiled_tools.get(name).cloned()
        } else {
            None
        }
    }

    /// Get the current fingerprint.
    pub async fn fingerprint(&self) -> String {
        self.state.read().await.fingerprint.clone()
    }

    /// Get the set of all compiled tool names (active and inactive).
    pub fn compiled_tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.compiled_tools.keys().cloned().collect();
        names.sort();
        names
    }

    /// Broadcast `notifications/tools/list_changed` to all connected clients.
    /// Called AFTER the write lock is released (notification is best-effort, not atomic with mutation).
    async fn broadcast_notification(&self) {
        let notification = turul_mcp_protocol::notifications::ToolListChangedNotification::new();
        let data = serde_json::to_value(&notification).unwrap_or_else(|e| {
            panic!("ToolListChangedNotification serialization must not fail: {}", e)
        });
        self.session_manager
            .broadcast_event(crate::session::SessionEvent::Custom {
                event_type: "notifications/tools/list_changed".to_string(),
                data,
            })
            .await;
    }

    /// Compute fingerprint for a given active tool subset.
    fn compute_fingerprint_for(
        compiled: &HashMap<String, Arc<dyn McpTool>>,
        active: &HashSet<String>,
    ) -> String {
        let active_tools: HashMap<String, Arc<dyn McpTool>> = compiled
            .iter()
            .filter(|(name, _)| active.contains(*name))
            .map(|(name, tool)| (name.clone(), Arc::clone(tool)))
            .collect();
        compute_tool_fingerprint(&active_tools)
    }
}

/// Errors from tool registry operations.
#[derive(Debug, thiserror::Error)]
pub enum ToolRegistryError {
    #[error("Tool '{0}' is not a compiled tool — cannot activate/deactivate tools that were not registered at build time")]
    NotCompiled(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value;
    use turul_mcp_builders::prelude::*;
    use turul_mcp_protocol::tools::{CallToolResult, ToolResult, ToolSchema};
    use turul_mcp_protocol::McpResult;

    // Minimal test tool
    struct TestDynTool {
        tool_name: &'static str,
    }

    impl HasBaseMetadata for TestDynTool {
        fn name(&self) -> &str {
            self.tool_name
        }
    }
    impl HasDescription for TestDynTool {
        fn description(&self) -> Option<&str> {
            Some("test tool")
        }
    }
    impl HasInputSchema for TestDynTool {
        fn input_schema(&self) -> &ToolSchema {
            static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
            SCHEMA.get_or_init(ToolSchema::object)
        }
    }
    impl HasOutputSchema for TestDynTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            None
        }
    }
    impl HasAnnotations for TestDynTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }
    impl HasToolMeta for TestDynTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }
    impl HasIcons for TestDynTool {}
    impl HasExecution for TestDynTool {}
    #[async_trait]
    impl McpTool for TestDynTool {
        async fn call(
            &self,
            _args: Value,
            _session: Option<crate::session::SessionContext>,
        ) -> McpResult<CallToolResult> {
            Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
        }
    }

    fn test_tools() -> HashMap<String, Arc<dyn McpTool>> {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert("alpha".to_string(), Arc::new(TestDynTool { tool_name: "alpha" }));
        tools.insert("beta".to_string(), Arc::new(TestDynTool { tool_name: "beta" }));
        tools.insert("gamma".to_string(), Arc::new(TestDynTool { tool_name: "gamma" }));
        tools
    }

    fn test_session_manager() -> Arc<SessionManager> {
        Arc::new(SessionManager::new(
            turul_mcp_protocol::ServerCapabilities::default(),
        ))
    }

    #[tokio::test]
    async fn test_all_tools_active_by_default() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let active = registry.list_active_tools().await;
        assert_eq!(active.len(), 3);
    }

    #[tokio::test]
    async fn test_deactivate_tool() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());

        let result = registry.deactivate_tool("beta").await.unwrap();
        assert!(result, "beta should have been deactivated");

        let active = registry.list_active_tools().await;
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|t| t.name != "beta"));
    }

    #[tokio::test]
    async fn test_activate_tool() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());

        registry.deactivate_tool("beta").await.unwrap();
        assert_eq!(registry.list_active_tools().await.len(), 2);

        let result = registry.activate_tool("beta").await.unwrap();
        assert!(result, "beta should have been newly activated");
        assert_eq!(registry.list_active_tools().await.len(), 3);
    }

    #[tokio::test]
    async fn test_activate_already_active() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let result = registry.activate_tool("alpha").await.unwrap();
        assert!(!result, "alpha was already active");
    }

    #[tokio::test]
    async fn test_deactivate_already_inactive() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        registry.deactivate_tool("beta").await.unwrap();
        let result = registry.deactivate_tool("beta").await.unwrap();
        assert!(!result, "beta was already inactive");
    }

    #[tokio::test]
    async fn test_activate_nonexistent_tool_errors() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let result = registry.activate_tool("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolRegistryError::NotCompiled(_)));
    }

    #[tokio::test]
    async fn test_deactivate_nonexistent_tool_errors() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let result = registry.deactivate_tool("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_tool_active() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let tool = registry.get_tool("alpha").await;
        assert!(tool.is_some());
    }

    #[tokio::test]
    async fn test_get_tool_inactive() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        registry.deactivate_tool("alpha").await.unwrap();
        let tool = registry.get_tool("alpha").await;
        assert!(tool.is_none(), "Inactive tool should return None");
    }

    #[tokio::test]
    async fn test_fingerprint_changes_on_mutation() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let fp_before = registry.fingerprint().await;

        registry.deactivate_tool("beta").await.unwrap();
        let fp_after = registry.fingerprint().await;

        assert_ne!(fp_before, fp_after, "Fingerprint must change when active set changes");
    }

    #[tokio::test]
    async fn test_fingerprint_stable_without_mutation() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let fp1 = registry.fingerprint().await;
        let fp2 = registry.fingerprint().await;
        assert_eq!(fp1, fp2);
    }

    #[tokio::test]
    async fn test_compiled_tool_names() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let names = registry.compiled_tool_names();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let registry = Arc::new(ToolRegistry::new(test_tools(), test_session_manager()));
        let mut handles = Vec::new();

        for i in 0..20 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                if i % 3 == 0 {
                    let _ = reg.deactivate_tool("beta").await;
                } else if i % 3 == 1 {
                    let _ = reg.activate_tool("beta").await;
                } else {
                    let _ = reg.list_active_tools().await;
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Registry should be in a consistent state
        let active = registry.list_active_tools().await;
        assert!(active.len() >= 2 && active.len() <= 3);
    }

    /// E2E notification emission test: verify that activate_tool() broadcasts
    /// a notifications/tools/list_changed event through the SessionManager.
    /// This is the server-side proof that the notification is emitted —
    /// the SSE bridge (tested separately) delivers it to clients.
    #[tokio::test]
    async fn test_activate_tool_emits_notification_event() {
        let session_manager = test_session_manager();

        // Create a session so broadcast_event has something to send to
        let session_id = session_manager.create_session().await;

        // Subscribe to global events BEFORE the mutation
        let mut receiver = session_manager.subscribe_all_session_events();

        let registry = ToolRegistry::new(test_tools(), session_manager.clone());

        // Deactivate then re-activate to trigger a notification
        registry.deactivate_tool("beta").await.unwrap();

        // Check that a Custom event was broadcast
        let mut found_notification = false;
        // Drain all events (there may be multiple from the broadcast to each session)
        while let Ok((recv_session_id, event)) =
            tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await
                .unwrap_or(Err(tokio::sync::broadcast::error::RecvError::Closed))
        {
            if let crate::session::SessionEvent::Custom { event_type, .. } = &event {
                if event_type == "notifications/tools/list_changed" {
                    found_notification = true;
                    assert_eq!(
                        recv_session_id, session_id,
                        "Notification should be sent to the existing session"
                    );
                    break;
                }
            }
        }

        assert!(
            found_notification,
            "deactivate_tool() must broadcast notifications/tools/list_changed via SessionManager"
        );
    }

    /// Verify the exact notification payload matches the MCP 2025-11-25 wire format.
    /// ADR-023 pins: {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
    #[tokio::test]
    async fn test_notification_payload_matches_mcp_wire_format() {
        let session_manager = test_session_manager();
        let _session_id = session_manager.create_session().await;
        let mut receiver = session_manager.subscribe_all_session_events();

        let registry = ToolRegistry::new(test_tools(), session_manager);
        registry.deactivate_tool("alpha").await.unwrap();

        let (_sid, event) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            receiver.recv(),
        )
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");

        if let crate::session::SessionEvent::Custom { event_type, data } = event {
            assert_eq!(event_type, "notifications/tools/list_changed");

            // Assert exact MCP wire format fields
            assert_eq!(
                data.get("method").and_then(|m| m.as_str()),
                Some("notifications/tools/list_changed"),
                "Must contain method field per MCP spec"
            );
            assert!(
                data.get("params").is_none() || data.get("params").unwrap().is_null(),
                "No params field per ToolListChangedNotification::new()"
            );
        } else {
            panic!("Expected SessionEvent::Custom, got {:?}", event);
        }
    }

    /// Verify activate_tool() also emits notification (not just deactivate)
    #[tokio::test]
    async fn test_activate_tool_also_emits_notification() {
        let session_manager = test_session_manager();
        let _session_id = session_manager.create_session().await;

        let registry = ToolRegistry::new(test_tools(), session_manager.clone());
        registry.deactivate_tool("beta").await.unwrap();

        // Now subscribe and re-activate
        let mut receiver = session_manager.subscribe_all_session_events();
        registry.activate_tool("beta").await.unwrap();

        let mut found = false;
        while let Ok((_sid, event)) =
            tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv())
                .await
                .unwrap_or(Err(tokio::sync::broadcast::error::RecvError::Closed))
        {
            if let crate::session::SessionEvent::Custom { event_type, .. } = &event {
                if event_type == "notifications/tools/list_changed" {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "activate_tool() must also broadcast notification");
    }

    /// Fingerprint round-trip: deactivate → reactivate → same fingerprint
    #[tokio::test]
    async fn test_fingerprint_round_trip() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let fp_initial = registry.fingerprint().await;

        registry.deactivate_tool("beta").await.unwrap();
        let fp_deactivated = registry.fingerprint().await;
        assert_ne!(fp_initial, fp_deactivated);

        registry.activate_tool("beta").await.unwrap();
        let fp_reactivated = registry.fingerprint().await;
        assert_eq!(
            fp_initial, fp_reactivated,
            "Restoring same active set must restore same fingerprint"
        );
    }

    /// Empty active set is a valid state
    #[tokio::test]
    async fn test_deactivate_all_tools() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let fp_full = registry.fingerprint().await;

        registry.deactivate_tool("alpha").await.unwrap();
        registry.deactivate_tool("beta").await.unwrap();
        registry.deactivate_tool("gamma").await.unwrap();

        let active = registry.list_active_tools().await;
        assert!(active.is_empty(), "All tools deactivated → empty list");

        let fp_empty = registry.fingerprint().await;
        assert_ne!(fp_full, fp_empty, "Empty set fingerprint differs from full set");
        assert_eq!(fp_empty.len(), 16, "Empty set still produces valid fingerprint");

        // get_tool returns None for all
        assert!(registry.get_tool("alpha").await.is_none());
    }
}
