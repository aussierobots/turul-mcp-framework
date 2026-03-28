# ADR-023: Tool Change Detection and Notification

**Status:** Accepted
**Date:** 2026-03-28
**Context:** MCP 2025-11-25 compliance for tool change signaling

## Decision

Two complementary mechanisms handle tool change detection at different layers:

### Phase A: Tool Fingerprint — Session Versioning

Pull-based stale session detection for restarts and redeploys.

- FNV-1a hash of the full serialized `Tool` descriptor set, computed at build time
- Stored per-session during `initialize`, validated on every request via `validate_session_exists()`
- Mismatch returns HTTP 404, triggering client re-initialization
- `tools.listChanged = false` — this is session versioning, not runtime notification
- Works for all transports: Streamable HTTP, legacy JSON-RPC, Lambda
- Always on (default `ToolChangeMode::FingerprintOnly`)

### Phase B: Dynamic Tool Registry — Live Notification

Push-based `notifications/tools/list_changed` for single-process HTTP servers.

- `ToolChangeMode::DynamicInProcess` — explicit opt-in via builder, requires `dynamic-tools` Cargo feature
- `ToolRegistry` activates/deactivates precompiled tools at runtime (not runtime plugin loading)
- Handlers read from live registry instead of static snapshot
- `tools.listChanged = true` — truthful: the server will emit notifications
- Notification emitted via `SessionManager.broadcast_event(SessionEvent::Custom)` — transport-agnostic
- SSE bridge (`setup_sse_event_bridge`) delivers to connected clients with event persistence and Last-Event-ID replay

## Configuration

```rust
pub enum ToolChangeMode {
    FingerprintOnly,                     // Default. listChanged=false.
    #[cfg(feature = "dynamic-tools")]
    DynamicInProcess,                    // Opt-in. listChanged=true.
}
```

Capability mapping:

| Mode | `listChanged` | Fingerprint | Live Notifications |
|------|---|---|---|
| FingerprintOnly | false | Yes | No |
| DynamicInProcess | true | Yes | Yes |

## Architectural Boundaries

**Transport boundary:** Core server emits `SessionEvent::Custom` via `SessionManager.broadcast_event()`. The HTTP layer's SSE event bridge handles delivery. No HTTP types in core server code.

**Concurrency:** `RwLock<HashSet<String>>` for the active tool set. Read lock → clone `Arc<dyn McpTool>` → release lock → call tool. Lock is never held across await points. Matches existing `SessionManager` pattern.

**Lambda:** Excluded from Phase B at the type level. `LambdaMcpServerBuilder` does not expose `tool_change_mode()`. Lambda uses `FingerprintOnly` for stale session detection.

**Scope:** Phase B is single-process only. Multi-instance coordination (durable registry, cross-instance polling) is a separate future milestone requiring `ServerStateStorage` and an explicit consistency model.

## Notification Wire Format

Exact payload emitted by `ToolRegistry` on mutation:

```json
{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
```

No `params` field. Uses `ToolListChangedNotification::new()` from the protocol crate.

## Consequences

- Servers using `FingerprintOnly` (default) have zero behavioral change from pre-feature baseline
- Servers opting into `DynamicInProcess` get truthful `listChanged=true` and live notifications
- `activate_tool()` / `deactivate_tool()` names communicate that this toggles precompiled tools, not runtime plugin loading
- The `dynamic-tools` Cargo feature ensures zero compiled overhead when the feature is not used
- Phase A fingerprint remains the safety net: even in `DynamicInProcess` mode, existing sessions with stale fingerprints get 404'd on next request
