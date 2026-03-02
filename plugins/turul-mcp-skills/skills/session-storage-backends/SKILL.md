---
name: session-storage-backends
description: >
  This skill should be used when the user asks about "session storage",
  "SessionStorage trait", "SqliteSessionStorage", "PostgresSessionStorage",
  "DynamoDbSessionStorage", "InMemorySessionStorage", "session backend",
  "session persistence", "session events", "SSE reconnection storage",
  "which storage backend", "session TTL", "session cleanup",
  "session event management", "SseEvent", or "SessionStorageError".
  Covers the SessionStorage trait, backend selection, event management
  for SSE resumability, error types, and background cleanup in the
  Turul MCP Framework (Rust).
---

# Session Storage Backends — Turul MCP Framework

Session storage persists MCP sessions across requests and manages SSE events for reconnection resumability. Four backends serve different deployment scenarios — choose based on persistence needs and scaling requirements.

## Backend Decision Tree

```
Need session persistence across restarts?
├─ No ──────────────────────────────────────→ InMemory (default, zero config)
└─ Yes
    ├─ Single server instance? ─────────────→ SQLite (file-based, no external deps)
    └─ Multiple instances / horizontal scaling?
        ├─ AWS / serverless? ───────────────→ DynamoDB (managed, auto-scaling)
        └─ Traditional infrastructure? ─────→ PostgreSQL (shared state, optimistic locking)
```

**Config details** (feature flags, Cargo.toml patterns, env vars, config struct fields) are in the [storage-backend-matrix](../../references/storage-backend-matrix.md) reference. This skill covers architecture, the trait API, event management, and decision-making.

## SessionStorage Trait

The `SessionStorage` trait has three method groups:

### Session Management

| Method | Signature | Purpose |
|---|---|---|
| `create_session` | `(capabilities) → SessionInfo` | Create session with auto-generated UUID v7 |
| `create_session_with_id` | `(id, capabilities) → SessionInfo` | Create with specific ID (testing only) |
| `get_session` | `(id) → Option<SessionInfo>` | Look up session by ID |
| `update_session` | `(SessionInfo) → ()` | Update entire session info |
| `set_session_state` | `(id, key, value) → ()` | Set a state key-value pair |
| `get_session_state` | `(id, key) → Option<Value>` | Get a state value by key |
| `remove_session_state` | `(id, key) → Option<Value>` | Remove and return a state value |
| `delete_session` | `(id) → bool` | Delete session completely |
| `list_sessions` | `() → Vec<String>` | List all session IDs |

### Event Management (SSE Resumability)

| Method | Signature | Purpose |
|---|---|---|
| `store_event` | `(id, SseEvent) → SseEvent` | Store event, assigns monotonic ID |
| `get_events_after` | `(id, after_event_id) → Vec<SseEvent>` | Get events since last seen (for reconnection) |
| `get_recent_events` | `(id, limit) → Vec<SseEvent>` | Get latest N events (for initial connection) |
| `delete_events_before` | `(id, before_event_id) → u64` | Delete old events (cleanup) |

### Cleanup

| Method | Signature | Purpose |
|---|---|---|
| `expire_sessions` | `(older_than) → Vec<String>` | Remove expired sessions, return their IDs |
| `session_count` | `() → usize` | Active session count (monitoring) |
| `event_count` | `() → usize` | Total event count across all sessions |
| `maintenance` | `() → ()` | Backend-specific maintenance (compaction, etc.) |

## Event Management for SSE

SSE reconnection relies on event storage. When a client disconnects and reconnects with `Last-Event-ID`, the server replays missed events.

```
Client connects     → GET /mcp (Accept: text/event-stream)
Server sends events → id: 1, id: 2, id: 3  (stored via store_event)
Client disconnects  → network interruption
Client reconnects   → GET /mcp (Last-Event-ID: 2)
Server replays      → get_events_after(session_id, 2) → [event 3, ...]
```

**Key behaviors:**
- `store_event()` assigns monotonic IDs — each event gets an ID higher than the previous
- `get_events_after()` is exclusive — `after_event_id: 2` returns events 3, 4, 5, ...
- `max_events_per_session` caps storage (default: 1000) — oldest events are evicted
- Keepalive events use SSE comment syntax (`: keepalive\n\n`) to preserve `Last-Event-ID`

## Backend-Specific Considerations

### InMemory
- **Zero config** — `McpServer::builder()` uses it by default
- **No persistence** — data lost on process restart
- **Synchronous cleanup** — no background task needed; expired sessions removed on access

### SQLite
- **File-based persistence** — single `.db` file, no external server
- **Connection pool** — `max_connections` default: 10
- **Background cleanup** — uses `tokio::time::sleep` loop (NOT `interval` — first-tick race)
- **Test gotcha** — `:memory:` with pools gives each connection its own DB. Use `file:{uuid}?mode=memory&cache=shared` for shared in-memory test databases

### PostgreSQL
- **Multi-instance safe** — multiple servers share the same session state
- **Optimistic locking** — `version` column prevents lost updates; `ConcurrentModification` error on conflict
- **Connection tuning** — `min_connections` (default: 2), `max_connections` (default: 20), `statement_timeout_secs` (default: 30)
- **Background cleanup** — same `tokio::time::sleep` pattern as SQLite

### DynamoDB
- **AWS-managed** — no server to operate, auto-scales
- **Native TTL** — AWS handles expiration (no background cleanup task needed)
- **Short default TTL** — `session_ttl_minutes: 5` and `event_ttl_minutes: 5` — **override for production** (e.g., 1440 for 24 hours)
- **Auto-create tables** — `create_tables_if_missing: true` (default) creates tables + GSIs on first use

## Error Types

All backends share `SessionStorageError` as a unified error type, with `From` conversions from backend-specific errors.

| Variant | When |
|---|---|
| `SessionNotFound(id)` | Session ID doesn't exist |
| `MaxSessionsReached(limit)` | InMemory session limit exceeded |
| `MaxEventsReached(limit)` | Event storage limit exceeded |
| `DatabaseError(msg)` | SQLite/PostgreSQL query failure |
| `SerializationError(msg)` | JSON serialization/deserialization failure |
| `ConnectionError(msg)` | Database connection failure |
| `MigrationError(msg)` | Schema migration failure |
| `AwsError(msg)` | DynamoDB SDK error |
| `AwsConfigurationError(msg)` | AWS credential/region configuration issue |
| `TableNotFound(table)` | DynamoDB table doesn't exist |
| `InvalidData(msg)` | Corrupt or unexpected session data |
| `ConcurrentModification(msg)` | PostgreSQL optimistic lock conflict |
| `Generic(msg)` | Catch-all |

## Background Cleanup

SQLite and PostgreSQL run background cleanup tasks. DynamoDB uses native TTL. InMemory is synchronous.

```rust
// Internal pattern (for reference — you don't write this code)
// SQLite/PostgreSQL cleanup loop:
loop {
    tokio::time::sleep(Duration::from_secs(cleanup_interval_secs)).await;
    storage.expire_sessions(cutoff_time).await?;
    storage.maintenance().await?;  // compaction, vacuum, etc.
}
```

**Why `sleep` not `interval`**: `tokio::time::interval` fires immediately on the first tick, which can race with newly-created sessions in tests. `sleep` defers the first cleanup by the full interval.

## Common Mistakes

1. **DynamoDB 5-minute TTL in production** — Default `session_ttl_minutes: 5` is for testing. Production deployments should set 60–1440+ minutes via `DynamoDbConfig { session_ttl_minutes: 1440, event_ttl_minutes: 1440, ..Default::default() }`.

2. **SQLite `:memory:` with connection pools** — Each pool connection gets its own isolated database. For shared test databases, use `file:{uuid}?mode=memory&cache=shared` as the database path.

3. **Forgetting `create_tables_if_missing: true`** — All backends default to `true`, but if you explicitly construct configs, ensure this is set. Without it, the first operation fails with a table-not-found error.

4. **Using InMemory in Lambda** — Lambda containers recycle unpredictably. Sessions stored in memory are lost. Use `DynamoDbSessionStorage` for persistence across invocations.

## Beyond This Skill

**Feature flags, Cargo.toml patterns, config fields?** → See the [storage-backend-matrix](../../references/storage-backend-matrix.md) reference for the complete config reference.

**Lambda deployment with DynamoDB?** → See the `lambda-deployment` skill for `LambdaMcpServerBuilder`, cold-start caching, and DynamoDB setup.

**Task storage backends?** → See the `task-patterns` skill for `TaskStorage` trait and backend configuration (mirrors session storage patterns).

**Server builder setup?** Use `McpServer::builder().with_session_storage(storage)`. See: [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)
