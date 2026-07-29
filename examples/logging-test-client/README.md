# logging-test-client (MCP 2025-11-25)

Automated prover for session-aware log filtering. It sets a session log
level, fires one request at each of the eight RFC 5424 severities, watches a
persistent SSE stream, and asserts exactly which notifications arrive.

## Spec lane and why

**Pinned to 2025-11-25** (`turul-mcp-protocol` with `protocol-2025-11-25`).
It uses `logging/setLevel`, a per-session threshold, and a long-lived GET SSE
stream keyed by `Mcp-Session-Id` — all three are 2025 constructs. Logging is
deprecated in 2026-07-28 and the 2026 stateless core has neither the session
nor the GET SSE stream.

## Run

```bash
RUST_LOG=info cargo run -p logging-test-server -- --port 53103   # terminal 1
RUST_LOG=info cargo run -p logging-test-client -- --port 53103 --quick-test
RUST_LOG=info cargo run -p logging-test-client -- --port 53103   # comprehensive
```

## What it does on the wire

1. `initialize`, reads `Mcp-Session-Id` from the response header.
2. `notifications/initialized` — mandatory: strict lifecycle mode rejects
   every other method with `-32031` until this arrives.
3. Opens a persistent `GET /mcp` SSE stream for that session.
4. `logging/setLevel`, then eight `tools/call`s, each tagged with a
   correlation id the resulting notification echoes.
5. Counts arrivals per correlation id and compares against the threshold.

## What to expect

```text
📊 TEST 3 - ERROR Session (threshold: Error)
   ✅ Debug    [...]: FILTERED (expected)
   ✅ Warning  [...]: FILTERED (expected)
   ✅ Error    [...]: RECEIVED (expected)
   ✅ Emergency[...]: RECEIVED (expected)
Expected: 4 notifications | Received: 4 | Result: ✅ PASS

🏆 OVERALL RESULT: ✅ ALL TESTS PASSED
```

Note: `--test-post-sse` / `--test-both-modes` currently only change the
banner. The client reads JSON from every POST and observes notifications on
the persistent GET stream regardless of those flags.

## Related

- `logging-test-server` — the server half
- `client-initialise-report` — the broader 2025 lifecycle probe
