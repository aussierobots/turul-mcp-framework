# ADR-023: Tool Change Detection and Notification

**Status:** Accepted
**Date:** 2026-03-28
**Context:** MCP 2025-11-25 compliance for tool change signaling

## Decision

Two separate mechanisms handle tool changes at different boundaries:

- **Fingerprint** — restart/redeploy boundary. Detects compiled tool set changes.
- **Live registry + notification** — runtime boundary. Handles in-process and cross-instance tool mutations.

These are separate code paths serving different purposes. They do not intersect.

## Session Validation vs Tool Version Sync

These are distinct concerns:

| Check | Purpose | When |
|-------|---------|------|
| Session existence | Is this `Mcp-Session-Id` valid? | Every request, first |
| Session termination | Has this session been ended? | Every request, second |
| Fingerprint comparison | Has the compiled tool set changed since this session was created? | Every request, after session is confirmed valid |

If the session does not exist or is terminated, the request is rejected immediately. Fingerprint is NOT checked for invalid sessions.

## Fingerprint — Tool Version Sync

FNV-1a hash of the full serialized `Tool` descriptor set, computed once at server build time.

- Stored per-session during `initialize` as `mcp:tool_fingerprint`
- Checked on every request via `validate_session_exists()` (after session existence check)
- In the current implementation, because the HTTP handler fingerprint is build-time static, fingerprint mismatch occurs only across restart/redeploy boundaries, not from in-process mutations
- **Mismatch does NOT invalidate the session** — it means tools changed since the session was created
- On mismatch: stored fingerprint is updated to current, session continues
- 404 is ONLY for missing or terminated sessions, never for fingerprint mismatch

### FingerprintOnly mode

- `listChanged=false` — no live notification capability
- Fingerprint mismatch: stored fingerprint updated silently, session continues
- Client discovers new tools on their next `tools/list` call
- No runtime tool mutation support

## Live Registry + Notification — Runtime Changes

### DynamicInProcess mode

- `listChanged=true` — truthful: server emits `notifications/tools/list_changed`
- `ToolRegistry` with `activate_tool()` / `deactivate_tool()` for precompiled tools
- Handlers read from the live registry — same session sees updated tools immediately
- `notifications/tools/list_changed` broadcast to connected clients as advisory signal
- **Session continues without disruption** — no 404, no re-initialization for runtime changes
- Client calls `tools/list` to refresh after receiving notification
- The handler's build-time fingerprint does not change on runtime mutations, so no fingerprint mismatch is triggered by activate/deactivate
- Restart/redeploy with different compiled tools: fingerprint mismatch → 404 (same as FingerprintOnly)

### DynamicClustered mode

Extends DynamicInProcess with cross-instance coordination via shared `ServerStateStorage`:

- Tool activation state stored in shared storage (SQLite, PostgreSQL, DynamoDB)
- Startup sync via `sync_from_storage()`: compares local fingerprint against storage, updates if newer, logs warning for other stale instances
- Restart/redeploy: fingerprint mismatch → 404 (same as other modes)

**EC2 (long-lived) coordination:** Background polling (default 10-second interval) via `ToolRegistry::start_polling()`. Each instance checks the shared storage fingerprint; on mismatch, reloads tool state and broadcasts `notifications/tools/list_changed` to connected clients.

**Lambda (ephemeral) coordination:** Request-time change detection via `ToolRegistry::check_for_changes()`. Lambda cannot run background polling. Instead, on each request, if the cached fingerprint TTL (default 5 seconds) has expired, Lambda reads the current fingerprint from shared storage (single DynamoDB GetItem). If it differs from the local cache, Lambda reloads the full tool state and broadcasts `notifications/tools/list_changed`. Cold starts always sync via `sync_from_storage()`.

`listChanged=true` is truthful for both EC2 and Lambda — both detect tool changes and deliver notifications to connected clients during their respective interaction models.

## Configuration

```rust
pub enum ToolChangeMode {
    FingerprintOnly,                     // Default. listChanged=false.
    #[cfg(feature = "dynamic-tools")]
    DynamicInProcess,                    // Opt-in. listChanged=true. Single-process.
    #[cfg(feature = "dynamic-clustered")]
    DynamicClustered,                    // Opt-in. listChanged=true. Multi-instance.
}
```

| Mode | `listChanged` | Restart fingerprint | Runtime changes | EC2 | Lambda |
|------|---|---|---|---|---|
| FingerprintOnly | false | Updated silently | Not supported | Yes | Yes |
| DynamicInProcess | true | Updated + notification | Live registry + notification | Yes | N/A (single-process) |
| DynamicClustered | true | Updated + notification | Live registry + notification | Polling (10s) | Request-time (5s TTL) |

Note: "Restart fingerprint" column describes what happens when a session's stored fingerprint doesn't match the server's current fingerprint. The session is NEVER invalidated — fingerprint is updated and the session continues.

## Client Flows

### Restart/redeploy (all modes)

1. Server restarts with different compiled tools → new build-time fingerprint
2. Client's next request → session valid, fingerprint mismatch
3. Stored fingerprint updated to current → session continues
4. In dynamic modes: `notifications/tools/list_changed` broadcast
5. Client calls `tools/list` → sees current tools
6. No manual reconnect needed, no re-initialization needed

### In-process tool mutation (DynamicInProcess)

1. `activate_tool()` / `deactivate_tool()` → live registry updated
2. `notifications/tools/list_changed` broadcast to connected clients
3. Client receives notification → invalidates tool cache → calls `tools/list`
4. Session continues — no disruption, no 404

### Cross-instance mutation — EC2 (DynamicClustered)

1. Instance A changes tools → writes to shared storage
2. Instance B's polling task (10s) detects fingerprint change → reloads from storage
3. Instance B broadcasts `notifications/tools/list_changed` to its clients
4. Clients refresh → see updated tools
5. Session continues on all instances

### Cross-instance mutation — Lambda (DynamicClustered)

1. EC2 or another Lambda changes tools → writes to shared storage
2. Client sends next POST to Lambda with `Mcp-Session-Id`
3. Lambda loads session from DynamoDB — valid
4. Lambda checks cached tool fingerprint (TTL-based) against shared storage
5. Fingerprint changed → Lambda reloads tool state from storage
6. Lambda broadcasts `notifications/tools/list_changed`
7. Request processed with current tool state
8. Client receives notification → refreshes tools

## Architectural Boundaries

**Transport boundary:** Core server emits `SessionEvent::Custom` via `SessionManager.broadcast_event()`. HTTP SSE bridge handles delivery. No HTTP types in core server code.

**Separation of concerns:** `validate_session_exists()` checks session validity AND fingerprint, but never sends notifications. Notification delivery is `ToolRegistry`'s responsibility. These are separate code paths.

**Concurrency:** `RwLock<ToolState>` holds active tool set + fingerprint under a single lock. Read lock → clone `Arc<dyn McpTool>` → release → call. Never hold lock across await points.

**Lambda:** Participates in `DynamicClustered` via request-time change detection. `LambdaMcpServerBuilder` exposes `tool_change_mode()` and `server_state_storage()`. In `FingerprintOnly` mode (default), Lambda behaves as before. In `DynamicClustered` mode, Lambda checks shared storage on each request and delivers notifications via Streamable HTTP responses.

## Fingerprint Storage

Session-scoped compatibility metadata, not global server state.

- Key: `mcp:tool_fingerprint`
- Written during `initialize` (`SessionAwareInitializeHandler`)
- Compared against the handler's build-time static fingerprint
- Not updated after initial write

## Notification Wire Format

Uses `JsonRpcNotification` (not `ToolListChangedNotification` — protocol notification types lack the `jsonrpc` field):

```json
{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
```

## ServerStateStorage (DynamicClustered)

Server-global state storage following the same pluggable-backend pattern as `turul-mcp-session-storage` and `turul-mcp-task-storage`.

- **InMemory** — test double only
- **SQLite** — local durable mode
- **PostgreSQL** — shared relational deployments
- **DynamoDB** — serverless/AWS (camelCase attribute names)

**Generic, not tool-specific.** Same trait backs all entity types with `list_changed` notifications: tools, resources, prompts.

## Consequences

- `FingerprintOnly` (default): zero behavioral change from pre-feature baseline
- `DynamicInProcess`: live tool changes without session disruption, truthful `listChanged=true`
- `DynamicClustered`: extends DynamicInProcess with shared storage and cross-instance polling
- Fingerprint 404 handles deployment boundaries only
- Runtime mutations use the live registry — sessions are not disrupted
- `dynamic-tools` / `dynamic-clustered` Cargo features ensure zero compiled overhead when disabled
