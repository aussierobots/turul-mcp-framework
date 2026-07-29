# MRTR Elicitation (2026-07-28)

The Multi-Round-Trip-Request pattern that replaces server-initiated requests
on the 2026 stateless core: a tool that needs user input returns an
`InputRequiredResult` and the client retries the **original** request with
`inputResponses` plus the echoed `requestState`.

Two binaries in this package walk both legs:

```bash
cargo run -p mrtr-elicitation-server                                  # server, port 8642
cargo run -p mrtr-elicitation-server --bin mrtr-elicitation-client    # client leg
```

## The round trip

```text
client                                         server
  │  tools/call deploy_service ───────────────▶│  tool returns McpError::InputRequired
  │◀─── result { resultType: "input_required", │
  │       inputRequests: { confirm:            │
  │         elicitation/create form },         │
  │       requestState: "deploy:billing-api" } │
  │  (render form, ask the user)               │
  │  tools/call deploy_service ───────────────▶│  session.input_responses() is Some
  │    + inputResponses { confirm: {...} }     │  session.mrtr_request_state() echoes
  │    + requestState (verbatim)               │
  │◀─── result { "deployed billing-api ✅" }   │
```

## Server side (`src/main.rs`)

- First leg: return `McpError::InputRequired { input_requests, request_state }`
  — the framework renders it as `resultType: "input_required"` with HTTP 200.
- Retry leg: `session.input_responses()` is `Some`, and
  `session.mrtr_request_state()` returns the client's echoed state.
- Capability gate (framework-enforced): a client whose per-request `_meta`
  `clientCapabilities` does not declare `elicitation` gets JSON-RPC `-32003`
  instead of an input request. Try it: comment out the
  `declared_capabilities.elicitation = true` line in the client.

## Client side (`src/client.rs`)

- `ClientConfig::declared_capabilities.elicitation = true` — declared in every
  request's `_meta`.
- `call_tool` surfaces the first leg as `McpClientError::InputRequired`.
- `call_tool_with_input_responses(name, args, input_responses, request_state)`
  performs the retry under a new JSON-RPC id.

## See also

- `elicitation-server` — despite its name, form-schema generation over plain
  `tools/call`, not the elicitation protocol; it is the E2E fixture for
  `tests/elicitation`. No example implements the 2025-11-25 session-stream
  `elicitation/create` idiom this pattern replaces.
- `crates/turul-mcp-server/tests/mrtr_2026.rs` — the wire contract tests,
  including sub-capability gating (`elicitation.url`, `sampling.tools`)
