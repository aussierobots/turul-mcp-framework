//! Core server state storage trait and data models.
//!
//! Defines the `ServerStateStorage` trait for persisting server-global entity
//! activation state across different backends (InMemory, SQLite, PostgreSQL, DynamoDB).
//!
//! This is **generic server-global state**, not tool-specific. The same trait backs
//! all MCP entity types that support `list_changed` notifications:
//! - `notifications/tools/list_changed` — tool activation registry
//! - `notifications/resources/list_changed` — resource activation registry
//! - `notifications/prompts/list_changed` — prompt activation registry
//!
//! Session state (`mcp:tool_fingerprint`) is session-scoped compatibility metadata.
//! Server state (this trait) is instance-global and shared across a cluster.

use crate::error::ServerStateError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Known entity types for server state storage.
pub mod entity_types {
    pub const TOOLS: &str = "tools";
    pub const RESOURCES: &str = "resources";
    pub const PROMPTS: &str = "prompts";
}

/// An entity's activation state in the server-global registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    /// Entity identifier (e.g., tool name)
    pub entity_id: String,
    /// Whether the entity is currently active
    pub active: bool,
    /// Optional metadata about the entity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// ISO 8601 datetime when the state was last updated
    pub updated_at: String,
}

/// Summary of an entity type's registry state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    /// Entity type (e.g., "tools")
    pub entity_type: String,
    /// Current fingerprint for this entity type
    pub fingerprint: String,
    /// List of active entity IDs
    pub active_entities: Vec<String>,
    /// ISO 8601 datetime when the registry was last modified
    pub updated_at: String,
}

/// Server-global state storage for MCP entity registries.
///
/// Provides persistence and cross-instance coordination for entity activation
/// state (tools, resources, prompts). This is separate from `SessionStorage` —
/// session state is client-scoped, server state is instance-global.
///
/// # Backend Pattern
///
/// Follows the same pluggable-backend pattern as `turul-mcp-session-storage`
/// and `turul-mcp-task-storage`:
/// - InMemory — test double (cannot satisfy clustered semantics)
/// - SQLite — local durable mode
/// - PostgreSQL — shared relational deployments
/// - DynamoDB — serverless/AWS deployments
#[async_trait]
pub trait ServerStateStorage: Send + Sync {
    /// Storage backend name (for logging/diagnostics)
    fn backend_name(&self) -> &'static str;

    // ==================== Entity State ====================

    /// Get the activation state of a specific entity.
    async fn get_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<EntityState>, ServerStateError>;

    /// Set the activation state of a specific entity (upsert).
    async fn set_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
        state: EntityState,
    ) -> Result<(), ServerStateError>;

    /// Delete an entity's state.
    async fn delete_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), ServerStateError>;

    /// List all active entity IDs for a given type.
    async fn get_active_entities(
        &self,
        entity_type: &str,
    ) -> Result<Vec<String>, ServerStateError>;

    // ==================== Fingerprint ====================

    /// Get the current fingerprint for an entity type.
    /// Returns None if no fingerprint has been set.
    async fn get_fingerprint(
        &self,
        entity_type: &str,
    ) -> Result<Option<String>, ServerStateError>;

    /// Set the fingerprint for an entity type.
    async fn set_fingerprint(
        &self,
        entity_type: &str,
        fingerprint: String,
    ) -> Result<(), ServerStateError>;

    // ==================== Registry Snapshot ====================

    /// Get a full snapshot of the registry for an entity type.
    /// Useful for startup comparison and diagnostics.
    async fn get_registry_snapshot(
        &self,
        entity_type: &str,
    ) -> Result<Option<RegistrySnapshot>, ServerStateError>;

    // ==================== Maintenance ====================

    /// Perform storage maintenance (cleanup, compaction, etc.)
    async fn maintenance(&self) -> Result<(), ServerStateError>;
}

/// Type alias for boxed server state storage (mirrors SessionStorage pattern)
pub type BoxedServerStateStorage = dyn ServerStateStorage<>;
