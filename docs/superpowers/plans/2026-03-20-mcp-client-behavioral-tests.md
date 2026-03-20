# MCP Client Behavioral Test Suite — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 24 behavioral tests to `turul-mcp-client` that validate MCP 2025-11-25 protocol compliance at the wire/API boundary — not internal method outcomes.

**Architecture:** Three prerequisite tasks (infrastructure), then 5 test workstreams. Tests use `StatefulMockTransport` for session lifecycle and `wiremock` for wire-level HTTP assertions. The SSE routing code is refactored to extract a testable `parse_sse_lines()` free function.

**Tech Stack:** Rust, tokio, wiremock (new dev-dep), serde_json, async-trait

**Branch:** `feat/mcp-client-spec-compliance` (11 commits already — implementation done, tests needed)

---

## Test Classification

| Type | Meaning | Failure semantics |
|------|---------|-------------------|
| **Protocol compliance** | Validates a MUST/SHOULD rule from MCP 2025-11-25 spec | Failure = spec violation |
| **Regression** | Guards against a specific bug found during this branch's development | Failure = known regression |
| **Discovery (red-first)** | Documents a newly discovered bug — expected to FAIL initially | Write test → verify red → fix bug → verify green |

---

## File Ownership

| File | Purpose |
|------|---------|
| `crates/turul-mcp-client/Cargo.toml` | Add `wiremock` dev-dependency |
| `crates/turul-mcp-client/src/transport/http.rs` | Extract `parse_sse_lines()`, fix M1/M5 bugs, add transport-level tests |
| `crates/turul-mcp-client/src/client.rs` | Add `StatefulMockTransport`, session/error/builder tests |
| `crates/turul-mcp-client/tests/wire_compliance.rs` | NEW — wiremock-based wire tests (Cat 5) |

---

## Prerequisites (dependency order)

### Prereq 1: Add wiremock dev-dependency

**File:** `crates/turul-mcp-client/Cargo.toml`

- [ ] Add to `[dev-dependencies]`:
```toml
wiremock = "0.6"
```
- [ ] `cargo check -p turul-mcp-client --tests`
- [ ] Commit: `chore(client): add wiremock dev-dependency for wire-level tests`

### Prereq 2: Build StatefulMockTransport

**File:** `crates/turul-mcp-client/src/client.rs` (test module)

A configurable mock transport that supports multi-step response sequences and call tracking. Extends the existing `MockTransport` pattern.

- [ ] Create `StatefulMockTransport` struct with:
```rust
struct StatefulMockTransport {
    /// Sequence of responses for send_request_with_headers (initialize)
    init_responses: Arc<Mutex<VecDeque<McpClientResult<TransportResponse>>>>,
    /// Sequence of responses for send_request (normal requests)
    request_responses: Arc<Mutex<VecDeque<McpClientResult<Value>>>>,
    /// Tracks set_session_id calls
    set_session_ids: Arc<Mutex<Vec<String>>>,
    /// Tracks clear_session_id calls
    clear_count: Arc<AtomicU32>,
    /// Event channel for server events
    event_tx: Option<mpsc::UnboundedSender<ServerEvent>>,
    event_rx: Option<mpsc::UnboundedReceiver<ServerEvent>>,
    /// Capabilities to advertise
    capabilities: TransportCapabilities,
}
```
- [ ] Implement `Transport` trait — `send_request` pops from `request_responses`, `send_request_with_headers` pops from `init_responses`, `set_session_id` records to vec, `clear_session_id` increments counter
- [ ] Add builder methods: `with_init_response(...)`, `with_request_response(...)`, `with_capabilities(...)`
- [ ] Add helper to create a valid initialize `TransportResponse`:
```rust
fn make_init_response(session_id: Option<&str>, protocol_version: &str) -> TransportResponse
```
- [ ] Test the mock itself: `test_stateful_mock_transport_sequences`
- [ ] Commit: `test(client): add StatefulMockTransport for multi-step session lifecycle tests`

### Prereq 3: Extract parse_sse_lines() from handle_sse_stream

**File:** `crates/turul-mcp-client/src/transport/http.rs`

Refactor `handle_sse_stream` to extract the inner parsing/routing logic into a free function that takes explicit parameters (not `&mut self`). This makes the routing code testable without a `reqwest::Response`.

- [ ] Extract free function:
```rust
/// Parse SSE lines, route server requests/notifications, return final response frame.
/// Extracted from handle_sse_stream for testability.
async fn parse_sse_lines<R: tokio::io::AsyncBufRead + Unpin>(
    lines: &mut tokio::io::Lines<R>,
    event_sender: &Option<mpsc::UnboundedSender<ServerEvent>>,
    queued_events: &Arc<parking_lot::Mutex<Vec<ServerEvent>>>,
    stats: &Arc<parking_lot::Mutex<TransportStatistics>>,
) -> McpClientResult<Value> {
    // Move the loop body from handle_sse_stream here
}
```
- [ ] Update `handle_sse_stream` to call `parse_sse_lines()`
- [ ] Update `test_handle_sse_stream` helper to also call `parse_sse_lines()` (so it tests real routing)
- [ ] Verify existing SSE tests still pass: `cargo test -p turul-mcp-client test_handle_sse -- --nocapture`
- [ ] Commit: `refactor(client): extract parse_sse_lines() for testable SSE routing`

---

## Workstream 1: Builder / Transport Selection (2 regression tests)

**File:** `crates/turul-mcp-client/src/client.rs` (test module)

These guard against the builder refactor regression where deferred `build()` lost SSE URL detection.

### Test 1.1: `test_builder_with_sse_url_yields_sse_transport`
- **Type:** Regression
- **Spec:** §Transports > Backwards Compatibility — SSE URL detection
- [ ] Write test: `McpClientBuilder::new().with_url("http://localhost:9999/sse")?.build()` → assert `transport_type == Sse`
- [ ] Run, verify green

### Test 1.2: `test_builder_with_mcp_url_yields_http_transport`
- **Type:** Regression
- **Spec:** §Transports > Streamable HTTP
- [ ] Write test: `McpClientBuilder::new().with_url("http://localhost:9999/mcp")?.build()` → assert `transport_type == Http`
- [ ] Run, verify green

- [ ] Commit: `test(client): builder transport selection regression tests`

---

## Workstream 2: Session Lifecycle (5 tests — 3 protocol compliance, 2 regression)

**File:** `crates/turul-mcp-client/src/client.rs` (test module, uses StatefulMockTransport)

### Test 2.1: `test_404_reinitialize_clears_stale_session_id`
- **Type:** Protocol compliance
- **Spec:** §Session Management rule 4 (MUST) — "client MUST start a new session by sending a new InitializeRequest without a session ID attached"
- [ ] Write test using StatefulMockTransport: init→"AAA", request→404, re-init→"BBB", retry→success
- [ ] Assert: `clear_count == 1`, `set_session_ids` contains "BBB", final request succeeds
- [ ] Run, verify green

### Test 2.1a: `test_404_on_last_retry_attempt_still_recovers`
- **Type:** Regression
- **Spec:** §Session Management rule 4 — no exception for last attempt
- [ ] Write test: `max_attempts = 2`, 404 on attempt 0, re-init succeeds, attempt 1 succeeds
- [ ] Assert: request succeeds (the `continue` doesn't silently exit the loop)
- [ ] Run, verify green

### Test 2.1b: `test_404_reinit_failure_surfaces_original_error`
- **Type:** Regression
- [ ] Write test: request→404, re-init→connection error
- [ ] Assert: returned error is the original 404 error, not the reinit error
- [ ] Run, verify green

### Test 2.2: `test_optional_session_id_no_hard_failure`
- **Type:** Protocol compliance
- **Spec:** §Session Management rules 1,2 (MAY/MUST) — session ID is optional
- [ ] Write test: MockTransport returns init response with NO `mcp-session-id` header
- [ ] Assert: `connect()` succeeds, `is_ready()` true, `session_id` is `None`
- [ ] Run, verify green

### Test 2.3: `test_unsupported_protocol_version_rejected`
- **Type:** Protocol compliance
- **Spec:** §Lifecycle > Version Negotiation (SHOULD disconnect)
- [ ] Write test: MockTransport returns `protocolVersion: "2099-01-01"`
- [ ] Assert: `connect()` returns `Err(McpClientError::Protocol(ProtocolError::UnsupportedVersion(_)))`, message contains both "2099-01-01" and "2025-11-25"
- [ ] Run, verify green

- [ ] Commit: `test(client): session lifecycle protocol compliance and regression tests`

---

## Workstream 3: Streamable HTTP Routing (3 tests — 2 protocol compliance, 1 regression)

**File:** `crates/turul-mcp-client/src/transport/http.rs` (test module)

**Depends on:** Prereq 3 (parse_sse_lines extraction)

### Test 3.1: `test_sse_post_with_server_request_routed_correctly`
- **Type:** Protocol compliance
- **Spec:** §Sending Messages rule 6 + §Base Protocol > Requests (id MUST be present)
- [ ] Write test using `parse_sse_lines()` with SSE stream containing server request (`method+id`) then final result
- [ ] Assert: returned value is final frame; `queued_events` contains exactly one `ServerEvent::Request` with correct id/method
- [ ] Run, verify green

### Test 3.2: `test_sse_post_with_notification_routed_correctly`
- **Type:** Protocol compliance
- **Spec:** §Base Protocol > Notifications — "Notifications MUST NOT include an ID"
- [ ] Write test: SSE stream with notification (method, no id) then final result
- [ ] Assert: queued event is `ServerEvent::Notification`, not `Request`
- [ ] Run, verify green

### Test 3.3: `test_byte_stream_request_vs_notification_discrimination`
- **Type:** Regression (guards the handle_byte_stream routing fix)
- [ ] Write test: JSON byte stream with both request frame (method+id) and notification frame (method only), then final result
- [ ] Assert: `queued_events` contains one `Request` and one `Notification`, correctly discriminated
- [ ] Run, verify green

- [ ] Commit: `test(client): streamable HTTP routing protocol compliance tests`

---

## Workstream 4: Error Propagation (4 tests — 2 protocol compliance, 2 regression)

**File:** `crates/turul-mcp-client/src/client.rs` (test module, uses StatefulMockTransport)

### Test 4.1: `test_jsonrpc_error_surfaces_as_server_error_with_code_message_data`
- **Type:** Protocol compliance
- **Spec:** §Base Protocol > Error Responses — code and message MUST be preserved
- [ ] Write test: MockTransport returns `{"error":{"code":-32602,"message":"Invalid params","data":{"detail":"missing"}}}`
- [ ] Assert: `list_tools()` returns `Err(e)` where `e.error_code() == Some(-32602)`, message contains "Invalid params", data contains the detail
- [ ] Run, verify green

### Test 4.2: `test_jsonrpc_error_without_data_field`
- **Type:** Protocol compliance
- **Spec:** §Base Protocol > Error Responses — data is optional
- [ ] Write test: error response without `data` field
- [ ] Assert: error code/message preserved, data is `None`
- [ ] Run, verify green

### Test 4.3: `test_call_tool_malformed_response_returns_error`
- **Type:** Regression (runtime resilience)
- [ ] Write test: MockTransport returns `{"unexpected":"shape"}` as tools/call result
- [ ] Assert: `call_tool()` returns `Err` (deserialization failure), not panic
- [ ] Run, verify green

### Test 4.4: `test_get_prompt_malformed_response_returns_error`
- **Type:** Regression (runtime resilience)
- [ ] Write test: MockTransport returns `{"wrong":"format"}` as prompts/get result
- [ ] Assert: `get_prompt()` returns `Err`, not panic
- [ ] Run, verify green

- [ ] Commit: `test(client): error propagation protocol compliance and resilience tests`

---

## Workstream 5: Wire-Level Config & Headers (5 protocol compliance tests)

**File:** `crates/turul-mcp-client/tests/wire_compliance.rs` (NEW)

**Depends on:** Prereq 1 (wiremock dev-dependency)

All tests use `wiremock::MockServer` to capture actual HTTP requests on the wire.

### Test 5.1: `test_custom_headers_appear_on_outbound_requests`
- **Type:** Protocol compliance
- **Spec:** §Sending Messages rule 2 — custom headers must not clobber spec-required headers
- [ ] Write test: wiremock server + `config.headers = {"X-Custom": "val", "Authorization": "Bearer tok"}`
- [ ] Assert: mock received `X-Custom`, `Authorization`, AND `Accept: application/json, text/event-stream`
- [ ] Run, verify green

### Test 5.2: `test_custom_user_agent`
- **Type:** Regression (config wiring)
- [ ] Write test: wiremock + `config.user_agent = Some("my-app/2.0")`
- [ ] Assert: `User-Agent: my-app/2.0` on wire
- [ ] Run, verify green

### Test 5.3: `test_no_redirects_policy`
- **Type:** Regression (config wiring)
- [ ] Write test: wiremock returns 302, `follow_redirects: false`
- [ ] Assert: transport returns error with status 302 (redirect not followed)
- [ ] Run, verify green

### Test 5.4: `test_accept_header_on_post_requests`
- **Type:** Protocol compliance
- **Spec:** §Sending Messages rule 2 (MUST) — "client MUST include an Accept header"
- [ ] Write test: wiremock captures POST request headers
- [ ] Assert: `Accept` is exactly `"application/json, text/event-stream"`
- [ ] Run, verify green

### Test 5.5: `test_mcp_protocol_version_header_on_requests`
- **Type:** Protocol compliance
- **Spec:** §Protocol Version Header (MUST) — "client MUST include MCP-Protocol-Version"
- [ ] Write test: wiremock captures POST request headers
- [ ] Assert: `MCP-Protocol-Version: 2025-11-25` present
- [ ] Run, verify green

- [ ] Commit: `test(client): wire-level protocol compliance tests with wiremock`

---

## Discovery Tests (5 red-first tests — document bugs before fixing)

**IMPORTANT sequencing:** Write test → verify it FAILS (red) → commit the red test → fix the bug → verify green → commit the fix. This proves the test catches the bug.

### M1: `test_notification_post_includes_accept_header`
- **Type:** Discovery — probable spec violation
- **Bug:** `send_notification` (http.rs ~line 692) omits `Accept` header. Spec says all POST requests MUST include it.
- **File:** `crates/turul-mcp-client/tests/wire_compliance.rs`
- [ ] Write test: wiremock captures notification POST headers
- [ ] Assert: `Accept: application/json, text/event-stream` present
- [ ] Run — **expected RED** (Accept header missing)
- [ ] Commit red test: `test(client): document missing Accept header on notification POST (M1)`
- [ ] Fix `send_notification` in http.rs — add `.header("Accept", MCP_POST_ACCEPT)`
- [ ] Run — verify GREEN
- [ ] Commit fix: `fix(client): add Accept header to notification POST per MCP spec`

### M2: `test_sse_data_field_without_space_after_colon`
- **Type:** Discovery — SSE parser limitation
- **Bug:** `strip_prefix("data: ")` requires the space. SSE spec (WHATWG) says space is optional.
- **File:** `crates/turul-mcp-client/src/transport/http.rs` (test module)
- [ ] Write test: SSE stream with `data:{"jsonrpc":"2.0","id":"req_0","result":{}}` (no space)
- [ ] Run — **expected RED** ("SSE stream ended without final result")
- [ ] Commit red test: `test(client): document SSE data field space requirement (M2)`
- [ ] Fix: use `strip_prefix("data:").map(|s| s.strip_prefix(' ').unwrap_or(s))` in both `parse_sse_lines` and GET SSE listener
- [ ] Run — verify GREEN
- [ ] Commit fix: `fix(client): accept SSE data fields with or without space after colon`

### M3: `test_sse_multiline_data_in_post_response`
- **Type:** Discovery — POST/GET SSE parity gap
- **Bug:** GET SSE path handles multi-line `data:` correctly. POST `parse_sse_lines` treats each line independently.
- **File:** `crates/turul-mcp-client/src/transport/http.rs` (test module)
- [ ] Write test: SSE stream with split JSON across two `data:` lines
- [ ] Run — **expected RED**
- [ ] Commit red test: `test(client): document SSE multi-line data limitation in POST path (M3)`
- [ ] Assess fix complexity — if non-trivial, document as known limitation and skip fix for this branch

### M4: `test_session_id_header_case_insensitive`
- **Type:** Discovery — consistency check
- **File:** `crates/turul-mcp-client/src/client.rs` (test module)
- [ ] Write test: MockTransport returns `MCP-SESSION-ID` (all caps) as header key
- [ ] Assert: session ID correctly captured
- [ ] Run — may be GREEN (if reqwest normalizes headers) or RED (if MockTransport doesn't)
- [ ] Commit: `test(client): verify session ID header case insensitivity (M4)`

### M5: `test_sse_post_no_duplicate_event_delivery`
- **Type:** Discovery — probable bug
- **Bug:** `handle_sse_stream` unconditionally pushes to both `event_sender` AND `queued_events`. `handle_byte_stream` only falls back to `queued_events` when sender is closed.
- **File:** `crates/turul-mcp-client/src/transport/http.rs` (test module)
- [ ] Write test: set up `event_sender` channel, process SSE stream with notification frame via `parse_sse_lines`
- [ ] Assert: event appears in `event_sender` receiver XOR `queued_events` — not both
- [ ] Run — **expected RED** (double delivery)
- [ ] Commit red test: `test(client): document SSE double-routing inconsistency (M5)`
- [ ] Fix `parse_sse_lines`: match `handle_byte_stream`'s fallback pattern
- [ ] Run — verify GREEN
- [ ] Commit fix: `fix(client): eliminate SSE double-routing — match JSON stream fallback pattern`

---

## Execution Order

```
Prereq 1 (wiremock)           ─┐
Prereq 2 (StatefulMockTransport) ─┼─ can run in parallel
Prereq 3 (parse_sse_lines)    ─┘

Workstream 1 (Builder)         ─┐
Workstream 2 (Session)         ─┤
Workstream 4 (Error)           ─┼─ can run in parallel (independent files/concerns)
Workstream 5 (Wire/Config)     ─┘

Workstream 3 (SSE Routing)     ── depends on Prereq 3

Discovery M1                   ── depends on Prereq 1 (wiremock)
Discovery M2, M3               ── depends on Prereq 3 (parse_sse_lines)
Discovery M4                   ── depends on Prereq 2 (StatefulMockTransport)
Discovery M5                   ── depends on Prereq 3 (parse_sse_lines)
```

## Verification

```bash
# All client tests
cargo test -p turul-mcp-client -- --nocapture

# Wire compliance tests specifically
cargo test -p turul-mcp-client --test wire_compliance -- --nocapture

# Clippy
cargo clippy -p turul-mcp-client -- -D warnings

# Full workspace
cargo check --workspace
```
