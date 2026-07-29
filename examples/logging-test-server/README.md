# logging-test-server (MCP 2025-11-25)

Server half of the session-aware logging pair. Its tools emit
`notifications/message` at every RFC 5424 severity so a client can prove the
server filters them against **that session's** `logging/setLevel` threshold.

## Spec lane and why

**Pinned to 2025-11-25.** Logging is deprecated in 2026-07-28 (12-month
window, annotation-only in that revision), and the per-session `logging/setLevel`
threshold this example exists to demonstrate depends on the session the 2026
stateless core removed. On 2026 the equivalent is a per-request opt-in carried
in `_meta`.

## Run

```bash
RUST_LOG=info cargo run -p logging-test-server                        # port 8003
RUST_LOG=info cargo run -p logging-test-server -- --port 53103
RUST_LOG=info cargo run -p logging-test-server -- --disable-post-sse  # JSON-only responses
```

`--enable-post-sse` (the default) lets a POST answer with
`Content-Type: text/event-stream` when the caller's `Accept` allows it;
`--disable-post-sse` forces JSON responses.

Pick a port explicitly if you run several examples at once — 8003 is a common
collision.

## What to expect

Drive it with the client:

```bash
RUST_LOG=info cargo run -p logging-test-client -- --port 8003 --quick-test
```

```text
📊 TEST 2 - INFO Session (threshold: Info)
   ✅ Debug   [019fad3063b17252afec34268d017265]: FILTERED (expected)
   ✅ Info    [019fad3065077e018a36df3fe8d45e2d]: RECEIVED (expected)
   ...
Expected: 7 notifications | Received: 7 | Result: ✅ PASS

🏆 OVERALL RESULT: ✅ ALL TESTS PASSED
```

Each request carries a correlation id that the emitted notification echoes,
so a received notification can be matched to the request that caused it
rather than counted in aggregate.

## Related

- `logging-test-client` — the automated PASS/FAIL prover
- `session-logging-proof-test` — the same idea with three concurrent sessions,
  but verified by hand rather than automatically
