//! PostgreSQL [`TaskStore`] — restart-durable and **shared across
//! instances**, which is the reason this backend exists.
//!
//! Every instance pointed at the same database sees the same tasks: any of
//! them can serve `tasks/get`, record a `tasks/update`, or flip a
//! `tasks/cancel`. That is the deployment shape the superseded 2025 store was
//! built for and the one this extension needs.
//!
//! Schema and division of labour are identical to [`super::sqlite`]: one JSON
//! document per task keyed by `task_id` (SEP-2663 removed `tasks/list`, so
//! nothing queries by anything else), with every status rule, owner check and
//! `tasks/update` key decision owned by [`super::traits`]. This module only
//! loads, applies, and stores.
//!
//! Atomicity: `SELECT … FOR UPDATE` inside a transaction. Two instances
//! racing the same task serialise on the row rather than clobbering each
//! other — the difference that matters once more than one process is writing.

use std::collections::HashMap;

use serde_json::Value;
use sqlx::{PgPool, Row};
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

/// PostgreSQL-backed [`TaskStore`].
pub struct PostgresTaskStore {
    pool: PgPool,
}

impl PostgresTaskStore {
    /// Connect with a libpq-style URL (`postgres://…`, or
    /// `postgres:///db?host=/var/run/postgresql` for a unix socket).
    pub async fn connect(url: &str) -> Result<Self, TaskStoreError> {
        let pool = PgPool::connect(url).await.map_err(backend)?;
        Self::from_pool(pool).await
    }

    /// Adopt an existing pool (shared with the rest of an application).
    pub async fn from_pool(pool: PgPool) -> Result<Self, TaskStoreError> {
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
                -- The document keeps the RFC3339 STRING verbatim so it round
                -- trips unchanged; this column is a real timestamp so age
                -- comparisons do not depend on every producer emitting the
                -- same offset and precision. Text ordering only equals
                -- chronological ordering by convention, which is too fragile
                -- to hang retention on.
                last_updated_at TIMESTAMPTZ NOT NULL,
                state           JSONB NOT NULL
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

    /// Load under a row lock, apply a shared transition, store — one
    /// transaction. `FOR UPDATE` is what makes this safe with several
    /// instances writing concurrently.
    async fn with_task<T>(
        &self,
        task_id: &str,
        f: impl FnOnce(&mut TaskState) -> Result<T, TaskStoreError>,
    ) -> Result<T, TaskStoreError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;

        let row = sqlx::query("SELECT state FROM ext_tasks WHERE task_id = $1 FOR UPDATE")
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;

        let mut state: TaskState =
            serde_json::from_value(row.get::<Value, _>("state")).map_err(backend)?;

        // A failed transition must not persist a partial mutation: the
        // transaction drops without commit, releasing the lock.
        let out = f(&mut state)?;

        write_state(&mut tx, &state).await?;
        tx.commit().await.map_err(backend)?;
        Ok(out)
    }
}

async fn write_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &TaskState,
) -> Result<(), TaskStoreError> {
    let doc = serde_json::to_value(state).map_err(backend)?;
    // Parsed into a real instant for the column; the document keeps the
    // original string byte-for-byte, so `tasks/get` returns exactly what the
    // caller stored rather than this backend's re-rendering of it.
    let ts: chrono::DateTime<chrono::Utc> =
        chrono::DateTime::parse_from_rfc3339(&state.fields.last_updated_at)
            .map_err(backend)?
            .with_timezone(&chrono::Utc);
    sqlx::query(
        r#"
        INSERT INTO ext_tasks (task_id, status, last_updated_at, state)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (task_id) DO UPDATE SET
            status = EXCLUDED.status,
            last_updated_at = EXCLUDED.last_updated_at,
            state = EXCLUDED.state
        "#,
    )
    .bind(&state.fields.task_id)
    .bind(super::traits::status_name(state.status))
    .bind(ts)
    .bind(doc)
    .execute(&mut **tx)
    .await
    .map_err(backend)?;
    Ok(())
}

#[async_trait::async_trait]
impl TaskStore for PostgresTaskStore {
    async fn create(&self, state: TaskState) -> Result<(), TaskStoreError> {
        // Committed before this returns: SEP-2663 line 302 requires a
        // `tasks/get` to resolve the instant `CreateTaskResult` is answered —
        // including a `tasks/get` served by a different instance.
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
        let row = sqlx::query("SELECT state FROM ext_tasks WHERE task_id = $1")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let state: TaskState =
            serde_json::from_value(row.get::<Value, _>("state")).map_err(backend)?;
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
        // Candidates are read WITHOUT a lock, then each is mutated through
        // `with_task`, which takes that one row's lock.
        //
        // The obvious implementation — `SELECT … FOR UPDATE` over the table
        // in one transaction — locks EVERY row for the sweep's duration,
        // stalling task mutations on every instance. That is worst precisely
        // where this backend earns its place: the shared multi-instance
        // deployment. Re-checking `sweep_action` inside `with_task` makes the
        // unlocked read safe: a task that a real writer touched in between is
        // simply no longer a candidate.
        let mut report = SweepReport::default();

        // Narrow with `idx_ext_tasks_sweep (status, last_updated_at)` where a
        // cutoff is known. A per-task `ttlMs` lives inside the JSON document
        // and cannot use that index, so those rows are fetched by status
        // alone — bounded by live tasks, not by history.
        let orphan_cutoff = policy
            .orphan_after_ms
            .map(|ms| now - chrono::Duration::milliseconds(ms as i64));
        let terminal_cutoff = policy
            .delete_terminal_after_ms
            .map(|ms| now - chrono::Duration::milliseconds(ms as i64));

        let rows = sqlx::query(
            r#"
            SELECT state FROM ext_tasks
            WHERE ($1::timestamptz IS NOT NULL
                     AND status IN ('working','input_required')
                     AND last_updated_at < $1)
               OR ($2::timestamptz IS NOT NULL
                     AND status IN ('completed','failed','cancelled')
                     AND last_updated_at < $2)
               OR ($3 AND status IN ('working','input_required'))
            "#,
        )
        .bind(orphan_cutoff)
        .bind(terminal_cutoff)
        .bind(policy.honour_task_ttl)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;

        for row in rows {
            let state: TaskState =
                serde_json::from_value(row.get::<Value, _>("state")).map_err(backend)?;
            let id = state.fields.task_id.clone();
            match sweep_action(&state, now, policy) {
                SweepAction::Keep => {}
                SweepAction::Delete => {
                    let deleted = sqlx::query("DELETE FROM ext_tasks WHERE task_id = $1")
                        .bind(&id)
                        .execute(&self.pool)
                        .await
                        .map_err(backend)?;
                    if deleted.rows_affected() > 0 {
                        report.deleted.push(id);
                    }
                }
                SweepAction::MarkFailed(reason) => {
                    // Re-decides under the row lock, so a task a real writer
                    // advanced between the read and here is left alone.
                    let outcome = self
                        .with_task(&id, |s| {
                            if sweep_action(s, now, policy) == SweepAction::Keep {
                                return Err(TaskStoreError::NotFound(id.clone()));
                            }
                            s.status = TaskStatus::Failed;
                            s.error = Some(sweep_error(reason));
                            s.input_requests = None;
                            s.fields.last_updated_at = now_rfc3339();
                            Ok(())
                        })
                        .await;
                    match outcome {
                        Ok(()) => report.failed.push(id),
                        Err(TaskStoreError::NotFound(_)) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod conformance {
    use super::*;
    use crate::v2026_07_28::parity;

    /// Base URL with NO database path — the scratch database name is
    /// appended. Default targets the local unix socket with an explicit role,
    /// because the role cannot be inferred: sqlx falls back to "anonymous"
    /// when `$USER` is absent, which is exactly how these tests silently
    /// skipped the first time they were run.
    /// No fallback on purpose. The previous default hardcoded one developer's
    /// unix username, so it connected on exactly one machine and failed
    /// everywhere else, CI included. The URL is derived once, by
    /// `scripts/ext-tasks-backends.sh`, which is also what starts the server —
    /// deriving it a second time here is how the two drift apart.
    fn base_url() -> String {
        std::env::var("TURUL_TEST_PG_URL").expect(
            "TURUL_TEST_PG_URL is unset — run this suite via scripts/ext-tasks-backends.sh, \
             which provisions Postgres and exports the URL",
        )
    }

    fn admin_url() -> String {
        format!("{}/postgres", base_url())
    }

    fn db_url(db: &str) -> String {
        format!("{}/{db}", base_url())
    }

    /// Connect to the admin database, or explain why the test cannot run.
    ///
    /// **Unreachable Postgres FAILS the test.** It does not skip. An earlier
    /// version returned early with a printed SKIP and the suite reported
    /// green with Postgres never contacted — the precise failure mode this
    /// crate's whole parity effort exists to prevent. Opt out deliberately
    /// with `TURUL_SKIP_PG_TESTS=1` if a machine genuinely has no server.
    async fn admin_pool() -> Option<PgPool> {
        if std::env::var("TURUL_SKIP_PG_TESTS").is_ok() {
            eprintln!("TURUL_SKIP_PG_TESTS set — skipping Postgres conformance deliberately");
            return None;
        }
        match PgPool::connect(&admin_url()).await {
            Ok(p) => Some(p),
            Err(e) => panic!(
                "Postgres unreachable at {}: {e}\n\
                 These tests must not pass without contacting a server. Start \
                 Postgres, set TURUL_TEST_PG_URL to a base URL with no database \
                 path, or set TURUL_SKIP_PG_TESTS=1 to opt out on purpose.",
                admin_url()
            ),
        }
    }

    /// Each run gets its own database, dropped afterwards, so a failed run
    /// never poisons the next one.
    async fn scratch_db() -> Option<(String, PgPool)> {
        let admin = admin_pool().await?;
        // A panicking test skips drop_scratch, so databases accumulate on a
        // developer machine. Reap anything older than an hour first — old
        // enough that it cannot belong to a concurrent run.
        if let Ok(rows) = sqlx::query_scalar::<_, String>(
            "SELECT datname FROM pg_database WHERE datname LIKE 'turul_ext_tasks_%'",
        )
        .fetch_all(&admin)
        .await
        {
            let cutoff = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
                .saturating_sub(3_600_000_000_000u128);
            for db in rows {
                if let Some(ns) = db
                    .strip_prefix("turul_ext_tasks_")
                    .and_then(|n| n.parse::<u128>().ok())
                    && ns < cutoff
                {
                    let _ =
                        sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE IF EXISTS {db}")))
                            .execute(&admin)
                            .await;
                }
            }
        }

        let name = format!(
            "turul_ext_tasks_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        // `name` is generated from a timestamp — digits and underscores only,
        // never user input. AssertSqlSafe is the sanctioned way to say so;
        // sqlx rejects dynamic SQL strings otherwise, which is right.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .expect("create scratch database");
        admin.close().await;

        let pool = PgPool::connect(&db_url(&name))
            .await
            .expect("connect scratch");
        Some((name, pool))
    }

    async fn drop_scratch(name: &str, pool: PgPool) {
        pool.close().await;
        if let Ok(admin) = PgPool::connect(&admin_url()).await {
            let _ = sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE IF EXISTS {name}"
            )))
            .execute(&admin)
            .await;
            admin.close().await;
        }
    }

    /// The same contract the other backends satisfy, against real Postgres.
    #[tokio::test]
    async fn postgres_satisfies_the_contract() {
        let Some((name, pool)) = scratch_db().await else {
            return;
        };
        let store = PostgresTaskStore::from_pool(pool.clone())
            .await
            .expect("migrate");
        parity::run_all(&store).await;
        drop_scratch(&name, pool).await;
    }

    /// The reason for this backend: two independent stores over one database
    /// are two instances. One opens an input round; the other completes it.
    /// An in-memory store cannot do this at all.
    #[tokio::test]
    async fn a_second_instance_completes_a_round_the_first_opened() {
        let Some((name, pool)) = scratch_db().await else {
            return;
        };
        let url = db_url(&name);

        let instance_a = PostgresTaskStore::from_pool(pool.clone())
            .await
            .expect("migrate");
        instance_a
            .create(parity::seed("t-shared", None))
            .await
            .expect("create");
        instance_a
            .require_input("t-shared", parity::one_request("k"), Some("st".into()))
            .await
            .expect("require_input");

        // A genuinely separate connection pool — a different process would
        // look exactly like this.
        let instance_b = PostgresTaskStore::connect(&url)
            .await
            .expect("second instance");
        let seen = instance_b
            .get("t-shared", None)
            .await
            .expect("get")
            .expect("a second instance must see the first instance's task");
        assert_eq!(seen.status, super::super::types::TaskStatus::InputRequired);

        let mut responses = HashMap::new();
        responses.insert("k".to_string(), parity::accept());
        match instance_b.provide_input("t-shared", None, responses).await {
            Ok(InputDelivery::Complete { request_state, .. }) => {
                assert_eq!(request_state.as_deref(), Some("st"));
            }
            other => panic!("the second instance must complete the round; got {other:?}"),
        }

        // And instance A sees the result of instance B's write.
        let after = instance_a
            .get("t-shared", None)
            .await
            .expect("get")
            .expect("task");
        assert_eq!(
            after.status,
            super::super::types::TaskStatus::Working,
            "the first instance must observe the second's transition"
        );

        drop_scratch(&name, pool).await;
    }
}
