# ADR-023: Tool Change Detection and Notification

**Status:** Accepted
**Date:** 2026-03-29
**Context:** MCP 2025-11-25 compliance for tool change signaling

## Decision

Two separate mechanisms handle tool changes at different boundaries:

- **Fingerprint** — restart/redeploy boundary. Detects compiled tool set changes across server restarts.
- **Live registry + notification** — runtime boundary. Handles in-process and cross-instance tool mutations.

These are separate code paths serving different purposes. They do not intersect.

## Session Validation vs Tool Version Sync

| Check | Purpose | When |
|-------|---------|------|
| Session existence | Is this `Mcp-Session-Id` valid? | Every request, first |
| Session termination | Has this session been ended? | Every request, second |
| Fingerprint comparison | Has the compiled tool set changed since this session was created? | Every request, after session is confirmed valid |

If the session does not exist or is terminated, the request is rejected immediately (404). Fingerprint is NOT checked for invalid sessions.

## Fingerprint — Tool Version Sync (Dynamic Mode Only)

FNV-1a hash of the full canonicalized `Tool` descriptor set. JSON is recursively key-sorted before hashing to ensure determinism regardless of HashMap iteration order. Only computed and checked when `listChanged=true` (Dynamic mode).

- Stored per-session during `initialize` as `mcp:tool_fingerprint`
- Checked on every request via `validate_session_exists()` (after session existence check)
- **Mismatch does NOT invalidate the session** — it means tools changed since the session was created
- On mismatch: stored fingerprint is updated to current, `notifications/tools/list_changed` broadcast, session continues
- 404 is ONLY for missing or terminated sessions, never for fingerprint mismatch

### Static mode

- `listChanged=false` — no fingerprint check, no notifications
- Tools are fixed at build time
- No change detection of any kind

## Live Registry + Notification — Runtime Changes

### Dynamic mode

- `listChanged=true` — truthful: server emits `notifications/tools/list_changed`
- `ToolRegistry` with `activate_tool()` / `deactivate_tool()` for precompiled tools
- Handlers read from the live registry — same session sees updated tools immediately
- `notifications/tools/list_changed` broadcast to connected clients as advisory signal
- **Session continues without disruption** — no 404, no re-initialization for runtime changes
- Client calls `tools/list` to refresh after receiving notification

### Cross-instance coordination (optional)

When `.server_state_storage()` is provided with a **shared** backend (PostgreSQL or DynamoDB):

- Tool activation state stored in shared storage accessible by all instances
- Startup sync via `sync_from_storage()`: compares local fingerprint against storage, updates if newer
- Without explicit storage, an in-memory backend is used (single-process, no coordination)
- SQLite is local durable — persists across restarts but does NOT enable cross-instance coordination

**EC2 (long-lived):** Background polling (default 10s) via `ToolRegistry::start_polling()`. On fingerprint mismatch, reloads tool state and broadcasts notification.

**Lambda (ephemeral):** Request-time detection via `ToolRegistry::check_for_changes()`. TTL-gated (default 10s, configurable via `TURUL_TOOL_CHECK_TTL_SECS`). Cold starts sync via `sync_from_storage()`.

## Client Flows

### New session (initialize)

1. Client sends `initialize` → server creates session
2. Server stores `mcp:tool_fingerprint` = current fingerprint in session state
3. Client sends `notifications/initialized` → session active
4. Client calls `tools/list` → reads from live ToolRegistry (current active tools)
5. No notifications needed — the session starts with current state

**Note:** The fingerprint stored during `initialize` is the baseline for future mismatch detection. Any tool change after session creation will be detected on the next request.

### Restart/redeploy (existing session)

1. Server restarts with different compiled tools → new fingerprint
2. Client's next request → session valid, fingerprint mismatch
3. Stored fingerprint updated to current → session continues
4. `notifications/tools/list_changed` broadcast (persisted to session events + live delivery)
5. Client calls `tools/list` → sees current tools
6. No manual reconnect or re-initialization needed

### In-process tool mutation (Dynamic)

1. `activate_tool()` / `deactivate_tool()` → live registry updated, fingerprint recomputed
2. `notifications/tools/list_changed` broadcast to all sessions
3. Notification persisted to session event storage AND delivered to active connections
4. Client receives notification → calls `tools/list`
5. Session continues — no disruption

### Cross-instance mutation — EC2 (Dynamic + storage)

1. Instance A changes tools → writes to shared storage
2. Instance B's polling task (10s) detects fingerprint change → reloads from storage
3. Instance B broadcasts `notifications/tools/list_changed` to its clients
4. Notification persisted + delivered
5. Session continues on all instances

### Cross-instance mutation — Lambda (Dynamic + storage)

1. EC2 or another Lambda changes tools → writes to shared storage
2. Client sends next POST to Lambda with `Mcp-Session-Id`
3. Lambda validates session — valid
4. Lambda calls `check_for_changes()` — reads fingerprint from shared storage (TTL-gated)
5. Fingerprint changed → Lambda reloads tool state
6. `notifications/tools/list_changed` broadcast, persisted to session events
7. Request processed with current tool state
8. Client receives notification → refreshes tools

## Notification Persistence Architecture

### Requirement

`notifications/tools/list_changed` MUST be persisted to session event storage before the request completes. Best-effort or detached-task delivery is not acceptable.

### SessionEventDispatcher

`SessionManager` is the single event bus. All emitters (ToolRegistry, SessionContext) go through `SessionManager::broadcast_event()`. Guaranteed persistence is provided by an awaited `SessionEventDispatcher` installed at the SessionManager layer — not at individual emitters.

```
Emitter (ToolRegistry, SessionContext, etc.)
  → SessionManager::broadcast_event()
    Phase 1: collect session IDs, fire in-memory listeners (under read lock)
    Phase 2: clone dispatcher Arc, drop lock, await dispatch per-session
      → StreamManager::broadcast_to_session() → store_event() + live delivery (AWAITED)
    Phase 3: fire global broadcast channel (observer-only, tests/debugging)
```

The dispatcher is:
- Defined as a trait in `turul-mcp-server` (`SessionEventDispatcher`)
- Implemented in `turul-mcp-server` behind `#[cfg(feature = "http")]` using `StreamManager`
- Installed from the runtime (McpServer, LambdaMcpServer) during server construction
- Without a dispatcher, events are best-effort only (in-memory channels)

### SSE Event Bridge

The existing SSE bridge task (spawned during server construction) is **observer-only** for `SessionEvent::Custom` events. It does NOT persist or deliver these events — the dispatcher handles that on the request path. The bridge remains for passive observation and non-Custom event types.

### No duplicate persistence

One authoritative persistence path: the awaited dispatcher. The bridge does not re-persist Custom events. Exactly 1 stored event per notification per session.

## Configuration

```rust
pub enum ToolChangeMode {
    Static,                     // Default. listChanged=false.
    #[cfg(feature = "dynamic-tools")]
    Dynamic,                    // Opt-in. listChanged=true.
}
```

| Mode | `listChanged` | Restart fingerprint | Runtime changes | Coordination |
|------|---|---|---|---|
| Static | false | Updated silently | Not supported | N/A |
| Dynamic | true | Updated + notification | Live registry + notification | InMemory (default) or shared backend |
| Dynamic + storage | true | Updated + notification | Live registry + notification | Polling (EC2, 10s) / Request-time (Lambda, 10s TTL) |

## Architectural Boundaries

**Transport boundary:** Core server emits `SessionEvent::Custom` via `SessionManager::broadcast_event()`. The `SessionEventDispatcher` (backed by StreamManager) handles persistence + delivery. No HTTP types in core server code — the dispatcher trait is transport-agnostic.

**Separation of concerns:** `validate_session_exists()` checks session validity AND fingerprint, but never sends notifications. Notification emission is the emitter's responsibility (ToolRegistry for tool changes). Persistence is the dispatcher's responsibility.

**Concurrency:** `RwLock<ToolState>` holds active tool set + fingerprint under a single lock. Read lock → clone → release → call. `broadcast_event()` collects session IDs under read lock, drops it, then awaits dispatcher. No lock held across await points.

**Lambda:** Participates in Dynamic mode via request-time change detection. `LambdaMcpServerBuilder` exposes `tool_change_mode()` and `server_state_storage()`. Dispatcher guarantees persistence before invocation completes.

## Fingerprint Storage

### Session fingerprint

- Key: `mcp:tool_fingerprint` in session state
- Written during `initialize`
- Compared against the server's current fingerprint on every request
- Updated on mismatch (session continues)

### Server fingerprint

- Key: `entityType=tools`, `entityId=#fingerprint` in `mcp-server-state` table
- Written by `sync_from_storage()` on cold start
- Updated by `activate_tool()` / `deactivate_tool()`
- Read by `check_for_changes()` (Lambda, TTL-gated) and `start_polling()` (EC2)

### Fingerprint determinism

`compute_tool_fingerprint()` canonicalizes JSON (recursive key sorting via BTreeMap) before FNV-1a hashing. This ensures identical tool sets produce identical fingerprints regardless of HashMap iteration order across processes or Lambda instances.

## Notification Wire Format

Uses `JsonRpcNotification` (not `ToolListChangedNotification` — protocol notification types lack the `jsonrpc` field):

```json
{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
```

## ServerStateStorage

Server-global state storage following the same pluggable-backend pattern as session and task storage.

- **InMemory** — default for single-process Dynamic mode
- **SQLite** — local durable mode
- **PostgreSQL** — shared relational deployments
- **DynamoDB** — serverless/AWS (camelCase attribute names)

Generic, not tool-specific. Same trait backs all entity types with `list_changed` notifications.

## Feature Flags

Default features: `["http", "sse"]` — no storage backends compiled by default.

Backend features (`sqlite`, `postgres`, `dynamodb`) forward to all storage crates uniformly. Weak dependency syntax (`?/`) activates backend features on `turul-mcp-server-state-storage` when `dynamic-tools` is also enabled.

```toml
# Minimal (in-memory only)
turul-mcp-server = "0.3"

# With DynamoDB + dynamic tools
turul-mcp-server = { version = "0.3", features = ["dynamodb", "dynamic-tools"] }
```

## Consequences

- `Static` (default): zero behavioral change, zero compiled overhead
- `Dynamic`: live tool changes without session disruption, truthful `listChanged=true`
- `Dynamic` + storage: cross-instance coordination with guaranteed notification persistence
- Fingerprint mismatch triggers update + notification, never session invalidation
- SessionEventDispatcher ensures notifications reach session event storage before request completes
- Single event bus architecture — all emitters go through SessionManager
