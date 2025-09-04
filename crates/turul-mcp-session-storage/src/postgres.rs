//! PostgreSQL Session Storage Implementation
//!
//! Production-ready PostgreSQL backend for persistent session storage across
//! multiple server instances. Ideal for distributed deployments requiring
//! session sharing and coordination.

use std::collections::HashMap;
use std::time::SystemTime;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Row};
use thiserror::Error;
use tracing::{info, warn, debug};

use crate::{SessionStorage, SessionInfo, SseEvent, SessionStorageError};
use turul_mcp_protocol::{ServerCapabilities, ClientCapabilities};

/// PostgreSQL-specific error types
#[derive(Error, Debug)]
pub enum PostgresError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Concurrent modification error: {0}")]
    ConcurrentModification(String),
}

/// Configuration for PostgreSQL session storage
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    /// Database connection URL
    pub database_url: String,
    /// Maximum number of database connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Session expiration timeout in minutes
    pub session_timeout_minutes: u32,
    /// Event cleanup interval in minutes
    pub cleanup_interval_minutes: u32,
    /// Maximum events to keep per session (for memory management)
    pub max_events_per_session: u32,
    /// Allow table creation if tables don't exist
    pub create_tables_if_missing: bool,
    /// Enable connection pooling optimizations
    pub enable_pooling_optimizations: bool,
    /// Statement timeout in seconds
    pub statement_timeout_secs: u32,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://localhost:5432/mcp_sessions".to_string(),
            max_connections: 20,
            min_connections: 2,
            connection_timeout_secs: 30,
            session_timeout_minutes: 30,
            cleanup_interval_minutes: 5,
            max_events_per_session: 1000,
            create_tables_if_missing: true, // PostgreSQL defaults to creating tables
            enable_pooling_optimizations: true,
            statement_timeout_secs: 30,
        }
    }
}

/// PostgreSQL-backed session storage implementation
pub struct PostgresSessionStorage {
    pool: PgPool,
    config: PostgresConfig,
}

impl PostgresSessionStorage {
    /// Create new PostgreSQL session storage with default configuration
    pub async fn new() -> Result<Self, PostgresError> {
        Self::with_config(PostgresConfig::default()).await
    }
    
    /// Create PostgreSQL session storage with custom configuration
    pub async fn with_config(config: PostgresConfig) -> Result<Self, PostgresError> {
        info!("Initializing PostgreSQL session storage at {}", mask_db_url(&config.database_url));
        
        // Build connection pool with optimized settings
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout_secs))
            .idle_timeout(Some(std::time::Duration::from_secs(300))) // 5 minutes
            .max_lifetime(Some(std::time::Duration::from_secs(1800))) // 30 minutes
            .test_before_acquire(true)
            .connect(&config.database_url)
            .await
            .map_err(|e| PostgresError::Connection(format!("Failed to connect to PostgreSQL: {}", e)))?;
        
        let storage = Self { pool, config };
        
        // Run database migrations
        storage.migrate().await?;
        
        // Start background cleanup task
        storage.start_cleanup_task().await;
        
        info!("PostgreSQL session storage initialized successfully");
        Ok(storage)
    }
    
    /// Run database schema migrations
    async fn migrate(&self) -> Result<(), PostgresError> {
        debug!("Running PostgreSQL database migrations");
        
        // Create sessions table with PostgreSQL-specific optimizations
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS sessions (
                session_id VARCHAR(36) PRIMARY KEY,
                client_capabilities JSONB,
                server_capabilities JSONB NOT NULL,
                state JSONB NOT NULL DEFAULT '{}',
                created_at BIGINT NOT NULL,
                last_activity BIGINT NOT NULL,
                is_initialized BOOLEAN NOT NULL DEFAULT FALSE,
                metadata JSONB NOT NULL DEFAULT '{}',
                version INTEGER NOT NULL DEFAULT 1 -- For optimistic locking
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(PostgresError::Database)?;
        
        // Create events table with partitioning support
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS events (
                id BIGSERIAL PRIMARY KEY,
                session_id VARCHAR(36) NOT NULL,
                timestamp BIGINT NOT NULL,
                event_type VARCHAR(100) NOT NULL,
                data JSONB NOT NULL,
                retry INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions (session_id) ON DELETE CASCADE
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(PostgresError::Database)?;
        
        // Create indexes optimized for PostgreSQL
        let indexes = [
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_sessions_last_activity ON sessions (last_activity)",
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_sessions_created_at ON sessions (created_at)",
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_events_session_id ON events (session_id)",
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_events_session_timestamp ON events (session_id, timestamp)",
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_events_session_id_id ON events (session_id, id)",
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_events_timestamp ON events (timestamp)", // For cleanup
        ];
        
        for index_sql in indexes.iter() {
            if let Err(e) = sqlx::query(index_sql).execute(&self.pool).await {
                // Concurrent index creation might fail if index already exists
                debug!("Index creation note: {}", e);
            }
        }
        
        // Create partial indexes for performance
        sqlx::query("CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_sessions_active ON sessions (last_activity) WHERE is_initialized = TRUE")
            .execute(&self.pool)
            .await
            .ok(); // Ignore errors for concurrent creation
            
        // Create materialized view for session statistics (optional optimization)
        if self.config.enable_pooling_optimizations {
            sqlx::query(r#"
                CREATE MATERIALIZED VIEW IF NOT EXISTS session_stats AS
                SELECT 
                    COUNT(*) as total_sessions,
                    COUNT(*) FILTER (WHERE is_initialized = TRUE) as initialized_sessions,
                    AVG(last_activity - created_at) as avg_session_duration,
                    NOW()::BIGINT as last_updated
                FROM sessions
            "#)
            .execute(&self.pool)
            .await
            .ok(); // Ignore errors if already exists
        }
        
        debug!("PostgreSQL database migrations completed");
        Ok(())
    }
    
    /// Start background cleanup task for expired sessions and old events
    async fn start_cleanup_task(&self) {
        let pool = self.pool.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(config.cleanup_interval_minutes as u64 * 60)
            );
            
            loop {
                interval.tick().await;
                
                if let Err(e) = cleanup_expired_data(&pool, &config).await {
                    warn!("Background cleanup failed: {}", e);
                }
                
                // Refresh materialized view if enabled
                if config.enable_pooling_optimizations {
                    if let Err(e) = sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY session_stats")
                        .execute(&pool)
                        .await {
                        debug!("Materialized view refresh failed: {}", e);
                    }
                }
            }
        });
    }
    
    /// Get session with optimistic locking support
    async fn get_session_with_version(&self, session_id: &str) -> Result<Option<(SessionInfo, i32)>, PostgresError> {
        let row = sqlx::query(r#"
            SELECT session_id, client_capabilities, server_capabilities, state, 
                   created_at, last_activity, is_initialized, metadata, version
            FROM sessions WHERE session_id = $1
        "#)
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let client_capabilities: Option<ClientCapabilities> = 
                    row.get::<Option<Value>, _>("client_capabilities")
                        .map(serde_json::from_value)
                        .transpose()?;
                
                let server_capabilities: ServerCapabilities = 
                    serde_json::from_value(row.get::<Value, _>("server_capabilities"))?;
                
                let state: HashMap<String, Value> = 
                    serde_json::from_value(row.get::<Value, _>("state"))?;
                
                let metadata: HashMap<String, Value> = 
                    serde_json::from_value(row.get::<Value, _>("metadata"))?;
                
                let version: i32 = row.get("version");
                
                let session_info = SessionInfo {
                    session_id: row.get("session_id"),
                    client_capabilities,
                    server_capabilities: Some(server_capabilities),
                    state,
                    created_at: row.get::<i64, _>("created_at") as u64,
                    last_activity: row.get::<i64, _>("last_activity") as u64,
                    is_initialized: row.get("is_initialized"),
                    metadata,
                };
                
                Ok(Some((session_info, version)))
            }
            None => Ok(None),
        }
    }
}

/// Background cleanup for expired sessions and old events
async fn cleanup_expired_data(pool: &PgPool, config: &PostgresConfig) -> Result<(), PostgresError> {
    let now = chrono::Utc::now().timestamp_millis() as i64;
    let expiration_threshold = now - (config.session_timeout_minutes as i64 * 60 * 1000);
    
    // Use transaction for consistency
    let mut tx = pool.begin().await?;
    
    // Clean up expired sessions
    let deleted_sessions = sqlx::query("DELETE FROM sessions WHERE last_activity < $1")
        .bind(expiration_threshold)
        .execute(&mut *tx)
        .await
        .map_err(PostgresError::Database)?
        .rows_affected();
    
    if deleted_sessions > 0 {
        info!("Cleaned up {} expired sessions", deleted_sessions);
    }
    
    // Clean up old events using PostgreSQL window functions for efficiency
    let deleted_events = sqlx::query(r#"
        DELETE FROM events WHERE id IN (
            SELECT id FROM (
                SELECT id,
                       ROW_NUMBER() OVER (PARTITION BY session_id ORDER BY id DESC) as rn
                FROM events
            ) ranked WHERE rn > $1
        )
    "#)
    .bind(config.max_events_per_session as i64)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    
    if deleted_events > 0 {
        debug!("Cleaned up {} old events", deleted_events);
    }
    
    tx.commit().await?;
    Ok(())
}

/// Mask sensitive information in database URL for logging
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
impl SessionStorage for PostgresSessionStorage {
    type Error = SessionStorageError;

    fn backend_name(&self) -> &'static str {
        "PostgreSQL"
    }
    
    async fn create_session(&self, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::new();
        session.server_capabilities = Some(capabilities.clone());
        
        let capabilities_json = serde_json::to_value(&capabilities)?;
        let state_json = serde_json::to_value(&session.state)?;
        let metadata_json = serde_json::to_value(&session.metadata)?;
        
        sqlx::query(r#"
            INSERT INTO sessions (session_id, server_capabilities, state, created_at, last_activity, is_initialized, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#)
        .bind(&session.session_id)
        .bind(capabilities_json)
        .bind(state_json)
        .bind(session.created_at as i64)
        .bind(session.last_activity as i64)
        .bind(session.is_initialized)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(PostgresError::Database)?;
        
        debug!("Created session: {}", session.session_id);
        Ok(session)
    }
    
    async fn create_session_with_id(&self, session_id: String, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::with_id(session_id);
        session.server_capabilities = Some(capabilities.clone());
        
        let capabilities_json = serde_json::to_value(&capabilities)?;
        let state_json = serde_json::to_value(&session.state)?;
        let metadata_json = serde_json::to_value(&session.metadata)?;
        
        sqlx::query(r#"
            INSERT INTO sessions (session_id, server_capabilities, state, created_at, last_activity, is_initialized, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#)
        .bind(&session.session_id)
        .bind(capabilities_json)
        .bind(state_json)
        .bind(session.created_at as i64)
        .bind(session.last_activity as i64)
        .bind(session.is_initialized)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(PostgresError::Database)?;
        
        debug!("Created session with ID: {}", session.session_id);
        Ok(session)
    }
    
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error> {
        match self.get_session_with_version(session_id).await? {
            Some((session, _version)) => Ok(Some(session)),
            None => Ok(None),
        }
    }
    
    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error> {
        let client_capabilities_json = session_info.client_capabilities
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        
        let server_capabilities_json = session_info.server_capabilities
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?
            .unwrap_or(Value::Null);
        
        let state_json = serde_json::to_value(&session_info.state)?;
        let metadata_json = serde_json::to_value(&session_info.metadata)?;
        
        let rows_affected = sqlx::query(r#"
            UPDATE sessions SET 
                client_capabilities = $1, 
                server_capabilities = $2,
                state = $3, 
                last_activity = $4, 
                is_initialized = $5,
                metadata = $6,
                version = version + 1
            WHERE session_id = $7
        "#)
        .bind(client_capabilities_json)
        .bind(server_capabilities_json)
        .bind(state_json)
        .bind(session_info.last_activity as i64)
        .bind(session_info.is_initialized)
        .bind(metadata_json)
        .bind(&session_info.session_id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        if rows_affected == 0 {
            return Err(PostgresError::SessionNotFound(session_info.session_id).into());
        }
        
        Ok(())
    }
    
    async fn set_session_state(&self, session_id: &str, key: &str, value: Value) -> Result<(), Self::Error> {
        // Use JSONB operations for efficient updates
        let rows_affected = sqlx::query(r#"
            UPDATE sessions 
            SET state = state || jsonb_build_object($2, $3),
                last_activity = $4,
                version = version + 1
            WHERE session_id = $1
        "#)
        .bind(session_id)
        .bind(key)
        .bind(&value)
        .bind(chrono::Utc::now().timestamp_millis() as i64)
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        if rows_affected == 0 {
            return Err(PostgresError::SessionNotFound(session_id.to_string()).into());
        }
        
        Ok(())
    }
    
    async fn get_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>, Self::Error> {
        let value = sqlx::query_scalar::<_, Option<Value>>(
            "SELECT state -> $2 FROM sessions WHERE session_id = $1"
        )
        .bind(session_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        
        Ok(value)
    }
    
    async fn remove_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>, Self::Error> {
        let old_value = sqlx::query_scalar::<_, Option<Value>>(
            "SELECT state -> $2 FROM sessions WHERE session_id = $1"
        )
        .bind(session_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        
        let rows_affected = sqlx::query(r#"
            UPDATE sessions 
            SET state = state - $2,
                last_activity = $3,
                version = version + 1
            WHERE session_id = $1
        "#)
        .bind(session_id)
        .bind(key)
        .bind(chrono::Utc::now().timestamp_millis() as i64)
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        if rows_affected == 0 {
            return Err(PostgresError::SessionNotFound(session_id.to_string()).into());
        }
        
        Ok(old_value)
    }
    
    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error> {
        let rows_affected = sqlx::query("DELETE FROM sessions WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        
        debug!("Deleted session: {} (existed: {})", session_id, rows_affected > 0);
        Ok(rows_affected > 0)
    }
    
    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error> {
        let session_ids = sqlx::query_scalar::<_, String>("SELECT session_id FROM sessions ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        
        Ok(session_ids)
    }
    
    async fn store_event(&self, session_id: &str, mut event: SseEvent) -> Result<SseEvent, Self::Error> {
        // Check if session exists and get timestamp for consistency
        let session_exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM sessions WHERE session_id = $1)")
            .bind(session_id)
            .fetch_one(&self.pool)
            .await?;
        
        if !session_exists {
            return Err(PostgresError::SessionNotFound(session_id.to_string()).into());
        }
        
        let data_json = serde_json::to_value(&event.data)?;
        
        let row_id: i64 = sqlx::query_scalar(r#"
            INSERT INTO events (session_id, timestamp, event_type, data, retry)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
        "#)
        .bind(session_id)
        .bind(event.timestamp as i64)
        .bind(&event.event_type)
        .bind(data_json)
        .bind(event.retry.map(|r| r as i32))
        .fetch_one(&self.pool)
        .await?;
        
        event.id = row_id as u64;
        Ok(event)
    }
    
    async fn get_events_after(&self, session_id: &str, after_event_id: u64) -> Result<Vec<SseEvent>, Self::Error> {
        let rows = sqlx::query(r#"
            SELECT id, timestamp, event_type, data, retry
            FROM events 
            WHERE session_id = $1 AND id > $2
            ORDER BY id ASC
        "#)
        .bind(session_id)
        .bind(after_event_id as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let data: Value = row.get("data");
            
            events.push(SseEvent {
                id: row.get::<i64, _>("id") as u64,
                timestamp: row.get::<i64, _>("timestamp") as u64,
                event_type: row.get("event_type"),
                data,
                retry: row.get::<Option<i32>, _>("retry").map(|r| r as u32),
            });
        }
        
        Ok(events)
    }
    
    async fn get_recent_events(&self, session_id: &str, limit: usize) -> Result<Vec<SseEvent>, Self::Error> {
        let rows = sqlx::query(r#"
            SELECT id, timestamp, event_type, data, retry
            FROM events 
            WHERE session_id = $1
            ORDER BY id DESC
            LIMIT $2
        "#)
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let data: Value = row.get("data");
            
            events.push(SseEvent {
                id: row.get::<i64, _>("id") as u64,
                timestamp: row.get::<i64, _>("timestamp") as u64,
                event_type: row.get("event_type"),
                data,
                retry: row.get::<Option<i32>, _>("retry").map(|r| r as u32),
            });
        }
        
        // Reverse to get chronological order
        events.reverse();
        Ok(events)
    }
    
    async fn delete_events_before(&self, session_id: &str, before_event_id: u64) -> Result<u64, Self::Error> {
        let deleted = sqlx::query("DELETE FROM events WHERE session_id = $1 AND id < $2")
            .bind(session_id)
            .bind(before_event_id as i64)
            .execute(&self.pool)
            .await?
            .rows_affected();
        
        Ok(deleted)
    }
    
    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>, Self::Error> {
        let threshold_millis = older_than
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        
        // Get session IDs that will be expired
        let expired_sessions: Vec<String> = sqlx::query_scalar(
            "SELECT session_id FROM sessions WHERE last_activity < $1"
        )
        .bind(threshold_millis)
        .fetch_all(&self.pool)
        .await?;
        
        // Delete expired sessions
        sqlx::query("DELETE FROM sessions WHERE last_activity < $1")
            .bind(threshold_millis)
            .execute(&self.pool)
            .await?;
        
        Ok(expired_sessions)
    }
    
    async fn session_count(&self) -> Result<usize, Self::Error> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions")
            .fetch_one(&self.pool)
            .await?;
        
        Ok(count as usize)
    }
    
    async fn event_count(&self) -> Result<usize, Self::Error> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM events")
            .fetch_one(&self.pool)
            .await?;
        
        Ok(count as usize)
    }
    
    async fn maintenance(&self) -> Result<(), Self::Error> {
        let now = chrono::Utc::now().timestamp_millis() as i64;
        let expiration_threshold = now - (self.config.session_timeout_minutes as i64 * 60 * 1000);
        
        // Use transaction for consistency
        let mut tx = self.pool.begin().await?;
        
        // Clean up expired sessions
        let deleted_sessions = sqlx::query("DELETE FROM sessions WHERE last_activity < $1")
            .bind(expiration_threshold)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        
        if deleted_sessions > 0 {
            info!("Maintenance: Cleaned up {} expired sessions", deleted_sessions);
        }
        
        // Clean up old events using efficient PostgreSQL query
        let deleted_events = sqlx::query(r#"
            DELETE FROM events WHERE id IN (
                SELECT id FROM (
                    SELECT id,
                           ROW_NUMBER() OVER (PARTITION BY session_id ORDER BY id DESC) as rn
                    FROM events
                ) ranked WHERE rn > $1
            )
        "#)
        .bind(self.config.max_events_per_session as i64)
        .execute(&mut *tx)
        .await
        .map_err(PostgresError::Database)?
        .rows_affected();
        
        if deleted_events > 0 {
            debug!("Maintenance: Cleaned up {} old events", deleted_events);
        }
        
        // Analyze tables for query optimization
        sqlx::query("ANALYZE sessions").execute(&mut *tx).await.map_err(PostgresError::Database)?;
        sqlx::query("ANALYZE events").execute(&mut *tx).await.map_err(PostgresError::Database)?;
        
        // Refresh materialized view if enabled
        if self.config.enable_pooling_optimizations {
            sqlx::query("REFRESH MATERIALIZED VIEW session_stats")
                .execute(&mut *tx)
                .await
                .ok(); // Ignore errors if view doesn't exist
        }
        
        tx.commit().await.map_err(PostgresError::Database)?;
        
        debug!("PostgreSQL maintenance completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turul_mcp_protocol::ServerCapabilities;
    
    // Note: These tests require a running PostgreSQL instance
    // You can start one with: docker run -d -p 5432:5432 -e POSTGRES_DB=test -e POSTGRES_PASSWORD=test postgres:15
    
    async fn create_test_storage() -> Result<PostgresSessionStorage, PostgresError> {
        let config = PostgresConfig {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:test@localhost:5432/test".to_string()),
            ..PostgresConfig::default()
        };
        PostgresSessionStorage::with_config(config).await
    }
    
    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_session_lifecycle() {
        let storage = create_test_storage().await.unwrap();
        let capabilities = ServerCapabilities::default();
        
        // Create session
        let session = storage.create_session(capabilities.clone()).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Get session
        let retrieved = storage.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().session_id, session_id);
        
        // Delete session
        let deleted = storage.delete_session(&session_id).await.unwrap();
        assert!(deleted);
        
        // Verify deletion
        let not_found = storage.get_session(&session_id).await.unwrap();
        assert!(not_found.is_none());
    }
    
    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_jsonb_state_operations() {
        let storage = create_test_storage().await.unwrap();
        let capabilities = ServerCapabilities::default();
        
        let session = storage.create_session(capabilities).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Set complex state using JSONB operations
        storage.set_session_state(&session_id, "user", json!({"name": "Alice", "age": 30})).await.unwrap();
        storage.set_session_state(&session_id, "preferences", json!({"theme": "dark", "notifications": true})).await.unwrap();
        
        // Get state
        let user = storage.get_session_state(&session_id, "user").await.unwrap();
        assert_eq!(user, Some(json!({"name": "Alice", "age": 30})));
        
        // Remove state
        let removed = storage.remove_session_state(&session_id, "user").await.unwrap();
        assert_eq!(removed, Some(json!({"name": "Alice", "age": 30})));
        
        let not_found = storage.get_session_state(&session_id, "user").await.unwrap();
        assert!(not_found.is_none());
        
        // Preferences should still exist
        let prefs = storage.get_session_state(&session_id, "preferences").await.unwrap();
        assert!(prefs.is_some());
    }
    
    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance  
    async fn test_concurrent_operations() {
        let storage = create_test_storage().await.unwrap();
        let capabilities = ServerCapabilities::default();
        
        let session = storage.create_session(capabilities).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Test concurrent state updates
        let storage_clone = std::sync::Arc::new(storage);
        let session_id_clone = session_id.clone();
        
        let handles: Vec<_> = (0..10).map(|i| {
            let storage = storage_clone.clone();
            let session_id = session_id_clone.clone();
            tokio::spawn(async move {
                storage.set_session_state(&session_id, &format!("key_{}", i), json!(i)).await
            })
        }).collect();
        
        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        // Verify all keys were set
        for i in 0..10 {
            let value = storage_clone.get_session_state(&session_id, &format!("key_{}", i)).await.unwrap();
            assert_eq!(value, Some(json!(i)));
        }
    }
}