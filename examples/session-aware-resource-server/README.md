# Session-Aware Resource Server

Resources whose **content depends on session state**. `McpResource::read`
receives an `Option<&SessionContext>`, so a resource can personalise what it
returns and can write state back.

## Which spec lane, and why

**Deliberately pinned to MCP 2025-11-25** (see `Cargo.toml`: every framework
dependency carries `features = ["protocol-2025-11-25"]`).

Session-aware resources need state that survives *between* requests. The
2026-07-28 stateless core removed sessions entirely — `initialize`,
`notifications/initialized` and the `Mcp-Session-Id` header are all gone — so
on that lane each request gets a fresh ephemeral context and the activity log
below would reset on every read. Pinning 2025-11-25 is what makes the example
demonstrate anything.

For the 2026 resource pattern (no session) see
[`resources-server`](../resources-server/) and
[`function-resource-server`](../function-resource-server/).

## Run

```bash
cargo run -p session-aware-resource-server    # http://127.0.0.1:8010/mcp
```

## Resources

| URI | mimeType | Behaviour |
|---|---|---|
| `file:///session/profile.json` | `application/json` | Reflects `user_data` session state, falls back to an anonymous profile with no session |
| `file:///session/activity.log` | `text/plain` | Appends a line per read; **grows across requests in the same session** |

## Walkthrough (verified)

```bash
U=http://127.0.0.1:8010/mcp

SID=$(curl -si -X POST $U -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}' \
  | grep -i '^mcp-session-id:' | tr -d '\r' | awk '{print $2}')

H=(-H 'Content-Type: application/json' -H 'Accept: application/json'
   -H "MCP-Protocol-Version: 2025-11-25" -H "Mcp-Session-Id: $SID")

curl -s -X POST $U "${H[@]}" -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'

# Read the activity log twice — the second read has one more line than the first.
curl -s -X POST $U "${H[@]}" -d '{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"file:///session/activity.log"}}'
curl -s -X POST $U "${H[@]}" -d '{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"file:///session/activity.log"}}'
```

`resources/templates/list` answers with an empty `resourceTemplates` array —
this server registers concrete resources, not templates.

## Picking the right `ResourceContent` constructor

| Constructor | Emits | Use for |
|---|---|---|
| `ResourceContent::text(uri, s)` | `TextResourceContents`, `mimeType: text/plain` | plain text |
| `ResourceContent::json(uri, s)` | `TextResourceContents`, `mimeType: application/json` | JSON documents |
| `ResourceContent::blob(uri, b64, mime)` | `BlobResourceContents` | **base64-encoded binary only** |
| `.with_mime_type(m)` | overrides the default above | text that is neither `text/plain` nor JSON |

`blob` is not "text with a custom mimeType" — its `blob` field is specified as
base64, so handing it a raw string produces contents a client cannot decode.
The profile resource here uses `json`, not `blob`, for exactly that reason.
