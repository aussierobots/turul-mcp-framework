# Header-Bound Tools (SEP-2243 `x-mcp-header` / `Mcp-Param-*`)

Annotate a tool input property with `x-mcp-header` and clients mirror that
argument into an `Mcp-Param-<Name>` request header — so API gateways and
load balancers can route on tool arguments (region pinning, tenant
sharding) **without parsing JSON bodies**.

```bash
cargo run -p header-bound-tools-server
# → http://127.0.0.1:8644/mcp
```

## The annotation

```rust
properties.insert(
    "region".to_string(),
    json!({
        "type": "string",
        "x-mcp-header": "Region"   // ← mirror into Mcp-Param-Region
    }),
);
```

## The validation contract (verified live)

| Request | Result |
|---|---|
| `Mcp-Param-Region: ap-southeast-2` matching the body argument | 200, tool runs |
| Header omitted while the body carries `region` | **400 + `-32001`** "header omitted but the parameter is present in the request body" |
| `Mcp-Param-Region: us-east-1` vs body `ap-southeast-2` | **400 + `-32001`** "does not match the request body value" |

Values that aren't valid header tokens (non-tchar) ride a Base64 sentinel —
the server decodes before comparing.

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8644/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: route_query' \
  -H 'Mcp-Param-Region: ap-southeast-2' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"route_query\",\"arguments\":{\"region\":\"ap-southeast-2\",\"query\":\"SELECT 1\"}}}"
```

Drop the `Mcp-Param-Region` header (or change its value) to see the
`-32001` rejection.

## See also

- `crates/turul-mcp-server/tests/mcp_param_2026.rs` — the wire contract
  suite, including the Base64 sentinel and integer-typed parameters
