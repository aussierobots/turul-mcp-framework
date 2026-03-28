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

## Fingerprint — Restart/Redeploy Detection

FNV-1a hash of the full serialized `Tool` descriptor set, computed once at server build time.

- Stored per-session during `initialize` as `mcp:tool_fingerprint`
- Validated on every request via `validate_session_exists()` (after session existence check)
- In the current implementation, because the HTTP handler fingerprint is build-time static, fingerprint mismatch occurs only across restart/redeploy boundaries, not from in-process mutations
- **Mismatch → HTTP 404** — forces client re-initialization with fresh tools
- The server **MUST NOT** update the stored fingerprint for an existing session
- Only a fresh `initialize` writes the current fingerprint

### FingerprintOnly mode

- `listChanged=false` — no live notification capability
- Fingerprint mismatch → 404 is the only mechanism
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

Extends DynamicInProcess with cross-instance coordination:

- Tool activation state stored in shared `ServerStateStorage` (SQLite, PostgreSQL, DynamoDB)
- Background polling (default 10-second interval) via `ToolRegistry::start_polling()`
- Each instance checks the shared storage fingerprint; on mismatch, reloads tool state and broadcasts `notifications/tools/list_changed` to its connected clients
- Startup sync via `sync_from_storage()`: compares local fingerprint against storage, updates if newer, logs warning for other stale instances
- Restart/redeploy: fingerprint mismatch → 404 (same as other modes)

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

| Mode | `listChanged` | Restart 404 | Runtime changes | Scope |
|------|---|---|---|---|
| FingerprintOnly | false | Yes | Not supported | All runtimes |
| DynamicInProcess | true | Yes | Live registry + notification | Single-process HTTP |
| DynamicClustered | true | Yes | Live registry + notification + polling | Multi-instance |

## Client Flows

### Restart/redeploy (all modes)

1. Server restarts with different compiled tools → new build-time fingerprint
2. Client's next request → session valid, fingerprint mismatch → HTTP 404
3. Client auto-re-initializes → new session → fresh tools
4. No manual reconnect needed

### In-process tool mutation (DynamicInProcess)

1. `activate_tool()` / `deactivate_tool()` → live registry updated
2. `notifications/tools/list_changed` broadcast to connected clients
3. Client receives notification → invalidates tool cache → calls `tools/list`
4. Session continues — no disruption, no 404

### Cross-instance mutation (DynamicClustered)

1. Instance A changes tools → writes to shared storage
2. Instance B's polling task detects fingerprint change → reloads from storage
3. Instance B broadcasts `notifications/tools/list_changed` to its clients
4. Clients refresh → see updated tools
5. Session continues on all instances

## Architectural Boundaries

**Transport boundary:** Core server emits `SessionEvent::Custom` via `SessionManager.broadcast_event()`. HTTP SSE bridge handles delivery. No HTTP types in core server code.

**Separation of concerns:** `validate_session_exists()` checks session validity AND fingerprint, but never sends notifications. Notification delivery is `ToolRegistry`'s responsibility. These are separate code paths.

**Concurrency:** `RwLock<ToolState>` holds active tool set + fingerprint under a single lock. Read lock → clone `Arc<dyn McpTool>` → release → call. Never hold lock across await points.

**Lambda:** Currently excluded from dynamic modes at the type level. `LambdaMcpServerBuilder` does not expose `tool_change_mode()`. Lambda uses `FingerprintOnly`.

**Open production requirement:** Determine whether Lambda must participate in `DynamicClustered` live notifications for production deployments (Lambda + EC2 with shared DynamoDB). If yes, this requires a dedicated design for Lambda-in-cluster coordination — including server-global state model, change detection cadence, notification delivery under Streamable HTTP, and cost/latency tradeoffs for per-request shared-store reads. This should be a fresh design proposal, not a patch to the current architecture.

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
