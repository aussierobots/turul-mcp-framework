# client-initialise-report (MCP 2025-11-25)

A raw-wire compliance probe for the 2025-11-25 stateful lifecycle. It links
no turul protocol crate — it builds every request with `serde_json` and
`reqwest`, so the bytes on the wire are the whole story.

## Spec lane and why

**2025-11-25 only, by design.** The probe hardcodes `initialize` →
`notifications/initialized` → `Mcp-Session-Id` on every subsequent request →
`DELETE`. All of that was removed by 2026-07-28's stateless core, so this
example has no 2026 counterpart to port to. The 2026 stateless pair is
`streamable-http-client` + `minimal-server`.

## Run

```bash
cargo run -p client-initialise-server                                    # terminal 1
RUST_LOG=info cargo run -p client-initialise-report -- --url http://127.0.0.1:52950/mcp
```

`--test-sse-notifications` additionally drives the notification-flow checks
(off by default because they add wall-clock wait).

Nothing is printed without `RUST_LOG` — the report is emitted through
`tracing` at INFO.

## What it checks

1. `initialize` response shape, negotiated `protocolVersion`, advertised
   capabilities.
2. Session id sourced from the **`Mcp-Session-Id` response header**, not from
   the body and not client-generated.
3. Streamable HTTP: a POST carrying `Accept: text/event-stream` answers with
   an SSE stream that carries both notifications and the final JSON-RPC
   result.
4. SSE resumability: `Last-Event-ID` replay, per-stream event ids.
5. Session data read back through the server's inspection tools *while the
   session is still live* — this runs before step 6 for that reason.
6. `DELETE` termination, then a follow-up request that must fail 404.

## What to expect

```text
✅ SESSION MANAGEMENT: COMPLIANT
   • Session ID: 019fad2a05947c03ae1d06d437d35bb5
   • Source: Mcp-Session-Id header (proper MCP protocol)
🎯 RECOMMENDATION:
   ✅ 🎆 FULLY MCP COMPLIANT: Session management + Streamable HTTP working!
🔍 SESSION DATA VERIFICATION:
   • Initialization status: true
   • Total events stored: 4
```

Tool results are read out of `structuredContent`, unwrapping the single-key
wrapper named after the tool's output field — reading `content[0].text` and
indexing the top level directly would silently yield defaults.

## Related

- `client-initialise-server` — the server this is pointed at
- `streamable-http-client-2025-11-25` — same lane, focused on concurrent SSE
