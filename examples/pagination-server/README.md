# Pagination Server Example

Paging through a result set that is far larger than one response. On startup
the server creates a throwaway SQLite database in a temp dir, seeds it with
10,000 synthetic users across 8 departments, and exposes tools that hand back
one page plus a cursor for the next.

> **This is application-level pagination, not the protocol's.** The cursor here
> lives inside the tool's own JSON payload (`pagination.next_cursor`) and comes
> back as the tool's `cursor` argument — a design the tool author owns.
> MCP's `cursor`/`nextCursor` are a different mechanism: the framework produces
> them on list operations (`tools/list`, `resources/list`,
> `resources/templates/list`, `prompts/list`) and they never appear inside a
> `tools/call` result. This example does not exercise them.
>
> Read it for the pattern you need when a *tool* has more rows than fit in one
> response.

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id` —
every request carries its own `_meta` plus the `MCP-Protocol-Version`,
`Mcp-Method` and `Mcp-Name` headers.

## Run

```bash
cargo run -p pagination-server -- --port 8777
cargo run -p pagination-server                # --port 0: OS picks, logged at startup
```

Startup takes a few seconds while the 10,000 rows are inserted. The database
lives in a `TempDir` the server holds alive, so it is deleted on exit and every
run starts from fresh synthetic data.

## Tools

### `list_users`

| Param | Type | Default | Notes |
|---|---|---|---|
| `cursor` | string, optional | start | the previous call's `pagination.next_cursor` |
| `limit` | integer, optional | 25 | over 100 → `-32602` out of range |
| `filter` | string, optional | — | substring match on name or email |
| `department` | string, optional | all | exact department match |
| `active_only` | bool, optional | `false` | |

### `search_users`

| Param | Type | Default | Notes |
|---|---|---|---|
| `query` | string, **required** | — | matched against name, email and department |
| `cursor` | string, optional | start | |
| `limit` | integer, optional | 20 | over 50 → `-32602` out of range |

Results are ordered by a SQL-computed relevance score (exact name match scores
highest, then name prefix, then email, then department), tie-broken by
`created_at DESC`.

### `refresh_data`

| Param | Type | Default | Notes |
|---|---|---|---|
| `operation` | string, optional | `update_activity` | `update_activity` flips activity flags and reports the affected row count; `full_stats` reports dataset statistics. Anything else → `-32602` |

## Where the cursor comes out

Each tool's `execute` returns `McpResult<Value>` and declares no `output =`
type, so the derive macro wraps the payload under the default `output` field.
The full path to the cursor on the wire is:

```
structuredContent.output.data.pagination.next_cursor
```

with the page under `structuredContent.output.data.users` (or `.results` for
`search_users`). The `pagination` block carries `has_more`, `next_cursor`,
`total` and `current_page_size`.

## How the cursor works

The cursor is a decimal row offset rendered as a string:

```rust
// SELECT ... ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?
let next_cursor = if (offset + limit) < total {
    Some((offset + limit).to_string())
} else {
    None            // no next page
};
```

Offset paging is the simplest thing that demonstrates the round trip, and it is
what this example does — it is **not** keyset pagination, so cost grows with
depth and a concurrent `refresh_data` can shift rows between pages. Reach for a
keyset cursor (last-seen `created_at`+`id`) if either matters to you.

The server keeps no per-client cursor state: two clients paging concurrently
cannot interfere, because position lives entirely in the argument the client
sends back.

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'
call() { curl -s -X POST http://127.0.0.1:8777/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H "Mcp-Name: $1" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"$1\",\"arguments\":$2}}"; }

# page 1 → pagination.next_cursor == "3"
call list_users '{"limit":3}'

# page 2 — feed that cursor straight back
call list_users '{"limit":3,"cursor":"3"}'

# filtered
call list_users '{"limit":5,"department":"Engineering","active_only":true}'

# search
call search_users '{"query":"Grace","limit":2}'

# over the cap → -32602 "Parameter 'limit' value 500 is out of range: 1-100"
call list_users '{"limit":500}'
```

## Data model

```rust
struct User {
    id: i64,
    name: String,
    email: String,
    created_at: DateTime<Utc>,
    is_active: bool,                     // ~80% of the seed data
    department: String,                  // one of 8
    last_login: Option<DateTime<Utc>>,
    profile_data: String,                // JSON blob, re-parsed into the result
}
```

A single `SqlitePool` is shared through a module-level `OnceLock<DatabaseManager>`;
the pool owns connection concurrency, so no lock is held across an `.await`.
Indexes exist on `created_at`, `is_active`, `department` and `name`.
