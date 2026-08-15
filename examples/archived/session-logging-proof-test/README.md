# session-logging-proof-test (MCP 2025-11-25 server, 2025-06-18 client)

A single binary that starts a logging server and then drives it with three
concurrent sessions, each at a different `logging/setLevel` threshold, so the
same tool call produces different notification sets per session.

## Spec lane and why

The **server** is pinned to 2025-11-25 (`default-features = false`,
`protocol-2025-11-25` on every framework dep) because per-session log levels
need the session the 2026-07-28 stateless core removed, and Logging is
deprecated in 2026-07-28 anyway.

The embedded **client** deliberately negotiates `2025-06-18` — that is a
backward-compatibility check on version negotiation, not an oversight, and
the source says so at both the header and the body field.

## Run

```bash
RUST_LOG=info cargo run -p session-logging-proof-test
```

It binds **127.0.0.1:8001** — not configurable. Stop anything else on that
port first.

## What it does

1. Boots the server in-process (tools: `log_proof`, `level_cascade`).
2. Opens three sessions and sets them to `debug`, `warning`, `error`.
3. Calls the tools on each, so the server emits `notifications/message` at
   every severity for all three.
4. Prints a `curl` command per session and **stops**, leaving the server
   running.

Each session's `initialize` is followed by `notifications/initialized`;
without it, strict lifecycle mode rejects every subsequent call with `-32031`.

## What to expect

The run ends with instructions rather than a verdict:

```text
ALL TESTS COMPLETED!

VERIFICATION INSTRUCTIONS:
1. Open 3 separate terminals
2. Run the SSE curl commands shown above for each session
...
Server will keep running for manual verification...
```

**The "proof" is manual.** This binary sends the traffic; a human opens the
three SSE streams and compares them. It asserts nothing and can fail no test.
For an automated PASS/FAIL check of the same behaviour, use
`logging-test-client` + `logging-test-server` — that pair correlates each
request to the notification it caused and prints per-level expected/received
counts.

Its `TEST 4: Session isolation` heading prints no output; the isolation claim
rests entirely on the manual comparison above.

## Related

- `logging-test-client` / `logging-test-server` — the automated equivalent
- `client-initialise-report` — the broader 2025-11-25 lifecycle probe
