//! SQLite Session Storage Implementation
//!
//! Production-ready SQLite backend for persistent session storage.
//! Ideal for single-instance deployments requiring data persistence
//! across server restarts.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{SqlitePool, Row, sqlite::SqliteConnectOptions};
use thiserror::Error;
use tracing::{info, warn, debug};

use crate::{SessionStorage, SessionInfo, SseEvent, SessionStorageError};
use turul_mcp_protocol::ServerCapabilities;

/// SQLite-specific error types
#[derive(Error, Debug)]
pub enum SqliteError {
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
}

/// Configuration for SQLite session storage
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// Database file path
    pub database_path: PathBuf,
    /// Maximum number of database connections in the pool
    pub max_connections: u32,
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
    /// Create database file if it doesn't exist
    pub create_database_if_missing: bool,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("mcp_sessions.db"),
            max_connections: 10,
            connection_timeout_secs: 30,
            session_timeout_minutes: 30,
            cleanup_interval_minutes: 5,
            max_events_per_session: 1000,
            create_tables_if_missing: true, // SQLite defaults to creating tables
            create_database_if_missing: true, // SQLite defaults to creating database
        }
    }
}

/// SQLite-backed session storage implementation
pub struct SqliteSessionStorage {
    pool: SqlitePool,
    config: SqliteConfig,
}

impl SqliteSessionStorage {
    /// Create new SQLite session storage with default configuration
    pub async fn new() -> Result<Self, SqliteError> {
        Self::with_config(SqliteConfig::default()).await
    }
    
    /// Create SQLite session storage with custom configuration
    pub async fn with_config(config: SqliteConfig) -> Result<Self, SqliteError> {
        info!("Initializing SQLite session storage at {:?}", config.database_path);
        
        // Ensure parent directory exists
        if let Some(parent) = config.database_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::sqlite::SqliteError::Connection(format!("Failed to create database directory: {}", e)))?;
        }
        
        // Build connection options with configurable create_if_missing
        let connect_options = SqliteConnectOptions::new()
            .filename(&config.database_path)
            .create_if_missing(config.create_database_if_missing);
        
        // Create connection pool
        let pool = SqlitePool::connect_with(connect_options).await?;
        
        let storage = Self { pool, config };
        
        // Run database migrations
        storage.migrate().await?;
        
        // Start background cleanup task
        storage.start_cleanup_task().await;
        
        info!("SQLite session storage initialized successfully");
        Ok(storage)
    }
    
    /// Run database schema migrations
    async fn migrate(&self) -> Result<(), SqliteError> {
        debug!("Running database migrations");
        
        // Create sessions table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                client_capabilities TEXT,
                server_capabilities TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                last_activity INTEGER NOT NULL,
                is_initialized BOOLEAN NOT NULL DEFAULT FALSE,
                metadata TEXT NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;
        
        // Create events table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL,
                retry INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions (session_id) ON DELETE CASCADE
            )
        "#)
        .execute(&self.pool)
        .await?;
        
        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions (last_activity)")
            .execute(&self.pool)
            .await?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_session_id ON events (session_id)")
            .execute(&self.pool)
            .await?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_session_id_id ON events (session_id, id)")
            .execute(&self.pool)
            .await?;
        
        debug!("Database migrations completed");
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
            }
        });
    }
}

/// Background cleanup for expired sessions and old events
async fn cleanup_expired_data(pool: &SqlitePool, config: &SqliteConfig) -> Result<(), SqliteError> {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let expiration_threshold = now - (config.session_timeout_minutes as u64 * 60 * 1000);
    
    // Clean up expired sessions
    let deleted_sessions = sqlx::query("DELETE FROM sessions WHERE last_activity < ?")
        .bind(expiration_threshold as i64)
        .execute(pool)
        .await?
        .rows_affected();
    
    if deleted_sessions > 0 {
        info!("Cleaned up {} expired sessions", deleted_sessions);
    }
    
    // Clean up old events (keep only recent events per session)
    let deleted_events = sqlx::query(r#"
        DELETE FROM events WHERE id NOT IN (
            SELECT id FROM events e1 WHERE (
                SELECT COUNT(*) FROM events e2 
                WHERE e2.session_id = e1.session_id AND e2.id >= e1.id
            ) <= ?
        )
    "#)
    .bind(config.max_events_per_session as i64)
    .execute(pool)
    .await?
    .rows_affected();
    
    if deleted_events > 0 {
        debug!("Cleaned up {} old events", deleted_events);
    }
    
    Ok(())
}

#[async_trait]
impl SessionStorage for SqliteSessionStorage {
    type Error = SessionStorageError;

    fn backend_name(&self) -> &'static str {
        "SQLite"
    }
    
    async fn create_session(&self, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::new();
        session.server_capabilities = Some(capabilities.clone());
        
        let capabilities_json = serde_json::to_string(&capabilities)?;
        let state_json = serde_json::to_string(&session.state)?;
        let metadata_json = serde_json::to_string(&session.metadata)?;
        
        sqlx::query(r#"
            INSERT INTO sessions (session_id, server_capabilities, state, created_at, last_activity, is_initialized, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&session.session_id)
        .bind(capabilities_json)
        .bind(state_json)
        .bind(session.created_at as i64)
        .bind(session.last_activity as i64)
        .bind(session.is_initialized)
        .bind(metadata_json)
        .execute(&self.pool)
        .await?;
        
        debug!("Created session: {}", session.session_id);
        Ok(session)
    }
    
    async fn create_session_with_id(&self, session_id: String, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::with_id(session_id);
        session.server_capabilities = Some(capabilities.clone());
        
        let capabilities_json = serde_json::to_string(&capabilities)?;
        let state_json = serde_json::to_string(&session.state)?;
        let metadata_json = serde_json::to_string(&session.metadata)?;
        
        sqlx::query(r#"
            INSERT INTO sessions (session_id, server_capabilities, state, created_at, last_activity, is_initialized, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&session.session_id)
        .bind(capabilities_json)
        .bind(state_json)
        .bind(session.created_at as i64)
        .bind(session.last_activity as i64)
        .bind(session.is_initialized)
        .bind(metadata_json)
        .execute(&self.pool)
        .await?;
        
        debug!("Created session with ID: {}", session.session_id);
        Ok(session)
    }
    
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error> {
        let row = sqlx::query(r#"
            SELECT session_id, client_capabilities, server_capabilities, state, 
                   created_at, last_activity, is_initialized, metadata
            FROM sessions WHERE session_id = ?
        "#)
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let client_capabilities: Option<turul_mcp_protocol::ClientCapabilities> = 
                    if let Some(caps_str) = row.get::<Option<String>, _>("client_capabilities") {
                        Some(serde_json::from_str(&caps_str)?)
                    } else {
                        None
                    };
                
                let server_capabilities: turul_mcp_protocol::ServerCapabilities = 
                    serde_json::from_str(&row.get::<String, _>("server_capabilities"))?;
                
                let state: HashMap<String, Value> = 
                    serde_json::from_str(&row.get::<String, _>("state"))?;
                
                let metadata: HashMap<String, Value> = 
                    serde_json::from_str(&row.get::<String, _>("metadata"))?;
                
                Ok(Some(SessionInfo {
                    session_id: row.get("session_id"),
                    client_capabilities,
                    server_capabilities: Some(server_capabilities),
                    state,
                    created_at: row.get::<i64, _>("created_at") as u64,
                    last_activity: row.get::<i64, _>("last_activity") as u64,
                    is_initialized: row.get("is_initialized"),
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }
    
    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error> {
        let client_capabilities_json = session_info.client_capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c))
            .transpose()?;
        
        let server_capabilities_json = session_info.server_capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c))
            .transpose()?
            .unwrap_or_default();
        
        let state_json = serde_json::to_string(&session_info.state)?;
        let metadata_json = serde_json::to_string(&session_info.metadata)?;
        
        let rows_affected = sqlx::query(r#"
            UPDATE sessions SET 
                client_capabilities = ?, 
                server_capabilities = ?,
                state = ?, 
                last_activity = ?, 
                is_initialized = ?,
                metadata = ?
            WHERE session_id = ?
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
            return Err(crate::sqlite::SqliteError::SessionNotFound(session_info.session_id).into());
        }
        
        Ok(())
    }
    
    async fn set_session_state(&self, session_id: &str, key: &str, value: Value) -> Result<(), Self::Error> {
        // Get current state
        let current_state_json = sqlx::query_scalar::<_, String>("SELECT state FROM sessions WHERE session_id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| crate::sqlite::SqliteError::SessionNotFound(session_id.to_string()))?;
        
        let mut state: HashMap<String, Value> = serde_json::from_str(&current_state_json)?;
        state.insert(key.to_string(), value);
        
        let new_state_json = serde_json::to_string(&state)?;
        let now = chrono::Utc::now().timestamp_millis() as i64;
        
        sqlx::query("UPDATE sessions SET state = ?, last_activity = ? WHERE session_id = ?")
            .bind(new_state_json)
            .bind(now)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    async fn get_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>, Self::Error> {
        let state_json = sqlx::query_scalar::<_, String>("SELECT state FROM sessions WHERE session_id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;
        
        match state_json {
            Some(json) => {
                let state: HashMap<String, Value> = serde_json::from_str(&json)?;
                Ok(state.get(key).cloned())
            }
            None => Ok(None),
        }
    }
    
    async fn remove_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>, Self::Error> {
        let current_state_json = sqlx::query_scalar::<_, String>("SELECT state FROM sessions WHERE session_id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| crate::sqlite::SqliteError::SessionNotFound(session_id.to_string()))?;
        
        let mut state: HashMap<String, Value> = serde_json::from_str(&current_state_json)?;
        let removed_value = state.remove(key);
        
        let new_state_json = serde_json::to_string(&state)?;
        let now = chrono::Utc::now().timestamp_millis() as i64;
        
        sqlx::query("UPDATE sessions SET state = ?, last_activity = ? WHERE session_id = ?")
            .bind(new_state_json)
            .bind(now)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        
        Ok(removed_value)
    }
    
    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error> {
        let rows_affected = sqlx::query("DELETE FROM sessions WHERE session_id = ?")
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
        // Check if session exists
        let session_exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM sessions WHERE session_id = ?)")
            .bind(session_id)
            .fetch_one(&self.pool)
            .await?;
        
        if !session_exists {
            return Err(crate::sqlite::SqliteError::SessionNotFound(session_id.to_string()).into());
        }
        
        let data_json = serde_json::to_string(&event.data)?;
        
        let row_id: i64 = sqlx::query_scalar(r#"
            INSERT INTO events (session_id, timestamp, event_type, data, retry)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
        "#)
        .bind(session_id)
        .bind(event.timestamp as i64)
        .bind(&event.event_type)
        .bind(data_json)
        .bind(event.retry.map(|r| r as i64))
        .fetch_one(&self.pool)
        .await?;
        
        event.id = row_id as u64;
        Ok(event)
    }
    
    async fn get_events_after(&self, session_id: &str, after_event_id: u64) -> Result<Vec<SseEvent>, Self::Error> {
        let rows = sqlx::query(r#"
            SELECT id, timestamp, event_type, data, retry
            FROM events 
            WHERE session_id = ? AND id > ?
            ORDER BY id ASC
        "#)
        .bind(session_id)
        .bind(after_event_id as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let data: Value = serde_json::from_str(&row.get::<String, _>("data"))?;
            
            events.push(SseEvent {
                id: row.get::<i64, _>("id") as u64,
                timestamp: row.get::<i64, _>("timestamp") as u64,
                event_type: row.get("event_type"),
                data,
                retry: row.get::<Option<i64>, _>("retry").map(|r| r as u32),
            });
        }
        
        Ok(events)
    }
    
    async fn get_recent_events(&self, session_id: &str, limit: usize) -> Result<Vec<SseEvent>, Self::Error> {
        let rows = sqlx::query(r#"
            SELECT id, timestamp, event_type, data, retry
            FROM events 
            WHERE session_id = ?
            ORDER BY id DESC
            LIMIT ?
        "#)
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let data: Value = serde_json::from_str(&row.get::<String, _>("data"))?;
            
            events.push(SseEvent {
                id: row.get::<i64, _>("id") as u64,
                timestamp: row.get::<i64, _>("timestamp") as u64,
                event_type: row.get("event_type"),
                data,
                retry: row.get::<Option<i64>, _>("retry").map(|r| r as u32),
            });
        }
        
        // Reverse to get chronological order
        events.reverse();
        Ok(events)
    }
    
    async fn delete_events_before(&self, session_id: &str, before_event_id: u64) -> Result<u64, Self::Error> {
        let deleted = sqlx::query("DELETE FROM events WHERE session_id = ? AND id < ?")
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
            "SELECT session_id FROM sessions WHERE last_activity < ?"
        )
        .bind(threshold_millis)
        .fetch_all(&self.pool)
        .await?;
        
        // Delete expired sessions
        sqlx::query("DELETE FROM sessions WHERE last_activity < ?")
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
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let expiration_threshold = now - (self.config.session_timeout_minutes as u64 * 60 * 1000);
        
        // Clean up expired sessions
        let deleted_sessions = sqlx::query("DELETE FROM sessions WHERE last_activity < ?")
            .bind(expiration_threshold as i64)
            .execute(&self.pool)
            .await?
            .rows_affected();
        
        if deleted_sessions > 0 {
            info!("Maintenance: Cleaned up {} expired sessions", deleted_sessions);
        }
        
        // Clean up old events (keep only recent events per session)
        let deleted_events = sqlx::query(r#"
            DELETE FROM events WHERE id NOT IN (
                SELECT id FROM events e1 WHERE (
                    SELECT COUNT(*) FROM events e2 
                    WHERE e2.session_id = e1.session_id AND e2.id >= e1.id
                ) <= ?
            )
        "#)
        .bind(self.config.max_events_per_session as i64)
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        if deleted_events > 0 {
            debug!("Maintenance: Cleaned up {} old events", deleted_events);
        }
        
        // Optimize database
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await?;
        
        debug!("Maintenance completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turul_mcp_protocol::ServerCapabilities;
    
    async fn create_test_storage() -> SqliteSessionStorage {
        let config = SqliteConfig {
            database_path: ":memory:".into(), // Use in-memory SQLite for tests
            ..SqliteConfig::default()
        };
        SqliteSessionStorage::with_config(config).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_session_lifecycle() {
        let storage = create_test_storage().await;
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
    async fn test_session_state_management() {
        let storage = create_test_storage().await;
        let capabilities = ServerCapabilities::default();
        
        let session = storage.create_session(capabilities).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Set state
        storage.set_session_state(&session_id, "key1", json!("value1")).await.unwrap();
        storage.set_session_state(&session_id, "key2", json!(42)).await.unwrap();
        
        // Get state
        let value1 = storage.get_session_state(&session_id, "key1").await.unwrap();
        assert_eq!(value1, Some(json!("value1")));
        
        let value2 = storage.get_session_state(&session_id, "key2").await.unwrap();
        assert_eq!(value2, Some(json!(42)));
        
        // Remove state
        let removed = storage.remove_session_state(&session_id, "key1").await.unwrap();
        assert_eq!(removed, Some(json!("value1")));
        
        let not_found = storage.get_session_state(&session_id, "key1").await.unwrap();
        assert!(not_found.is_none());
    }
    
    #[tokio::test]
    async fn test_event_storage() {
        let storage = create_test_storage().await;
        let capabilities = ServerCapabilities::default();
        
        let session = storage.create_session(capabilities).await.unwrap();
        let session_id = session.session_id.clone();
        
        // Store events
        let event1 = SseEvent::new("test".to_string(), json!({"message": "Hello"}));
        let stored_event1 = storage.store_event(&session_id, event1).await.unwrap();
        assert!(stored_event1.id > 0);
        
        let event2 = SseEvent::new("test".to_string(), json!({"message": "World"}));
        let stored_event2 = storage.store_event(&session_id, event2).await.unwrap();
        assert!(stored_event2.id > stored_event1.id);
        
        // Get events after first event
        let events = storage.get_events_after(&session_id, stored_event1.id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, stored_event2.id);
        assert_eq!(events[0].data, json!({"message": "World"}));
        
        // Get all events
        let all_events = storage.get_events_after(&session_id, 0).await.unwrap();
        assert_eq!(all_events.len(), 2);
    }
}