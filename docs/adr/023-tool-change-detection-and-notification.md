# ADR-023: Tool Change Detection and Notification

**Status:** Accepted
**Date:** 2026-03-28
**Context:** MCP 2025-11-25 compliance for tool change signaling

## Decision

Two complementary mechanisms handle tool change detection at different layers. They serve different purposes and are not alternatives:

- **Fingerprint** — correctness boundary. Guarantees clients refresh.
- **Notification** — UX improvement. Faster discovery of changes.

### Phase A: Tool Fingerprint — Correctness Boundary

Stale session detection for restarts, redeploys, and tool mutations.

- FNV-1a hash of the full serialized `Tool` descriptor set, computed at build time (and recomputed on tool activation/deactivation in DynamicInProcess mode)
- Stored per-session during `initialize`, validated on every request via `validate_session_exists()`
- **Mismatch always returns HTTP 404** — regardless of mode, connection state, or notification status
- Client re-initializes: new session with current fingerprint + fresh tools
- This is the only mechanism that guarantees a fresh client view

### Phase B: Dynamic Tool Registry — UX Notification

Push-based `notifications/tools/list_changed` for single-process HTTP servers.

- `ToolChangeMode::DynamicInProcess` — explicit opt-in via builder, requires `dynamic-tools` Cargo feature
- `ToolRegistry` activates/deactivates precompiled tools at runtime (not runtime plugin loading)
- Handlers read from live registry instead of static snapshot
- `tools.listChanged = true` — truthful: the server emits notifications on tool changes
- Notification emitted via `SessionManager.broadcast_event(SessionEvent::Custom)` — transport-agnostic
- SSE bridge (`setup_sse_event_bridge`) delivers to connected clients via GET SSE stream

**Notifications are advisory only.** They tell connected clients "tools changed" as an early warning. The client may hear about the change before hitting the 404. They still must re-initialize to continue safely.

## Why Notifications Don't Replace 404

We evaluated and rejected several alternatives to always-404:

- **`has_connections()` as proof of receipt**: Connection presence is not proof that the client received or processed the specific notification. Rejected.
- **Silent fingerprint advancement**: Advancing the session fingerprint without an explicit client refresh boundary means clients can operate with stale tool lists indefinitely. Rejected.
- **Inline POST SSE notification**: Would create a second ad-hoc SSE delivery path bypassing the existing notification infrastructure. Rejected.
- **Session-ack model**: Only advance fingerprint after `tools/list` call. Adds complexity for marginal gain. Deferred to future milestone.

The strict model (always 404) is the simplest and most defensible correctness story.

## Fingerprint Storage

The fingerprint is **session-scoped compatibility metadata**, not global server state. It is stored per-session under the key `mcp:tool_fingerprint` in the session storage backend (InMemory, SQLite, PostgreSQL, DynamoDB).

- **Written during `initialize`** (`crates/turul-mcp-server/src/server.rs`, `SessionAwareInitializeHandler`)
- **Checked on every subsequent request** (`validate_session_exists()` in both HTTP handlers)
- **Represents**: "what tool surface this session was initialized against"
- **Compared against**: the server's current tool fingerprint (computed at build time or recomputed on mutation)

## Configuration

```rust
pub enum ToolChangeMode {
    FingerprintOnly,                     // Default. listChanged=false.
    #[cfg(feature = "dynamic-tools")]
    DynamicInProcess,                    // Opt-in. listChanged=true. Single-process only.
    // DynamicClustered,                 // Reserved for future milestone. Multi-instance.
}
```

Capability mapping:

| Mode | `listChanged` | Fingerprint 404 | Notifications | Scope |
|------|---|---|---|---|
| FingerprintOnly | false | Yes (always) | No | All runtimes |
| DynamicInProcess | true | Yes (always) | Yes (advisory) | Single-process HTTP |

See [Future Work](#future-work-dynamicclustered-mode) for the deferred `DynamicClustered` multi-instance mode.

## Architectural Boundaries

**Transport boundary:** Core server emits `SessionEvent::Custom` via `SessionManager.broadcast_event()`. The HTTP layer's SSE event bridge handles delivery. No HTTP types in core server code.

**Separation of concerns:** The fingerprint validation in `validate_session_exists()` never sends notifications. Notification delivery is the ToolRegistry's responsibility. These are separate code paths.

**Concurrency:** `RwLock<ToolState>` holds both active tool set and fingerprint under a single lock. Read lock → clone `Arc<dyn McpTool>` → release lock → call tool. Lock is never held across await points.

**Lambda:** Excluded from Phase B at the type level. `LambdaMcpServerBuilder` does not expose `tool_change_mode()`. Lambda uses `FingerprintOnly`.

**Scope:** Phase B is single-process only. Multi-instance coordination is a separate future milestone.

## Notification Wire Format

Exact payload emitted by `ToolRegistry` on mutation. Uses `JsonRpcNotification` (not `ToolListChangedNotification` — protocol notification types are NOT wire-complete; they lack the `jsonrpc` field):

```json
{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
```

No `params` field.

## Client Flow

1. Tool changes on server → `ToolRegistry` broadcasts `notifications/tools/list_changed` to connected GET SSE clients
2. Client receives notification (advisory — early warning)
3. Client's next request → fingerprint mismatch → HTTP 404
4. Client re-initializes → new session with current fingerprint + fresh `tools/list`

Steps 1-2 are UX (faster discovery). Steps 3-4 are correctness (guaranteed refresh).

## Consequences

- Servers using `FingerprintOnly` (default) have zero behavioral change from pre-feature baseline
- Servers opting into `DynamicInProcess` get truthful `listChanged=true` and advisory notifications
- `activate_tool()` / `deactivate_tool()` names communicate that this toggles precompiled tools, not runtime plugin loading
- The `dynamic-tools` Cargo feature ensures zero compiled overhead when the feature is not used
- Fingerprint 404 is the correctness boundary in both modes — notifications do not change session validation behavior

## Future Work: DynamicClustered Mode

`DynamicClustered` is a deferred future mode for multi-instance deployments. It is **not implemented** and reserved for a future milestone.

### What It Would Provide

- Shared active tool registry across server instances
- Shared current tool fingerprint/version
- Cross-instance coordination and invalidation
- Truthful `tools.listChanged=true` across all instances
- `notifications/tools/list_changed` emission from any instance reaching clients on any other instance

### Key Design Constraint: Not Session State

Session state (`mcp:tool_fingerprint`) is session-scoped compatibility metadata — it records what a specific client session was initialized against. **Clustered tool activation state is server-global, not session-scoped.** Storing the active tool registry in session state would conflate two different scopes, create duplication, and require expensive fan-out writes on every tool mutation.

`DynamicClustered` requires a separate **server-global storage layer**, following the same pluggable-backend pattern as `turul-mcp-session-storage` and `turul-mcp-task-storage`. A new `ServerStateStorage` trait (or similar) would provide:

- **SQLite** — for local durable mode / small deployments
- **PostgreSQL** — for shared relational deployments
- **DynamoDB** — for serverless/AWS deployments
- **InMemory** — as a test double for the storage abstraction only (cannot satisfy clustered semantics across instances)

This trait is separate from `SessionStorage` and has different lifecycle semantics. Session state is client-scoped; server state is instance-global and shared across the cluster.

**`ServerStateStorage` is generic server-global state, not tool-specific.** While tools are the first consumer, the same storage and coordination pattern is intended to back all MCP entity types that support `list_changed` notifications:

- `notifications/tools/list_changed` — tool activation registry
- `notifications/resources/list_changed` — resource activation registry
- `notifications/prompts/list_changed` — prompt activation registry

Each entity type would store its own activation state and fingerprint in server-global storage. The storage trait should be designed as a general key-value store, not as a tool-only abstraction. Per-entity change notifications (`notifications/resources/updated`) are a different pattern and may require separate consideration.

### Startup Behavior

When a server instance starts in `DynamicClustered` mode, for each entity type (tools, resources, prompts):

1. Compute local fingerprint from the compiled entity set
2. Read the current fingerprint from shared storage
3. If they differ: update shared storage with the new fingerprint (this instance has newer definitions)
4. Other running instances that have not restarted should detect the fingerprint change (via coordination) and **issue a warning log** — they are serving a stale entity set until they restart or reload

This handles rolling deployments where instances restart at different times.

### Coordination Strategy

Cross-instance delivery and convergence require an explicit coordination strategy. Options include:

- **Polling** — each instance periodically checks shared storage for fingerprint changes (simple, bounded latency)
- **Storage-native change events** — PostgreSQL `LISTEN/NOTIFY`, DynamoDB Streams (low latency, backend-specific)
- **External pub/sub** — Redis, SNS, or similar (lowest latency, new infrastructure dependency)

The choice of coordination strategy is not decided and will be determined when this mode is implemented.

### Correctness Rule

Unless a future ADR explicitly introduces a session-ack refresh boundary (e.g., advancing the session fingerprint only after a successful `tools/list` call), the current rule applies: **fingerprint mismatch always 404s.** `DynamicClustered` notifications remain advisory, same as `DynamicInProcess`.
