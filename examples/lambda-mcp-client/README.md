# Lambda MCP Smoke Client

Companion client for [`lambda-mcp-server`](../lambda-mcp-server/). It performs
the MCP handshake, lists tools, and calls them — either as a one-shot probe
suitable for a post-deploy check, or as an interactive prompt.

## Which spec lane, and why

**Pinned to MCP 2025-11-25.** `lambda-mcp-server` is pinned to that lane
because it demonstrates DynamoDB-backed sessions and SSE, and the 2026-07-28
stateless core removed sessions entirely (`initialize`,
`notifications/initialized` and the `Mcp-Session-Id` header are all gone).
The bilingual `turul-mcp-client` negotiates per connection and locks 2025-11-25
against this server.

For the 2026-07-28 equivalents:

| Want | Use |
|---|---|
| High-level 2026 stateless client | [`streamable-http-client`](../streamable-http-client/) |
| Raw 2025 wire, every header visible | [`streamable-http-client-2025-11-25`](../streamable-http-client-2025-11-25/) |
| 2025 session lifecycle compliance | [`session-management-compliance-test`](../session-management-compliance-test/) |

## Run it

```bash
# Terminal 1 — the server under test
cargo lambda watch --package lambda-turul-mcp-server

# Terminal 2 — one-shot probe (exits non-zero on failure)
cargo run -p lambda-turul-mcp-client -- probe

# …or against a deployed Function URL
cargo run -p lambda-turul-mcp-client -- probe \
  --url https://<id>.lambda-url.<region>.on.aws

# Interactive prompt
cargo run -p lambda-turul-mcp-client -- connect
```

`--url` accepts a base URL; `/mcp` is appended when absent. `--debug` prints
the full JSON for each response.

## What `probe` asserts

| Step | Gates the exit code? |
|---|---|
| Connect (handshake, negotiated version) | **yes** — non-zero on failure |
| `tools/list` | **yes** — non-zero on failure |
| `tools/call <name>` with empty arguments, per tool | only if *every* advertised tool errors |

Each tool is called with `{}`, so a tool with required parameters answers with
an error. That is a real round-trip and is reported (`•`) rather than treated
as a deployment fault. The probe fails when a server advertises tools and none
of them answer at all.

Verified against `client-initialise-server` (a 2025-11-25 server): passes with
`3/4 tool(s) answered`, and exits 1 against a closed port.

## Interactive commands

```
help                    Show help
tools                   List available tools (full JSON)
call <tool> [json_args] Call a tool
endpoint                Show the URL being posted to
quit                    Exit
```

## What this client does NOT report

`turul-mcp-client` does not retain the 2025-11-25 `initialize` result:
`discovered_server()` and `server_capabilities()` are populated from
`server/discover` on 2026 connections only, and the negotiated
`Mcp-Session-Id` is private to the transport. So this client reports the
negotiated wire version and nothing more — it deliberately does not
synthesise server capabilities or a session id it cannot observe. If you need
those visible on the 2025 lane, use `streamable-http-client-2025-11-25`, which
speaks the wire directly.
