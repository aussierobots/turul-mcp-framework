# ADR-018: Task Pagination Cursor Contract

**Status**: Accepted

**Date**: 2026-02-11

## Context

The `turul-mcp-task-storage` crate provides cursor-based pagination for task listing across four storage backends: InMemory, SQLite, PostgreSQL, and DynamoDB. Each backend has different native query capabilities, yet MCP clients expect a consistent pagination experience regardless of the backend in use.

The two listing operations exposed by `TaskStorage` are:

- `list_tasks(cursor, limit)` -- global listing of all tasks
- `list_tasks_for_session(session_id, cursor, limit)` -- scoped to a single session

Both return `TaskListPage { tasks: Vec<TaskRecord>, next_cursor: Option<String> }`. When `next_cursor` is `Some`, the client passes it back on the next request to continue iteration. When it is `None`, the client has reached the end of the result set.

The challenge is defining a cursor contract that works deterministically across relational databases (SQLite, PostgreSQL) and a key-value store (DynamoDB) while keeping each implementation idiomatic to its backend.

## Decision

### Sort Order

All backends sort task listings by `(created_at ASC, task_id ASC)`. This compound ordering is deterministic even when multiple tasks share the same `created_at` timestamp, because `task_id` is unique and breaks ties lexicographically.

SQL backends enforce this with indexes:

```sql
CREATE INDEX idx_tasks_list ON tasks (created_at, task_id);
CREATE INDEX idx_tasks_session ON tasks (session_id, created_at, task_id);
```

The InMemory backend sorts an in-memory `Vec` with the same comparator:

```rust
sorted.sort_by(|a, b| {
    a.created_at
        .cmp(&b.created_at)
        .then_with(|| a.task_id.cmp(&b.task_id))
});
```

### Cursor Semantics

For InMemory, SQLite, and PostgreSQL, the cursor value is the `task_id` of the last record returned in the current page. To resume, the backend performs a two-step cursor resolution:

1. **Resolve**: Look up the cursor `task_id` to obtain its `(created_at, task_id)` tuple.
2. **Paginate**: Return records strictly after that position in the sort order.

In SQL this is expressed as a compound row comparison:

```sql
WHERE (created_at, task_id) > ($cursor_created_at, $cursor_task_id)
ORDER BY created_at ASC, task_id ASC
LIMIT $limit
```

If the cursor `task_id` does not exist (e.g., the task was deleted by TTL cleanup between requests), all three backends degrade gracefully by starting from the beginning of the result set rather than returning an error.

The `next_cursor` in the response is set to the `task_id` of the last record on the current page when more records exist, or `None` when the page contains the final records.

### DynamoDB Exception

DynamoDB does not support arbitrary compound `ORDER BY` across partitions. The backend handles its two listing modes differently:

**`list_tasks_for_session`** (session-scoped): Uses a GSI Query on `SessionIndex` with `session_id` as the partition key. Within a single partition, DynamoDB returns items in sort-key order, which provides deterministic `(created_at, task_id)` ordering. The cursor is `base64(JSON(LastEvaluatedKey))`, using DynamoDB's native `ExclusiveStartKey` pagination. This operation is fully deterministic.

**`list_tasks`** (global): Uses a DynamoDB Scan operation. Items within each Scan page are sorted in Rust by `(created_at, task_id)`, but cross-page determinism is NOT guaranteed because Scan returns items in arbitrary partition order. The cursor is still `base64(JSON(LastEvaluatedKey))`. This operation is best-effort ordered.

**Rationale for not adding a global ordering GSI**: A GSI with a fixed partition key (e.g., `pk = "ALL"`) would funnel every write to a single partition, creating a hot key that violates DynamoDB best practices and throttles under load. The trade-off of best-effort global ordering is acceptable because:

- Session-scoped listing is the primary use case (tasks belong to sessions).
- Global listing is an administrative/debugging operation where approximate order is sufficient.
- Production usage typically relies on `list_tasks_for_session` (session-scoped) rather than global listing.

## Consequences

### Positive

- **Deterministic session-scoped pagination** on all four backends, verified by the shared `test_cursor_determinism` parity test.
- **Graceful cursor invalidation** -- deleted cursors restart from the beginning rather than failing, which is resilient to TTL cleanup between paginated requests.
- **Idiomatic implementations** -- SQL backends use compound row comparisons with supporting indexes; DynamoDB uses native `LastEvaluatedKey`/`ExclusiveStartKey`; InMemory uses simple Vec scanning. No backend is forced into an unnatural access pattern.
- **Opaque cursors** -- clients treat cursors as opaque strings. The backend is free to change the cursor encoding (task_id vs. base64 key map) without breaking the API contract.
- **Efficient keyset pagination** -- SQL backends avoid `OFFSET`-based pagination, which degrades at large offsets. The compound `WHERE (created_at, task_id) > (?, ?)` clause uses index seeks.

### Negative

- **Global `list_tasks` on DynamoDB is best-effort** -- clients that page through all tasks globally may see items in inconsistent order across pages. This is documented and tested only for session-scoped listing in the parity suite.
- **Cursor is backend-specific** -- a cursor from one backend cannot be used with another. This is not a practical concern (backends are not swapped at runtime) but means cursor values are not portable.
- **Graceful degradation on missing cursors may silently restart** -- if a cursor is invalidated by TTL cleanup, the client receives a page from the beginning rather than an error. This prevents failures but could cause the client to re-process already-seen tasks if it does not track seen IDs independently.

## Parity Testing

The shared `test_cursor_determinism` function in `crates/turul-mcp-task-storage/src/parity_tests.rs` validates the contract:

1. Creates 10 tasks with identical `created_at` timestamps but different `task_id` values.
2. Pages through with `limit=3` using `list_tasks_for_session`.
3. Collects all returned `task_id` values across pages.
4. Asserts the collected order matches alphabetically sorted `task_id` -- confirming that `task_id` correctly breaks `created_at` ties.

This test runs on all four backends for session-scoped listing. DynamoDB's global `list_tasks` is explicitly excluded from cross-page determinism assertions.

## See Also

- [ADR-016](./016-task-storage-architecture.md) -- Task storage trait design and backend selection
- [ADR-017](./017-task-runtime-executor-boundary.md) -- Task runtime-executor boundary and state management
- [ADR-015: MCP 2025-11-25 Protocol Crate Strategy](./015-mcp-2025-11-25-protocol-crate.md) -- Protocol crate that defines task types
