//! SQLite [`TaskStore`] — restart-durable, and shared by every instance that
//! opens the same file.
//!
//! **Schema: one JSON document per task, keyed by `task_id`.** Unlike the
//! superseded 2025 store's column-per-field table, nothing here needs to
//! query by anything but the id: SEP-2663 removed `tasks/list`, so the whole
//! trait is keyed on `task_id`. A document keeps the backends' storage shapes
//! aligned with DynamoDB's and keeps the state machine in one place — see
//! [`super::traits`], which owns every status rule, owner check and
//! `tasks/update` key decision. This module only loads, applies, and stores.
//!
//! Atomicity: each mutation runs in a transaction that reads the row, applies
//! the shared transition, and writes it back. Two instances racing on the
//! same task serialise on the row.

use std::collections::HashMap;

use serde_json::Value;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use turul_mcp_protocol_2026_07_28::input_required::InputRequests;

use super::traits::{
    InputDelivery, RetentionPolicy, SweepAction, SweepReport, TaskState, TaskStore, TaskStoreError,
    apply_cancel, apply_complete, apply_fail, apply_provide_input, apply_require_input,
    now_rfc3339, owner_matches, sweep_action, sweep_error,
};
use super::types::TaskStatus;

fn backend(e: impl std::fmt::Display) -> TaskStoreError {
    TaskStoreError::Backend(e.to_string())
}

/// SQLite-backed [`TaskStore`].
pub struct SqliteTaskStore {
    pool: SqlitePool,
}

impl SqliteTaskStore {
    /// Open (creating if absent) a task database at `path`.
    pub async fn open(path: impl AsRef<std::path::Path>) -> Result<Self, TaskStoreError> {
        let options = SqliteConnectOptions::new()
            .filename(path.as_ref())
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await.map_err(backend)?;
        Self::from_pool(pool).await
    }

    /// A private in-memory database. Shared-cache URI with a unique name so
    /// every pooled connection sees the same database — a plain `:memory:`
    /// gives each connection its own, which silently breaks every test.
    pub async fn in_memory() -> Result<Self, TaskStoreError> {
        let unique = uuid_like();
        let uri = format!("file:{unique}?mode=memory&cache=shared");
        let pool = SqlitePool::connect(&uri).await.map_err(backend)?;
        Self::from_pool(pool).await
    }

    /// Adopt an existing pool (shared with the rest of an application).
    pub async fn from_pool(pool: SqlitePool) -> Result<Self, TaskStoreError> {
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<(), TaskStoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ext_tasks (
                task_id         TEXT PRIMARY KEY,
                -- Denormalised out of the document purely so a future
                -- recovery sweep can find stale `working` rows without
                -- deserialising every task.
                status          TEXT NOT NULL,
                last_updated_at TEXT NOT NULL,
                state           TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(backend)?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ext_tasks_sweep ON ext_tasks (status, last_updated_at)",
        )
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    /// Load, apply a shared transition, store — in one transaction.
    async fn with_task<T>(
        &self,
        task_id: &str,
        f: impl FnOnce(&mut TaskState) -> Result<T, TaskStoreError>,
    ) -> Result<T, TaskStoreError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;

        let row = sqlx::query("SELECT state FROM ext_tasks WHERE task_id = ?")
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;

        let mut state: TaskState =
            serde_json::from_str(row.get::<String, _>("state").as_str()).map_err(backend)?;

        // A failed transition must not persist a partial mutation: the
        // transaction is dropped without commit.
        let out = f(&mut state)?;

        write_state(&mut tx, &state).await?;
        tx.commit().await.map_err(backend)?;
        Ok(out)
    }
}

async fn write_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    state: &TaskState,
) -> Result<(), TaskStoreError> {
    let doc = serde_json::to_string(state).map_err(backend)?;
    sqlx::query(
        r#"
        INSERT INTO ext_tasks (task_id, status, last_updated_at, state)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(task_id) DO UPDATE SET
            status = excluded.status,
            last_updated_at = excluded.last_updated_at,
            state = excluded.state
        "#,
    )
    .bind(&state.fields.task_id)
    .bind(super::traits::status_name(state.status))
    .bind(&state.fields.last_updated_at)
    .bind(doc)
    .execute(&mut **tx)
    .await
    .map_err(backend)?;
    Ok(())
}

/// A unique name for the shared-cache in-memory URI. Avoids taking a `uuid`
/// dependency for one string.
fn unique_suffix() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn uuid_like() -> String {
    format!("turul-ext-tasks-{}", unique_suffix())
}

#[async_trait::async_trait]
impl TaskStore for SqliteTaskStore {
    async fn create(&self, state: TaskState) -> Result<(), TaskStoreError> {
        // Committed before this returns: SEP-2663 line 302 requires a
        // `tasks/get` to resolve the instant `CreateTaskResult` is answered.
        let mut tx = self.pool.begin().await.map_err(backend)?;
        write_state(&mut tx, &state).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn get(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        let row = sqlx::query("SELECT state FROM ext_tasks WHERE task_id = ?")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let state: TaskState =
            serde_json::from_str(row.get::<String, _>("state").as_str()).map_err(backend)?;
        // A foreign task reads as absent, never as forbidden.
        Ok(owner_matches(&state, owner).then_some(state))
    }

    async fn complete(&self, task_id: &str, result: Value) -> Result<TaskState, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_complete(state, result)?;
            Ok(state.clone())
        })
        .await
    }

    async fn fail(&self, task_id: &str, error: Value) -> Result<TaskState, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_fail(state, error)?;
            Ok(state.clone())
        })
        .await
    }

    async fn cancel(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        self.with_task(task_id, |state| {
            Ok(apply_cancel(state, owner)?.then(|| state.clone()))
        })
        .await
    }

    async fn require_input(
        &self,
        task_id: &str,
        requests: InputRequests,
        request_state: Option<String>,
    ) -> Result<TaskState, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_require_input(state, requests, request_state)?;
            Ok(state.clone())
        })
        .await
    }

    async fn provide_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: HashMap<String, Value>,
    ) -> Result<InputDelivery, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_provide_input(state, owner, responses)
        })
        .await
    }

    async fn sweep(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        policy: &RetentionPolicy,
    ) -> Result<SweepReport, TaskStoreError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let rows = sqlx::query("SELECT state FROM ext_tasks")
            .fetch_all(&mut *tx)
            .await
            .map_err(backend)?;

        let mut report = SweepReport::default();
        for row in rows {
            let mut state: TaskState =
                serde_json::from_str(row.get::<String, _>("state").as_str()).map_err(backend)?;
            match sweep_action(&state, now, policy) {
                SweepAction::Keep => {}
                SweepAction::Delete => {
                    sqlx::query("DELETE FROM ext_tasks WHERE task_id = ?")
                        .bind(&state.fields.task_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(backend)?;
                    report.deleted.push(state.fields.task_id);
                }
                SweepAction::MarkFailed(reason) => {
                    state.status = TaskStatus::Failed;
                    state.error = Some(sweep_error(reason));
                    state.input_requests = None;
                    state.fields.last_updated_at = now_rfc3339();
                    write_state(&mut tx, &state).await?;
                    report.failed.push(state.fields.task_id);
                }
            }
        }
        tx.commit().await.map_err(backend)?;
        Ok(report)
    }
}

#[cfg(test)]
mod conformance {
    use super::*;
    use crate::v2026_07_28::parity;

    /// The same contract the in-memory backend satisfies, against real SQL.
    #[tokio::test]
    async fn sqlite_satisfies_the_contract() {
        let store = SqliteTaskStore::in_memory().await.expect("open");
        parity::run_all(&store).await;
    }

    /// The point of this backend: a task outlives the process that made it.
    /// A second `SqliteTaskStore` over the same file is a stand-in for a
    /// second instance, which is the deployment shape a shared database is
    /// for.
    #[tokio::test]
    async fn a_task_survives_reopening_the_database() {
        let dir = std::env::temp_dir().join(format!("turul-tasks-{}", unique_suffix()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let path = dir.join("tasks.db");

        {
            let first = SqliteTaskStore::open(&path).await.expect("open");
            first
                .create(parity::seed("t-durable", None))
                .await
                .expect("create");
            first
                .require_input("t-durable", parity::one_request("k"), Some("st".into()))
                .await
                .expect("require_input");
        }

        let second = SqliteTaskStore::open(&path).await.expect("reopen");
        let got = second
            .get("t-durable", None)
            .await
            .expect("get")
            .expect("the task must survive a reopen");
        assert_eq!(got.status, super::super::types::TaskStatus::InputRequired);
        assert!(
            got.input_requests.expect("requests").contains_key("k"),
            "the outstanding input round must survive too, not just the row"
        );

        // A different instance can answer the round the first one opened.
        let mut responses = HashMap::new();
        responses.insert("k".to_string(), parity::accept());
        match second.provide_input("t-durable", None, responses).await {
            Ok(InputDelivery::Complete { request_state, .. }) => {
                assert_eq!(request_state.as_deref(), Some("st"));
            }
            other => panic!("a second instance must be able to complete the round; got {other:?}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
