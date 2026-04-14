//! In-memory server state storage implementation.
//!
//! **Test double only.** Cannot satisfy clustered semantics across instances.
//! Use SQLite, PostgreSQL, or DynamoDB for production multi-instance deployments.

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::ServerStateError;
use crate::traits::{EntityState, RegistrySnapshot, ServerStateStorage};

/// In-memory server state storage.
///
/// Suitable for testing and single-process Dynamic mode (default when no
/// explicit storage is provided). NOT suitable for multi-instance coordination.
pub struct InMemoryServerStateStorage {
    /// entity_type -> (entity_id -> EntityState)
    entities: RwLock<HashMap<String, HashMap<String, EntityState>>>,
    /// entity_type -> fingerprint
    fingerprints: RwLock<HashMap<String, String>>,
}

impl InMemoryServerStateStorage {
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            fingerprints: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryServerStateStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServerStateStorage for InMemoryServerStateStorage {
    fn backend_name(&self) -> &'static str {
        "InMemory"
    }

    async fn get_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<EntityState>, ServerStateError> {
        let entities = self.entities.read().await;
        Ok(entities
            .get(entity_type)
            .and_then(|m| m.get(entity_id))
            .cloned())
    }

    async fn set_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
        state: EntityState,
    ) -> Result<(), ServerStateError> {
        let mut entities = self.entities.write().await;
        entities
            .entry(entity_type.to_string())
            .or_default()
            .insert(entity_id.to_string(), state);
        Ok(())
    }

    async fn delete_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), ServerStateError> {
        let mut entities = self.entities.write().await;
        if let Some(type_map) = entities.get_mut(entity_type) {
            type_map.remove(entity_id);
        }
        Ok(())
    }

    async fn get_active_entities(
        &self,
        entity_type: &str,
    ) -> Result<Vec<String>, ServerStateError> {
        let entities = self.entities.read().await;
        Ok(entities
            .get(entity_type)
            .map(|m| {
                m.iter()
                    .filter(|(_, state)| state.active)
                    .map(|(id, _)| id.clone())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn get_fingerprint(&self, entity_type: &str) -> Result<Option<String>, ServerStateError> {
        let fps = self.fingerprints.read().await;
        Ok(fps.get(entity_type).cloned())
    }

    async fn set_fingerprint(
        &self,
        entity_type: &str,
        fingerprint: String,
    ) -> Result<(), ServerStateError> {
        let mut fps = self.fingerprints.write().await;
        fps.insert(entity_type.to_string(), fingerprint);
        Ok(())
    }

    async fn get_registry_snapshot(
        &self,
        entity_type: &str,
    ) -> Result<Option<RegistrySnapshot>, ServerStateError> {
        let entities = self.entities.read().await;
        let fps = self.fingerprints.read().await;

        let fingerprint = match fps.get(entity_type) {
            Some(fp) => fp.clone(),
            None => return Ok(None),
        };

        let active_entities = entities
            .get(entity_type)
            .map(|m| {
                m.iter()
                    .filter(|(_, state)| state.active)
                    .map(|(id, _)| id.clone())
                    .collect()
            })
            .unwrap_or_default();

        Ok(Some(RegistrySnapshot {
            entity_type: entity_type.to_string(),
            fingerprint,
            active_entities,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn maintenance(&self) -> Result<(), ServerStateError> {
        // No-op for in-memory storage
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entity(id: &str, active: bool) -> EntityState {
        EntityState {
            entity_id: id.to_string(),
            active,
            metadata: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_entity_crud() {
        let storage = InMemoryServerStateStorage::new();

        // Set
        storage
            .set_entity_state("tools", "add", test_entity("add", true))
            .await
            .unwrap();

        // Get
        let state = storage.get_entity_state("tools", "add").await.unwrap();
        assert!(state.is_some());
        assert!(state.unwrap().active);

        // Get missing
        let missing = storage
            .get_entity_state("tools", "nonexistent")
            .await
            .unwrap();
        assert!(missing.is_none());

        // Delete
        storage.delete_entity_state("tools", "add").await.unwrap();
        let deleted = storage.get_entity_state("tools", "add").await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_active_entities() {
        let storage = InMemoryServerStateStorage::new();

        storage
            .set_entity_state("tools", "add", test_entity("add", true))
            .await
            .unwrap();
        storage
            .set_entity_state("tools", "multiply", test_entity("multiply", true))
            .await
            .unwrap();
        storage
            .set_entity_state("tools", "disabled", test_entity("disabled", false))
            .await
            .unwrap();

        let active = storage.get_active_entities("tools").await.unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"add".to_string()));
        assert!(active.contains(&"multiply".to_string()));
        assert!(!active.contains(&"disabled".to_string()));
    }

    #[tokio::test]
    async fn test_fingerprint_crud() {
        let storage = InMemoryServerStateStorage::new();

        // Missing
        let fp = storage.get_fingerprint("tools").await.unwrap();
        assert!(fp.is_none());

        // Set
        storage
            .set_fingerprint("tools", "abc123".to_string())
            .await
            .unwrap();

        let fp = storage.get_fingerprint("tools").await.unwrap();
        assert_eq!(fp, Some("abc123".to_string()));

        // Update
        storage
            .set_fingerprint("tools", "def456".to_string())
            .await
            .unwrap();
        let fp = storage.get_fingerprint("tools").await.unwrap();
        assert_eq!(fp, Some("def456".to_string()));
    }

    #[tokio::test]
    async fn test_entity_type_isolation() {
        let storage = InMemoryServerStateStorage::new();

        storage
            .set_entity_state("tools", "add", test_entity("add", true))
            .await
            .unwrap();
        storage
            .set_entity_state("resources", "file", test_entity("file", true))
            .await
            .unwrap();
        storage
            .set_fingerprint("tools", "fp_tools".to_string())
            .await
            .unwrap();
        storage
            .set_fingerprint("resources", "fp_resources".to_string())
            .await
            .unwrap();

        // Entity types are isolated
        let tools = storage.get_active_entities("tools").await.unwrap();
        assert_eq!(tools.len(), 1);
        assert!(tools.contains(&"add".to_string()));

        let resources = storage.get_active_entities("resources").await.unwrap();
        assert_eq!(resources.len(), 1);
        assert!(resources.contains(&"file".to_string()));

        // Fingerprints are isolated
        assert_eq!(
            storage.get_fingerprint("tools").await.unwrap(),
            Some("fp_tools".to_string())
        );
        assert_eq!(
            storage.get_fingerprint("resources").await.unwrap(),
            Some("fp_resources".to_string())
        );
    }

    #[tokio::test]
    async fn test_registry_snapshot() {
        let storage = InMemoryServerStateStorage::new();

        // No snapshot before fingerprint set
        let snap = storage.get_registry_snapshot("tools").await.unwrap();
        assert!(snap.is_none());

        // Set entities and fingerprint
        storage
            .set_entity_state("tools", "add", test_entity("add", true))
            .await
            .unwrap();
        storage
            .set_entity_state("tools", "off", test_entity("off", false))
            .await
            .unwrap();
        storage
            .set_fingerprint("tools", "snap_fp".to_string())
            .await
            .unwrap();

        let snap = storage.get_registry_snapshot("tools").await.unwrap();
        assert!(snap.is_some());
        let snap = snap.unwrap();
        assert_eq!(snap.entity_type, "tools");
        assert_eq!(snap.fingerprint, "snap_fp");
        assert_eq!(snap.active_entities.len(), 1);
        assert!(snap.active_entities.contains(&"add".to_string()));
    }

    #[tokio::test]
    async fn test_backend_name() {
        let storage = InMemoryServerStateStorage::new();
        assert_eq!(storage.backend_name(), "InMemory");
    }
}
