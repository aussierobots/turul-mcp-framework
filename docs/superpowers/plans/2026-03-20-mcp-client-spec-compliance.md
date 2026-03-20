# MCP Client Spec Compliance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `turul-mcp-client` into compliance with MCP 2025-11-25 specification across three coupled implementation tracks.

**Architecture:** Three tracks that map to natural code boundaries: (1) HTTP transport correctness — Accept headers, SSE response parsing, server-event routing; (2) session/version lifecycle — optional session IDs, protocol version enforcement, 404 re-initialization; (3) typed semantics end-to-end — preserve JSON-RPC errors through transport, stop dropping fields in high-level APIs, wire `ConnectionConfig` into transport construction.

**Tech Stack:** Rust, tokio, reqwest, serde_json, async-trait

**Spec reference:** https://modelcontextprotocol.io/specification/2025-11-25

---

## File Map

| File | Responsibility | Tracks |
|------|---------------|--------|
| `crates/turul-mcp-client/src/transport/http.rs` | HTTP transport: request sending, response parsing, SSE handling | 1, 2, 3 |
| `crates/turul-mcp-client/src/client.rs` | High-level API, session handshake, retry loop, request dispatch | 2, 3 |
| `crates/turul-mcp-client/src/session.rs` | Session state machine, capability creation, version storage | 1, 2 |
| `crates/turul-mcp-client/src/config.rs` | `ClientConfig`, `ConnectionConfig` (currently disconnected) | 3 |
| `crates/turul-mcp-client/src/error.rs` | Error types, retryability logic | 2 |
| `crates/turul-mcp-client/src/transport.rs` | `Transport` trait, `ServerEvent` enum, `TransportCapabilities` | 1 |
| `crates/turul-mcp-client/src/streaming.rs` | Server-event dispatch, request/notification callbacks | 1 |

---

## Track 1: Streamable HTTP Transport Correctness

### Defects addressed
- **B1**: POST requests send only `Accept: application/json` — spec requires both `application/json` and `text/event-stream`
- **G1**: `HttpTransport::capabilities()` returns `server_events: false`, preventing GET SSE listener from ever starting; POST SSE responses rejected at `http.rs:160-162`

### Task 1.1: Fix Accept Header on POST Requests

**Files:**
- Modify: `crates/turul-mcp-client/src/transport/http.rs:436-437` (`send_request`)
- Modify: `crates/turul-mcp-client/src/transport/http.rs:502-503` (`send_request_with_headers`)

- [ ] **Step 1: Write failing test**

In `http.rs` test module, add a test that constructs a request and asserts the Accept header contains both media types. Since we can't easily inspect reqwest headers, instead write an integration-style test using a mock server (or a unit test that validates the constant). Alternatively, add a const and test that:

```rust
#[cfg(test)]
mod tests {
    /// The Accept header value for all POST requests per MCP spec
    const MCP_ACCEPT_HEADER: &str = "application/json, text/event-stream";

    #[test]
    fn test_accept_header_contains_both_media_types() {
        assert!(MCP_ACCEPT_HEADER.contains("application/json"));
        assert!(MCP_ACCEPT_HEADER.contains("text/event-stream"));
    }
}
```

- [ ] **Step 2: Run test to verify it passes (this is a constant definition test)**

Run: `cargo test -p turul-mcp-client test_accept_header -- --nocapture`

- [ ] **Step 3: Update Accept headers in both send methods**

At `http.rs:436-437`, change:
```rust
// BEFORE
.header("Accept", "application/json")

// AFTER
.header("Accept", "application/json, text/event-stream")
```

Same change at `http.rs:502-503` in `send_request_with_headers`.

Extract a module-level constant:
```rust
/// Accept header for MCP POST requests per spec (MUST include both)
const MCP_POST_ACCEPT: &str = "application/json, text/event-stream";
```

Use it in both places:
```rust
.header("Accept", MCP_POST_ACCEPT)
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 5: Commit**

```
fix(client): send Accept: application/json, text/event-stream on POST per MCP spec
```

---

### Task 1.2: Handle SSE Responses to POST Requests

**Files:**
- Modify: `crates/turul-mcp-client/src/transport/http.rs:160-162` (`handle_response`)
- Modify: `crates/turul-mcp-client/src/transport/http.rs:347-352` (`handle_response_with_headers`)

**Dependencies:** `tokio-util` is available via the `sse` feature (default-enabled). The `handle_sse_stream` method uses `tokio_util::io::StreamReader` + `AsyncBufReadExt::lines()` for line-based SSE parsing — this is intentionally separate from `handle_byte_stream` (which does streaming JSON deserialization) because SSE has fundamentally different framing (`data: ` prefix per line). The duplication of JSON-RPC routing logic (notifications, final frames) is acceptable since SSE parsing extracts text lines while JSON stream parsing works on raw bytes.

Currently both `handle_response` and `handle_response_with_headers` return `Err("Unexpected SSE response in send_request")` when `Content-Type: text/event-stream` is received. The spec allows servers to respond with SSE to any POST.

- [ ] **Step 1: Write failing test**

Add a test that feeds an SSE-formatted byte stream and expects a parsed JSON-RPC response:

```rust
#[tokio::test]
async fn test_handle_sse_post_response() {
    // Simulated SSE response body containing a JSON-RPC result
    let sse_body = b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":\"req_0\",\"result\":{\"tools\":[]}}\n\n";
    let stream = futures::stream::once(async { Ok::<_, std::io::Error>(sse_body.as_slice()) });

    let mut transport = HttpTransport::new("http://localhost:9999/mcp").unwrap();
    let result = transport.test_handle_sse_stream(stream).await;
    assert!(result.is_ok(), "SSE POST response should parse successfully");
    let json = result.unwrap();
    assert_eq!(json["id"], "req_0");
    assert!(json["result"]["tools"].is_array());
}
```

- [ ] **Step 2: Run test — should FAIL (method doesn't exist yet)**

Run: `cargo test -p turul-mcp-client test_handle_sse_post_response -- --nocapture`
Expected: compilation error

- [ ] **Step 3: Implement SSE response parsing for POST**

Add `handle_sse_stream` method to `HttpTransport` that parses SSE `data:` lines, extracts JSON-RPC frames, routes notifications to `event_sender`, and returns the final response (the frame with both `id` and `result`/`error`):

```rust
/// Parse an SSE stream from a POST response.
/// Extracts JSON-RPC frames from `data:` lines, routes notifications
/// to event_sender, returns the final result/error frame.
async fn handle_sse_stream(&mut self, response: Response) -> McpClientResult<Value> {
    use futures::StreamExt;
    use tokio::io::AsyncBufReadExt;

    let byte_stream = response.bytes_stream();
    let reader = tokio_util::io::StreamReader::new(
        byte_stream.map(|r| r.map_err(std::io::Error::other)),
    );
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await
        .map_err(|e| TransportError::Http(format!("SSE read error: {}", e)))?
    {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                // Check if this is the final response frame
                if json.get("id").is_some()
                    && (json.get("result").is_some() || json.get("error").is_some())
                {
                    self.update_stats(|stats| stats.responses_received += 1);
                    return Ok(json);
                }
                // Notification or progress — route to event channel
                if json.get("method").is_some() {
                    let event = ServerEvent::Notification(json);
                    if let Some(sender) = &self.event_sender {
                        let _ = sender.send(event.clone());
                    }
                    self.queued_events.lock().push(event);
                }
            }
        }
    }

    Err(TransportError::Http("SSE stream ended without final result".to_string()).into())
}

/// Test helper for SSE stream parsing
#[doc(hidden)]
pub async fn test_handle_sse_stream<S, B, E>(&mut self, stream: S) -> McpClientResult<Value>
where
    S: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::error::Error + Send + Sync + 'static,
{
    use futures::StreamExt;
    let mut buffer = Vec::new();
    let mut stream = stream;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| TransportError::Http(format!("Stream error: {}", e)))?;
        buffer.extend_from_slice(chunk.as_ref());
    }
    let text = String::from_utf8_lossy(&buffer);
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                if json.get("id").is_some()
                    && (json.get("result").is_some() || json.get("error").is_some())
                {
                    return Ok(json);
                }
            }
        }
    }
    Err(TransportError::Http("SSE stream ended without final result".to_string()).into())
}
```

Then update `handle_response` at line 160:
```rust
// BEFORE
} else if content_type.contains("text/event-stream") {
    Err(TransportError::Http("Unexpected SSE response in send_request".to_string()).into())

// AFTER
} else if content_type.contains("text/event-stream") {
    self.handle_sse_stream(response).await
```

Same for `handle_response_with_headers` at line 347. Note: headers are captured at lines 330-339 before the content-type branch, so SSE consumption does not lose them:
```rust
// BEFORE
} else if content_type.contains("text/event-stream") {
    return Err(TransportError::Http(
        "Unexpected SSE response in send_request_with_headers".to_string(),
    ).into());

// AFTER
} else if content_type.contains("text/event-stream") {
    self.handle_sse_stream(response).await?
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client -- --nocapture`

- [ ] **Step 5: Commit**

```
feat(client): parse SSE responses to POST requests per MCP spec
```

---

### Task 1.3: Enable GET SSE Listener for HttpTransport

**Files:**
- Modify: `crates/turul-mcp-client/src/transport/http.rs:374-381` (`capabilities()`)
- Verify: `crates/turul-mcp-client/src/client.rs:121` (the `server_events` gate in `connect()`)

The GET SSE listener code already exists in `start_event_listener` (~line 652). The only reason it never runs is `capabilities()` returning `server_events: false`.

- [ ] **Step 1: Write test**

```rust
#[test]
fn test_http_transport_advertises_server_events() {
    let transport = HttpTransport::new("http://localhost:9999/mcp").unwrap();
    assert!(transport.capabilities().server_events,
        "HttpTransport must advertise server_events so McpClient wires the SSE listener");
}
```

- [ ] **Step 2: Run test — should FAIL**

Run: `cargo test -p turul-mcp-client test_http_transport_advertises_server_events`

- [ ] **Step 3: Change capabilities**

At `http.rs:374-381`, enable `server_events` only. Do NOT change `bidirectional` — it is not checked anywhere in the client code and HTTP is not truly bidirectional (server events flow via a separate GET SSE stream, not the same connection):

```rust
fn capabilities(&self) -> TransportCapabilities {
    TransportCapabilities {
        streaming: true,
        bidirectional: false,  // unchanged — HTTP uses separate GET for server events
        server_events: true,   // was false — enables GET SSE listener in McpClient::connect()
        max_message_size: None,
        persistent: false,
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 5: Verify routing**

Read `client.rs:121` to confirm that `transport.capabilities().server_events == true` now causes `start_event_listener()` to be called during `connect()`. Trace through to confirm `ServerEvent::Request` is correctly dispatched to the `StreamHandler`. Check `handle_byte_stream` at `http.rs:240` — server-initiated requests (JSON with `method` + `id`) should be sent as `ServerEvent::Request`, not `ServerEvent::Notification`.

At `http.rs:240-258`, currently all frames with `method` are routed as `ServerEvent::Notification`. Fix:

```rust
// BEFORE (line 241-242)
if json.get("method").is_some() {
    let event = ServerEvent::Notification(json.clone());

// AFTER
if json.get("method").is_some() {
    let event = if json.get("id").is_some() && !json["id"].is_null() {
        ServerEvent::Request(json.clone())
    } else {
        ServerEvent::Notification(json.clone())
    };
```

- [ ] **Step 6: Commit**

```
feat(client): enable GET SSE listener and fix server-request routing for HttpTransport
```

---

## Track 2: Session and Version Lifecycle

### Defects addressed
- **B2**: Client hard-fails if server doesn't return `Mcp-Session-Id` — spec says optional
- **B3**: Protocol version negotiation not enforced
- **G4**: No 404 re-initialization

### Task 2.1: Make Session ID Optional

**Files:**
- Modify: `crates/turul-mcp-client/src/client.rs:258-271` (`initialize_session`)

- [ ] **Step 1: Write test**

```rust
#[tokio::test]
async fn test_initialize_succeeds_without_session_id() {
    // Mock transport that returns valid initialize response WITHOUT Mcp-Session-Id header
    // ... (use MockTransport pattern from existing tests in client.rs)
    // Assert: initialization succeeds, session_id_optional() returns None
}
```

- [ ] **Step 2: Run test — should FAIL**

Expected: `"Server did not provide Mcp-Session-Id header during initialization"`

- [ ] **Step 3: Fix**

At `client.rs:258-271`, change from hard error to optional:

```rust
// BEFORE
if let Some(session_id) = session_id {
    info!("Server provided session ID: {}", session_id);
    self.session.set_session_id(session_id.clone()).await?;
    let mut transport = self.transport.lock().await;
    transport.set_session_id(session_id);
} else {
    return Err(McpClientError::generic(
        "Server did not provide Mcp-Session-Id header during initialization",
    ));
}

// AFTER
if let Some(session_id) = session_id {
    info!("Server provided session ID: {}", session_id);
    self.session.set_session_id(session_id.clone()).await?;
    let mut transport = self.transport.lock().await;
    transport.set_session_id(session_id);
} else {
    debug!("Server did not provide Mcp-Session-Id — stateless session (spec-valid)");
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 5: Commit**

```
fix(client): accept servers that don't provide Mcp-Session-Id (spec-optional)
```

---

### Task 2.2: Enforce Protocol Version Negotiation

**Files:**
- Modify: `crates/turul-mcp-client/src/client.rs:273-297` (after parsing `InitializeResult`)
- Modify: `crates/turul-mcp-client/src/session.rs:341-357` (`validate_server_capabilities`)

The client stores whatever version the server returns and proceeds. Spec says client should disconnect if it doesn't support the negotiated version.

- [ ] **Step 1: Write test**

```rust
#[tokio::test]
async fn test_rejects_unsupported_protocol_version() {
    // Mock transport that returns protocol_version: "2099-01-01"
    // Assert: initialization fails with ProtocolError::UnsupportedVersion
}
```

- [ ] **Step 2: Run test — should FAIL (client currently accepts any version)**

- [ ] **Step 3: Add version validation**

Define the advertised version as a constant in `session.rs`. This client claims MCP 2025-11-25 compliance — enforce only that version. Adding legacy version support (e.g., 2024-11-05) would be a separate product decision, not a compliance fix.

```rust
/// The protocol version this client advertises and supports
pub const PROTOCOL_VERSION: &str = "2025-11-25";
```

In `validate_server_capabilities` (or a new `validate_protocol_version` method), add:

```rust
pub fn validate_protocol_version(version: &str) -> McpClientResult<()> {
    if version == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(crate::error::ProtocolError::UnsupportedVersion(
            format!("Server negotiated '{}', client supports '{}'",
                    version, PROTOCOL_VERSION)
        ).into())
    }
}
```

Also update `create_initialize_request` to use the constant:
```rust
protocol_version: PROTOCOL_VERSION.to_string(),
```

Call it in `client.rs` after parsing the initialize response, before calling `self.session.initialize(...)`:

```rust
// After line 283: let init_response: InitializeResult = ...
SessionManager::validate_protocol_version(&init_response.protocol_version)?;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 5: Commit**

```
fix(client): enforce protocol version negotiation — reject unsupported versions
```

---

### Task 2.3: Re-initialize on HTTP 404

**Files:**
- Modify: `crates/turul-mcp-client/src/client.rs:321-354` (`send_request_internal` retry loop)
- Modify: `crates/turul-mcp-client/src/error.rs` (add `is_session_expired` helper)

The fix belongs in the session-aware client layer, not the transport. Currently 404s are converted to `TransportError::Http("HTTP error 404: ...")` at `http.rs:127-140` — a stringly-typed error that loses the status code. Fix this with a structured error variant.

- [ ] **Step 1: Add structured HTTP status error variant**

In `error.rs`, add a new `TransportError` variant that preserves the status code:

```rust
pub enum TransportError {
    // ... existing variants ...

    #[error("HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },
}
```

Add classifier to `McpClientError`:

```rust
impl McpClientError {
    /// Check if this error indicates the session is expired/unknown (HTTP 404)
    /// Per MCP spec, client MUST start a new session on 404.
    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::Transport(TransportError::HttpStatus { status: 404, .. }))
    }
}
```

- [ ] **Step 2: Update `handle_response` to use structured variant**

At `http.rs:127-140`, change:
```rust
// BEFORE
return Err(TransportError::Http(format!("HTTP error {}: {}", status, error_text)).into());

// AFTER
return Err(TransportError::HttpStatus {
    status: status.as_u16(),
    message: error_text,
}.into());
```

Same change in `handle_response_with_headers` at `http.rs:306-319`.

- [ ] **Step 3: Write test for the classifier**

```rust
#[test]
fn test_404_is_session_expired() {
    let err = McpClientError::Transport(TransportError::HttpStatus {
        status: 404, message: "Not Found".to_string()
    });
    assert!(err.is_session_expired());

    let err = McpClientError::Transport(TransportError::HttpStatus {
        status: 500, message: "Internal".to_string()
    });
    assert!(!err.is_session_expired());
}
```

- [ ] **Step 4: Run test**

Run: `cargo test -p turul-mcp-client test_404`

- [ ] **Step 5: Add re-initialization logic in retry loop**

In `send_request_internal` at `client.rs:321-354`, when the error is a 404:

```rust
Err(e) => {
    warn!(attempt = attempt, error = %e, "Request failed");

    // MCP spec: 404 means session unknown — must re-initialize
    if e.is_session_expired() {
        warn!("Session expired (HTTP 404) — attempting re-initialization");
        self.session.reset().await;
        if let Err(reinit_err) = self.initialize_session().await {
            warn!(error = %reinit_err, "Re-initialization failed");
            return Err(e);
        }
        // Retry the request with the new session
        continue;
    }

    if !e.is_retryable() || !self.config.retry.should_retry(attempt + 1) {
        return Err(e);
    }
    last_error = Some(e);
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 7: Commit**

```
feat(client): re-initialize session on HTTP 404 per MCP spec
```

---

## Track 3: Typed Semantics End-to-End

### Defects addressed
- **B4**: `HttpTransport` converts JSON-RPC error frames into `TransportError::Http` before `McpClient` can preserve code/message/data
- **G3**: `call_tool()` drops `isError`, `structuredContent`, `_meta`; `get_prompt()` drops `description`, `_meta`
- **G6**: `ConnectionConfig` is architecturally disconnected from transport construction

### Task 3.1: Preserve JSON-RPC Errors Through Transport

**Files:**
- Modify: `crates/turul-mcp-client/src/transport/http.rs:222-236` (`handle_byte_stream`)
- Modify: `crates/turul-mcp-client/src/transport.rs` (or `error.rs` for a new variant)

Currently at `http.rs:227-236`, when a JSON-RPC error frame is received, it's converted into `TransportError::Http(format!("Server error: {}", error))` — flattening the structured error into a string. The client layer at `client.rs:426-436` then tries to re-extract code/message/data from `response.get("error")`, but never sees it because the transport already threw.

The fix: return the JSON-RPC error frame as a successful transport response (it IS a valid JSON-RPC message), and let `send_request_raw` at `client.rs:426` do the structured extraction.

- [ ] **Step 1: Write test**

```rust
#[tokio::test]
async fn test_jsonrpc_error_preserved_through_transport() {
    let error_response = br#"{"jsonrpc":"2.0","id":"req_0","error":{"code":-32602,"message":"Invalid params","data":{"detail":"missing field"}}}"#;
    let stream = futures::stream::once(async { Ok::<_, std::io::Error>(error_response.as_slice()) });

    let mut transport = HttpTransport::new("http://localhost:9999/mcp").unwrap();
    let result = transport.test_handle_byte_stream(stream).await;

    // Transport should return the frame, not error
    assert!(result.is_ok(), "JSON-RPC error should pass through transport as Ok(Value)");
    let json = result.unwrap();
    assert_eq!(json["error"]["code"], -32602);
    assert_eq!(json["error"]["message"], "Invalid params");
    assert_eq!(json["error"]["data"]["detail"], "missing field");
}
```

- [ ] **Step 2: Run test — should FAIL**

Expected: `Err(TransportError::Http("Server error: ..."))`

- [ ] **Step 3: Fix transport to pass through JSON-RPC errors**

At `http.rs:222-236`, change:
```rust
// BEFORE
} else if let Some(error) = json.get("error") {
    self.update_stats(|stats| {
        stats.errors += 1;
        stats.last_error = Some(format!("JSON-RPC error: {}", error));
    });
    return Err(TransportError::Http(format!(
        "Server error: {}",
        error
    ))
    .into());
}

// AFTER
} else if json.get("error").is_some() {
    self.update_stats(|stats| {
        stats.errors += 1;
        stats.last_error = json["error"]["message"]
            .as_str()
            .map(|s| s.to_string());
    });
    return Ok(json); // Let client layer extract structured error
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 5: Commit**

```
fix(client): preserve JSON-RPC error code/message/data through HTTP transport
```

---

### Task 3.2: Return Full CallToolResult and GetPromptResult

**Files:**
- Modify: `crates/turul-mcp-client/src/client.rs:500-528` (`call_tool`)
- Modify: `crates/turul-mcp-client/src/client.rs:717-749` (`get_prompt`)

- [ ] **Step 1: Change `call_tool` return type**

Change from `Vec<ToolResult>` (drops `is_error`, `structuredContent`, `_meta`) to `CallToolResult`:

```rust
// BEFORE
pub async fn call_tool(&self, name: &str, arguments: Value) -> McpClientResult<Vec<ToolResult>> {
    // ...
    Ok(call_response.content)
}

// AFTER
pub async fn call_tool(&self, name: &str, arguments: Value) -> McpClientResult<CallToolResult> {
    // ...
    Ok(call_response)
}
```

- [ ] **Step 2: Change `get_prompt` return type**

Change from `Vec<PromptMessage>` (drops `description`, `_meta`) to `GetPromptResult`:

```rust
// BEFORE
pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> McpClientResult<Vec<turul_mcp_protocol::PromptMessage>> {
    // ...
    Ok(prompt_response.messages)
}

// AFTER
pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> McpClientResult<GetPromptResult> {
    // ...
    Ok(prompt_response)
}
```

- [ ] **Step 3: Fix all `call_tool` callers (69 call sites across 13 files)**

The fix pattern depends on how the caller uses the return value:

**Pattern A — caller uses `result` directly as `Vec<ToolResult>`:**
Add `.content` after the call. This is the most common pattern.
```rust
// BEFORE: let result = client.call_tool("name", args).await?;
// AFTER:  let result = client.call_tool("name", args).await?.content;
```

**Pattern B — caller checks emptiness or length:**
Same `.content` fix, but verify `is_error` isn't being silently dropped.

**Pattern C — caller wraps in new CallToolResult (lambda-mcp-client):**
At `examples/lambda-mcp-client/src/client.rs:145-157` — currently destructures into `.content` and re-wraps with `is_error: Some(false)`. Fix: pass through the full `CallToolResult` directly.

**Affected files (call_tool — 69 occurrences):**
| File | Count | Fix Pattern |
|------|-------|-------------|
| `tests/tools/tests/e2e_integration.rs` | 29 | A |
| `tests/tools/tests/mcp_error_code_coverage.rs` | 9 | A |
| `tests/elicitation/tests/elicitation_protocol_e2e.rs` | 9 | A |
| `tests/tools/tests/large_message_handling.rs` | 7 | A |
| `tests/roots/tests/roots_security_e2e.rs` | 4 | A |
| `tests/client_integration_test.rs` | 2 | A |
| `examples/lambda-mcp-client/src/test_runner.rs` | 2 | A |
| `examples/lambda-mcp-client/src/main.rs` | 2 | A |
| `examples/lambda-mcp-client/src/client.rs` | 1 | C |
| `tests/tasks_e2e_inmemory.rs` | 1 | A/B (calls `.is_empty()`) |
| `examples/tasks-e2e-inmemory-client/src/main.rs` | 1 | A |
| `examples/performance-testing/src/memory_benchmark.rs` | 1 | A |
| `examples/performance-testing/src/performance_client.rs` | 1 | A |
| `crates/turul-mcp-client/src/lib.rs` (doctest) | 1 | A |

Also update plugin skill example files (non-compiled reference code):
- `plugins/turul-mcp-skills/skills/mcp-client-patterns/examples/*.rs`
- `plugins/turul-mcp-skills/skills/testing-patterns/examples/*.rs`

- [ ] **Step 4: Fix all `get_prompt` callers (24 call sites across 7 files)**

Same `.messages` pattern:
```rust
// BEFORE: let messages = client.get_prompt("name", args).await?;
// AFTER:  let messages = client.get_prompt("name", args).await?.messages;
```

**Affected files (get_prompt — 24 occurrences):**
| File | Count |
|------|-------|
| `tests/prompts/tests/e2e_shared_integration.rs` | 8 |
| `tests/prompts/tests/e2e_integration.rs` | 6 |
| `tests/prompts/tests/sse_notifications_test.rs` | 5 |
| `tests/shared/tests/concurrent_session_advanced.rs` | 2 |
| `tests/shared/tests/session_validation_comprehensive.rs` | 1 |
| `tests/shared/src/e2e_utils.rs` | 1 |
| `crates/turul-mcp-client/src/lib.rs` (doctest) | 1 |

- [ ] **Step 5: Compile check all affected crates**

```bash
cargo check --workspace 2>&1 | grep "error\[E"
# Must be zero errors
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p turul-mcp-client
cargo test -p turul-mcp-framework-integration-tests
```

- [ ] **Step 7: Commit**

```
fix(client): return full CallToolResult and GetPromptResult — stop dropping fields
```

---

### Task 3.3: Wire ConnectionConfig into Transport Construction

**Files:**
- Modify: `crates/turul-mcp-client/src/transport/http.rs:43-60` (`HttpTransport::new`)
- Modify: `crates/turul-mcp-client/src/client.rs:1063-1069` (`McpClientBuilder::build`)
- Modify: `crates/turul-mcp-client/src/transport.rs:234-240` (`TransportFactory::from_url`)

The architectural defect: `HttpTransport::new()` builds a fresh `reqwest::Client` with hardcoded settings. `McpClientBuilder::with_config()` stores a `ClientConfig` but never threads `ConnectionConfig` (user_agent, custom headers, redirect policy, pool settings) into transport construction.

- [ ] **Step 1: Add `HttpTransport::with_config` constructor**

```rust
/// Create HTTP transport with connection configuration
pub fn with_config(endpoint: &str, config: &ConnectionConfig) -> McpClientResult<Self> {
    let url = Url::parse(endpoint)
        .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(TransportError::ConnectionFailed(format!(
            "Invalid scheme: {}", url.scheme()
        )).into());
    }

    let user_agent = config.user_agent.as_deref()
        .unwrap_or(&format!("mcp-client/{}", env!("CARGO_PKG_VERSION")));

    let mut builder = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent);

    if !config.follow_redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    // Apply custom default headers
    if let Some(ref headers) = config.headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in headers {
            if let (Ok(name), Ok(value)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                header_map.insert(name, value);
            }
        }
        builder = builder.default_headers(header_map);
    }

    let client = builder.build()
        .map_err(|e| TransportError::Http(format!("Failed to create HTTP client: {}", e)))?;

    Ok(Self {
        client,
        endpoint: url,
        connected: AtomicBool::new(false),
        request_counter: AtomicU64::new(0),
        stats: Arc::new(parking_lot::Mutex::new(TransportStatistics::default())),
        event_sender: None,
        queued_events: Arc::new(parking_lot::Mutex::new(Vec::new())),
        session_id: Arc::new(parking_lot::Mutex::new(None)),
    })
}
```

- [ ] **Step 2: Wire into McpClientBuilder — preserve existing API surface**

**Design note:** The existing builder API (`build() -> McpClient`) is widely used. Breaking it to return `McpClientResult` creates unnecessary churn across 15+ call sites. Instead, keep `build()` infallible and have `with_config` apply settings to an already-constructed transport, or apply config when transport is constructed via `with_url`.

Approach: `with_url` keeps its current `-> McpClientResult<Self>` signature but now passes default `ConnectionConfig` to `HttpTransport::with_config`. If `with_config` is called afterward, store the config but don't reconstruct — the transport is already built. For users who need config-aware transport, call `with_config` BEFORE `with_url`:

```rust
// Recommended: config first, then URL (config applied to transport)
let client = McpClientBuilder::new()
    .with_config(config)
    .with_url("http://localhost:8080/mcp")?
    .build();

// Also works: URL first (uses default config for transport)
let client = McpClientBuilder::new()
    .with_url("http://localhost:8080/mcp")?
    .build();

// Also works: custom transport (config not applied to transport)
let client = McpClientBuilder::new()
    .with_transport(Box::new(transport))
    .with_config(config)
    .build();
```

Update `with_url` to use stored config if available:
```rust
pub fn with_url(mut self, url: &str) -> McpClientResult<Self> {
    let connection_config = self.config.as_ref()
        .map(|c| &c.connection)
        .cloned()
        .unwrap_or_default();
    let transport = HttpTransport::with_config(url, &connection_config)?;
    self.transport = Some(Box::new(transport));
    Ok(self)
}
```

`build()` signature stays `-> McpClient` (no breaking change). No call-site updates needed.

- [ ] **Step 3: Write test**

```rust
#[test]
fn test_custom_headers_wired_into_transport() {
    use std::collections::HashMap;
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer test-token".to_string());

    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };

    let transport = HttpTransport::with_config("http://localhost:9999/mcp", &config);
    assert!(transport.is_ok());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p turul-mcp-client`

- [ ] **Step 5: Commit**

```
feat(client): wire ConnectionConfig into HttpTransport — custom headers, user-agent, redirects
```

---

## Post-Track: Verification

### Task V.1: Full Workspace Build and Test

- [ ] **Step 1: Run full workspace check**

```bash
cargo check --workspace
cargo test -p turul-mcp-client
cargo clippy -p turul-mcp-client -- -D warnings
cargo fmt -- --check
```

- [ ] **Step 2: Run integration tests against live server**

```bash
# Start a test server
cd examples/tools-test-server && cargo run &
sleep 2

# Run the client initialization examples
cargo run --example client-initialise-server -- --port 8080
cargo run --example client-initialise-report -- --url http://127.0.0.1:8080/mcp
```

- [ ] **Step 3: Commit final cleanup if needed**

---

## Out of Scope (Future Work)

These items are noted but deliberately excluded from this plan:

1. **Capability truthfulness (G2):** Configurable `ClientCapabilities` tied to handler registration. Requires API design decisions about builder ergonomics.
2. **Unsupported negotiated features:** `resources/subscribe`, `completion/complete`, `logging/setLevel`. Feature additions, not compliance fixes.
3. **Auth / RFC 9728:** Bearer token injection and protected resource metadata discovery. Optional per spec.
4. **Last-Event-ID on SSE reconnect (G5):** SHOULD-level, not MUST.
5. **Batch requests:** Optional per spec.
