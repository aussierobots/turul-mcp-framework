# Origin Policy Server (DNS-rebinding protection)

Origin validation is **on by default** in this framework: a request whose
`Origin` header is present but neither loopback nor same-host is rejected
with HTTP 403 *before* auth or dispatch. This example makes the policy
visible and configurable.

## Why this exists

DNS rebinding: a malicious page at `https://evil.example` rebinds its
hostname to `127.0.0.1`, so the victim's **browser** posts to your local MCP
server from inside the network perimeter. The browser faithfully stamps
`Origin: https://evil.example` — origin validation is the server-side check
that stops it. The MCP spec makes Origin validation a MUST for HTTP servers.

## Run the three policies

```bash
# Default: SameOriginOrLoopback — same-host or loopback origins only
cargo run -p origin-policy-server

# Browser app on another origin: explicit allowlist (repeatable flag;
# the literal value "null" admits Origin: null from sandboxed iframes)
cargo run -p origin-policy-server -- --allow-origin https://app.example.com

# Origin enforced upstream (API Gateway / ALB / reverse proxy)
cargo run -p origin-policy-server -- --disable-origin-check
```

## Probe it (verified matrix, AllowList run)

| Request | Status |
|---|---|
| No `Origin` header (curl, native clients) | 200 |
| `Origin: https://evil.example` | **403** |
| `Origin: https://app.example.com` (allowlisted) | 200 |
| `Origin: http://localhost:3000` (loopback) | 200 |

```bash
BODY='{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}}}'
curl -s -o /dev/null -w '%{http_code}\n' -X POST http://127.0.0.1:8643/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -H 'Origin: https://evil.example' \
  -d "$BODY"
```

## Origin policy vs CORS

Two different layers that must agree for a browser app to work:

- **Origin policy** (this example) — server-side *rejection* (403). The
  security boundary; a non-browser client can't be helped or hurt by it
  since it sends no `Origin`.
- **CORS headers** — browser-side *consent*. Without
  `Access-Control-Allow-Origin` the browser blocks the response from a
  cross-origin page even when the server would have answered. The builder's
  `.cors(true)` (default) handles the headers; the origin policy decides
  whether the request is dispatched at all.

`Disabled` is correct only when something upstream (API gateway, ALB,
reverse proxy) enforces origin — or the server is not browser-reachable.
