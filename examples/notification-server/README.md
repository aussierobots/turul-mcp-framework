# Notification Server (2026-07-28)

Demonstrates **both** server-initiated notification surfaces of the 2026
stateless core. There is no GET SSE stream and no session on this lane —
the endpoint is POST-only.

| Surface | Carried by | Opt-in |
|---|---|---|
| Subscription notifications (`*/list_changed`, `resources/updated`) | a long-lived `subscriptions/listen` POST SSE stream | the listen request's filter |
| Request-scoped notifications (`notifications/progress`, `notifications/message`) | the originating POST's own SSE response | `_meta.progressToken` / `_meta` `logLevel` |

## Run

```bash
cargo run -p notification-server
# → http://127.0.0.1:8005/mcp
```

## 1. Open a listen stream

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -N -X POST http://127.0.0.1:8005/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: subscriptions/listen' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"subscriptions/listen\",\"params\":{\"notifications\":{\"resourcesListChanged\":true,\"toolsListChanged\":true,\"resourceSubscriptions\":[\"file:///watched.txt\"]},$META}}"
```

The first frame is `notifications/subscriptions/acknowledged` (the honored
filter). The stream then carries ONLY the requested types, each stamped with
`io.modelcontextprotocol/subscriptionId`. Dropping the stream cancels the
subscription — clients reconnect by re-issuing the request.

## 2. Trigger subscription notifications

In another terminal:

```bash
curl -s -X POST http://127.0.0.1:8005/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: trigger_changes' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"trigger_changes\",\"arguments\":{},$META}}"
```

The listen stream receives `resources/list_changed`, `tools/list_changed`,
and the watched-URI `resources/updated` — but NOT `prompts/list_changed`
(not in the filter).

## 3. Request-scoped notifications

```bash
META2='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{},"progressToken":"job-1","io.modelcontextprotocol/logLevel":"info"}'

curl -N -X POST http://127.0.0.1:8005/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: long_job' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"long_job\",\"arguments\":{},$META2}}"
```

Three `notifications/progress` and three `notifications/message` frames ride
this request's own stream before the final result. Omit `progressToken` or
`logLevel` from `_meta` and the server stays silent for that surface — both
are per-request opt-ins.

## Framework APIs shown

- `SessionContext::notify_request_progress_with_message` — progress
  referencing the request's token (no-op without one)
- `SessionContext::notify_log` — gated by the request's declared `logLevel`
- `SharedNotificationBroadcaster::broadcast_to_all_sessions` — feeds every
  open listen stream, which filters per its own subscription
