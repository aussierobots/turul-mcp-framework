# sampling-server (MCP 2025-11-25)

A server that answers `sampling/createMessage` through the `McpSampling`
trait, using canned responses rather than a real model. It is the fixture the
`mcp-sampling-tests` E2E suite drives.

## Spec lane and why

**Pinned to 2025-11-25** (`default-features = false`, `protocol-2025-11-25`
on every framework dep). **Sampling is deprecated in MCP 2026-07-28**
(SEP-2577, 12-month window) and is not served on this branch's 2026 default
lane. The pin is the reason this still builds.

## What it demonstrates

Implementing the fine-grained sampling traits and registering the result:

```rust
impl HasSamplingConfig for CreativeWritingSampler { /* max_tokens, temperature */ }
impl HasSamplingContext for CreativeWritingSampler { /* system messages */ }
impl HasModelPreferences for CreativeWritingSampler { /* None here */ }
impl HasSamplingTools for CreativeWritingSampler {}

#[async_trait]
impl McpSampling for CreativeWritingSampler {
    async fn sample(&self, req: CreateMessageRequest) -> McpResult<CreateMessageResult> { ... }
}
```

`validate_request` runs before `sample`, which is where an invalid request
(e.g. `maxTokens: 0`) is rejected.

No model is called. Each sampler returns a fixed string. The value here is
the trait wiring and the request/result types, not generation.

## Run

```bash
cargo run -p sampling-server -- --port 8051     # `--port 0` picks an ephemeral port
```

## What to expect

2025-11-25 is stateful, so handshake first (`initialize` → capture the
`Mcp-Session-Id` response header → `notifications/initialized`), then:

```bash
curl -X POST http://127.0.0.1:8051/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"sampling/createMessage","params":{
        "messages":[{"role":"user","content":{"type":"text","text":"hi"}}],
        "maxTokens":100}}'
```

Verified response:

```json
{"jsonrpc":"2.0","id":2,"result":{
  "_meta":{"currentStep":1,"progress":1.0,"progressToken":"sampling-provided","totalSteps":1},
  "content":{"type":"text","text":"I'm your creative writing assistant, ready to help with: ..."},
  "model":"creative-assistant-v1","role":"assistant"}}
```

Ask for `"maxTokens": 0` and `validate_request` rejects it before `sample`
runs: `-32020 Validation error: max_tokens must be greater than 0`.

Add `text/event-stream` to `Accept` and the same result arrives as an SSE
`data:` frame instead of a JSON body.

## Known limitation: only one of the three samplers is reachable

`main()` registers three providers (creative, technical, conversational), but
`sampling/createMessage` has no way to select between them — the framework's
`ProvidedSamplingHandler` dispatches to `providers.values().next()` on a
`HashMap`. Which sampler answers is therefore **arbitrary and varies between
process starts**; observed across five restarts of this server, requests were
answered by the creative sampler (`creative-assistant-v1`) three times and the technical sampler twice,
with identical input.

Treat the three registrations as showing that `.sampling_provider()` can be
called repeatedly, not as showing per-request routing. Register one provider
and branch inside its `sample()` if you need behaviour to depend on the
request.

## Tests

```bash
cargo test -p mcp-sampling-tests
```

Those suites launch this binary via `TestServerManager::start_sampling_server()`.
`test_sampling_different_models` sends three differently-worded prompts but
only asserts that the returned text is non-empty and longer than 20 chars —
it cannot assert which sampler answered, for the reason above.
