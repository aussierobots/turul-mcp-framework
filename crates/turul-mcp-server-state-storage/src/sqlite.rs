//! SQLite server state storage implementation.
//!
//! Production-ready SQLite backend for persistent server-global entity state.
//! Ideal for single-instance deployments requiring data persistence across
//! server restarts.

use async_trait::async_trait;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Row, SqlitePool};
use tracing::{debug, info};

use crate::error::ServerStateError;
use crate::traits::{EntityState, RegistrySnapshot, ServerStateStorage};

/// Configuration for SQLite server state storage.
#[derive(Debug, Clone)]
pub struct SqliteServerStateConfig {
    /// SQLite connection URL (e.g., `"sqlite:server_state.db"` or a
    /// `file:{name}?mode=memory&cache=shared` URI for in-memory pools).
    pub database_url: String,
    /// Maximum connections in the pool (default: 5).
    pub max_connections: u32,
}

impl Default for SqliteServerStateConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:server_state.db".to_string(),
            max_connections: 5,
        }
    }
}

/// SQLite-backed server state storage.
pub struct SqliteServerStateStorage {
    pool: SqlitePool,
}

impl SqliteServerStateStorage {
    /// Create a new SQLite server state storage from config.
    pub async fn new(config: SqliteServerStateConfig) -> Result<Self, ServerStateError> {
        info!(
            "Initializing SQLite server state storage: {}",
            config.database_url
        );

        let options: SqliteConnectOptions = config
            .database_url
            .parse()
            .map_err(|e: sqlx::Error| ServerStateError::ConfigError(e.to_string()))?;

        let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(config.max_connections)
            .connect_with(options.create_if_missing(true))
            .await
            .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        let storage = Self { pool };
        storage.ensure_tables().await?;

        info!("SQLite server state storage initialized successfully");
        Ok(storage)
    }

    /// Create from an existing pool (useful for sharing connections).
    pub async fn from_pool(pool: SqlitePool) -> Result<Self, ServerStateError> {
        let storage = Self { pool };
        storage.ensure_tables().await?;
        Ok(storage)
    }

    /// Create tables if they do not exist.
    async fn ensure_tables(&self) -> Result<(), ServerStateError> {
        debug!("Ensuring server state tables exist");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_entity_state (
                entity_type TEXT NOT NULL,
                entity_id   TEXT NOT NULL,
                active      INTEGER NOT NULL DEFAULT 1,
                metadata    TEXT,
                updated_at  TEXT NOT NULL,
                PRIMARY KEY (entity_type, entity_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_fingerprints (
                entity_type TEXT NOT NULL PRIMARY KEY,
                fingerprint TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        debug!("Server state tables ready");
        Ok(())
    }
}

#[async_trait]
impl ServerStateStorage for SqliteServerStateStorage {
    fn backend_name(&self) -> &'static str {
        "SQLite"
    }

    async fn get_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<EntityState>, ServerStateError> {
        let row = sqlx::query(
            r#"
            SELECT entity_id, active, metadata, updated_at
            FROM server_entity_state
            WHERE entity_type = ? AND entity_id = ?
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => {
                let metadata: Option<serde_json::Value> =
                    row.get::<Option<String>, _>("metadata")
                        .map(|s| serde_json::from_str(&s))
                        .transpose()?;

                Ok(Some(EntityState {
                    entity_id: row.get("entity_id"),
                    active: row.get::<i32, _>("active") != 0,
                    metadata,
                    updated_at: row.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn set_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
        state: EntityState,
    ) -> Result<(), ServerStateError> {
        let metadata_json = state
            .metadata
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO server_entity_state
                (entity_type, entity_id, active, metadata, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(state.active as i32)
        .bind(metadata_json)
        .bind(&state.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), ServerStateError> {
        sqlx::query(
            "DELETE FROM server_entity_state WHERE entity_type = ? AND entity_id = ?",
        )
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_active_entities(
        &self,
        entity_type: &str,
    ) -> Result<Vec<String>, ServerStateError> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT entity_id FROM server_entity_state WHERE entity_type = ? AND active = 1",
        )
        .bind(entity_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        Ok(rows)
    }

    async fn get_fingerprint(
        &self,
        entity_type: &str,
    ) -> Result<Option<String>, ServerStateError> {
        let fp = sqlx::query_scalar::<_, String>(
            "SELECT fingerprint FROM server_fingerprints WHERE entity_type = ?",
        )
        .bind(entity_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        Ok(fp)
    }

    async fn set_fingerprint(
        &self,
        entity_type: &str,
        fingerprint: String,
    ) -> Result<(), ServerStateError> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO server_fingerprints
                (entity_type, fingerprint, updated_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(entity_type)
        .bind(&fingerprint)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_registry_snapshot(
        &self,
        entity_type: &str,
    ) -> Result<Option<RegistrySnapshot>, ServerStateError> {
        // Get fingerprint first — if absent, no snapshot
        let fp_row = sqlx::query(
            "SELECT fingerprint, updated_at FROM server_fingerprints WHERE entity_type = ?",
        )
        .bind(entity_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        let (fingerprint, updated_at) = match fp_row {
            Some(row) => {
                let fp: String = row.get("fingerprint");
                let ua: String = row.get("updated_at");
                (fp, ua)
            }
            None => return Ok(None),
        };

        let active_entities = self.get_active_entities(entity_type).await?;

        Ok(Some(RegistrySnapshot {
            entity_type: entity_type.to_string(),
            fingerprint,
            active_entities,
            updated_at,
        }))
    }

    async fn maintenance(&self) -> Result<(), ServerStateError> {
        // Run VACUUM to reclaim space
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        debug!("SQLite server state maintenance completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Create a test storage using shared in-memory SQLite
    /// (same pattern as session-storage and task-storage).
    async fn create_test_storage() -> SqliteServerStateStorage {
        let db_name = Uuid::now_v7().as_simple().to_string();
        let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_name);
        SqliteServerStateStorage::new(SqliteServerStateConfig {
            database_url: url,
            max_connections: 2,
        })
        .await
        .expect("failed to create test storage")
    }

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
        let storage = create_test_storage().await;

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
        let storage = create_test_storage().await;

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
        let storage = create_test_storage().await;

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
        let storage = create_test_storage().await;

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
        let storage = create_test_storage().await;

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
        let storage = create_test_storage().await;
        assert_eq!(storage.backend_name(), "SQLite");
    }

    #[tokio::test]
    async fn test_metadata_roundtrip() {
        let storage = create_test_storage().await;

        let state = EntityState {
            entity_id: "calc".to_string(),
            active: true,
            metadata: Some(serde_json::json!({"version": "1.0", "tags": ["math"]})),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        storage
            .set_entity_state("tools", "calc", state.clone())
            .await
            .unwrap();

        let loaded = storage
            .get_entity_state("tools", "calc")
            .await
            .unwrap()
            .expect("entity should exist");

        assert_eq!(loaded.metadata, state.metadata);
    }
}
