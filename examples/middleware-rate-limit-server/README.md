# Middleware Rate Limit Server

Rate limiting as `McpMiddleware`, keyed on a **stateless client identity**
rather than a session.

## Spec lane

**MCP 2026-07-28** (the workspace default). This example exists in its
current shape *because* of the 2026 stateless core: there is no session to
count requests against, so the limiter declares `runs_before_session() ->
true` and keys on the `X-API-Key` header instead. Requests with no key share
one `anonymous` bucket.

Production deployments key on whatever identity they actually trust — a
validated token subject, a client certificate, or the source address supplied
by the load balancer. `X-API-Key` is used here only because it is trivially
settable with `curl`.

## Run

```bash
cargo run -p middleware-rate-limit-server            # port 8671
cargo run -p middleware-rate-limit-server -- --port 9000
```

Limit: **5 requests per identity per 60 seconds**.

## Verified behaviour

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

for i in $(seq 1 7); do
  curl -s -X POST http://127.0.0.1:8671/mcp \
    -H 'Content-Type: application/json' -H 'Accept: application/json' \
    -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' \
    -H 'X-API-Key: alice' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":$i,\"method\":\"server/discover\",\"params\":{$META}}"
  echo
done
```

Requests 1–5 succeed. Request 6 onward:

```json
{"code":-32003,
 "message":"Rate limit exceeded: 5 requests per 60 seconds",
 "data":{"retryAfter":59}}
```

A different `X-API-Key` (e.g. `bob`) gets its own fresh bucket — verified.

## The pattern

```rust
impl McpMiddleware for RateLimitMiddleware {
    // Headers are in ctx.metadata() and no session is needed, so the
    // limiter can reject before any session work happens.
    fn runs_before_session(&self) -> bool { true }

    async fn before_dispatch(...) -> Result<(), MiddlewareError> {
        // ...
        Err(MiddlewareError::RateLimitExceeded {
            message: "...".into(),
            retry_after: Some(retry_after),   // → data.retryAfter
        })
    }
}
```

`MiddlewareError::RateLimitExceeded` maps to JSON-RPC `-32003` with
`retryAfter` in `data`. The in-memory `HashMap` here is per-process; a
multi-instance deployment needs a shared counter store.
