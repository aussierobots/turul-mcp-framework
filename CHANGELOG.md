# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.37] - 2026-04-24

### Fixed

- **HTTP/2 connection drop detection** (`turul-mcp-client`): `HttpTransport` now configures `reqwest`'s h2 keepalive PINGs (`http2_keep_alive_interval = 30s`, `http2_keep_alive_timeout = 10s`, `http2_keep_alive_while_idle = true`) on both `new()` and `with_config()` construction paths. Without these, a connection silently dropped by the server or an intermediary (API Gateway ~350s idle, NAT, ALB) looks alive to the client pool until the next request — which then pays the full reconnect cost. PING keepalives surface the drop proactively so idle pooled connections either stay alive or fail fast and reconnect before a user-facing request uses them. No-op on h1-only backends (ALPN-negotiated h1 connections don't engage h2 keepalive state).

### Note

Values chosen as conservative defaults: 30s interval detects drops well before typical intermediary idle windows without being wasteful (~10 bytes per PING). 10s timeout halves reqwest's default 20s for faster fail-over on flaky paths. `while_idle = true` is the load-bearing bit — it keeps pooled idle connections being probed, which is precisely where silent-drop bimodality manifests. No new `ConnectionConfig` fields were added; if tuning becomes necessary it will land alongside other pending 0.4 surface changes.

## [0.3.36] - 2026-04-24

### Changed

- **`turul-mcp-client` now compiled with `reqwest/http2` feature**: reqwest auto-negotiates HTTP/2 via ALPN when the backend advertises `h2`. For servers that only speak HTTP/1.1, ALPN falls back to h1 — no behavior change. For h2-capable backends (AWS API Gateway, ALB, CloudFront, most modern HTTPS servers), concurrent `call_tool` invocations on one `Arc<McpClient>` are now multiplexed over a single TLS connection instead of opening N separate h1 connections. Resolves #13.

### Testing

- `tests/http2_feature.rs`: compile-time regression test that fails if a future `Cargo.toml` edit accidentally disables `reqwest/http2`.

### Note on validation

This change enables h2 at the dependency layer; the wire-level negotiation is handled entirely by reqwest + rustls ALPN. End-to-end validation (latency improvement on concurrent fan-out against h2-capable backends) is owned by downstream consumers — no specific latency claim is attached to this release. See #13 for the measurement plan and expected behavior.

## [0.3.35] - 2026-04-24

### Fixed

- **`ConnectionConfig` fields now honored** (`turul-mcp-client`): `HttpTransport::with_config` previously advertised six configuration fields but consumed only three (`user_agent`, `follow_redirects`, `headers`). `max_redirects`, `pool_settings.max_idle_per_host`, and `pool_settings.idle_timeout` were silent no-ops — callers set them and `reqwest` defaults applied instead. These three are now wired through to `reqwest::ClientBuilder` (`Policy::limited`, `pool_max_idle_per_host`, `pool_idle_timeout`).

### Deprecated

- **`ConnectionConfig::keep_alive`** and **`PoolConfig::max_lifetime`** (`turul-mcp-client`): no reqwest equivalent. `reqwest` exposes `tcp_keepalive(Option<Duration>)`, not a boolean, and has no per-connection max-lifetime API. Both fields will be removed in 0.4. Callers who do not reference them are unaffected.

### Changed

- **`PoolConfig::default().max_idle_per_host`** raised from 5 to 32 (`turul-mcp-client`): the previous default was silently ignored (reqwest's internal default `usize::MAX` applied). Now that the field is honored, the previous default would cap callers at 5 idle connections per host — a regression for fan-out workloads. 32 matches typical HTTP client sizing; callers can still set their own value.

### Note

This release fixes `ConnectionConfig` API truthfulness only. It does not change HTTP/2 support, connection protocol negotiation, or any other transport-layer behavior. A separate investigation (#13) is evaluating whether enabling `reqwest/http2` measurably affects cold-path tail latency; no decision has been made on that feature.

## [0.3.34] - 2026-04-21

### Fixed

- **DynamoDB read-your-writes on critical paths** (`turul-mcp-session-storage`, `turul-mcp-server-state-storage`): Added `consistent_read(true)` to the DynamoDB read sites that must observe just-written values across instances. Eventual-consistency reads on these paths could cause cold-start Lambda instances to miss sessions, session state, persisted events, or fingerprints written by other instances — breaking MCP SSE resumability and the `initialize` handshake.
  - `get_session`, `set_session_state` read-before-write, `store_event` session-exists check, and `store_event` max-eventId query (visibility; races still handled by the existing conditional `PutItem` + `MAX_RETRIES` loop).
  - `get_fingerprint` — cold-start instance must observe the latest fingerprint.

### Added

- **Storage contract regression tests** (`#[ignore = "requires DynamoDB"]`): `read_your_writes_contract` (session, state, event-replay) and `read_your_writes_contract_fingerprint`. Classified as storage contract regression tests; documented that DynamoDB-Local / LocalStack does not reliably reproduce AWS eventual reads, so passing locally does not prove AWS consistency correctness.

## [0.3.33] - 2026-04-21

### Changed

- **`Transport` trait — `&self` on hot-path methods** (`turul-mcp-client`): `connect`, `disconnect`, `send_request`, `send_request_with_headers`, `send_notification`, `send_delete`, `set_session_id`, `clear_session_id`, `start_event_listener`, and `health_check` now take `&self`. `McpClient::transport` is now `Arc<BoxedTransport>` — the outer `tokio::sync::Mutex` that serialized every request has been removed.

### Fixed

- **Concurrent client requests no longer serialize** (`turul-mcp-client`): N parallel `call_tool` / `list_tools` / etc. on one `Arc<McpClient>` now run in parallel through `reqwest`'s internal connection pool. Before: total wall time ≈ Σ per-call latency (Mutex-serialized). After: wall time ≈ max per-call latency.

### Breaking

- External implementors of `turul_mcp_client::transport::Transport` must change `&mut self` to `&self` on the listed methods and move any bare-mutable state into interior-mutable wrappers (`Atomic*` / `parking_lot::Mutex`). The stock `HttpTransport` and `SseTransport` already use interior mutability on all hot-path state.

## [0.3.32] - 2026-04-15

### Fixed

- **Client session retry on -32031**: `McpClient::call_tool()` (and all request methods) now detect JSON-RPC error code `-32031` ("Session not initialized") and automatically disconnect, reconnect, and retry once. Fixes cold-start race condition where `notifications/initialized` hasn't been processed before the first request arrives — especially visible on Lambda behind API Gateway.

### Added

- **`McpClientError::is_session_not_initialized()`**: Detects session-not-initialized errors by code (-32031) or message content.

## [0.3.31] - 2026-03-30

### Fixed

- **SSE replay**: No replay without `Last-Event-ID` — reverted bounded replay that caused duplicate notifications on API Gateway timeout reconnections. With `Last-Event-ID`: exact resume. Without: live events only.
- **Dead SSE connections**: Removed immediately on send failure, delivery falls back to next live connection. `has_connections()` now ignores closed senders.
- **DynamoDB event ID monotonic**: Conditional write (`attribute_not_exists`) with retry prevents duplicate event IDs across Lambda cold starts.
- **DynamoDB timestamp read**: Fixed numeric millis read (was parsing as RFC3339 string, always fell back to `Utc::now()`).
- **Distributed session targeting**: `broadcast_event()` enumerates targets from `storage.list_sessions()` for Custom events. `dispatch_custom_event()` for per-session delivery without cache dependency.
- **SessionEventDispatcher**: Guaranteed notification persistence on request path. `broadcast_event()` returns `Result` — dispatcher failures propagate.
- **Initialize live fingerprint**: Dynamic mode uses `ToolRegistry::fingerprint()` for new sessions, not build-time static.
- **DynamoDB `get_active_entities`**: Removed `entityId` from filter expression (DynamoDB rejects sort keys in filters).

### Added

- **`ToolChangeNotifier` trait**: Awaitable callback for restart/redeploy fingerprint mismatch notifications, backed by `SessionManager::dispatch_custom_event()`.
- **`dispatch_custom_event()`**: Storage-backed per-session event dispatch, not cache-gated.
- **`SessionEventDispatcher` trait**: Awaitable dispatcher on `SessionManager` for guaranteed Custom event persistence.
- **ADR-023 updates**: Distributed session targeting, session-backed event sequencing future consideration.

## [0.3.30] - 2026-03-29

### Fixed

- **DynamoDB `get_active_entities` filter** (`turul-mcp-server-state-storage`): Removed `entityId` (sort key) from `filter_expression` — DynamoDB rejects primary key attributes in filter expressions. Now uses application-level filtering.
- **Restart/redeploy notification persistence** (`turul-http-mcp-server`): Fingerprint mismatch in `validate_session_exists()` now emits `notifications/tools/list_changed` through the `ToolChangeNotifier` → `SessionManager` → dispatcher architecture. Failure propagates (500), not warn-and-continue.
- **DynamoDB TTL defaults** (`turul-mcp-session-storage`): Session and event TTL defaults increased from 5 to 30 minutes.

### Added

- **`ToolChangeNotifier` trait** (`turul-http-mcp-server`): Awaitable callback for restart/redeploy fingerprint mismatch notifications. Implemented by the server layer via `SessionManager::send_event_to_session()`.
- **`send_event_to_session()` with dispatcher** (`turul-mcp-server`): Per-session event dispatch with guaranteed persistence for Custom events. Retains NotFound error for missing sessions.

## [0.3.29] - 2026-03-29

### Added

- **SessionEventDispatcher** (`turul-mcp-server`): Awaitable dispatcher trait on `SessionManager` for guaranteed notification persistence on the request path. Custom events are persisted via `StreamManager::broadcast_to_session()` before `broadcast_event()` returns. Installed by the runtime (HTTP server, Lambda).
- **Mandatory persistence enforcement**: `broadcast_event()` returns `Result<(), String>` for Custom events. `broadcast_notification()` returns `Result<(), ToolRegistryError::NotificationFailed>`. `activate_tool()`, `deactivate_tool()`, `check_for_changes()` propagate dispatcher failures — no silent success when mandatory persistence fails.
- **Live registry fingerprint for new sessions**: In Dynamic mode, `SessionAwareInitializeHandler` reads `ToolRegistry::fingerprint()` instead of the build-time static value. New sessions after runtime tool mutations get the correct baseline — no spurious mismatch notification.

### Fixed

- **DynamoDB error observability** (`turul-mcp-server-state-storage`): `dynamo_err_debug()` uses `{:?}` (Debug) format instead of `{}`  (Display) for AWS SDK errors, surfacing error code, message, HTTP status, and request ID instead of generic "service error".
- **SSE bridge narrowed to observer-only**: The detached bridge task no longer persists or delivers `SessionEvent::Custom` events — the awaited dispatcher handles that on the request path. Eliminates duplicate persistence.

### Changed

- **BREAKING: `broadcast_event()` returns `Result`**: Callers that previously ignored the return value of `SessionManager::broadcast_event()` must now handle the `Result<(), String>` return for Custom events. Non-custom events always return `Ok(())`.

## [0.3.28] - 2026-03-29

### Fixed

- **Non-deterministic tool fingerprint** (`turul-mcp-server`): `compute_tool_fingerprint()` now canonicalizes JSON (recursive key sorting) before FNV hashing.

## [0.3.27] - 2026-03-29

### Changed

- **BREAKING: Default features reduced** (`turul-mcp-server`): Default features now `["http", "sse"]` only. SQLite, PostgreSQL, and DynamoDB backends are opt-in via `features = ["sqlite"]`, `features = ["postgres"]`, `features = ["dynamodb"]`. This significantly reduces compile time and binary size for projects that only need in-memory storage.
- **Backend features forward to all storage crates** (`turul-mcp-server`): `sqlite`/`postgres`/`dynamodb` features now forward to both `turul-mcp-session-storage` AND `turul-mcp-task-storage` (previously only session-storage).
- **Unified backend features** (`turul-mcp-server`): `sqlite`/`postgres`/`dynamodb` features use weak dependency forwarding (`?/`) to also enable backends on `turul-mcp-server-state-storage` when `dynamic-tools` is active. No separate compound features needed.
- **Lambda backend features** (`turul-mcp-aws-lambda`): Added `sqlite`, `postgres` forwarding features.

### Migration

If you previously depended on `turul-mcp-server` without specifying features and used SQLite, PostgreSQL, or DynamoDB backends, add the backend feature explicitly:

```toml
# Before (backends included by default)
turul-mcp-server = "0.3.26"

# After (backends opt-in)
turul-mcp-server = { version = "0.3.27", features = ["sqlite"] }
```

## [0.3.26] - 2026-03-29

### Fixed

- **Non-deterministic tool fingerprint** (`turul-mcp-server`): `compute_tool_fingerprint()` now canonicalizes JSON (recursive key sorting) before hashing. HashMap iteration order in `ToolSchema.properties`, `ToolSchema.additional`, and nested `JsonSchema.properties` caused different Lambda instances to compute different fingerprints for the same tool set, triggering spurious mismatch cycles on every cold start.

## [0.3.25] - 2026-03-29

### Added

- **Dynamic tool activation** (`turul-mcp-server`): `ToolChangeMode::Dynamic` enables runtime `activate_tool()`/`deactivate_tool()` with MCP-compliant `notifications/tools/list_changed`. Requires `dynamic-tools` feature.
- **ToolRegistry** (`turul-mcp-server`): Live registry for precompiled tools with `RwLock<ToolState>`, fingerprint tracking, and cross-instance coordination via `ServerStateStorage`.
- **ServerStateStorage** (`turul-mcp-server-state-storage`): New crate with InMemory, SQLite, PostgreSQL, DynamoDB backends for cross-instance tool state coordination.
- **Lambda dynamic tools** (`turul-mcp-aws-lambda`): `tool_change_mode()` and `server_state_storage()` on `LambdaMcpServerBuilder`. Request-time change detection with configurable TTL (`TURUL_TOOL_CHECK_TTL_SECS`, default 10s).
- **Client tool change notifications** (`turul-mcp-client`): `refresh_tools()`, cached tool lists, `notifications/tools/list_changed` auto-invalidation.
- **Dynamic tools example**: `examples/dynamic-tools-server` and `examples/dynamic-tools-test-client`.

### Fixed

- **POST SSE notification replay** (`turul-http-mcp-server`): Removed event replay from POST SSE responses — connection is registered before dispatch, so all events are delivered live. Prevents duplicate notification delivery.
- **Derive macro zero-config output preservation** (`turul-mcp-derive`): `#[tool(output = Type)]` without `name`/`description` now correctly preserves the output type via `extract_tool_meta_partial()`. Previously, the fallback path discarded all attributes.
- **OAuth dev-deps** (`turul-mcp-oauth`): Migrated to workspace dependency references. Updated `rsa` to 0.10, `jsonwebtoken` to 10 with `rust_crypto` feature.
- **Test suite MCP handshake** (tests): Added missing `notifications/initialized` to all E2E test suites (prompts, resources, elicitation, roots, sampling, session validation).

### Changed

- **Workspace dependency rule**: All crate dependencies must use `workspace = true` references (added to CLAUDE.md).
- **reqwest workspace default**: `default-features = false` at workspace level; crates opt-in to features individually.

## [0.3.24] - 2026-03-21

### Fixed

- **MCP client Accept header** (`turul-mcp-client`): POST requests now send `Accept: application/json, text/event-stream` per MCP spec. Notifications also include Accept header.
- **MCP client SSE POST responses** (`turul-mcp-client`): Client can now parse `text/event-stream` responses to POST requests instead of rejecting them.
- **MCP client session ID optional** (`turul-mcp-client`): Client no longer hard-fails when server doesn't return `Mcp-Session-Id` — stateless sessions are spec-valid.
- **MCP client protocol version enforcement** (`turul-mcp-client`): Client rejects servers that negotiate unsupported protocol versions.
- **MCP client 404 re-initialization** (`turul-mcp-client`): HTTP 404 triggers session reset, clears stale session ID from transport, and re-initializes.
- **MCP client JSON-RPC error preservation** (`turul-mcp-client`): Error frames pass through transport preserving code/message/data instead of flattening to opaque strings.
- **MCP client SSE double-routing** (`turul-mcp-client`): SSE path no longer duplicates events to both event channel and queue.
- **MCP client SSE data field parsing** (`turul-mcp-client`): Accepts `data:` with or without space after colon per SSE spec.

### Changed

- **`call_tool()` return type** (`turul-mcp-client`): Returns `CallToolResult` instead of `Vec<ToolResult>` — preserves `is_error`, `structuredContent`, `_meta` fields. **Breaking:** callers need `.content` to get the previous `Vec<ToolResult>`.
- **`get_prompt()` return type** (`turul-mcp-client`): Returns `GetPromptResult` instead of `Vec<PromptMessage>` — preserves `description`, `_meta` fields. **Breaking:** callers need `.messages` to get the previous `Vec<PromptMessage>`.
- **`Transport` trait** (`turul-mcp-client`): Added required `clear_session_id()` method. **Breaking** for custom `Transport` implementations.

### Added

- **GET SSE listener for HttpTransport** (`turul-mcp-client`): `server_events: true` enables server-initiated requests/notifications over GET SSE stream.
- **Server request routing** (`turul-mcp-client`): JSON-RPC frames with `method` + non-null `id` are routed as `ServerEvent::Request` (not `Notification`) in both SSE and JSON stream paths.
- **`HttpTransport::with_config()`** (`turul-mcp-client`): Constructor that applies `ConnectionConfig` (custom headers, user-agent, redirect policy).
- **`TransportError::HttpStatus`** (`turul-mcp-client`): Structured error variant preserving HTTP status code.
- **Builder transport detection** (`turul-mcp-client`): `McpClientBuilder` defers transport construction to `build()` so `with_config()` works regardless of call order.
- **21 behavioral tests** (`turul-mcp-client`): Protocol compliance, regression, and wire-level tests using `StatefulMockTransport` and `wiremock`.

## [0.3.23] - 2026-03-20

### Fixed

- **`after_dispatch` middleware mutations silently discarded** (`turul-http-mcp-server`): `DispatcherResult` was cloned into middleware, mutated, then the original `JsonRpcMessage` returned unchanged — mutations now applied back via `apply_dispatcher_result()`.
- **`after_dispatch` middleware errors silently ignored** (`turul-http-mcp-server`): `let _ = execute_after(...)` swallowed `Err(MiddlewareError)` — errors now propagated through `map_middleware_error_to_jsonrpc()` with correct semantic error codes.

## [0.3.22] - 2026-03-16

### Fixed

- **SSE wire-format test compliance** (`tests`): Replaced `strip_prefix("data: ").unwrap_or(...)` workaround in `session_id_compliance` test with explicit Content-Type assertion — tests now branch on the response's declared Content-Type instead of silently accepting both SSE and JSON formats.
- **DynamoDB events table check** (`turul-mcp-session-storage`): `ensure_events_table_exists()` now skipped when `verify_tables` is false (table assumed to exist via CloudFormation/Terraform).

### Added

- **Content-Type negotiation policy** (`turul-http-mcp-server`): `StreamableHttpContext::should_use_sse()` — conservative method-level heuristic for combined `Accept: application/json, text/event-stream`. Non-streaming methods (`tools/list`, `resources/list`, etc.) return `application/json`; streaming-capable methods (`tools/call`, `sampling/createMessage`, `elicitation/create`) return `text/event-stream`.
- **Content-Type negotiation tests** (`tests`): 4 new tests asserting wire-format consistency for JSON-only, SSE-only, combined+tools/call, and combined+tools/list Accept patterns.
- **Test Compliance rule** (`CLAUDE.md`): Tests must assert wire-format compliance — never silently accept multiple formats.
- **ADR-006 amendment**: Documented Content-Type negotiation policy, its architectural limitations, and the per-tool metadata improvement path.

## [0.3.21] - 2026-03-16

### Fixed

- **Lambda `resources/read` handler missing by default** (`turul-mcp-aws-lambda`): HTTP server registered it unconditionally; Lambda only added it when resources were configured. Now registered in `new()` matching HTTP parity.
- **Lambda `resources/templates/list` registered unconditionally** (`turul-mcp-aws-lambda`): Was registered even with no template resources, unlike HTTP which only adds it conditionally. Removed from `new()`, now only added in `build()` when templates exist.
- **Strict lifecycle tests made explicit** (`turul-mcp-aws-lambda`): `build_strict_streaming_handler()` now explicitly sets `.strict_lifecycle(true)` instead of relying on the default.

### Added

- **Lambda handler parity tests** (`turul-mcp-aws-lambda`): `resources/read` registered-by-default test and `resources/templates/list` absent-without-templates test.

## [0.3.20] - 2026-03-16

### Fixed

- **P0: Lambda missing `notifications/initialized` handler** (`turul-mcp-aws-lambda`): Lambda server never registered `InitializedNotificationHandler`, making `strict_lifecycle: true` (default since v0.3.19) non-functional — clients could never complete the MCP handshake. Now registered identically to the HTTP server path.
- **P1: Lambda `tools/list` not session-aware** (`turul-mcp-aws-lambda`): `ListToolsHandler` in Lambda was constructed without session manager, bypassing strict lifecycle checks. Now uses `new_with_session_manager()` consistent with the HTTP server.
- **P1: Streamable HTTP notification race** (`turul-http-mcp-server`): `notifications/initialized` was processed asynchronously via `tokio::spawn`, returning 202 before `is_initialized` was set. If the client sent `tools/list` immediately after, the session would be rejected. Now processed synchronously for `notifications/initialized` specifically; other notifications remain async.

### Added

- **Lambda strict lifecycle E2E tests** (`turul-mcp-aws-lambda`): 4 new tests over `handle_streaming()` with `MCP-Protocol-Version: 2025-11-25` — full handshake, rejection before initialized (with `-32031` error code assertions), immediate post-initialized race proof, and lenient mode fallback.

## [0.3.19] - 2026-03-15

### Changed

- **Strict MCP lifecycle is now the default** (`turul-mcp-server`, `turul-mcp-aws-lambda`): Both `McpServerBuilder` and `LambdaMcpServerBuilder` now default to `strict_lifecycle: true`, requiring clients to send `notifications/initialized` after `initialize` before any other operations. This matches the MCP 2025-11-25 spec. Use `.strict_lifecycle(false)` for legacy clients that skip the notification.

### Fixed

- **Integration tests now perform full MCP handshake** — `mcp_behavioral_compliance`, `session_id_compliance`, and `sse_progress_delivery` tests updated to send `notifications/initialized` after `initialize`.

## [0.3.18] - 2026-03-15

### Changed

- **`create_tables_if_missing` replaced with `verify_tables` + `create_tables`** (`turul-mcp-session-storage`, `turul-mcp-task-storage`): All 6 storage config structs (SQLite, PostgreSQL, DynamoDB × session + task) now use two granular flags. `verify_tables: false` (default) skips all startup verification — eliminates ~1,884 DynamoDB API calls/hour per Lambda server. `create_tables: true` creates tables when missing (only when `verify_tables: true`). **Breaking:** default changed from auto-create to skip-all. For first-time setup, use `verify_tables: true, create_tables: true`.

### Fixed

- **SQLite/PostgreSQL session storage now respect table verification flag** — previously called `migrate()` unconditionally, ignoring the config flag.

## [0.3.17] - 2026-03-15

### Added

- **Custom struct input parameter schema via schemars** (`turul-mcp-derive`): Unknown types in `#[mcp_tool]` parameters (e.g., `Vec<ObserverPoint>`, `MyStruct`) now use `schemars::schema_for!()` to generate correct JSON Schema at runtime instead of falling back to `"type": "string"`. Requires the parameter type to derive `schemars::JsonSchema`. This fixes `Vec<CustomStruct>` parameters generating `{"type": "array", "items": {"type": "string"}}` — they now correctly produce `{"type": "array", "items": {"type": "object", "properties": {...}}}`.

## [0.3.16] - 2026-03-15

### Added

- **Fixed-size array `[T; N]` support in `#[mcp_tool]` schema generation** (`turul-mcp-derive`): `type_to_schema` now handles `[f64; 3]`, `[String; 2]`, `[i32; 4]`, etc. — generating `{"type": "array", "items": ..., "minItems": N, "maxItems": N}` instead of silently falling back to `"type": "string"`. Also handles `Option<[T; N]>`.
- **`with_min_items()` / `with_max_items()` builder methods** (`turul-mcp-protocol-2025-11-25`): `JsonSchema::Array` now supports min/max item count constraints via builder chain.

### Fixed

- **E2E test expected 401 instead of 404 for nonexistent session** (`streamable_http_e2e.rs`): Updated `test_strict_lifecycle_enforcement_over_streamable_http` to expect 404 per MCP 2025-11-25 spec (regression from v0.3.14 session-404 fix).

## [0.3.15] - 2026-03-14

### Added

- **`.icons()` builder method** (`turul-mcp-server`, `turul-mcp-aws-lambda`): Both `McpServerBuilder` and `LambdaMcpServerBuilder` now support `.icons(vec![...])` for setting server icons displayed by MCP clients (e.g., Claude Desktop). Use `Icon::new("https://...")` for URL icons or `Icon::data_uri("image/svg+xml", "<base64>")` for embedded data URIs.
- **`Icon` in protocol prelude** (`turul-mcp-protocol-2025-11-25`): `Icon` is now re-exported via `turul_mcp_server::prelude::*` for convenience.

## [0.3.14] - 2026-03-14

### Fixed

- **Stale/terminated sessions now return 404 per MCP spec** (`turul-http-mcp-server`): `StreamableHttpHandler` previously returned 401 Unauthorized for nonexistent or terminated session IDs. MCP 2025-11-25 requires 404 Not Found so clients know to create a fresh session (not re-authenticate). Missing `Mcp-Session-Id` header (no session ID at all) still returns 401. Storage backend errors return 500.

## [0.3.13] - 2026-03-13

### Changed

- **CORS headers centralized behind constants** (`turul-http-mcp-server`): All CORS header values (`Allow-Methods`, `Allow-Headers`, `Expose-Headers`, `Max-Age`) are now defined as `pub(crate)` constants in `cors.rs`. Inline CORS headers removed from `options_response()`, `StreamableHttpHandler` OPTIONS handler, and `sse_response_headers()`. `CorsLayer::apply_cors_headers()` in `server.rs` is now the single source of truth.
- **`enable_cors = false` now fully respected** (`turul-http-mcp-server`): Previously, inline OPTIONS handlers leaked partial CORS headers even when CORS was disabled. Now `enable_cors = false` produces zero CORS headers on all responses.

### Removed

- **`CorsLayer::apply_cors_headers_for_origin()`** (`turul-http-mcp-server`): Removed — was never wired into the server request pipeline and would be overwritten by the wildcard `apply_cors_headers()` in `server.rs`. For origin-restricted CORS, configure at the reverse proxy layer.
- **`sse_response_headers()`** (`turul-http-mcp-server`): Removed — was never called by the framework. SSE responses are built inline by `StreamableHttpHandler` and `SessionMcpHandler`.
- **Orphan test files** (`turul-http-mcp-server`): Deleted `http_transport_tests.rs` and `sse_tests.rs` — not compiled (missing from `tests/mod.rs`) with 93 compilation errors against the current API.

## [0.3.12] - 2026-03-12

### Fixed

- **CORS: expose `Mcp-Session-Id` header for browser MCP clients** (`turul-http-mcp-server`): Browser-based MCP clients couldn't read the `Mcp-Session-Id` response header because CORS didn't expose it. Added `Access-Control-Expose-Headers: Mcp-Session-Id`, added `Mcp-Session-Id` to `Access-Control-Allow-Headers`, and added `DELETE` to `Access-Control-Allow-Methods` for session teardown. Applies to both wildcard and origin-specific CORS configurations.

## [0.3.11] - 2026-03-09

### Added

- **`run_streaming_with()` custom dispatch** (`turul-mcp-aws-lambda`): Accepts a custom `Fn(Request) -> Future<Response>` closure for Lambda streaming, with the same completion-invocation handling as `run_streaming()`. Use this when you need pre-dispatch logic (e.g., `.well-known` routing) that runs before the MCP handler. Fixes completion-invocation ERROR logs for custom dispatch patterns; does not claim to resolve all Lambda streaming timeout behavior.
- **Prelude re-exports**: `run_streaming` and `run_streaming_with` are now available via `turul_mcp_aws_lambda::prelude::*`.

### Changed

- **`lambda-mcp-server-streaming` example**: Refactored from raw `lambda_http::run_with_streaming_response(service_fn(...))` to `turul_mcp_aws_lambda::run_streaming()`, demonstrating the framework's recommended streaming entry point.

## [0.3.10] - 2026-03-07

### Changed

- **`JwtValidator::new()` now requires audience** (`turul-mcp-oauth`): `JwtValidator::new(jwks_uri, audience)` — audience is a mandatory parameter per MCP spec requirement that servers MUST validate token audience. The optional `with_audience()` method has been removed.
- **`ProtectedResourceMetadata::new()` is now fallible** (`turul-mcp-oauth`): Returns `Result<Self, OAuthError>`. Validates `resource` and `authorization_servers` URIs using `url::Url` — requires http/https scheme, authority present, no fragment. Empty AS list rejected.
- **`oauth_resource_server()` is now fallible** (`turul-mcp-oauth`): Returns `Result<..., OAuthError>`. Enforces exactly one authorization server in metadata (no silent `[0]` fallback). Auto-wires audience from `metadata.resource` and issuer from single AS.

### Added

- **Scope in `WWW-Authenticate`** (`turul-mcp-oauth`): When `scopes_supported` is configured on metadata, challenge responses include `scope="scope1 scope2"` per RFC 6750 §3.
- **`Cache-Control: no-store`** (`turul-http-mcp-server`): All 401/403 challenge responses include `Cache-Control: no-store` per OAuth 2.1 §5.3. Applied in both Streamable HTTP and legacy transports.
- **Canonical URI validation** (`turul-mcp-oauth`): `ProtectedResourceMetadata::new()` validates resource and AS URIs — absolute URI with http/https scheme, authority required, no fragment allowed. New error variants: `OAuthError::InvalidResourceUri`, `OAuthError::InvalidConfiguration`.
- **Single-AS issuer enforcement** (`turul-mcp-oauth`): `oauth_resource_server()` rejects metadata with multiple authorization servers, preventing misconfigured deployments.

## [0.3.9] - 2026-03-06

### Added

- **Lambda streaming event classification** (`turul-mcp-aws-lambda`): Three-way classification of raw Lambda runtime payloads via `classify_runtime_event()` — distinguishes API Gateway events, streaming completion invocations, and unrecognized payloads. Prevents ERROR logs and CloudWatch Lambda Error metrics from completion invocations.
- **`run_streaming()` public API**: Replaces `lambda_http::run_with_streaming_response()` for MCP Lambda servers. Gracefully acknowledges completion invocations (200 + `debug` log) and unrecognized payloads (200 + `warn` log) instead of failing deserialization.
- **Testable surfacing contract**: `handle_runtime_payload()` returns typed `HandleResult { response, event_type }` for observability; `event_log_level()` maps event types to tracing levels — both independently testable without log capture.
- **OAuth resource server foundation** (`turul-http-mcp-server`): Bearer token middleware, route registry, request-scoped extensions on `SessionContext` for auth claims propagation.
- 25 classification/action-path/contract tests with `include_str!` fixture files for API Gateway v1/v2, streaming completion variants, and precedence edge cases.

### Fixed

- **Benchmark compilation**: `SessionContext` struct initializers in `performance-testing` benchmarks updated for new `extensions` field.

## [0.3.8] - 2026-03-05

### Fixed

- **Client streaming response forwarding** (P1): Server-initiated requests (`sampling/createMessage`, `elicitation/create`) now receive JSON-RPC responses back from the client callback. Previously responses were logged and discarded, causing servers to hang indefinitely. Architecture: `StreamHandler` → response channel → consumer task → `transport.send_notification()`. See [ADR-020](docs/adr/020-client-response-forwarding-architecture.md).
- **HTTP transport event classification**: Server-initiated requests (with both `method` and `id`) were misclassified as notifications. Fixed classification order: `method+id` → Request, `method` only → Notification, `id` only → Response.
- **`json_schema_derive.rs` `Option<T>` type-schema**: `generate_field_schema()` now uses `segments.last()` instead of `get_ident()` to handle generic types. `Option<u32>` correctly generates `integer` schema (was falling through to `string`). `is_option_type()` fixed to use `segments.last()` for qualified path support (`std::option::Option<T>`).

### Added

- **Resource `title` attribute**: All three macro paths (`#[derive(McpResource)]`, `#[mcp_resource]`, `resource!{}`) now support `title = "..."` attribute. `HasResourceMetadata::title()` returns the configured value.
- `ServerEvent::Response` variant for distinguishing id-only SSE frames (responses to client-originated requests) from server-initiated requests. `StreamHandler` ignores these — they are handled by the normal request/response matching path.
- Null/missing `id` guard: Server requests without a valid `id` invoke the callback but do not emit a response (per JSON-RPC 2.0 spec).
- 11 new tests covering client response forwarding pipeline (unit + integration + mock transport).

## [0.3.7] - 2026-03-04

### Added

- **Tool annotations macro support**: `#[derive(McpTool)]`, `#[mcp_tool]`, and `tool!{}` now support `read_only`, `destructive`, `idempotent`, `open_world`, `title`, and `annotation_title` attributes — generates `ToolAnnotations` with camelCase JSON keys (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`) per MCP 2025-11-25
- `title` attribute on all three macro paths sets `Tool.title` (via `HasBaseMetadata`); `annotation_title` sets `ToolAnnotations.title` independently
- Boolean annotation type validation: `#[mcp_tool]` rejects wrong types (e.g., `read_only = "true"`) with a compile error

### Fixed

- Terminated sessions (after `DELETE /mcp`) now correctly reject subsequent POST and GET requests in both Streamable HTTP and legacy JSON transports

## [0.3.6] - 2026-03-03

### Fixed

- `#[mcp_tool]` and `#[derive(McpTool)]`: `Option<bool>`, `Option<u32>`, `Option<f64>`, `Vec<T>`, and `Option<Vec<T>>` parameters now generate correct JSON Schema types in `tools/list` input schemas (was incorrectly advertising `"type": "string"` for all generic-arg types)
- Fully-qualified paths (`std::option::Option<T>`, `std::vec::Vec<T>`) now correctly detected across all `is_option_type` checks

## [0.3.5] - 2026-03-03

### Added

- `McpClient::list_resource_templates()` and `list_resource_templates_paginated()` for `resources/templates/list` discovery

### Fixed

- `HttpTransport`: downgraded spurious session ID warning on `initialize` request from `warn!` to `debug!`

## [0.3.4] - 2026-03-03

### Fixed

- `HttpTransport::connect()` and `SseTransport::connect()` no longer send OPTIONS/HEAD pre-flight requests that fail with 405 (direct servers) or 502 (Lambda streaming servers) — connectivity failures now surface at `initialize` time instead of preflight time, matching MCP Inspector behavior
- `#[mcp_tool]` function-attribute macro: `Option<T>` parameters are now correctly excluded from the `required` array in the generated JSON schema (was incorrectly marking them as required unless `#[param(optional)]` was explicitly set)

### Changed

**DynamoDB Storage: camelCase Attribute Names (One-Way Migration):**
- New DynamoDB tables created by `turul-mcp-session-storage` and `turul-mcp-task-storage` now use camelCase attribute names (`sessionId`, `taskId`, `createdAt`, `lastActivity`, etc.) — aligning with DynamoDB convention
- Existing snake_case tables (`session_id`, `task_id`, `created_at`, etc.) are auto-detected via `describe_table()` key schema inspection and continue to work without any changes
- Per-table detection: session and events tables are detected independently, supporting mixed-convention deployments
- Read tolerance: non-key attributes written with either convention are readable via fallback lookup

**Rollback Contract (Breaking Storage Format):**
- This is a **one-way storage format change**. Once new tables are created with camelCase key schemas, pre-v0.3.4 code cannot read them (it has hardcoded snake_case key names)
- New code reads legacy snake_case tables: **Yes** (auto-detected)
- New code creates fresh tables with camelCase: **Yes**
- Old code reads legacy snake_case tables: **Yes** (unchanged)
- **Old code reads new camelCase tables: No — will fail**
- Rolling back to pre-v0.3.4 code after creating camelCase tables will break. Plan accordingly.

## [0.3.3] - 2026-03-01

### Fixed

- PostgreSQL task storage: `tasks.session_id` column type changed from `TEXT` to `VARCHAR(36)` to match `sessions.session_id` and `events.session_id`

## [0.3.2] - 2026-02-28

### Added

- `HasExecution` trait for per-tool task support declaration (follows `HasIcons` supertrait pattern)
- `task_support` attribute on `#[derive(McpTool)]` and `#[mcp_tool]` (`"optional"` | `"required"` | `"forbidden"`)
- `.execution()` builder method on `ToolBuilder`
- Build-time coherence guard rejects `taskSupport=required` without task runtime configured
- `tools/list` strips `execution` field when server has no tasks capability (truthful capability advertisement)
- `tools/call` with `params.task` returns `InvalidParameters` when server has no task runtime (was silent sync fallback)

### Changed

- **Breaking**: `HasExecution` added to `ToolDefinition` supertrait — manual tool impls must add `impl HasExecution for MyTool {}`

### Fixed

- `ToolDefinition::to_tool()` now populates `execution` field from trait (was hardcoded `None`)
- `tools/call` rejects task-augmented requests to tools that don't declare `task_support` (was silently accepted)

## [0.3.1] - 2026-02-28

### Fixed

- `ToolSchemaExt::from_schemars()` now handles schemars v1 nullable type arrays (`"type": ["string", "null"]`) and `anyOf`/null patterns for `Option<T>` fields
- `from_schemars()` enforces `type: "object"` root schema validation per MCP protocol requirements
- `from_schemars()` resolves `$ref` references through both `$defs` and `definitions` maps (merged, not first-hit)

## [0.3.0] - 2026-02-26

### Added

**MCP 2025-11-25 Protocol Support:**
- `turul-mcp-protocol-2025-11-25` crate with full spec compliance (127+ protocol tests)
- `turul-mcp-protocol` alias now re-exports 2025-11-25 types (ADR-015)
- `Icon` struct (`src`, `mime_type`, `sizes`, `theme`) on tools, resources, prompts, resource templates, and implementations
- `Task` struct with `task_id`, `TaskStatus` (`Working`/`InputRequired`/`Completed`/`Failed`/`Cancelled`), `created_at`/`last_updated_at`, `ttl`, `poll_interval`
- `ToolUse` and `ToolResult` content block variants
- `ToolExecution`, `ToolChoice`, `ToolChoiceMode` (`Auto`/`None`/`Required`)
- `TaskStatusNotification` and `ElicitationCompleteNotification`
- URL elicitation mode (`ElicitRequestURLParams`) alongside existing form mode
- `$schema` field on `ElicitationSchema`
- `tools` field on `CreateMessageParams` for sampling with tools
- `ModelHint { name }` struct (replaces closed enum)
- `Implementation` gains `description` and `website_url` fields
- Structured `TasksCapabilities` with `list`, `cancel`, `requests` sub-fields

**Task Storage (`turul-mcp-task-storage` crate):**
- `TaskStorage` trait with zero-Tokio public API
- `InMemoryTaskStorage` with state machine enforcement
- SQLite backend (`SqliteTaskStorage`) — optimistic locking, `julianday()` TTL, background cleanup
- PostgreSQL backend (`PostgresTaskStorage`) — `version` column optimistic locking, JSONB, partial index for stuck tasks
- DynamoDB backend (`DynamoDbTaskStorage`) — conditional writes, GSIs, native TTL, base64 cursors
- 11-function parity test suite shared across all backends
- Feature flags: `sqlite`, `postgres`, `dynamodb` (each opt-in with Tokio)

**Task Runtime & Executor:**
- `TaskExecutor` trait and `TokioTaskExecutor` in `turul-mcp-server`
- `CancellationHandle` for cooperative task cancellation
- `TaskRuntime` with `::new(storage, executor)`, `::with_default_executor(storage)`, `::in_memory()` constructors
- Server handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result` (blocks until terminal per spec)
- Auto-capability advertisement via `McpServer::builder().with_task_runtime()`

**Task Examples:**
- `tasks-e2e-inmemory-server` — task-enabled MCP server with `slow_add` tool
- `tasks-e2e-inmemory-client` — full task lifecycle client (create, poll, cancel, result)
- `client-task-lifecycle` — task API demonstration
- `task-types-showcase` — print-only demo of Task, TaskStatus, TaskMetadata, CRUD types

**Lambda Examples:**
- `lambda-authorizer` — API Gateway REQUEST authorizer with wildcard methodArn for MCP Streamable HTTP

**README Testing Infrastructure:**
- `skeptic` crate for automated markdown code block testing
- README.md files validated as part of `cargo test` suite

### Changed

**Protocol Types (Breaking):**
- `CreateMessageResult` flattened — `role` and `content` at top level (no `message` wrapper)
- `Role` enum: only `User` and `Assistant` (removed `System` variant; system prompts use `systemPrompt` field)
- `ProgressNotificationParams.progress`: `f64` (was `u64`)
- `icon` fields renamed to `icons: Option<Vec<Icon>>` (singular string → plural object array)
- `HasIcon` trait renamed to `HasIcons`; `HasSamplingTools` trait added
- Notification method strings use underscores (`notifications/tools/list_changed`) per spec; JSON capability keys remain camelCase (`listChanged`)
- Default protocol version is 2025-11-25 everywhere; backward-compat 2025-06-18 paths annotated with `// Intentional`

**Test Infrastructure:**
- 1,560+ workspace tests passing, 98 doctests, zero warnings
- Test binaries reduced from 155 to 43 via consolidation (Phase F)
- Root integration tests: 39 → 8 binaries (5 consolidated in `tests/consolidated/` + 3 standalone)
- Sub-crate integration tests: 24 → 7 binaries (`tests/*/tests/all.rs` with `#[path]` imports)
- Derive crate integration tests moved to workspace root (2 binaries eliminated)

**Examples:**
- 58 active examples (up from 42+ in v0.1.0), 25 archived
- 12 core crates in workspace

**Documentation:**
- README narrative updated to reflect spec-pure protocol crate design
- All 20+ protocol crate README code examples tested and verified
- Documentation accuracy fixes across READMEs, ADRs, and compliance reports (repo URL, config field names, notification method strings, version references, port numbers)
- CHANGELOG duplicate `[0.2.0]` sections merged
- ADR-009 updated with `V2025_03_26` and `V2025_11_25` protocol versions
- ADR-004 status updated from CRITICAL to Accepted (Implemented)
- Stale MIGRATION_0.2.1.md references removed workspace-wide

### Fixed

- Sampling server README: removed `System` role, fixed `ModelHint` to object form, corrected snake_case JSON fields to camelCase
- Session storage README: corrected config field names (`session_timeout_minutes`, `database_url`, `PostgresConfig`)
- Compliance reports marked as historical with accurate resolution status
- Client README compatibility list now includes 2025-11-25
- Protocol alias ADR updated from 2025-06-18 to 2025-11-25
- Notification method strings in ADR-005 and E2E test plan corrected to `list_changed`

## [0.2.1] - 2025-10-08

### Breaking Changes

**Schemars Integration (Detailed Schema Generation):**
- **BREAKING**: Tool output types MUST now derive `schemars::JsonSchema`
- **Impact**: Tools with custom output types generate detailed schemas with full property information
- **Migration**: Add `#[derive(JsonSchema)]` to all tool output types:
  ```rust
  use schemars::JsonSchema;

  #[derive(Serialize, Deserialize, JsonSchema)]  // Added JsonSchema
  struct MyOutput {
      result: f64,
      message: String,
  }
  ```
- **Benefit**: All tools now provide detailed schemas in `tools/list` with property names, types, and descriptions
- **Note**: `schemars` is already a workspace dependency - no Cargo.toml changes needed

**Framework Trait Reorganization (Protocol Crate Purity):**
- **BREAKING**: All framework traits moved from `turul-mcp-protocol` to `turul-mcp-builders::traits`
- **BREAKING**: `HasNotificationPayload::payload()` now returns `Option<Value>` (owned) instead of `Option<&Value>` (reference)
- **Impact**: Protocol crate is now 100% MCP spec-pure (no framework-specific code)
- **Migration**: Update imports to use preludes:
  ```rust
  // Before
  use turul_mcp_protocol::{ToolDefinition, ResourceDefinition};

  // After
  use turul_mcp_builders::prelude::*;  // or turul_mcp_server::prelude::*
  ```
- **Migration Guide**: See the breaking changes listed above for step-by-step migration instructions

### Fixed

**Critical Notification Payload Regression:**
- Fixed all notification types returning `None` for payloads (data loss bug)
- Base Notification now properly serializes `params.other` and `_meta`
- ProgressNotification now preserves progressToken, progress, total, message, _meta
- ResourceUpdatedNotification now preserves uri, _meta
- CancelledNotification now preserves requestId, reason, _meta
- All list-changed notifications now preserve _meta fields
- Added 18 comprehensive tests validating notification payload correctness

### Changed

**Framework Trait Locations:**
- Moved 10 trait hierarchies (~1200 LOC) from protocol to builders crate
- All protocol type implementations now in `turul-mcp-builders/src/protocol_impls.rs`
- Derive macros updated to generate correct trait signatures
- All examples and tests updated to use new import paths

## [0.2.0] - 2025-10-05

### Added

**MCP 2025-06-18 Specification:**
- Full compliance with MCP 2025-06-18 spec
- Session-Aware Resources: All resources now support `session: Option<&SessionContext>` parameter
- Sampling Validation Framework: `ProvidedSamplingHandler` for request validation
- SSE Streaming: Chunked transfer encoding with real-time notifications
- CLI Support: All test servers now support `--port` argument with dynamic binding
- Path Normalization: Traversal attack detection in roots validation
- Strict Lifecycle Mode: Optional strict session initialization enforcement

**Middleware System:**
- Complete middleware architecture for HTTP and Lambda transports
- `.middleware()` builder method on `McpServer` and `LambdaMcpServerBuilder`
- Transport-agnostic middleware execution (FIFO before dispatch, LIFO after)
- Session-aware middleware with `StorageBackedSessionView` and `SessionInjection`
- Error short-circuiting with semantic JSON-RPC error codes

**Middleware Examples:**
- `middleware-auth-server` - API key authentication (HTTP)
- `middleware-auth-lambda` - API key authentication (AWS Lambda)
- `middleware-logging-server` - Request timing and tracing
- `middleware-rate-limit-server` - Per-session rate limiting

**Testing Infrastructure:**
- Shared verification utilities (`tests/shared/bin/wait_for_server.sh`)
- Test server bin targets in all test packages (tools, prompts, resources, sampling, roots, elicitation)
- Comprehensive example verification suite (5 phases, 31 servers)
- Session lifecycle compliance: `notifications/initialized` in all e2e tests

### Changed

- **Resource Trait**: Updated `read()` signature to include session parameter
- **Tool Output**: Tools with `outputSchema` automatically include `structuredContent`
- **Error Handling**: Session lifecycle violations use `SessionError` type
- **Pagination**: Reject `limit=0` to prevent stalls
- **HTTP Transport**: Protocol-based routing (≥2025-03-26 uses streaming, ≤2024-11-05 uses buffered)
- SSE keepalives use comment syntax for better client compatibility
- DynamoDB queries use strongly consistent reads
- Lambda `LambdaMcpHandler` now cached globally (preserves DynamoDB client, StreamManager, middleware instances)
- Test packages updated to Rust edition 2024 and tokio version "1"
- Middleware stack execution order documented (FIFO/LIFO)

### Fixed

**Examples (4 bugs fixed):**
- pagination-server: Database unique constraint error (email generation duplicates)
- comprehensive-server: Missing resources and prompts registration
- audit-trail-server: SQLite connection URL missing protocol and create mode
- All 30/31 examples now verified working (96.8% passing, 1 skipped for PostgreSQL)

**Protocol & Core:**
- SSE resumability: Keepalive events preserve Last-Event-ID for proper reconnection
- MCP Inspector compatibility: Events use standard `event: message` format
- Lambda notifications: DynamoDB consistent reads fix race condition
- Lambda handler caching: Global `OnceCell` preserves handler instance (DynamoDB client, StreamManager, middleware) across invocations
- Tool output: Schema and runtime field names now consistent
- CamelCase: Proper acronym handling (GPS → gps, HTTPServer → httpServer)
- Lambda compilation: Fixed `LambdaError::Config` reference
- **TestServerManager**: Blocking wait for process termination, prevents zombie processes
- **Session Tests**: Correct response structure (`output` vs `value`)
- **Prompt Arguments**: Fix argument name mismatches in test expectations
- **MCP Inspector**: Enable compatibility with MCP Inspector and FastMCP clients
- **Zero-Config**: Correct output field expectations for derived tools
- **Borrow Checker**: Resolve errors in `roots_derive` macro

**Code Quality:**
- Fixed 14 collapsible_if clippy warnings using Rust 2024 let-chain syntax
- Fixed unused variable warnings in test suite
- Fixed useless type conversions in Lambda tests
- All clippy warnings addressed (100% clean workspace builds with `-D warnings`)

**Verification Infrastructure:**
- Scripts use deterministic 15s polling instead of fixed sleeps
- Pre-built binaries eliminate compilation timeouts
- SKIPPED tracked separately from PASSED (no hidden failures)
- Build errors properly diagnosed with detailed logs

### Examples
- Restored `roots-server` with clap CLI (108 lines, down from 512)
- Updated `elicitation-server` with multi-path data loading
- Updated `sampling-server` with dynamic port binding
- Updated `pagination-server` with proper SQLite URI (`?mode=rwc`)
- All 31 core examples verified and working

### Documentation

- README middleware section with examples and testing commands
- AGENTS.md middleware guidance with ADR 012 reference
- Doctests passing: turul-mcp-derive (25/25), turul-mcp-protocol (7/7)
- Complete verification run documented with bug fixes and runbook
- Middleware testing scripts: `test_middleware_live.sh` and Lambda examples
- Updated CLAUDE.md with session-aware patterns
- Updated EXAMPLES.md with validation results
- Added curl and jq to auto-approved commands
- Comprehensive test coverage documentation

### Tests

- 440+ unit tests passing (161 integration tests across 20 test suites)
- 30/31 examples verified (Phases 1-5: 100% passing)
- Middleware parity tests verify HTTP/Lambda consistency
- All critical functionality validated

## [0.1.0] - Initial Release

### Added
- Core MCP server framework
- Tool creation patterns (function, derive, builder, manual)
- Resource management with templates
- Prompt generation system
- Session management with multiple storage backends
- HTTP transport layer
- Client library
- Builder patterns
- AWS Lambda support
- 42+ working examples

[Unreleased]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.36...HEAD
[0.3.37]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.36...v0.3.37
[0.3.36]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.35...v0.3.36
[0.3.35]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.34...v0.3.35
[0.3.34]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.33...v0.3.34
[0.3.22]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.21...v0.3.22
[0.3.21]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.20...v0.3.21
[0.3.20]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.19...v0.3.20
[0.3.19]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.18...v0.3.19
[0.3.18]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.17...v0.3.18
[0.3.17]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.16...v0.3.17
[0.3.16]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.15...v0.3.16
[0.3.15]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.14...v0.3.15
[0.3.14]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.13...v0.3.14
[0.3.13]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.12...v0.3.13
[0.3.12]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.11...v0.3.12
[0.3.11]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.10...v0.3.11
[0.3.10]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.9...v0.3.10
[0.3.9]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.8...v0.3.9
[0.3.8]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aussierobots/turul-mcp-framework/releases/tag/v0.1.0
