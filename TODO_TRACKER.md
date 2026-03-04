# TODO Tracker

**Last Updated**: 2026-03-05
**Version**: v0.3.8-dev (branch: `feature/v0.3.8`)
**Tests**: 1,590+ passing, 43 test binaries, zero clippy warnings
**Spec**: MCP 2025-11-25 (known gaps tracked below)

---

## Top Priority (P1) — MCP Correctness / Interoperability

(none)

---

## Medium Priority (P2) — Architecture / Testing Gaps

### StreamableResponse::Stream returns 202 stub

- **File**: `crates/turul-http-mcp-server/src/streamable_http.rs:300-312`
- **What**: The `StreamableResponse::Stream` variant exists in the enum but the handler returns `202 Accepted` with a text body instead of streaming SSE events. Chunked transfer encoding is not implemented.
- **Why it matters**: Any handler that returns a `Stream` variant gets a stub response. Currently no handlers produce this variant (SSE streaming uses a different code path), so this is latent — but it blocks future streaming handler work.
- **Compliance**: Indirect — no current handler triggers this path, and the enum variant is public API. Not an active interop risk since no handler produces this variant.
- **Exit criteria**: `StreamableResponse::Stream` produces a proper `text/event-stream` response with chunked SSE frames, or the variant is removed if unused.

### resources/subscribe not implemented

- **Files**: `crates/turul-mcp-server/src/builder.rs:1005,1503`, `crates/turul-mcp-aws-lambda/src/builder.rs:935`
- **What**: All three locations hardcode `subscribe: Some(false)`. Resource subscriptions (SSE-based change notifications) are not implemented.
- **Why it matters**: Clients that want real-time resource updates cannot use this framework. However, the capability is **truthfully advertised as `false`**, so no interop risk exists today.
- **Compliance**: None currently (truthful capability advertisement). Becomes P1 if a future MCP revision makes subscriptions mandatory.
- **Exit criteria**: `resources/subscribe` and `resources/unsubscribe` handlers implemented; `subscribe: Some(true)` when configured; E2E test for change notifications.

### Session notification broadcaster path incomplete

- **File**: `crates/turul-mcp-server/src/session.rs:337`
- **What**: `notify_progress()` detects whether a `NotificationBroadcaster` is available but falls through to the same `self.notify()` path regardless. The broadcaster branch should use direct SSE broadcast instead of the session manager notification closure.
- **Why it matters**: Progress notifications work but take a less efficient path when a broadcaster is available. May cause missed notifications under concurrent load.
- **Compliance**: Indirect — progress notifications are delivered, but not via the optimal path.
- **Exit criteria**: Broadcaster branch sends progress notifications directly via `NotificationBroadcaster`, bypassing the session manager closure. E2E test confirms progress events arrive via SSE under concurrent tool calls.

### Client response forwarding: live wire-format integration test

- **File**: `tests/client_server_request_response.rs`, `crates/turul-mcp-client/src/client.rs` (test module)
- **What**: The v0.3.8 client streaming response forwarding fix is validated with in-process channel tests and a mock transport test. Neither exercises the full wire-format path (HTTP request/response over a live server). Wire-format/network behavior is partially covered by existing transport tests for other flows, but not specifically for server→client→server request/response.
- **Why it matters**: A regression in HTTP framing, SSE event classification, or content-type negotiation specific to response forwarding would not be caught by current tests.
- **Compliance**: None (test coverage gap, not spec violation). The protocol logic itself is tested.
- **Exit criteria**: Integration test that starts a live test server sending a server-initiated request (e.g., `sampling/createMessage`), connects a real `McpClient` with `HttpTransport`, and verifies the server receives the JSON-RPC response over HTTP.

### Prompt title from derive/attribute macros

- **Files**: `crates/turul-mcp-derive/src/prompt_derive.rs`, `crates/turul-mcp-derive/src/prompt_attr.rs`
- **What**: Same gap that resource macros had before v0.3.8: prompt macros hardcode `title() -> None`. `PromptMeta` doesn't parse `title`.
- **Why it matters**: Prompt `title` is part of MCP 2025-11-25. Users must abandon macros to set prompt titles.
- **Compliance**: Indirect — `title` is optional in the spec but useful for client display.
- **Exit criteria**: `#[derive(McpPrompt)]` and `#[mcp_prompt]` support `title = "..."` attribute; `HasPromptMetadata::title()` returns the value.

### Lambda streaming tests are stubs

- **File**: `tests/lambda_streaming_real.rs` (15+ TODOs)
- **What**: Entire file is unimplemented. Tests have function signatures and TODO comments but no assertions. Corresponds to "verification phases 6-8" from original project plan.
- **Why it matters**: Lambda streaming behavior (chunked SSE via Lambda response streaming) has zero automated test coverage.
- **Compliance**: None (test debt, not spec violation).
- **Exit criteria**: At least 3 functional tests: Lambda initialize round-trip, SSE event framing, and session lifecycle. Stubs converted to `#[ignore]` or implemented.

### SSE progress event streaming in E2E tests

- **Files**: `tests/streamable_http_e2e.rs:795,1078`
- **What**: Two E2E tests have TODO comments about a broadcaster downcast fix needed for progress event streaming. Tests exist but cannot verify progress events arrive via SSE.
- **Why it matters**: Progress notification delivery over SSE is untested end-to-end.
- **Compliance**: None (test debt).
- **Exit criteria**: Broadcaster downcast resolved; tests verify `notifications/progress` events appear in SSE stream.

### Elicitation test server dead code

- **File**: `tests/elicitation/bin/main.rs` (14 `#[allow(dead_code)]` TODOs)
- **What**: Elicitation test server defines struct fields for workflow logic (validation rules, default values, security policies) that are declared but never used. Each has `#[allow(dead_code)]` with a TODO.
- **Why it matters**: The test server doesn't exercise the full elicitation workflow it models. Low risk — the actual elicitation protocol is tested elsewhere.
- **Compliance**: None (test completeness).
- **Exit criteria**: Fields either used in workflow logic or removed. No `#[allow(dead_code)]` TODOs remaining.

---

## Low Priority (P3) — Refactors / Polish

### streamable_http.rs buffered POST code duplication

- **File**: `crates/turul-http-mcp-server/src/streamable_http.rs:1588`
- **What**: Legacy buffered POST handler duplicates logic from the streaming handler. TODO notes to extract common logic.
- **Why it matters**: Code maintenance only. Both paths work correctly.
- **Compliance**: None.
- **Exit criteria**: Shared helper extracted; both paths call it.

### server.rs handler registration investigation note

- **File**: `crates/turul-mcp-server/src/server.rs:458`
- **What**: Comment asks whether the handler registration loop also adds `tools/list` and `tools/call` handlers. This is an investigation note, not a bug — the code works correctly.
- **Why it matters**: Developer clarity only.
- **Compliance**: None.
- **Exit criteria**: Comment updated with the answer (yes/no) or removed.

---

## Stale / Needs Revalidation

(none)

---

## Known Issues

- `tasks/result` error path wraps original error code in `McpError::ToolExecutionError` — loses original JSON-RPC error code. The client receives `-32603` (internal error) instead of the tool's specific error code.

---

## Next 2 Releases

### v0.3.9 (candidates)

- [ ] Prompt `title` attribute on derive/attribute macros (P2)
- [ ] Client response forwarding live wire-format integration test (P2)

### v0.4.0 (candidates)

- [ ] `resources/subscribe` implementation
- [ ] Session broadcaster direct-path for progress notifications
- [ ] Lambda streaming test implementation (at least 3 functional tests)
- [ ] `StreamableResponse::Stream` real implementation or removal

---

## Completed Recently

| Version | Item |
|---|---|
| v0.3.8 | Client streaming response forwarding: `StreamHandler` → response channel → consumer task → `transport.send_notification()`. Includes `ServerEvent::Response` variant, id-null guard, HTTP event classification fix, mock transport tests. See [ADR-020](docs/adr/020-client-response-forwarding-architecture.md). |
| v0.3.8 | Fix `json_schema_derive.rs` `Option<T>` type-schema unwrapping: `generate_field_schema()` now uses `segments.last()` to handle generic types; `is_option_type()` fixed for qualified paths |
| v0.3.8 | Resource `title` attribute support across all 3 macro paths (`#[derive(McpResource)]`, `#[mcp_resource]`, `resource!{}`) |
| v0.3.7 | ToolAnnotations macro support (`read_only`, `destructive`, `idempotent`, `open_world`, `title`, `annotation_title`) across all 3 macro paths |
| v0.3.7 | Session termination: reject requests on terminated sessions after `DELETE /mcp` |
| v0.3.6 | Fix `Option<T>`/`Vec<T>` JSON Schema types in tool derive macros (`utils.rs:type_to_schema`); legacy `#[derive(JsonSchema)]` path in `json_schema_derive.rs` not covered |
| v0.3.5 | `McpClient::list_resource_templates()` |
| v0.3.4 | DynamoDB camelCase migration, HTTP preflight removal, optional params fix |
