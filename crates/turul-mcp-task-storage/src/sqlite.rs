//! SQLite task storage backend.
//!
//! Production-ready SQLite backend for persistent task storage.
//! Ideal for single-instance deployments requiring data persistence
//! across server restarts.

use crate::error::TaskStorageError;
use crate::state_machine;
use crate::traits::{TaskListPage, TaskOutcome, TaskRecord, TaskStorage};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use turul_mcp_protocol::TaskStatus;

/// Configuration for SQLite task storage.
#[derive(Debug, Clone)]
pub struct SqliteTaskConfig {
    /// Database file path (use ":memory:" for in-memory)
    pub database_path: PathBuf,
    /// Maximum number of database connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Background cleanup interval in minutes
    pub cleanup_interval_minutes: u32,
    /// Maximum number of tasks to store (0 = unlimited)
    pub max_tasks: usize,
    /// Default page size for list operations
    pub default_page_size: u32,
    /// Allow table creation if tables don't exist
    pub create_tables_if_missing: bool,
    /// Create database file if it doesn't exist
    pub create_database_if_missing: bool,
}

impl Default for SqliteTaskConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("mcp_tasks.db"),
            max_connections: 10,
            connection_timeout_secs: 30,
            cleanup_interval_minutes: 5,
            max_tasks: 10_000,
            default_page_size: 50,
            create_tables_if_missing: true,
            create_database_if_missing: true,
        }
    }
}

/// SQLite-backed task storage implementation.
pub struct SqliteTaskStorage {
    pool: SqlitePool,
    config: SqliteTaskConfig,
}

fn status_to_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Working => "working",
        TaskStatus::InputRequired => "input_required",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn str_to_status(s: &str) -> Result<TaskStatus, TaskStorageError> {
    match s {
        "working" => Ok(TaskStatus::Working),
        "input_required" => Ok(TaskStatus::InputRequired),
        "completed" => Ok(TaskStatus::Completed),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(TaskStorageError::DatabaseError(format!(
            "Unknown task status: {}",
            other
        ))),
    }
}

fn row_to_task_record(row: sqlx::sqlite::SqliteRow) -> Result<TaskRecord, TaskStorageError> {
    let status_str: String = row.get("status");
    let status = str_to_status(&status_str)?;

    let original_params: Option<Value> =
        if let Some(params_str) = row.get::<Option<String>, _>("original_params") {
            Some(serde_json::from_str(&params_str)?)
        } else {
            None
        };

    let result: Option<TaskOutcome> =
        if let Some(result_str) = row.get::<Option<String>, _>("result") {
            Some(serde_json::from_str(&result_str)?)
        } else {
            None
        };

    let meta: Option<HashMap<String, Value>> =
        if let Some(meta_str) = row.get::<Option<String>, _>("meta") {
            Some(serde_json::from_str(&meta_str)?)
        } else {
            None
        };

    Ok(TaskRecord {
        task_id: row.get("task_id"),
        session_id: row.get("session_id"),
        status,
        status_message: row.get("status_message"),
        created_at: row.get("created_at"),
        last_updated_at: row.get("last_updated_at"),
        ttl: row.get::<Option<i64>, _>("ttl"),
        poll_interval: row
            .get::<Option<i64>, _>("poll_interval")
            .map(|v| v as u64),
        original_method: row.get("original_method"),
        original_params,
        result,
        meta,
    })
}

impl SqliteTaskStorage {
    /// Create new SQLite task storage with default configuration.
    pub async fn new() -> Result<Self, TaskStorageError> {
        Self::with_config(SqliteTaskConfig::default()).await
    }

    /// Create SQLite task storage with custom configuration.
    pub async fn with_config(config: SqliteTaskConfig) -> Result<Self, TaskStorageError> {
        info!(
            "Initializing SQLite task storage at {:?}",
            config.database_path
        );

        let db_path_str = config.database_path.to_string_lossy();
        let is_memory = db_path_str == ":memory:";

        // Ensure parent directory exists for file-based databases
        if !is_memory {
            if let Some(parent) = config.database_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    TaskStorageError::DatabaseError(format!(
                        "Failed to create database directory: {}",
                        e
                    ))
                })?;
            }
        }

        let pool = if is_memory {
            // For in-memory databases, connect via URI with a unique name and shared
            // cache so all pool connections see the same database instance.
            let unique_name = uuid::Uuid::now_v7();
            let uri = format!("file:{}?mode=memory&cache=shared", unique_name);
            SqlitePool::connect(&uri)
                .await
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?
        } else {
            let connect_options = SqliteConnectOptions::new()
                .filename(&config.database_path)
                .create_if_missing(config.create_database_if_missing);
            SqlitePool::connect_with(connect_options)
                .await
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?
        };

        let storage = Self { pool, config };

        storage.migrate().await?;
        storage.start_cleanup_task();

        info!("SQLite task storage initialized successfully");
        Ok(storage)
    }

    /// Run database schema migrations.
    async fn migrate(&self) -> Result<(), TaskStorageError> {
        debug!("Running task storage database migrations");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                session_id TEXT,
                status TEXT NOT NULL DEFAULT 'working',
                status_message TEXT,
                created_at TEXT NOT NULL,
                last_updated_at TEXT NOT NULL,
                ttl INTEGER,
                poll_interval INTEGER,
                original_method TEXT NOT NULL,
                original_params TEXT,
                result TEXT,
                meta TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_list ON tasks (created_at, task_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks (session_id, created_at, task_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status)")
            .execute(&self.pool)
            .await?;

        debug!("Task storage database migrations completed");
        Ok(())
    }

    /// Start background cleanup task for expired tasks.
    fn start_cleanup_task(&self) {
        let pool = self.pool.clone();
        let interval_mins = self.config.cleanup_interval_minutes;

        tokio::spawn(async move {
            let duration = std::time::Duration::from_secs(interval_mins as u64 * 60);

            loop {
                tokio::time::sleep(duration).await;

                if let Err(e) = run_cleanup(&pool).await {
                    warn!("Task storage background cleanup failed: {}", e);
                }
            }
        });
    }

    fn now_iso8601() -> String {
        Utc::now().to_rfc3339()
    }
}

/// Background cleanup: expire tasks that have exceeded their TTL.
async fn run_cleanup(pool: &SqlitePool) -> Result<(), TaskStorageError> {
    let deleted = sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE ttl IS NOT NULL
          AND (julianday('now') - julianday(created_at)) * 86400000 > ttl
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    if deleted > 0 {
        debug!("Background cleanup: expired {} tasks", deleted);
    }

    Ok(())
}

#[async_trait]
impl TaskStorage for SqliteTaskStorage {
    fn backend_name(&self) -> &'static str {
        "sqlite"
    }

    async fn create_task(&self, mut task: TaskRecord) -> Result<TaskRecord, TaskStorageError> {
        // Check max_tasks limit
        if self.config.max_tasks > 0 {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
                .fetch_one(&self.pool)
                .await?;
            if count as usize >= self.config.max_tasks {
                return Err(TaskStorageError::MaxTasksReached(self.config.max_tasks));
            }
        }

        // Ensure timestamps are set
        if task.created_at.is_empty() {
            task.created_at = Self::now_iso8601();
        }
        if task.last_updated_at.is_empty() {
            task.last_updated_at = task.created_at.clone();
        }

        let status_str = status_to_str(task.status);
        let original_params_json = task
            .original_params
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let result_json = task
            .result
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let meta_json = task
            .meta
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO tasks (task_id, session_id, status, status_message, created_at,
                               last_updated_at, ttl, poll_interval, original_method,
                               original_params, result, meta)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.task_id)
        .bind(&task.session_id)
        .bind(status_str)
        .bind(&task.status_message)
        .bind(&task.created_at)
        .bind(&task.last_updated_at)
        .bind(task.ttl)
        .bind(task.poll_interval.map(|v| v as i64))
        .bind(&task.original_method)
        .bind(&original_params_json)
        .bind(&result_json)
        .bind(&meta_json)
        .execute(&self.pool)
        .await?;

        Ok(task)
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError> {
        let row = sqlx::query(
            r#"
            SELECT task_id, session_id, status, status_message, created_at,
                   last_updated_at, ttl, poll_interval, original_method,
                   original_params, result, meta
            FROM tasks WHERE task_id = ?
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_task_record(row)?)),
            None => Ok(None),
        }
    }

    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError> {
        let status_str = status_to_str(task.status);
        let original_params_json = task
            .original_params
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let result_json = task
            .result
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let meta_json = task
            .meta
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let rows_affected = sqlx::query(
            r#"
            UPDATE tasks SET
                session_id = ?, status = ?, status_message = ?,
                created_at = ?, last_updated_at = ?, ttl = ?,
                poll_interval = ?, original_method = ?,
                original_params = ?, result = ?, meta = ?
            WHERE task_id = ?
            "#,
        )
        .bind(&task.session_id)
        .bind(status_str)
        .bind(&task.status_message)
        .bind(&task.created_at)
        .bind(&task.last_updated_at)
        .bind(task.ttl)
        .bind(task.poll_interval.map(|v| v as i64))
        .bind(&task.original_method)
        .bind(&original_params_json)
        .bind(&result_json)
        .bind(&meta_json)
        .bind(&task.task_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(TaskStorageError::TaskNotFound(task.task_id));
        }

        Ok(())
    }

    async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError> {
        let rows_affected = sqlx::query("DELETE FROM tasks WHERE task_id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    async fn list_tasks(
        &self,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        let limit = limit.unwrap_or(self.config.default_page_size) as i64;
        // Fetch limit + 1 to determine if there's a next page
        let fetch_limit = limit + 1;

        let rows = if let Some(cursor_id) = cursor {
            // Two-step cursor resolution: look up the cursor task's (created_at, task_id)
            let cursor_row = sqlx::query(
                "SELECT created_at, task_id FROM tasks WHERE task_id = ?",
            )
            .bind(cursor_id)
            .fetch_optional(&self.pool)
            .await?;

            match cursor_row {
                Some(crow) => {
                    let cursor_created_at: String = crow.get("created_at");
                    let cursor_task_id: String = crow.get("task_id");

                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message, created_at,
                               last_updated_at, ttl, poll_interval, original_method,
                               original_params, result, meta
                        FROM tasks
                        WHERE (created_at, task_id) > (?, ?)
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT ?
                        "#,
                    )
                    .bind(&cursor_created_at)
                    .bind(&cursor_task_id)
                    .bind(fetch_limit)
                    .fetch_all(&self.pool)
                    .await?
                }
                None => {
                    // Cursor not found — start from beginning (graceful degradation)
                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message, created_at,
                               last_updated_at, ttl, poll_interval, original_method,
                               original_params, result, meta
                        FROM tasks
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT ?
                        "#,
                    )
                    .bind(fetch_limit)
                    .fetch_all(&self.pool)
                    .await?
                }
            }
        } else {
            sqlx::query(
                r#"
                SELECT task_id, session_id, status, status_message, created_at,
                       last_updated_at, ttl, poll_interval, original_method,
                       original_params, result, meta
                FROM tasks
                ORDER BY created_at ASC, task_id ASC
                LIMIT ?
                "#,
            )
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };

        let has_more = rows.len() as i64 > limit;
        let take_count = if has_more { limit as usize } else { rows.len() };

        let mut tasks = Vec::with_capacity(take_count);
        for row in rows.into_iter().take(take_count) {
            tasks.push(row_to_task_record(row)?);
        }

        let next_cursor = if has_more {
            tasks.last().map(|t| t.task_id.clone())
        } else {
            None
        };

        Ok(TaskListPage { tasks, next_cursor })
    }

    async fn update_task_status(
        &self,
        task_id: &str,
        new_status: TaskStatus,
        status_message: Option<String>,
    ) -> Result<TaskRecord, TaskStorageError> {
        // Fetch current status
        let current_row = sqlx::query("SELECT status FROM tasks WHERE task_id = ?")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        let current_status_str: String = current_row.get("status");
        let current_status = str_to_status(&current_status_str)?;

        // Validate state machine transition
        state_machine::validate_transition(current_status, new_status)?;

        let now = Self::now_iso8601();
        let new_status_str = status_to_str(new_status);

        sqlx::query(
            "UPDATE tasks SET status = ?, status_message = ?, last_updated_at = ? WHERE task_id = ?",
        )
        .bind(new_status_str)
        .bind(&status_message)
        .bind(&now)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        // Return updated record
        self.get_task(task_id)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))
    }

    async fn store_task_result(
        &self,
        task_id: &str,
        result: TaskOutcome,
    ) -> Result<(), TaskStorageError> {
        let result_json = serde_json::to_string(&result)?;
        let now = Self::now_iso8601();

        let rows_affected =
            sqlx::query("UPDATE tasks SET result = ?, last_updated_at = ? WHERE task_id = ?")
                .bind(&result_json)
                .bind(&now)
                .bind(task_id)
                .execute(&self.pool)
                .await?
                .rows_affected();

        if rows_affected == 0 {
            return Err(TaskStorageError::TaskNotFound(task_id.to_string()));
        }

        Ok(())
    }

    async fn get_task_result(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskOutcome>, TaskStorageError> {
        let row = sqlx::query("SELECT result FROM tasks WHERE task_id = ?")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        let result_str: Option<String> = row.get("result");
        match result_str {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError> {
        // Find expired task IDs first
        let expired_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT task_id FROM tasks
            WHERE ttl IS NOT NULL
              AND (julianday('now') - julianday(created_at)) * 86400000 > ttl
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        if !expired_ids.is_empty() {
            sqlx::query(
                r#"
                DELETE FROM tasks
                WHERE ttl IS NOT NULL
                  AND (julianday('now') - julianday(created_at)) * 86400000 > ttl
                "#,
            )
            .execute(&self.pool)
            .await?;

            debug!("Expired {} tasks", expired_ids.len());
        }

        Ok(expired_ids)
    }

    async fn task_count(&self) -> Result<usize, TaskStorageError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as usize)
    }

    async fn maintenance(&self) -> Result<(), TaskStorageError> {
        self.expire_tasks().await?;
        Ok(())
    }

    async fn list_tasks_for_session(
        &self,
        session_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        let limit = limit.unwrap_or(self.config.default_page_size) as i64;
        let fetch_limit = limit + 1;

        let rows = if let Some(cursor_id) = cursor {
            let cursor_row = sqlx::query(
                "SELECT created_at, task_id FROM tasks WHERE task_id = ?",
            )
            .bind(cursor_id)
            .fetch_optional(&self.pool)
            .await?;

            match cursor_row {
                Some(crow) => {
                    let cursor_created_at: String = crow.get("created_at");
                    let cursor_task_id: String = crow.get("task_id");

                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message, created_at,
                               last_updated_at, ttl, poll_interval, original_method,
                               original_params, result, meta
                        FROM tasks
                        WHERE session_id = ? AND (created_at, task_id) > (?, ?)
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT ?
                        "#,
                    )
                    .bind(session_id)
                    .bind(&cursor_created_at)
                    .bind(&cursor_task_id)
                    .bind(fetch_limit)
                    .fetch_all(&self.pool)
                    .await?
                }
                None => {
                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message, created_at,
                               last_updated_at, ttl, poll_interval, original_method,
                               original_params, result, meta
                        FROM tasks
                        WHERE session_id = ?
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT ?
                        "#,
                    )
                    .bind(session_id)
                    .bind(fetch_limit)
                    .fetch_all(&self.pool)
                    .await?
                }
            }
        } else {
            sqlx::query(
                r#"
                SELECT task_id, session_id, status, status_message, created_at,
                       last_updated_at, ttl, poll_interval, original_method,
                       original_params, result, meta
                FROM tasks
                WHERE session_id = ?
                ORDER BY created_at ASC, task_id ASC
                LIMIT ?
                "#,
            )
            .bind(session_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };

        let has_more = rows.len() as i64 > limit;
        let take_count = if has_more { limit as usize } else { rows.len() };

        let mut tasks = Vec::with_capacity(take_count);
        for row in rows.into_iter().take(take_count) {
            tasks.push(row_to_task_record(row)?);
        }

        let next_cursor = if has_more {
            tasks.last().map(|t| t.task_id.clone())
        } else {
            None
        };

        Ok(TaskListPage { tasks, next_cursor })
    }

    async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError> {
        let now = Self::now_iso8601();

        // Find non-terminal tasks where last_updated_at is older than max_age_ms
        let stuck_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT task_id FROM tasks
            WHERE status NOT IN ('completed', 'failed', 'cancelled')
              AND (julianday(?) - julianday(last_updated_at)) * 86400000 > ?
            "#,
        )
        .bind(&now)
        .bind(max_age_ms as i64)
        .fetch_all(&self.pool)
        .await?;

        if !stuck_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE tasks
                SET status = 'failed',
                    status_message = 'Server restarted — task interrupted',
                    last_updated_at = ?
                WHERE status NOT IN ('completed', 'failed', 'cancelled')
                  AND (julianday(?) - julianday(last_updated_at)) * 86400000 > ?
                "#,
            )
            .bind(&now)
            .bind(&now)
            .bind(max_age_ms as i64)
            .execute(&self.pool)
            .await?;

            debug!("Recovered {} stuck tasks", stuck_ids.len());
        }

        Ok(stuck_ids)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use serde_json::json;

    async fn create_temp_sqlite_storage() -> SqliteTaskStorage {
        let config = SqliteTaskConfig {
            database_path: ":memory:".into(),
            max_tasks: 10_000,
            ..SqliteTaskConfig::default()
        };
        SqliteTaskStorage::with_config(config).await.unwrap()
    }

    fn make_task(task_id: &str, session_id: Option<&str>) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            status: TaskStatus::Working,
            status_message: None,
            created_at: Utc::now().to_rfc3339(),
            last_updated_at: Utc::now().to_rfc3339(),
            ttl: None,
            poll_interval: None,
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        }
    }

    fn make_task_with_time(task_id: &str, created_at: &str) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            session_id: None,
            status: TaskStatus::Working,
            status_message: None,
            created_at: created_at.to_string(),
            last_updated_at: created_at.to_string(),
            ttl: None,
            poll_interval: None,
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        }
    }

    #[tokio::test]
    async fn test_sqlite_create_and_retrieve() {
        let storage = create_temp_sqlite_storage().await;
        let task = make_task("task-1", Some("session-abc"));

        let created = storage.create_task(task).await.unwrap();
        assert_eq!(created.task_id, "task-1");
        assert_eq!(created.status, TaskStatus::Working);
        assert_eq!(created.session_id, Some("session-abc".to_string()));

        let fetched = storage.get_task("task-1").await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.task_id, "task-1");
        assert_eq!(fetched.original_method, "tools/call");
        assert_eq!(fetched.session_id, Some("session-abc".to_string()));

        // Non-existent task returns None
        let missing = storage.get_task("nonexistent").await.unwrap();
        assert!(missing.is_none());

        // Delete
        assert!(storage.delete_task("task-1").await.unwrap());
        assert!(!storage.delete_task("task-1").await.unwrap());
        assert!(storage.get_task("task-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_sqlite_state_machine_enforcement() {
        let storage = create_temp_sqlite_storage().await;
        storage
            .create_task(make_task("task-sm", None))
            .await
            .unwrap();

        // Working -> Completed: valid
        let updated = storage
            .update_task_status("task-sm", TaskStatus::Completed, Some("Done".to_string()))
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.status_message, Some("Done".to_string()));

        // Completed -> Working: invalid (terminal state)
        let err = storage
            .update_task_status("task-sm", TaskStatus::Working, None)
            .await
            .unwrap_err();
        match err {
            TaskStorageError::TerminalState(s) => assert_eq!(s, TaskStatus::Completed),
            other => panic!("Expected TerminalState, got: {:?}", other),
        }

        // Test InputRequired cycle
        storage
            .create_task(make_task("task-ir", None))
            .await
            .unwrap();
        storage
            .update_task_status(
                "task-ir",
                TaskStatus::InputRequired,
                Some("Need input".to_string()),
            )
            .await
            .unwrap();
        storage
            .update_task_status("task-ir", TaskStatus::Working, Some("Resuming".to_string()))
            .await
            .unwrap();
        storage
            .update_task_status("task-ir", TaskStatus::Failed, None)
            .await
            .unwrap();

        // Working -> Working: invalid
        storage
            .create_task(make_task("task-ww", None))
            .await
            .unwrap();
        let err = storage
            .update_task_status("task-ww", TaskStatus::Working, None)
            .await
            .unwrap_err();
        match err {
            TaskStorageError::InvalidTransition { current, requested } => {
                assert_eq!(current, TaskStatus::Working);
                assert_eq!(requested, TaskStatus::Working);
            }
            other => panic!("Expected InvalidTransition, got: {:?}", other),
        }

        // Non-existent task
        let err = storage
            .update_task_status("no-such-task", TaskStatus::Completed, None)
            .await
            .unwrap_err();
        match err {
            TaskStorageError::TaskNotFound(id) => assert_eq!(id, "no-such-task"),
            other => panic!("Expected TaskNotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqlite_cursor_pagination() {
        let storage = create_temp_sqlite_storage().await;

        // Create tasks with sequential timestamps for deterministic ordering
        for i in 0..5 {
            let task =
                make_task_with_time(&format!("task-{}", i), &format!("2025-01-01T00:00:0{}Z", i));
            storage.create_task(task).await.unwrap();
        }

        // Page 1: limit 2
        let page1 = storage.list_tasks(None, Some(2)).await.unwrap();
        assert_eq!(page1.tasks.len(), 2);
        assert_eq!(page1.tasks[0].task_id, "task-0");
        assert_eq!(page1.tasks[1].task_id, "task-1");
        assert!(page1.next_cursor.is_some());

        // Page 2: using cursor from page 1
        let page2 = storage
            .list_tasks(page1.next_cursor.as_deref(), Some(2))
            .await
            .unwrap();
        assert_eq!(page2.tasks.len(), 2);
        assert_eq!(page2.tasks[0].task_id, "task-2");
        assert_eq!(page2.tasks[1].task_id, "task-3");
        assert!(page2.next_cursor.is_some());

        // Page 3: last page
        let page3 = storage
            .list_tasks(page2.next_cursor.as_deref(), Some(2))
            .await
            .unwrap();
        assert_eq!(page3.tasks.len(), 1);
        assert_eq!(page3.tasks[0].task_id, "task-4");
        assert!(page3.next_cursor.is_none());

        // Invalid cursor falls back to beginning
        let fallback = storage.list_tasks(Some("no-such-id"), Some(2)).await.unwrap();
        assert_eq!(fallback.tasks.len(), 2);
        assert_eq!(fallback.tasks[0].task_id, "task-0");
    }

    #[tokio::test]
    async fn test_sqlite_session_scoping() {
        let storage = create_temp_sqlite_storage().await;

        storage
            .create_task(make_task("task-a", Some("session-1")))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-b", Some("session-1")))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-c", Some("session-2")))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-d", None))
            .await
            .unwrap();

        let s1 = storage
            .list_tasks_for_session("session-1", None, None)
            .await
            .unwrap();
        assert_eq!(s1.tasks.len(), 2);

        let s2 = storage
            .list_tasks_for_session("session-2", None, None)
            .await
            .unwrap();
        assert_eq!(s2.tasks.len(), 1);
        assert_eq!(s2.tasks[0].task_id, "task-c");

        let empty = storage
            .list_tasks_for_session("session-3", None, None)
            .await
            .unwrap();
        assert_eq!(empty.tasks.len(), 0);

        // All tasks visible via global list
        let all = storage.list_tasks(None, None).await.unwrap();
        assert_eq!(all.tasks.len(), 4);
    }

    #[tokio::test]
    async fn test_sqlite_ttl_expiry() {
        let storage = create_temp_sqlite_storage().await;

        // Create task with very short TTL and old timestamp
        let mut task = make_task("task-expire", None);
        task.ttl = Some(1); // 1ms TTL
        task.created_at = "2020-01-01T00:00:00Z".to_string();
        task.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(task).await.unwrap();

        // Create a task without TTL
        let task2 = make_task("task-keep", None);
        storage.create_task(task2).await.unwrap();

        let expired = storage.expire_tasks().await.unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "task-expire");

        // Verify expired task is gone
        assert!(storage.get_task("task-expire").await.unwrap().is_none());
        // Verify other task still exists
        assert!(storage.get_task("task-keep").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_sqlite_result_round_trip() {
        let storage = create_temp_sqlite_storage().await;
        storage
            .create_task(make_task("task-res", None))
            .await
            .unwrap();

        // Store a success outcome
        let outcome =
            TaskOutcome::Success(json!({"content": [{"type": "text", "text": "hello"}]}));
        storage
            .store_task_result("task-res", outcome)
            .await
            .unwrap();

        let result = storage.get_task_result("task-res").await.unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            TaskOutcome::Success(v) => {
                assert_eq!(v["content"][0]["text"], "hello");
            }
            _ => panic!("Expected Success"),
        }

        // Also verify it's in the task record
        let task = storage.get_task("task-res").await.unwrap().unwrap();
        assert!(task.result.is_some());

        // Store an error outcome on a different task
        storage
            .create_task(make_task("task-err", None))
            .await
            .unwrap();
        let error_outcome = TaskOutcome::Error {
            code: -32010,
            message: "Tool failed".to_string(),
            data: Some(json!({"detail": "oops"})),
        };
        storage
            .store_task_result("task-err", error_outcome)
            .await
            .unwrap();

        let err_result = storage.get_task_result("task-err").await.unwrap().unwrap();
        match err_result {
            TaskOutcome::Error {
                code,
                message,
                data,
            } => {
                assert_eq!(code, -32010);
                assert_eq!(message, "Tool failed");
                assert_eq!(data.unwrap()["detail"], "oops");
            }
            _ => panic!("Expected Error"),
        }

        // Non-existent task returns error
        let err = storage.get_task_result("no-task").await.unwrap_err();
        match err {
            TaskStorageError::TaskNotFound(id) => assert_eq!(id, "no-task"),
            other => panic!("Expected TaskNotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqlite_recover_stuck() {
        let storage = create_temp_sqlite_storage().await;

        // Create a "stuck" working task with old timestamp
        let mut stuck = make_task("task-stuck", None);
        stuck.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(stuck).await.unwrap();

        // Create a recent working task that should NOT be recovered
        let recent = make_task("task-recent", None);
        storage.create_task(recent).await.unwrap();

        // Create a completed task with old timestamp (should NOT be touched)
        let mut completed = make_task("task-done", None);
        completed.status = TaskStatus::Completed;
        completed.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(completed).await.unwrap();

        // Recover with 5 minute threshold
        let recovered = storage.recover_stuck_tasks(300_000).await.unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0], "task-stuck");

        // Verify stuck task is now Failed
        let task = storage.get_task("task-stuck").await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(
            task.status_message,
            Some("Server restarted — task interrupted".to_string())
        );

        // Verify recent task is still Working
        let recent = storage.get_task("task-recent").await.unwrap().unwrap();
        assert_eq!(recent.status, TaskStatus::Working);

        // Verify completed task is untouched
        let done = storage.get_task("task-done").await.unwrap().unwrap();
        assert_eq!(done.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_sqlite_max_tasks() {
        let config = SqliteTaskConfig {
            database_path: ":memory:".into(),
            max_tasks: 2,
            ..SqliteTaskConfig::default()
        };
        let storage = SqliteTaskStorage::with_config(config).await.unwrap();

        storage
            .create_task(make_task("task-1", None))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-2", None))
            .await
            .unwrap();

        let result = storage.create_task(make_task("task-3", None)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskStorageError::MaxTasksReached(n) => assert_eq!(n, 2),
            other => panic!("Expected MaxTasksReached, got: {:?}", other),
        }

        // Verify count
        assert_eq!(storage.task_count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_update_task() {
        let storage = create_temp_sqlite_storage().await;
        let task = make_task("task-upd", Some("sess-1"));
        storage.create_task(task).await.unwrap();

        let mut updated = storage.get_task("task-upd").await.unwrap().unwrap();
        updated.original_params = Some(json!({"key": "value"}));
        updated.meta = Some(HashMap::from([("info".to_string(), json!("data"))]));
        updated.poll_interval = Some(5000);

        storage.update_task(updated).await.unwrap();

        let fetched = storage.get_task("task-upd").await.unwrap().unwrap();
        assert_eq!(fetched.original_params, Some(json!({"key": "value"})));
        assert_eq!(fetched.poll_interval, Some(5000));
        assert!(fetched.meta.is_some());
        assert_eq!(fetched.meta.unwrap()["info"], json!("data"));

        // Update non-existent task errors
        let err = storage
            .update_task(make_task("no-such-task", None))
            .await
            .unwrap_err();
        match err {
            TaskStorageError::TaskNotFound(id) => assert_eq!(id, "no-such-task"),
            other => panic!("Expected TaskNotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqlite_maintenance() {
        let storage = create_temp_sqlite_storage().await;

        let mut task = make_task("task-maint", None);
        task.ttl = Some(1);
        task.created_at = "2020-01-01T00:00:00Z".to_string();
        task.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(task).await.unwrap();

        storage.maintenance().await.unwrap();

        assert!(storage.get_task("task-maint").await.unwrap().is_none());
    }

    // === Parity tests ===

    #[tokio::test]
    async fn parity_create_and_retrieve() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_create_and_retrieve(&storage).await;
    }

    #[tokio::test]
    async fn parity_state_machine_enforcement() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_state_machine_enforcement(&storage).await;
    }

    #[tokio::test]
    async fn parity_terminal_state_rejection() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_terminal_state_rejection(&storage).await;
    }

    #[tokio::test]
    async fn parity_cursor_determinism() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_cursor_determinism(&storage).await;
    }

    #[tokio::test]
    async fn parity_session_scoping() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_session_scoping(&storage).await;
    }

    #[tokio::test]
    async fn parity_ttl_expiry() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_ttl_expiry(&storage).await;
    }

    #[tokio::test]
    async fn parity_task_result_round_trip() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_task_result_round_trip(&storage).await;
    }

    #[tokio::test]
    async fn parity_recover_stuck_tasks() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_recover_stuck_tasks(&storage).await;
    }

    #[tokio::test]
    async fn parity_max_tasks_limit() {
        let config = SqliteTaskConfig {
            database_path: ":memory:".into(),
            max_tasks: 5,
            ..SqliteTaskConfig::default()
        };
        let storage = SqliteTaskStorage::with_config(config).await.unwrap();
        crate::parity_tests::test_max_tasks_limit(&storage, 5).await;
    }

    #[tokio::test]
    async fn parity_error_mapping() {
        let storage = create_temp_sqlite_storage().await;
        crate::parity_tests::test_error_mapping_parity(&storage).await;
    }

    #[tokio::test]
    async fn parity_concurrent_status_updates() {
        let storage = std::sync::Arc::new(create_temp_sqlite_storage().await);
        crate::parity_tests::test_concurrent_status_updates(storage).await;
    }
}
