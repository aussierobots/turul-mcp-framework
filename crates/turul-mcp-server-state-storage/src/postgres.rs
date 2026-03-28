//! PostgreSQL Server State Storage Implementation
//!
//! Production-ready PostgreSQL backend for persistent server-global entity state
//! across multiple server instances. Ideal for distributed deployments requiring
//! shared entity registries and cross-instance coordination.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use tracing::{debug, info};

use crate::error::ServerStateError;
use crate::traits::{EntityState, RegistrySnapshot, ServerStateStorage};

/// Configuration for PostgreSQL server state storage.
#[derive(Debug, Clone)]
pub struct PostgresServerStateConfig {
    /// Database connection URL
    pub database_url: String,
    /// Maximum number of database connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Verify table existence at startup and run migrations.
    /// When false, tables are assumed to exist.
    pub verify_tables: bool,
    /// Create tables if they don't exist during verification.
    /// Only has effect when `verify_tables` is true.
    pub create_tables: bool,
    /// Cleanup interval in minutes for maintenance tasks
    pub cleanup_interval_minutes: u32,
}

impl Default for PostgresServerStateConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://localhost:5432/mcp_server_state".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout_secs: 30,
            verify_tables: false,
            create_tables: false,
            cleanup_interval_minutes: 30,
        }
    }
}

/// PostgreSQL-backed server state storage implementation.
///
/// Stores entity activation state and fingerprints in PostgreSQL tables,
/// suitable for multi-instance deployments requiring shared state.
pub struct PostgresServerStateStorage {
    pool: PgPool,
    #[allow(dead_code)]
    config: PostgresServerStateConfig,
}

impl PostgresServerStateStorage {
    /// Create new PostgreSQL server state storage with default configuration.
    pub async fn new() -> Result<Self, ServerStateError> {
        Self::with_config(PostgresServerStateConfig::default()).await
    }

    /// Create PostgreSQL server state storage with custom configuration.
    pub async fn with_config(config: PostgresServerStateConfig) -> Result<Self, ServerStateError> {
        info!(
            "Initializing PostgreSQL server state storage at {}",
            mask_db_url(&config.database_url)
        );

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                config.connection_timeout_secs,
            ))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .test_before_acquire(true)
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                ServerStateError::DatabaseError(format!(
                    "Failed to connect to PostgreSQL: {}",
                    e
                ))
            })?;

        let verify = config.verify_tables;
        let storage = Self { pool, config };

        if verify {
            storage.migrate().await?;
        }

        info!("PostgreSQL server state storage initialized successfully");
        Ok(storage)
    }

    /// Create PostgreSQL server state storage from an existing connection pool.
    ///
    /// Useful when sharing a pool with other storage backends.
    pub async fn from_pool(pool: PgPool) -> Result<Self, ServerStateError> {
        let config = PostgresServerStateConfig::default();
        Ok(Self { pool, config })
    }

    /// Run database schema migrations.
    async fn migrate(&self) -> Result<(), ServerStateError> {
        debug!("Running PostgreSQL server state migrations");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_entity_state (
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                active BOOLEAN NOT NULL DEFAULT true,
                metadata JSONB,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
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
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        // Create indexes for common query patterns
        let indexes = [
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_server_entity_state_type_active ON server_entity_state (entity_type) WHERE active = true",
        ];

        for index_sql in indexes.iter() {
            if let Err(e) = sqlx::query(index_sql).execute(&self.pool).await {
                debug!("Index creation note: {}", e);
            }
        }

        debug!("PostgreSQL server state migrations completed");
        Ok(())
    }
}

/// Mask sensitive information in database URL for logging.
fn mask_db_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        let (prefix, suffix) = url.split_at(at_pos);
        if let Some(colon_pos) = prefix.rfind(':') {
            format!("{}:***{}", &prefix[..colon_pos], suffix)
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

#[async_trait]
impl ServerStateStorage for PostgresServerStateStorage {
    fn backend_name(&self) -> &'static str {
        "PostgreSQL"
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
            WHERE entity_type = $1 AND entity_id = $2
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => {
                let metadata: Option<serde_json::Value> = row.get("metadata");
                let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

                Ok(Some(EntityState {
                    entity_id: row.get("entity_id"),
                    active: row.get("active"),
                    metadata,
                    updated_at: updated_at.to_rfc3339(),
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
        sqlx::query(
            r#"
            INSERT INTO server_entity_state (entity_type, entity_id, active, metadata, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (entity_type, entity_id) DO UPDATE
            SET active = EXCLUDED.active,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(state.active)
        .bind(&state.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        debug!(
            "Set entity state: {}/{} active={}",
            entity_type, entity_id, state.active
        );
        Ok(())
    }

    async fn delete_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), ServerStateError> {
        sqlx::query(
            "DELETE FROM server_entity_state WHERE entity_type = $1 AND entity_id = $2",
        )
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        debug!("Deleted entity state: {}/{}", entity_type, entity_id);
        Ok(())
    }

    async fn get_active_entities(
        &self,
        entity_type: &str,
    ) -> Result<Vec<String>, ServerStateError> {
        let ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT entity_id FROM server_entity_state
            WHERE entity_type = $1 AND active = true
            ORDER BY entity_id
            "#,
        )
        .bind(entity_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        Ok(ids)
    }

    async fn get_fingerprint(
        &self,
        entity_type: &str,
    ) -> Result<Option<String>, ServerStateError> {
        let fp = sqlx::query_scalar::<_, String>(
            "SELECT fingerprint FROM server_fingerprints WHERE entity_type = $1",
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
        sqlx::query(
            r#"
            INSERT INTO server_fingerprints (entity_type, fingerprint, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (entity_type) DO UPDATE
            SET fingerprint = EXCLUDED.fingerprint,
                updated_at = NOW()
            "#,
        )
        .bind(entity_type)
        .bind(&fingerprint)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        debug!(
            "Set fingerprint for {}: {}",
            entity_type, fingerprint
        );
        Ok(())
    }

    async fn get_registry_snapshot(
        &self,
        entity_type: &str,
    ) -> Result<Option<RegistrySnapshot>, ServerStateError> {
        // Get fingerprint first; if none exists, no snapshot is available
        let fp_row = sqlx::query(
            "SELECT fingerprint, updated_at FROM server_fingerprints WHERE entity_type = $1",
        )
        .bind(entity_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        let (fingerprint, updated_at) = match fp_row {
            Some(row) => {
                let fp: String = row.get("fingerprint");
                let ts: chrono::DateTime<chrono::Utc> = row.get("updated_at");
                (fp, ts.to_rfc3339())
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
        // Run ANALYZE on tables for query optimization
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        sqlx::query("ANALYZE server_entity_state")
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        sqlx::query("ANALYZE server_fingerprints")
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ServerStateError::DatabaseError(e.to_string()))?;

        debug!("PostgreSQL server state maintenance completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Note: These tests require a running PostgreSQL instance.
    // Start one with: docker run -d -p 5432:5432 -e POSTGRES_DB=test -e POSTGRES_PASSWORD=test postgres:15

    async fn create_test_storage() -> Result<PostgresServerStateStorage, ServerStateError> {
        let config = PostgresServerStateConfig {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:test@localhost:5432/test".to_string()),
            verify_tables: true,
            create_tables: true,
            ..PostgresServerStateConfig::default()
        };
        PostgresServerStateStorage::with_config(config).await
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
    #[ignore] // Requires PostgreSQL instance
    async fn test_entity_crud() {
        let storage = create_test_storage().await.unwrap();

        // Set
        storage
            .set_entity_state("tools", "pg_add", test_entity("pg_add", true))
            .await
            .unwrap();

        // Get
        let state = storage
            .get_entity_state("tools", "pg_add")
            .await
            .unwrap();
        assert!(state.is_some());
        let state = state.unwrap();
        assert!(state.active);
        assert_eq!(state.entity_id, "pg_add");

        // Get missing
        let missing = storage
            .get_entity_state("tools", "nonexistent")
            .await
            .unwrap();
        assert!(missing.is_none());

        // Delete
        storage
            .delete_entity_state("tools", "pg_add")
            .await
            .unwrap();
        let deleted = storage
            .get_entity_state("tools", "pg_add")
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_active_entities() {
        let storage = create_test_storage().await.unwrap();

        storage
            .set_entity_state("tools", "pg_a", test_entity("pg_a", true))
            .await
            .unwrap();
        storage
            .set_entity_state("tools", "pg_b", test_entity("pg_b", true))
            .await
            .unwrap();
        storage
            .set_entity_state("tools", "pg_off", test_entity("pg_off", false))
            .await
            .unwrap();

        let active = storage.get_active_entities("tools").await.unwrap();
        assert!(active.contains(&"pg_a".to_string()));
        assert!(active.contains(&"pg_b".to_string()));
        assert!(!active.contains(&"pg_off".to_string()));

        // Cleanup
        storage.delete_entity_state("tools", "pg_a").await.unwrap();
        storage.delete_entity_state("tools", "pg_b").await.unwrap();
        storage
            .delete_entity_state("tools", "pg_off")
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_fingerprint_crud() {
        let storage = create_test_storage().await.unwrap();

        // Missing
        let fp = storage.get_fingerprint("pg_tools").await.unwrap();
        assert!(fp.is_none());

        // Set
        storage
            .set_fingerprint("pg_tools", "abc123".to_string())
            .await
            .unwrap();
        let fp = storage.get_fingerprint("pg_tools").await.unwrap();
        assert_eq!(fp, Some("abc123".to_string()));

        // Update (upsert)
        storage
            .set_fingerprint("pg_tools", "def456".to_string())
            .await
            .unwrap();
        let fp = storage.get_fingerprint("pg_tools").await.unwrap();
        assert_eq!(fp, Some("def456".to_string()));

        // Cleanup
        sqlx::query("DELETE FROM server_fingerprints WHERE entity_type = $1")
            .bind("pg_tools")
            .execute(&storage.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_entity_type_isolation() {
        let storage = create_test_storage().await.unwrap();

        storage
            .set_entity_state("tools", "pg_tool1", test_entity("pg_tool1", true))
            .await
            .unwrap();
        storage
            .set_entity_state("resources", "pg_res1", test_entity("pg_res1", true))
            .await
            .unwrap();

        let tools = storage.get_active_entities("tools").await.unwrap();
        let resources = storage.get_active_entities("resources").await.unwrap();

        assert!(tools.contains(&"pg_tool1".to_string()));
        assert!(!tools.contains(&"pg_res1".to_string()));
        assert!(resources.contains(&"pg_res1".to_string()));
        assert!(!resources.contains(&"pg_tool1".to_string()));

        // Cleanup
        storage
            .delete_entity_state("tools", "pg_tool1")
            .await
            .unwrap();
        storage
            .delete_entity_state("resources", "pg_res1")
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_registry_snapshot() {
        let storage = create_test_storage().await.unwrap();

        // No snapshot before fingerprint is set
        let snap = storage
            .get_registry_snapshot("pg_snap_type")
            .await
            .unwrap();
        assert!(snap.is_none());

        // Set entities and fingerprint
        storage
            .set_entity_state(
                "pg_snap_type",
                "snap_a",
                test_entity("snap_a", true),
            )
            .await
            .unwrap();
        storage
            .set_entity_state(
                "pg_snap_type",
                "snap_off",
                test_entity("snap_off", false),
            )
            .await
            .unwrap();
        storage
            .set_fingerprint("pg_snap_type", "snap_fp".to_string())
            .await
            .unwrap();

        let snap = storage
            .get_registry_snapshot("pg_snap_type")
            .await
            .unwrap();
        assert!(snap.is_some());
        let snap = snap.unwrap();
        assert_eq!(snap.entity_type, "pg_snap_type");
        assert_eq!(snap.fingerprint, "snap_fp");
        assert_eq!(snap.active_entities.len(), 1);
        assert!(snap.active_entities.contains(&"snap_a".to_string()));

        // Cleanup
        storage
            .delete_entity_state("pg_snap_type", "snap_a")
            .await
            .unwrap();
        storage
            .delete_entity_state("pg_snap_type", "snap_off")
            .await
            .unwrap();
        sqlx::query("DELETE FROM server_fingerprints WHERE entity_type = $1")
            .bind("pg_snap_type")
            .execute(&storage.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_entity_with_metadata() {
        let storage = create_test_storage().await.unwrap();

        let entity = EntityState {
            entity_id: "pg_meta_tool".to_string(),
            active: true,
            metadata: Some(json!({"version": "1.0", "author": "test"})),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        storage
            .set_entity_state("tools", "pg_meta_tool", entity)
            .await
            .unwrap();

        let state = storage
            .get_entity_state("tools", "pg_meta_tool")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            state.metadata,
            Some(json!({"version": "1.0", "author": "test"}))
        );

        // Cleanup
        storage
            .delete_entity_state("tools", "pg_meta_tool")
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_backend_name() {
        let storage = create_test_storage().await.unwrap();
        assert_eq!(storage.backend_name(), "PostgreSQL");
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_maintenance() {
        let storage = create_test_storage().await.unwrap();
        // Should complete without error
        storage.maintenance().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_upsert_entity_state() {
        let storage = create_test_storage().await.unwrap();

        // Insert
        storage
            .set_entity_state("tools", "pg_upsert", test_entity("pg_upsert", true))
            .await
            .unwrap();

        let state = storage
            .get_entity_state("tools", "pg_upsert")
            .await
            .unwrap()
            .unwrap();
        assert!(state.active);

        // Update via upsert
        storage
            .set_entity_state("tools", "pg_upsert", test_entity("pg_upsert", false))
            .await
            .unwrap();

        let state = storage
            .get_entity_state("tools", "pg_upsert")
            .await
            .unwrap()
            .unwrap();
        assert!(!state.active);

        // Cleanup
        storage
            .delete_entity_state("tools", "pg_upsert")
            .await
            .unwrap();
    }
}
