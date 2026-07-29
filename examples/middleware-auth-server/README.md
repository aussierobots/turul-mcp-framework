# Middleware Auth Server (API-key authentication)

Demonstrates `McpMiddleware` doing **authentication before dispatch**: the
middleware reads an `X-API-Key` header, rejects unknown keys, and injects the
resolved user id into the request's state so tools can read who is calling.

## Spec lane

**MCP 2026-07-28** (the workspace default — no `protocol-2025-11-25` pin in
`Cargo.toml`). Nothing here depends on cross-request sessions: the middleware
writes state via `SessionInjection` on the *current* request, and the tool
reads it back from the same request's `SessionContext`. That is exactly the
shape the stateless core supports.

For the Lambda equivalent see [`middleware-auth-lambda`](../middleware-auth-lambda/).

## Run

```bash
cargo run -p middleware-auth-server           # port 8672
cargo run -p middleware-auth-server -- --port 9000
```

Valid keys: `secret-key-123` → `user-alice`, `secret-key-456` → `user-bob`.

## Verified behaviour

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8672/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: whoami' \
  -H 'X-API-Key: secret-key-123' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"whoami\",\"arguments\":{},$META}}"
```

| Request | Response |
|---|---|
| `X-API-Key: secret-key-123` | `structuredContent.output.user_id = "user-alice"` |
| `X-API-Key: nope` | JSON-RPC error `-32001` "Invalid API key" |
| no `X-API-Key` | JSON-RPC error `-32001` "Missing X-API-Key header" |

Note the rejections are JSON-RPC errors carried in an HTTP **200** — the
middleware refuses the *call*, it does not fail the transport. Use HTTP 401
only when you are doing OAuth token validation; see
[`oauth-resource-server`](../oauth-resource-server/).

## The pattern

```rust
async fn before_dispatch(
    &self,
    ctx: &mut RequestContext<'_>,
    _session: Option<&dyn SessionView>,
    injection: &mut SessionInjection,
) -> Result<(), MiddlewareError> {
    let api_key = ctx.metadata().get("x-api-key").and_then(|v| v.as_str());
    // ...validate...
    injection.set_state("user_id", json!(user_id));   // tools read this back
    Ok(())
}
```

HTTP headers arrive lowercased in `ctx.metadata()`. Returning
`MiddlewareError::Unauthenticated` short-circuits dispatch — the tool never
runs. Middleware executes FIFO before dispatch and in reverse order after.
