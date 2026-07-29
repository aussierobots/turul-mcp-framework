# bilingual-fleet-client

One client binary talking to a mixed 2025-11-25 / 2026-07-28 server fleet —
the rolling-upgrade story.

## Spec lane

**Both, per connection.** `turul-mcp-client` links both versioned protocol
crates (the recorded exception to the protocol re-export rule). For each URL
it probes `server/discover`: a 2026 server answers and the connection locks
2026-07-28; a 2025 server refuses (`-32601` / `-32004`) and the client falls
back to the `initialize` handshake and locks 2025-11-25. The negotiated spec
then holds for that connection's lifetime.

The point: you can upgrade servers one at a time and clients are never
blocked on the fleet being uniform.

## Run

```bash
# One server from each generation:
cargo run -p minimal-server                              # 2026-07-28, port 8641
cargo run -p client-initialise-server                    # 2025-11-25, port 52950

# Sweep the fleet (these two URLs are also the built-in default):
cargo run -p bilingual-fleet-client -- \
    http://127.0.0.1:8641/mcp http://127.0.0.1:52950/mcp
```

## What to expect

```text
── http://127.0.0.1:8641/mcp
   negotiated: 2026-07-28 (server/discover answered — stateless wire)
   serverInfo: Some(Object {"name": String("minimal-server"), ...})
   supported : ["2026-07-28"]
   tools (1): ["echo"]

── http://127.0.0.1:52950/mcp
   negotiated: V2025_11_25 (discover refused — fell back to the initialize handshake; Mcp-Session-Id session is live)
   tools (4): ["echo_sse", "get_session_data", "get_session_events", "get_table_info"]
```

`list_tools()` is version-neutral: identical call, two different wires.
`disconnect()` routes by negotiated version — a no-op on 2026 (nothing to
tear down) and a session `DELETE` on 2025. An unreachable URL is reported
and the sweep continues.

## Related

- `streamable-http-client` — the same client against a single 2026 server
- `streamable-http-client-2025-11-25` — the 2025 wire spelled out byte by byte
