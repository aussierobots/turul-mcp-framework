# Streamable HTTP Client (MCP 2026-07-28)

The client half of the 2026 stateless pair, using the high-level
`turul-mcp-client`. Its canonical server partner is
[`minimal-server`](../minimal-server/) (port 8641).

## Spec lane

**MCP 2026-07-28.** `turul-mcp-client` is bilingual and negotiates per
connection; against a 2026 server it locks 2026-07-28 and the transport adds
the required `MCP-Protocol-Version` / `Mcp-Method` / `Mcp-Name` headers and
per-request `_meta` automatically. There is no `initialize` handshake and no
`Mcp-Session-Id` — both were removed by this spec.

For the 2025-11-25 lane see
[`streamable-http-client-2025-11-25`](../streamable-http-client-2025-11-25/).

## Run

```bash
# Terminal 1
cargo run -p minimal-server                     # port 8641

# Terminal 2
cargo run -p streamable-http-client
cargo run -p streamable-http-client -- http://127.0.0.1:8005/mcp   # any 2026 server
```

If the peer negotiates something other than 2026-07-28 the client says so and
exits rather than pretending.

## What it demonstrates

| Step | API |
|---|---|
| Negotiate the wire spec once per connection | `connect()` + `negotiated_version()` |
| Read the retained `server/discover` body | `discovered_server()` — capabilities, instructions, supported versions |
| List and call tools | `list_tools()`, `call_tool()` |
| Request-scoped progress | `call_tool_with_progress()` |
| Long-lived notification stream | `subscriptions_listen()` |

`subscriptions/listen` is the 2026 replacement for 2025's GET SSE stream: an
ack-first, long-lived **POST** stream. The first frame is the honored filter
plus a `subscriptionId`; dropping the stream *is* the unsubscribe, and clients
re-issue rather than resume.

## Seeing real notifications

`minimal-server`'s `echo` emits no progress and broadcasts nothing, so against
it `call_tool_with_progress` shows the API shape only and the listen stream
shows just the acknowledgement. Point the client at
[`notification-server`](../notification-server/) for live deliveries:

```bash
cargo run -p notification-server                                    # port 8005
cargo run -p streamable-http-client -- http://127.0.0.1:8005/mcp
```

Verified output — the client calls `trigger_changes` and drains:

```
subscriptions/listen acknowledged:
  honored filter: {"resourceSubscriptions":["file:///watched.txt"],"resourcesListChanged":true,"toolsListChanged":true}
  subscriptionId: Some("req_2")
called trigger_changes — draining notifications:
  ← notifications/resources/list_changed
  ← notifications/tools/list_changed
  ← notifications/resources/updated
```

`prompts/list_changed` is broadcast too but does **not** arrive — it is not in
the requested filter. That is the subscription filter working.
