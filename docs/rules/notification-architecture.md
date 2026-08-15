# Notification Wire Format: Always Use JsonRpcNotification

**CRITICAL**: Protocol notification types (e.g., `ToolListChangedNotification`, `ResourceListChangedNotification`) are **NOT wire-complete**. They contain MCP-specific fields (`method`, `params`) but lack the required `jsonrpc: "2.0"` envelope.

```rust
// CORRECT — wire-complete JSON-RPC notification for transport:
let notification = JsonRpcNotification::new("notifications/tools/list_changed".to_string());
// Serializes to: {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}

// WRONG — missing jsonrpc field, will fail client-side validation:
let notification = ToolListChangedNotification::new();
// Serializes to: {"method":"notifications/tools/list_changed"}  ← BROKEN
```

This applies to ALL notification types sent via SSE/HTTP transport. The protocol `*Notification` types are for parsing/type safety, not for direct wire emission.

## Notification Persistence Architecture

**SessionManager is the single event bus.** All notification emitters (ToolRegistry, SessionContext) go through `SessionManager::broadcast_event()`. Guaranteed persistence is provided by the `SessionEventDispatcher` — an awaited trait installed at the SessionManager layer, not at individual emitters.

- `broadcast_event()` for Custom events enumerates targets from `storage.list_sessions()` (NOT the in-memory cache), filters terminated sessions, dispatches per-session via the awaited dispatcher
- `dispatch_custom_event(session_id)` is for per-session delivery (e.g., fingerprint mismatch) — storage-backed, not cache-gated
- `send_event_to_session()` is cache-backed (unchanged) — used only when the session is known to be attached in this process
- The dispatcher calls `StreamManager::broadcast_to_session()` which persists to session event storage AND delivers to active connections
- The SSE bridge task is observer-only for Custom events — NOT the persistence path
- Without a dispatcher (e.g., no HTTP server), events are best-effort only (in-memory channels)

**Do NOT add notification sinks or persistence hooks to individual emitters** — that splits the event architecture into competing delivery paths.

**Distributed session targeting** (see ADR-023): In Lambda/multi-instance, the in-memory `SessionManager.sessions` cache may not contain sessions created by other instances. Notification targeting for Custom events uses `storage.list_sessions()`, not the cache.

## Critical Error Handling Rules

**MANDATORY**: Handlers return domain errors only. Dispatcher owns protocol conversion.

**Key Rules:**
1. Handlers return `Result<Value, McpError>` ONLY
2. Dispatcher converts McpError → JsonRpcError automatically
3. Never create JsonRpcError, JsonRpcResponse in business logic
4. Use `McpError::InvalidParameters`, `McpError::ToolNotFound`, etc.
