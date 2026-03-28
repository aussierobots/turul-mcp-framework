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
#[cfg(feature = "dynamic-clustered")]
use tracing::warn;

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
    /// Optional server-global storage for DynamicClustered mode.
    /// When present, activate/deactivate persist to shared storage for cross-instance coordination.
    #[cfg(feature = "dynamic-clustered")]
    server_state: Option<Arc<dyn turul_mcp_server_state_storage::ServerStateStorage>>,
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
            #[cfg(feature = "dynamic-clustered")]
            server_state: None,
        }
    }

    /// Create a new registry with server state storage for DynamicClustered mode.
    ///
    /// All compiled tools start active (same as `new()`), but activate/deactivate
    /// operations also persist to the shared storage backend.
    #[cfg(feature = "dynamic-clustered")]
    pub fn new_clustered(
        compiled_tools: HashMap<String, Arc<dyn McpTool>>,
        session_manager: Arc<SessionManager>,
        server_state: Arc<dyn turul_mcp_server_state_storage::ServerStateStorage>,
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
            server_state: Some(server_state),
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

            #[cfg(feature = "dynamic-clustered")]
            self.persist_entity_change(name, true).await;
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

            #[cfg(feature = "dynamic-clustered")]
            self.persist_entity_change(name, false).await;
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
        // Use JsonRpcNotification (includes "jsonrpc": "2.0") — NOT ToolListChangedNotification
        // (which is a protocol-level type without the JSON-RPC envelope).
        // Matches the pattern in session.rs:notify_tools_changed().
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/tools/list_changed".to_string(),
        );
        let data = serde_json::to_value(&notification).unwrap_or_else(|e| {
            panic!("JsonRpcNotification serialization must not fail: {}", e)
        });
        self.session_manager
            .broadcast_event(crate::session::SessionEvent::Custom {
                event_type: "notifications/tools/list_changed".to_string(),
                data,
            })
            .await;
    }

    /// Sync local tool state against shared storage on startup.
    ///
    /// Compares the local fingerprint (from compiled tools) against the stored
    /// fingerprint. If they differ, this instance's state wins and is written
    /// to storage. If they match, the active set from storage is loaded into
    /// the in-memory registry.
    #[cfg(feature = "dynamic-clustered")]
    pub async fn sync_from_storage(&self) -> Result<SyncResult, ToolRegistryError> {
        let storage = self
            .server_state
            .as_ref()
            .ok_or_else(|| ToolRegistryError::StorageError("No server state storage configured".to_string()))?;

        // 1. Compute local fingerprint from compiled tools
        let local_fp = self.fingerprint().await;

        // 2. Read stored fingerprint
        let stored_fp = storage
            .get_fingerprint("tools")
            .await
            .map_err(|e| ToolRegistryError::StorageError(e.to_string()))?;

        // 3. Compare
        match stored_fp {
            None => {
                // First server to start — write our state to storage
                self.write_state_to_storage().await?;
                Ok(SyncResult::InitializedStorage)
            }
            Some(stored) if stored == local_fp => {
                // Fingerprints match — load active set from storage
                self.load_state_from_storage().await?;
                Ok(SyncResult::InSync)
            }
            Some(stored) => {
                // Different — this instance has newer tools
                warn!(
                    "Tool fingerprint mismatch: local={}, storage={}. Updating storage.",
                    local_fp, stored
                );
                self.write_state_to_storage().await?;
                Ok(SyncResult::UpdatedStorage {
                    old_fingerprint: stored,
                })
            }
        }
    }

    /// Write the current in-memory active set and fingerprint to shared storage.
    #[cfg(feature = "dynamic-clustered")]
    async fn write_state_to_storage(&self) -> Result<(), ToolRegistryError> {
        let storage = self.server_state.as_ref().unwrap();
        let state = self.state.read().await;

        // Write each active tool
        for name in &state.active {
            let entity = turul_mcp_server_state_storage::EntityState {
                entity_id: name.clone(),
                active: true,
                metadata: None,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            storage
                .set_entity_state("tools", name, entity)
                .await
                .map_err(|e| ToolRegistryError::StorageError(e.to_string()))?;
        }

        // Write fingerprint
        storage
            .set_fingerprint("tools", state.fingerprint.clone())
            .await
            .map_err(|e| ToolRegistryError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Load the active set from shared storage into the in-memory registry.
    #[cfg(feature = "dynamic-clustered")]
    async fn load_state_from_storage(&self) -> Result<(), ToolRegistryError> {
        let storage = self.server_state.as_ref().unwrap();

        // Read active entities from storage
        let active_ids = storage
            .get_active_entities("tools")
            .await
            .map_err(|e| ToolRegistryError::StorageError(e.to_string()))?;

        // Update in-memory state
        let mut state = self.state.write().await;
        state.active = active_ids.into_iter().collect();
        state.fingerprint = Self::compute_fingerprint_for(&self.compiled_tools, &state.active);

        Ok(())
    }

    /// Persist a single entity activation/deactivation change to shared storage.
    /// Best-effort: logs warnings on failure rather than propagating errors.
    #[cfg(feature = "dynamic-clustered")]
    async fn persist_entity_change(&self, name: &str, active: bool) {
        if let Some(ref storage) = self.server_state {
            let entity = turul_mcp_server_state_storage::EntityState {
                entity_id: name.to_string(),
                active,
                metadata: None,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(e) = storage.set_entity_state("tools", name, entity).await {
                warn!("Failed to persist tool state to storage: {}", e);
            }
            // Update fingerprint in storage
            let fp = self.fingerprint().await;
            if let Err(e) = storage.set_fingerprint("tools", fp).await {
                warn!("Failed to persist fingerprint to storage: {}", e);
            }
        }
    }

    /// Start a background polling task that periodically checks shared storage
    /// for fingerprint changes from other instances.
    ///
    /// When a change is detected, reloads the active tool set from storage and
    /// broadcasts `notifications/tools/list_changed` to connected clients.
    ///
    /// Returns a `JoinHandle` that can be used to abort the polling task on shutdown.
    #[cfg(feature = "dynamic-clustered")]
    pub fn start_polling(
        self: &Arc<Self>,
        interval: std::time::Duration,
    ) -> tokio::task::JoinHandle<()> {
        let registry = Arc::clone(self);
        tokio::spawn(async move {
            // Use sleep instead of interval to avoid the immediate first tick
            // (tokio::time::interval fires immediately on the first tick)
            loop {
                tokio::time::sleep(interval).await;

                if let Some(ref storage) = registry.server_state {
                    match storage.get_fingerprint("tools").await {
                        Ok(Some(stored_fp)) => {
                            let local_fp = registry.fingerprint().await;
                            if stored_fp != local_fp {
                                info!(
                                    "DynamicClustered: detected tool change from another instance (stored={}, local={})",
                                    stored_fp, local_fp
                                );
                                if let Err(e) = registry.load_state_from_storage().await {
                                    warn!(
                                        "Failed to reload tool state from storage: {}",
                                        e
                                    );
                                    continue;
                                }
                                registry.broadcast_notification().await;
                                info!(
                                    "DynamicClustered: tool state reloaded and clients notified"
                                );
                            }
                        }
                        Ok(None) => {
                            debug!("No fingerprint in storage yet");
                        }
                        Err(e) => {
                            warn!("Failed to check storage fingerprint: {}", e);
                        }
                    }
                }
            }
        })
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

    #[cfg(feature = "dynamic-clustered")]
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Result of syncing local tool state against shared storage.
#[cfg(feature = "dynamic-clustered")]
#[derive(Debug)]
pub enum SyncResult {
    /// First server to start — wrote local state to storage.
    InitializedStorage,
    /// Fingerprints match — loaded active set from storage.
    InSync,
    /// Fingerprint mismatch — updated storage with local state.
    UpdatedStorage { old_fingerprint: String },
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

            // Assert exact JSON-RPC 2.0 notification wire format:
            // {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
            assert_eq!(
                data.get("jsonrpc").and_then(|j| j.as_str()),
                Some("2.0"),
                "Must contain jsonrpc: \"2.0\" per JSON-RPC 2.0 spec"
            );
            assert_eq!(
                data.get("method").and_then(|m| m.as_str()),
                Some("notifications/tools/list_changed"),
                "Must contain method field per MCP spec"
            );
            assert!(
                data.get("params").is_none() || data.get("params").unwrap().is_null(),
                "No params field for list_changed notification"
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

    /// ADR-023 MUST: Notification support does NOT bypass stale session rejection.
    /// Even after a notification is emitted, the server's fingerprint changes,
    /// meaning existing sessions have a stale fingerprint and MUST be rejected.
    #[tokio::test]
    async fn test_notification_does_not_prevent_fingerprint_change() {
        let registry = ToolRegistry::new(test_tools(), test_session_manager());
        let fp_before = registry.fingerprint().await;

        // Deactivate a tool — this sends a notification AND changes the fingerprint
        registry.deactivate_tool("beta").await.unwrap();
        let fp_after = registry.fingerprint().await;

        // Fingerprint MUST have changed — existing sessions with fp_before are now stale
        assert_ne!(
            fp_before, fp_after,
            "After tool mutation, fingerprint MUST change. \
             Existing sessions with the old fingerprint MUST be rejected (404). \
             The notification is advisory only and does not bypass this."
        );
    }

    // ===================================================================
    // DynamicClustered tests (storage-backed registry)
    // ===================================================================

    #[cfg(feature = "dynamic-clustered")]
    mod clustered_tests {
        use super::*;

        fn test_storage() -> Arc<dyn turul_mcp_server_state_storage::ServerStateStorage> {
            Arc::new(turul_mcp_server_state_storage::InMemoryServerStateStorage::new())
        }

        #[tokio::test]
        async fn test_new_clustered_all_tools_active() {
            let registry = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                test_storage(),
            );
            let active = registry.list_active_tools().await;
            assert_eq!(active.len(), 3);
        }

        #[tokio::test]
        async fn test_sync_from_storage_initializes_empty_storage() {
            let storage = test_storage();
            let registry = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            );

            let result = registry.sync_from_storage().await.unwrap();
            assert!(matches!(result, SyncResult::InitializedStorage));

            // Storage should now have the fingerprint
            let stored_fp = storage.get_fingerprint("tools").await.unwrap();
            assert!(stored_fp.is_some());
            assert_eq!(stored_fp.unwrap(), registry.fingerprint().await);
        }

        #[tokio::test]
        async fn test_sync_from_storage_in_sync() {
            let storage = test_storage();
            let registry = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            );

            // First sync initializes storage
            registry.sync_from_storage().await.unwrap();

            // Second sync with same tools should be in sync
            let registry2 = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            );
            let result = registry2.sync_from_storage().await.unwrap();
            assert!(matches!(result, SyncResult::InSync));
        }

        #[tokio::test]
        async fn test_sync_from_storage_detects_newer_tools() {
            let storage = test_storage();

            // First server writes its state
            let registry1 = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            );
            registry1.sync_from_storage().await.unwrap();
            let old_fp = storage.get_fingerprint("tools").await.unwrap().unwrap();

            // Second server has different tools (simulate by deactivating one first)
            // Create with only 2 of the 3 tools
            let mut fewer_tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
            fewer_tools.insert("alpha".to_string(), Arc::new(TestDynTool { tool_name: "alpha" }));
            fewer_tools.insert("beta".to_string(), Arc::new(TestDynTool { tool_name: "beta" }));

            let registry2 = ToolRegistry::new_clustered(
                fewer_tools,
                test_session_manager(),
                storage.clone(),
            );
            let result = registry2.sync_from_storage().await.unwrap();

            // Should detect mismatch and update storage
            match result {
                SyncResult::UpdatedStorage { old_fingerprint } => {
                    assert_eq!(old_fingerprint, old_fp);
                }
                other => panic!("Expected UpdatedStorage, got {:?}", other),
            }

            // Storage fingerprint should now match the new server
            let new_fp = storage.get_fingerprint("tools").await.unwrap().unwrap();
            assert_eq!(new_fp, registry2.fingerprint().await);
            assert_ne!(new_fp, old_fp);
        }

        #[tokio::test]
        async fn test_activate_persists_to_storage() {
            let storage = test_storage();
            let registry = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            );

            // Deactivate then activate
            registry.deactivate_tool("beta").await.unwrap();
            registry.activate_tool("beta").await.unwrap();

            // Storage should have beta as active
            let state = storage.get_entity_state("tools", "beta").await.unwrap();
            assert!(state.is_some());
            assert!(state.unwrap().active);

            // Fingerprint in storage should match in-memory
            let stored_fp = storage.get_fingerprint("tools").await.unwrap();
            assert_eq!(stored_fp, Some(registry.fingerprint().await));
        }

        #[tokio::test]
        async fn test_polling_detects_external_fingerprint_change() {
            let storage = test_storage();
            let registry = Arc::new(ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            ));

            // Sync initial state to storage
            registry.sync_from_storage().await.unwrap();
            let initial_fp = registry.fingerprint().await;

            // Simulate another instance deactivating a tool directly in storage
            let entity = turul_mcp_server_state_storage::EntityState {
                entity_id: "gamma".to_string(),
                active: false,
                metadata: None,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            storage
                .set_entity_state("tools", "gamma", entity)
                .await
                .unwrap();
            // Write a new fingerprint that differs from local
            storage
                .set_fingerprint("tools", "external_change".to_string())
                .await
                .unwrap();

            // Start polling with very short interval
            let handle = registry.start_polling(std::time::Duration::from_millis(50));

            // Wait for poll to detect change
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Local fingerprint should have been updated (recomputed from new active set)
            let new_fp = registry.fingerprint().await;
            assert_ne!(
                new_fp, initial_fp,
                "Polling should detect external fingerprint change and reload state"
            );

            // gamma should now be inactive
            let active = registry.list_active_tools().await;
            assert_eq!(active.len(), 2, "gamma should have been deactivated by external change");
            assert!(
                active.iter().all(|t| t.name != "gamma"),
                "gamma should not be in the active tool list"
            );

            handle.abort();
        }

        #[tokio::test]
        async fn test_polling_noop_when_fingerprints_match() {
            let storage = test_storage();
            let registry = Arc::new(ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            ));

            // Sync initial state
            registry.sync_from_storage().await.unwrap();
            let initial_fp = registry.fingerprint().await;

            // Start polling — fingerprints match, so nothing should change
            let handle = registry.start_polling(std::time::Duration::from_millis(50));

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let fp_after = registry.fingerprint().await;
            assert_eq!(
                fp_after, initial_fp,
                "Fingerprint should remain unchanged when storage matches"
            );
            assert_eq!(registry.list_active_tools().await.len(), 3);

            handle.abort();
        }

        #[tokio::test]
        async fn test_deactivate_persists_to_storage() {
            let storage = test_storage();
            let registry = ToolRegistry::new_clustered(
                test_tools(),
                test_session_manager(),
                storage.clone(),
            );

            registry.deactivate_tool("gamma").await.unwrap();

            // Storage should have gamma as inactive
            let state = storage.get_entity_state("tools", "gamma").await.unwrap();
            assert!(state.is_some());
            assert!(!state.unwrap().active);
        }
    }
}
