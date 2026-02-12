//! PostgreSQL task storage backend.
//!
//! Production-ready PostgreSQL backend for persistent task storage
//! across multiple server instances. Ideal for distributed deployments
//! requiring task sharing and coordination.

use crate::error::TaskStorageError;
use crate::state_machine;
use crate::traits::{TaskListPage, TaskOutcome, TaskRecord, TaskStorage};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use turul_mcp_protocol::TaskStatus;

/// Configuration for PostgreSQL task storage.
#[derive(Debug, Clone)]
pub struct PostgresTaskConfig {
    /// Database connection URL (e.g. "postgres://localhost:5432/mcp_tasks")
    pub database_url: String,
    /// Maximum number of database connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections in the pool
    pub min_connections: u32,
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
    /// Statement timeout in seconds
    pub statement_timeout_secs: u32,
}

impl Default for PostgresTaskConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://localhost:5432/mcp_tasks".to_string(),
            max_connections: 20,
            min_connections: 2,
            connection_timeout_secs: 30,
            cleanup_interval_minutes: 5,
            max_tasks: 10_000,
            default_page_size: 50,
            create_tables_if_missing: true,
            statement_timeout_secs: 30,
        }
    }
}

/// PostgreSQL-backed task storage implementation.
///
/// Uses connection pooling, optimistic locking via a `version` column,
/// JSONB for structured data, and cursor-based pagination ordered by
/// `(created_at, task_id)`.
pub struct PostgresTaskStorage {
    pool: PgPool,
    config: PostgresTaskConfig,
}

impl PostgresTaskStorage {
    /// Create a new PostgreSQL task storage with default configuration.
    pub async fn new() -> Result<Self, TaskStorageError> {
        Self::with_config(PostgresTaskConfig::default()).await
    }

    /// Create a new PostgreSQL task storage with custom configuration.
    pub async fn with_config(config: PostgresTaskConfig) -> Result<Self, TaskStorageError> {
        info!(
            "Initializing PostgreSQL task storage at {}",
            mask_db_url(&config.database_url)
        );

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                config.connection_timeout_secs,
            ))
            .idle_timeout(Some(std::time::Duration::from_secs(300))) // 5 minutes
            .max_lifetime(Some(std::time::Duration::from_secs(1800))) // 30 minutes
            .test_before_acquire(true)
            .connect(&config.database_url)
            .await?;

        let storage = Self { pool, config };

        if storage.config.create_tables_if_missing {
            storage.migrate().await?;
        }

        storage.start_cleanup_task();

        info!("PostgreSQL task storage initialized successfully");
        Ok(storage)
    }

    /// Create a PostgreSQL task storage from an existing connection pool.
    pub async fn with_pool(
        pool: PgPool,
        config: PostgresTaskConfig,
    ) -> Result<Self, TaskStorageError> {
        let storage = Self { pool, config };

        if storage.config.create_tables_if_missing {
            storage.migrate().await?;
        }

        storage.start_cleanup_task();

        Ok(storage)
    }

    /// Run database schema migrations.
    async fn migrate(&self) -> Result<(), TaskStorageError> {
        debug!("Running PostgreSQL task storage migrations");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                session_id TEXT,
                status TEXT NOT NULL DEFAULT 'working',
                status_message TEXT,
                created_at TEXT NOT NULL,
                last_updated_at TEXT NOT NULL,
                ttl BIGINT,
                poll_interval BIGINT,
                original_method TEXT NOT NULL,
                original_params JSONB,
                result JSONB,
                meta JSONB,
                version INTEGER NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes — not using CONCURRENTLY because IF NOT EXISTS + CONCURRENTLY
        // cannot run inside a transaction, and these are idempotent anyway.
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_tasks_list ON tasks (created_at, task_id)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks (session_id, created_at, task_id)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status)",
        ];

        for index_sql in &indexes {
            sqlx::query(index_sql).execute(&self.pool).await?;
        }

        // Partial index for active tasks — useful for recover_stuck_tasks
        if let Err(e) = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_active ON tasks (last_updated_at) WHERE status IN ('working', 'input_required')",
        )
        .execute(&self.pool)
        .await
        {
            debug!("Partial index creation note: {}", e);
        }

        debug!("PostgreSQL task storage migrations completed");
        Ok(())
    }

    /// Start background cleanup task for expired tasks.
    fn start_cleanup_task(&self) {
        let pool = self.pool.clone();
        let interval_minutes = self.config.cleanup_interval_minutes;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_minutes as u64 * 60));

            loop {
                interval.tick().await;

                if let Err(e) = run_background_cleanup(&pool).await {
                    warn!("Background task cleanup failed: {}", e);
                }
            }
        });
    }

    fn now_iso8601() -> String {
        Utc::now().to_rfc3339()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn row_to_task_record(row: &PgRow) -> Result<TaskRecord, TaskStorageError> {
    let status_str: String = row.get("status");
    let status = str_to_status(&status_str)?;

    let original_params: Option<Value> = row.get("original_params");
    let result_json: Option<Value> = row.get("result");
    let meta_json: Option<Value> = row.get("meta");

    let result = result_json
        .map(|v| serde_json::from_value::<TaskOutcome>(v))
        .transpose()?;

    let meta = meta_json
        .map(|v| serde_json::from_value::<HashMap<String, Value>>(v))
        .transpose()?;

    let poll_interval: Option<i64> = row.get("poll_interval");

    Ok(TaskRecord {
        task_id: row.get("task_id"),
        session_id: row.get("session_id"),
        status,
        status_message: row.get("status_message"),
        created_at: row.get("created_at"),
        last_updated_at: row.get("last_updated_at"),
        ttl: row.get("ttl"),
        poll_interval: poll_interval.map(|v| v as u64),
        original_method: row.get("original_method"),
        original_params,
        result,
        meta,
    })
}

/// Background cleanup: expire tasks whose TTL has elapsed.
async fn run_background_cleanup(pool: &PgPool) -> Result<(), TaskStorageError> {
    let mut tx = pool.begin().await?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE ttl IS NOT NULL
          AND created_at::timestamptz + make_interval(secs := ttl::double precision / 1000.0) < NOW()
        "#,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if deleted > 0 {
        debug!("Background cleanup: expired {} tasks", deleted);
    }

    tx.commit().await?;
    Ok(())
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

// ---------------------------------------------------------------------------
// TaskStorage implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl TaskStorage for PostgresTaskStorage {
    fn backend_name(&self) -> &'static str {
        "postgresql"
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
            .map(|v| serde_json::to_value(v))
            .transpose()?;
        let result_json = task
            .result
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;
        let meta_json = task
            .meta
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO tasks (
                task_id, session_id, status, status_message,
                created_at, last_updated_at, ttl, poll_interval,
                original_method, original_params, result, meta, version
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 1)
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
            SELECT task_id, session_id, status, status_message,
                   created_at, last_updated_at, ttl, poll_interval,
                   original_method, original_params, result, meta, version
            FROM tasks WHERE task_id = $1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(ref r) => Ok(Some(row_to_task_record(r)?)),
            None => Ok(None),
        }
    }

    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError> {
        let status_str = status_to_str(task.status);
        let original_params_json = task
            .original_params
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;
        let result_json = task
            .result
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;
        let meta_json = task
            .meta
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;

        let rows_affected = sqlx::query(
            r#"
            UPDATE tasks SET
                session_id = $1,
                status = $2,
                status_message = $3,
                created_at = $4,
                last_updated_at = $5,
                ttl = $6,
                poll_interval = $7,
                original_method = $8,
                original_params = $9,
                result = $10,
                meta = $11,
                version = version + 1
            WHERE task_id = $12
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
        let rows_affected = sqlx::query("DELETE FROM tasks WHERE task_id = $1")
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

        let rows = if let Some(cursor_id) = cursor {
            // Two-step cursor: resolve cursor → (created_at, task_id), then paginate
            let cursor_row =
                sqlx::query("SELECT created_at, task_id FROM tasks WHERE task_id = $1")
                    .bind(cursor_id)
                    .fetch_optional(&self.pool)
                    .await?;

            match cursor_row {
                Some(ref cr) => {
                    let cursor_created_at: String = cr.get("created_at");
                    let cursor_task_id: String = cr.get("task_id");

                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message,
                               created_at, last_updated_at, ttl, poll_interval,
                               original_method, original_params, result, meta, version
                        FROM tasks
                        WHERE (created_at, task_id) > ($1, $2)
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT $3
                        "#,
                    )
                    .bind(&cursor_created_at)
                    .bind(&cursor_task_id)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
                }
                // Cursor not found: start from beginning (graceful degradation)
                None => {
                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message,
                               created_at, last_updated_at, ttl, poll_interval,
                               original_method, original_params, result, meta, version
                        FROM tasks
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT $1
                        "#,
                    )
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
                }
            }
        } else {
            sqlx::query(
                r#"
                SELECT task_id, session_id, status, status_message,
                       created_at, last_updated_at, ttl, poll_interval,
                       original_method, original_params, result, meta, version
                FROM tasks
                ORDER BY created_at ASC, task_id ASC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        let tasks: Vec<TaskRecord> = rows
            .iter()
            .map(row_to_task_record)
            .collect::<Result<Vec<_>, _>>()?;

        // Determine next_cursor: if we got a full page, there may be more
        let next_cursor = if tasks.len() as i64 == limit {
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
        // Step 1: Read current status + version
        let current_row = sqlx::query("SELECT status, version FROM tasks WHERE task_id = $1")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        let current_status_str: String = current_row.get("status");
        let current_status = str_to_status(&current_status_str)?;
        let current_version: i32 = current_row.get("version");

        // Step 2: Validate state machine transition
        state_machine::validate_transition(current_status, new_status)?;

        // Step 3: UPDATE with optimistic locking
        let now = Self::now_iso8601();
        let new_status_str = status_to_str(new_status);

        let rows_affected = sqlx::query(
            r#"
            UPDATE tasks SET
                status = $1,
                status_message = $2,
                last_updated_at = $3,
                version = version + 1
            WHERE task_id = $4 AND version = $5
            "#,
        )
        .bind(new_status_str)
        .bind(&status_message)
        .bind(&now)
        .bind(task_id)
        .bind(current_version)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(TaskStorageError::ConcurrentModification(format!(
                "Task {} was modified by another writer",
                task_id
            )));
        }

        // Step 4: Fetch the updated record
        self.get_task(task_id)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))
    }

    async fn store_task_result(
        &self,
        task_id: &str,
        result: TaskOutcome,
    ) -> Result<(), TaskStorageError> {
        let result_json = serde_json::to_value(&result)?;
        let now = Self::now_iso8601();

        let rows_affected = sqlx::query(
            r#"
            UPDATE tasks SET
                result = $1,
                last_updated_at = $2,
                version = version + 1
            WHERE task_id = $3
            "#,
        )
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
        let row = sqlx::query("SELECT result FROM tasks WHERE task_id = $1")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        let result_json: Option<Value> = row.get("result");
        match result_json {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError> {
        let mut tx = self.pool.begin().await?;

        let expired_ids: Vec<String> = sqlx::query_scalar(
            r#"
            DELETE FROM tasks
            WHERE ttl IS NOT NULL
              AND created_at::timestamptz + make_interval(secs := ttl::double precision / 1000.0) < NOW()
            RETURNING task_id
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;

        if !expired_ids.is_empty() {
            debug!("Expired {} tasks", expired_ids.len());
        }

        tx.commit().await?;
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

        // Analyze the table so the query planner has up-to-date statistics
        sqlx::query("ANALYZE tasks").execute(&self.pool).await?;

        debug!("PostgreSQL task maintenance completed");
        Ok(())
    }

    async fn list_tasks_for_session(
        &self,
        session_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        let limit = limit.unwrap_or(self.config.default_page_size) as i64;

        let rows = if let Some(cursor_id) = cursor {
            let cursor_row =
                sqlx::query("SELECT created_at, task_id FROM tasks WHERE task_id = $1")
                    .bind(cursor_id)
                    .fetch_optional(&self.pool)
                    .await?;

            match cursor_row {
                Some(ref cr) => {
                    let cursor_created_at: String = cr.get("created_at");
                    let cursor_task_id: String = cr.get("task_id");

                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message,
                               created_at, last_updated_at, ttl, poll_interval,
                               original_method, original_params, result, meta, version
                        FROM tasks
                        WHERE session_id = $1
                          AND (created_at, task_id) > ($2, $3)
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT $4
                        "#,
                    )
                    .bind(session_id)
                    .bind(&cursor_created_at)
                    .bind(&cursor_task_id)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
                }
                None => {
                    sqlx::query(
                        r#"
                        SELECT task_id, session_id, status, status_message,
                               created_at, last_updated_at, ttl, poll_interval,
                               original_method, original_params, result, meta, version
                        FROM tasks
                        WHERE session_id = $1
                        ORDER BY created_at ASC, task_id ASC
                        LIMIT $2
                        "#,
                    )
                    .bind(session_id)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
                }
            }
        } else {
            sqlx::query(
                r#"
                SELECT task_id, session_id, status, status_message,
                       created_at, last_updated_at, ttl, poll_interval,
                       original_method, original_params, result, meta, version
                FROM tasks
                WHERE session_id = $1
                ORDER BY created_at ASC, task_id ASC
                LIMIT $2
                "#,
            )
            .bind(session_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        let tasks: Vec<TaskRecord> = rows
            .iter()
            .map(row_to_task_record)
            .collect::<Result<Vec<_>, _>>()?;

        let next_cursor = if tasks.len() as i64 == limit {
            tasks.last().map(|t| t.task_id.clone())
        } else {
            None
        };

        Ok(TaskListPage { tasks, next_cursor })
    }

    async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError> {
        let now = Self::now_iso8601();

        let recovered_ids: Vec<String> = sqlx::query_scalar(
            r#"
            UPDATE tasks SET
                status = 'failed',
                status_message = 'Server restarted — task interrupted',
                last_updated_at = $1,
                version = version + 1
            WHERE status IN ('working', 'input_required')
              AND last_updated_at::timestamptz + make_interval(secs := $2::double precision / 1000.0) < NOW()
            RETURNING task_id
            "#,
        )
        .bind(&now)
        .bind(max_age_ms as f64)
        .fetch_all(&self.pool)
        .await?;

        if !recovered_ids.is_empty() {
            info!("Recovered {} stuck tasks", recovered_ids.len());
        }

        Ok(recovered_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn create_test_storage() -> Result<PostgresTaskStorage, TaskStorageError> {
        let config = PostgresTaskConfig {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:test@localhost:5432/test".to_string()),
            max_tasks: 10_000,
            create_tables_if_missing: true,
            ..PostgresTaskConfig::default()
        };
        PostgresTaskStorage::with_config(config).await
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
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_create_and_retrieve() {
        let storage = create_test_storage().await.unwrap();

        let task = make_task("pg-test-create-1", Some("sess-1"));
        let created = storage.create_task(task).await.unwrap();
        assert_eq!(created.task_id, "pg-test-create-1");
        assert_eq!(created.status, TaskStatus::Working);
        assert_eq!(created.session_id, Some("sess-1".to_string()));

        let fetched = storage.get_task("pg-test-create-1").await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.task_id, "pg-test-create-1");
        assert_eq!(fetched.original_method, "tools/call");

        // Cleanup
        storage.delete_task("pg-test-create-1").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_state_machine_enforcement() {
        let storage = create_test_storage().await.unwrap();

        let task = make_task("pg-test-sm-1", None);
        storage.create_task(task).await.unwrap();

        // Valid: Working -> Completed
        let updated = storage
            .update_task_status(
                "pg-test-sm-1",
                TaskStatus::Completed,
                Some("Done".to_string()),
            )
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);

        // Invalid: Completed -> Working (terminal state)
        let err = storage
            .update_task_status("pg-test-sm-1", TaskStatus::Working, None)
            .await;
        assert!(err.is_err());
        match err.unwrap_err() {
            TaskStorageError::TerminalState(s) => assert_eq!(s, TaskStatus::Completed),
            other => panic!("Expected TerminalState, got: {:?}", other),
        }

        // Cleanup
        storage.delete_task("pg-test-sm-1").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_optimistic_locking() {
        let storage = create_test_storage().await.unwrap();

        let task = make_task("pg-test-lock-1", None);
        storage.create_task(task).await.unwrap();

        // First update succeeds
        storage
            .update_task_status(
                "pg-test-lock-1",
                TaskStatus::InputRequired,
                Some("Waiting".to_string()),
            )
            .await
            .unwrap();

        // Second update also succeeds (sequential, no real concurrency conflict)
        storage
            .update_task_status(
                "pg-test-lock-1",
                TaskStatus::Completed,
                Some("Done".to_string()),
            )
            .await
            .unwrap();

        // Cleanup
        storage.delete_task("pg-test-lock-1").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_cursor_pagination() {
        let storage = create_test_storage().await.unwrap();

        // Create tasks with sequential timestamps for consistent ordering
        for i in 0..5 {
            let task = make_task_with_time(
                &format!("pg-test-page-{}", i),
                &format!("2025-01-01T00:00:0{}Z", i),
            );
            storage.create_task(task).await.unwrap();
        }

        // Page 1: limit 2
        let page1 = storage.list_tasks(None, Some(2)).await.unwrap();
        assert_eq!(page1.tasks.len(), 2);
        assert_eq!(page1.tasks[0].task_id, "pg-test-page-0");
        assert_eq!(page1.tasks[1].task_id, "pg-test-page-1");
        assert!(page1.next_cursor.is_some());

        // Page 2
        let page2 = storage
            .list_tasks(page1.next_cursor.as_deref(), Some(2))
            .await
            .unwrap();
        assert_eq!(page2.tasks.len(), 2);
        assert_eq!(page2.tasks[0].task_id, "pg-test-page-2");
        assert_eq!(page2.tasks[1].task_id, "pg-test-page-3");

        // Page 3: last page
        let page3 = storage
            .list_tasks(page2.next_cursor.as_deref(), Some(2))
            .await
            .unwrap();
        assert_eq!(page3.tasks.len(), 1);
        assert_eq!(page3.tasks[0].task_id, "pg-test-page-4");
        assert!(page3.next_cursor.is_none());

        // Cleanup
        for i in 0..5 {
            storage
                .delete_task(&format!("pg-test-page-{}", i))
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_session_scoping() {
        let storage = create_test_storage().await.unwrap();

        storage
            .create_task(make_task("pg-test-sess-a", Some("session-alpha")))
            .await
            .unwrap();
        storage
            .create_task(make_task("pg-test-sess-b", Some("session-alpha")))
            .await
            .unwrap();
        storage
            .create_task(make_task("pg-test-sess-c", Some("session-beta")))
            .await
            .unwrap();

        let alpha = storage
            .list_tasks_for_session("session-alpha", None, None)
            .await
            .unwrap();
        assert_eq!(alpha.tasks.len(), 2);

        let beta = storage
            .list_tasks_for_session("session-beta", None, None)
            .await
            .unwrap();
        assert_eq!(beta.tasks.len(), 1);
        assert_eq!(beta.tasks[0].task_id, "pg-test-sess-c");

        let empty = storage
            .list_tasks_for_session("session-gamma", None, None)
            .await
            .unwrap();
        assert_eq!(empty.tasks.len(), 0);

        // Cleanup
        for id in ["pg-test-sess-a", "pg-test-sess-b", "pg-test-sess-c"] {
            storage.delete_task(id).await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_ttl_expiry() {
        let storage = create_test_storage().await.unwrap();

        // Create a task with very short TTL and old timestamp
        let mut task = make_task("pg-test-ttl-1", None);
        task.ttl = Some(1); // 1ms TTL
        task.created_at = "2020-01-01T00:00:00Z".to_string();
        task.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(task).await.unwrap();

        // Create a task without TTL
        let task2 = make_task("pg-test-ttl-keep", None);
        storage.create_task(task2).await.unwrap();

        let expired = storage.expire_tasks().await.unwrap();
        assert!(expired.contains(&"pg-test-ttl-1".to_string()));

        // Verify expired task is gone
        assert!(storage.get_task("pg-test-ttl-1").await.unwrap().is_none());
        // Verify other task still exists
        assert!(
            storage
                .get_task("pg-test-ttl-keep")
                .await
                .unwrap()
                .is_some()
        );

        // Cleanup
        storage.delete_task("pg-test-ttl-keep").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_result_round_trip() {
        let storage = create_test_storage().await.unwrap();

        let task = make_task("pg-test-result-1", None);
        storage.create_task(task).await.unwrap();

        // Store a success outcome
        let outcome = TaskOutcome::Success(json!({"content": [{"type": "text", "text": "hello"}]}));
        storage
            .store_task_result("pg-test-result-1", outcome)
            .await
            .unwrap();

        let result = storage.get_task_result("pg-test-result-1").await.unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            TaskOutcome::Success(v) => {
                assert_eq!(v["content"][0]["text"], "hello");
            }
            _ => panic!("Expected Success"),
        }

        // Store an error outcome (overwrite)
        let error_outcome = TaskOutcome::Error {
            code: -32010,
            message: "Tool failed".to_string(),
            data: Some(json!({"detail": "oops"})),
        };
        storage
            .store_task_result("pg-test-result-1", error_outcome)
            .await
            .unwrap();

        let result2 = storage
            .get_task_result("pg-test-result-1")
            .await
            .unwrap()
            .unwrap();
        match result2 {
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

        // Cleanup
        storage.delete_task("pg-test-result-1").await.unwrap();
    }

    // === Parity tests (all require Docker PostgreSQL) ===

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_create_and_retrieve() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_create_and_retrieve(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_state_machine_enforcement() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_state_machine_enforcement(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_terminal_state_rejection() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_terminal_state_rejection(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_cursor_determinism() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_cursor_determinism(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_session_scoping() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_session_scoping(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_ttl_expiry() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_ttl_expiry(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_task_result_round_trip() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_task_result_round_trip(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_recover_stuck_tasks() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_recover_stuck_tasks(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_max_tasks_limit() {
        let config = PostgresTaskConfig {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:test@localhost:5432/test".to_string()),
            max_tasks: 5,
            create_tables_if_missing: true,
            ..PostgresTaskConfig::default()
        };
        let storage = PostgresTaskStorage::with_config(config).await.unwrap();
        crate::parity_tests::test_max_tasks_limit(&storage, 5).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_error_mapping() {
        let storage = create_test_storage().await.unwrap();
        crate::parity_tests::test_error_mapping_parity(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL connection"]
    async fn parity_concurrent_status_updates() {
        let storage = std::sync::Arc::new(create_test_storage().await.unwrap());
        crate::parity_tests::test_concurrent_status_updates(storage).await;
    }
}
