# Manual E2E Matrix — client × server combinations

Runnable commands for driving this framework by hand, per spec lane and across
lanes. Companion to `docs/plans/2026-07-28-manual-verification.md`, which covers
judgement calls and a curl-level smoke test; this file covers **which binaries to
point at which**, and what a correct run looks like.

**What is verified here, and what is not.** On 2026-07-30 I ran §1 A1, §2 B1
(twice — before and after the `echo_sse` progress-token fix),
§3 (cross-lane), §6's `ci-gates.sh all` and the reachability guard, and the
lane-mutex error in §0 — the expected-output blocks for those are transcribed
from real runs, warnings included. The remaining commands are constructed from
the repo (every `-p` name is checked to resolve against `cargo metadata`, and
every script path exists) but I have **not** executed each one, so treat their
described outcomes as intent rather than observation. Where a run produces a
warning or a skip, it is stated rather than omitted.

---

## 0. The one thing that will bite you first

**The two spec lanes cannot be built in a single cargo invocation.** They are
mutually exclusive features on `turul-mcp-protocol`, enforced deliberately:

```bash
cargo build -p minimal-server -p client-initialise-server
```

```
error: turul-mcp-protocol: features `protocol-2025-11-25` and
       `protocol-2026-07-28` are mutually exclusive — a build re-exports
       exactly one MCP spec. Enable one.
```

That is the mutex working, not a broken tree. Consequences:

- Never put a 2025-lane and a 2026-lane package in the same `-p` list.
- `--workspace` fails for the same reason.
- Switching lanes in the *same* target dir triggers a full rebuild each way.
  Give each lane its own target dir and the flip becomes free:

```bash
export CARGO_TARGET_DIR_2026=target
export CARGO_TARGET_DIR_2025=target-2025
```

Prefix 2025-lane commands with `CARGO_TARGET_DIR=target-2025`. Every 2025 command
below already does. Add `/target-2025/` to `.gitignore` if you keep it around.

The E2E harness that spawns fixture servers (`tests/shared/src/e2e_utils.rs`)
resolves binaries from the same `CARGO_TARGET_DIR`, so a per-lane directory is
safe. It did not always: it rebuilt into `CARGO_TARGET_DIR` while launching from
a hardcoded `target/debug`, which meant the suites silently exercised whatever
stale binary sat there — possibly one built for the other lane. If you are on a
checkout older than 2026-07-30 and using a custom target dir, the nested
`tests/*` E2E results are not trustworthy.

---

## 1. Lane A — 2026-07-28 (stateless, the default)

No `initialize`, no `notifications/initialized`, no `Mcp-Session-Id`. The client
calls `server/discover` and carries capabilities per request in `_meta`.

### A1. Typed client → 2026 server

Terminal 1:
```bash
cargo run -p minimal-server -- --port 8641
```

Terminal 2:
```bash
cargo run -p streamable-http-client -- http://127.0.0.1:8641/mcp
```

Expect: `Negotiated protocol: Some(V2026_07_28)`, `Supported versions:
["2026-07-28"]`, one tool (`echo`), then a `call_tool` and a
`call_tool_with_progress` both returning `structuredContent`, and every result
carrying `_meta."io.modelcontextprotocol/serverInfo"`.

The run ends with a `subscriptions/listen` acknowledgement and the note
`(no broadcast source here — run notification-server on port 8005 …)`. That is
expected against `minimal-server`, which has nothing to broadcast. For an actual
notification stream use A2.

### A2. Notifications actually flowing

```bash
cargo run -p notification-server            # terminal 1, port 8005
cargo run -p streamable-http-client -- http://127.0.0.1:8005/mcp   # terminal 2
```

### A3. Raw-wire report (no client library)

```bash
cargo run -p minimal-server -- --port 8641
cargo run -p client-initialise-report -- --url http://127.0.0.1:8641/mcp
```

Use this when you suspect the client library is masking a server-side wire
problem — it builds the requests itself.

### A4. Server variety on the same client

All 2026-lane; swap the server, keep the client. Each demonstrates a different
surface, and pointing one client at all of them is the cheapest way to find a
server that disagrees with the others:

```bash
cargo run -p calculator-add-function-server      # #[mcp_tool] authoring
cargo run -p calculator-add-manual-server        # hand-written trait impls
cargo run -p resources-server                    # resources + templates
cargo run -p prompts-server                      # prompts + completion
cargo run -p pagination-server                   # cursor walks
cargo run -p completion-server                   # completion/complete
cargo run -p header-bound-tools-server           # Mcp-Method / Mcp-Name headers
cargo run -p ext-tasks-server                    # tasks extension
cargo run -p icon-showcase                       # icons on tools/list
cargo run -p tool-output-schemas                 # outputSchema + structuredContent
```

Then, for each, in another terminal (adjust the port the server prints):
```bash
cargo run -p streamable-http-client -- http://127.0.0.1:<port>/mcp
```

### A5. Middleware and auth

```bash
cargo run -p middleware-auth-server        # -32001 / -32005 JSON-RPC rejections
cargo run -p middleware-rate-limit-server  # -32003 with retryAfter
cargo run -p middleware-logging-server
cargo run -p oauth-resource-server         # 401 + WWW-Authenticate challenge
cargo run -p origin-policy-server          # Origin enforcement
```

Note the deliberate layering: `middleware-auth-server` answers HTTP 200 with a
JSON-RPC error, while `oauth-resource-server` answers HTTP 401 with a
`WWW-Authenticate` header. Different layers, not an inconsistency — see ADR-012
§Error Mapping.

All six `MiddlewareError` variants now reach the wire. `InvalidRequest` answers
`-32600` with the message in `data.reason`; `Internal` and `Custom` answer
`-32603`. Until 2026-07-30 those three panicked instead, so a middleware
rejection other than auth or rate-limiting aborted the request.

---

## 2. Lane B — 2025-11-25 (stateful, opt-in)

`initialize` → capture `Mcp-Session-Id` → `notifications/initialized` → that
header on every later request.

### B1. Raw-wire client → 2025 server

Terminal 1:
```bash
CARGO_TARGET_DIR=target-2025 cargo run -p client-initialise-server -- --port 52950
```

Terminal 2:
```bash
CARGO_TARGET_DIR=target-2025 cargo run -p streamable-http-client-2025-11-25 -- --url http://127.0.0.1:52950/mcp
```

Expect the full handshake, an SSE stream of 5 events, 2 progress notifications,
a final `structuredContent` result, and a `DELETE` session teardown → HTTP 200.

The progress notifications must carry **the token the client sent**:

```
📈 progress: ProgressUpdate { progress: Some(50.0),  token: Some("streamable-demo-1") }
📈 progress: ProgressUpdate { progress: Some(100.0), token: Some("streamable-demo-1") }
✅ Server echoed our progressToken 'streamable-demo-1'
```

If instead you see `⚠️ Server did NOT echo progressToken … — saw
["echo_processing", …]`, the tool has regressed to `notify_progress()` with a
token of its own choosing. `echo_sse` uses `notify_request_progress()`, which
reads the caller's `_meta.progressToken` and returns `false` when the caller
declared none — progress was then never opted into, so nothing is sent. The
underlying contract is pinned by
`crates/turul-mcp-server/tests/progress_token_match_2025_11_25.rs`; this run is
what covers the *example*.

### B2. Storage backends behind the same server

```bash
CARGO_TARGET_DIR=target-2025 cargo run -p client-initialise-server -- --port 52950 --storage-backend inmemory
CARGO_TARGET_DIR=target-2025 cargo run -p client-initialise-server -- --port 52950 --storage-backend sqlite
```

Postgres and DynamoDB need a live backend and the matching feature; the server
prints the exact command for each on startup.

### B3. 2025-lane server variety

```bash
CARGO_TARGET_DIR=target-2025 cargo run -p stateful-server
CARGO_TARGET_DIR=target-2025 cargo run -p session-aware-resource-server
CARGO_TARGET_DIR=target-2025 cargo run -p elicitation-server
CARGO_TARGET_DIR=target-2025 cargo run -p sampling-server
CARGO_TARGET_DIR=target-2025 cargo run -p roots-server
CARGO_TARGET_DIR=target-2025 cargo run -p dynamic-tools-server
CARGO_TARGET_DIR=target-2025 cargo run -p logging-test-server
```

`logging-test-server` has a matching client:
```bash
CARGO_TARGET_DIR=target-2025 cargo run -p logging-test-client
```

### B4. Tasks lifecycle (2025 lane)

```bash
CARGO_TARGET_DIR=target-2025 cargo run -p tasks-e2e-inmemory-server
CARGO_TARGET_DIR=target-2025 cargo run -p tasks-e2e-inmemory-client
```

---

## 3. Cross-lane — one client, both generations

This is the combination worth running before any release, because it is the only
one that proves version negotiation rather than assuming it.

Three terminals:
```bash
cargo run -p minimal-server -- --port 8641                                      # 2026-07-28
CARGO_TARGET_DIR=target-2025 cargo run -p client-initialise-server -- --port 52950   # 2025-11-25
cargo run -p bilingual-fleet-client -- http://127.0.0.1:8641/mcp http://127.0.0.1:52950/mcp
```

Correct output — one process, two servers, two different specs:

```
── http://127.0.0.1:8641/mcp
   negotiated: 2026-07-28 (server/discover answered — stateless wire)
   tools (1): ["echo"]

── http://127.0.0.1:52950/mcp
   negotiated: V2025_11_25 (discover refused — fell back to the initialize
               handshake; Mcp-Session-Id session is live)
   tools (4): ["echo_sse", "get_session_data", "get_session_events", "get_table_info"]
```

Two `WARN … Connection lost` lines at exit are the streaming listeners being
torn down, not a failure.

The client is built once, on the 2026 lane, and speaks both — it links both
protocol crates directly. If the 2025 server negotiates as `2026-07-28`, or the
2026 server falls back to the handshake, that is a real negotiation defect.

---

## 4. Foreign peers (interop)

Our code on one end, someone else's on the other. These are the only checks that
can catch a contract both halves of this repo get wrong the same way.

```bash
./scripts/interop-fastmcp.sh          # Python FastMCP peer   — needs: uv
./scripts/interop-typescript-sdk.sh   # TypeScript SDK peer   — needs: node, npm, jq
./scripts/interop-go-sdk.sh           # Go SDK v1.7.0 peer    — needs: go
./scripts/interop-turul-client.sh     # our client → FastMCP  — needs: uv
```

Each SKIPs with a named reason if its toolchain is absent rather than reporting a
pass. `interop-turul-client.sh` runs a turul→turul control cell first; if the
control fails, ignore the foreign-peer result until it passes.

The probe underneath the last one can be aimed at any 2026 server by hand:

```bash
cargo run -p interop-client-probe -- http://127.0.0.1:8641/mcp
```

It prints one `LEG` line per surface with `OK`, `FAIL`, or `SKIP`. `SKIP` means
the peer exposed nothing to drive that leg against — expected against a minimal
peer, and deliberately not `FAIL`. It ends `CORE ok` or `CORE failed`; only
`server/discover`, `tools/list` and `tools/call` affect that verdict.

---

## 5. Lambda (real Runtime API)

Needs `cargo-lambda` (https://cargo-lambda.info); each script SKIPs with that
message if it is missing.

```bash
./scripts/e2e-lambda-local.sh              # 2026-07-28 over the real Runtime API
./scripts/e2e-lambda-local-2025-11-25.sh   # 2025-11-25 lane
./scripts/e2e-lambda-client-local.sh       # turul-mcp-client over Lambda
```

The third prints an explicit SKIP for its 2025-11-25 leg:

```
SKIP  not exercisable: cargo lambda watch serves invocations serially and
      the 2025-11-25 client holds a GET SSE stream open, so the stream and
      the next POST race for the single function instance
```

That is a limitation of the local emulator, not of the framework. The 2026 leg
runs for real.

Some Lambda examples additionally need a live `mcp-sessions` DynamoDB table and
will SKIP without it, verifying only the build and the `cargo lambda` boot.

---

## 6. The automated suites, by hand

```bash
./scripts/ci-gates.sh all           # everything below, in order
./scripts/ci-gates.sh default       # 2026-07-28 lane
./scripts/ci-gates.sh opt-in-2025   # 2025-11-25 lane
./scripts/ci-gates.sh lambda
./scripts/ci-gates.sh mutex         # proves the two lanes refuse to co-compile
./scripts/ci-gates.sh docs
./scripts/ci-gates.sh examples
```

`ci-gates.sh` prints `ALL GATES PASSED` or `ONE OR MORE GATES FAILED` and exits
non-zero on failure. It emits **no numeric pass/fail total** — if you want counts,
count the `PASS` markers yourself. Capture the exit code deliberately; a pipe
into `tail` reports the exit status of `tail`, which has produced a false green
here before:

```bash
./scripts/ci-gates.sh all > /tmp/gates.log 2>&1; echo "exit=$?"
```

Targeted suites:

```bash
# 2026-07-28 lane
cargo test -p turul-mcp-server --test discover_stateless_2026
cargo test -p turul-mcp-server --test progress_2026
cargo test -p turul-mcp-server --test mcp_headers_2026
cargo test -p turul-mcp-server --test error_mapping_2026
cargo test -p turul-mcp-server --test wire_edges_2026
cargo test -p turul-mcp-server --test cancellation_2026
cargo test -p turul-mcp-server --test tool_icons_2026

# 2025-11-25 lane
CARGO_TARGET_DIR=target-2025 cargo test -p mcp-prompts-tests
CARGO_TARGET_DIR=target-2025 cargo test -p mcp-resources-tests
CARGO_TARGET_DIR=target-2025 cargo test -p mcp-tools-tests
CARGO_TARGET_DIR=target-2025 cargo test -p mcp-elicitation-tests
CARGO_TARGET_DIR=target-2025 cargo test -p mcp-roots-tests
CARGO_TARGET_DIR=target-2025 cargo test -p mcp-sampling-tests

# Reachability of the tests and test-crate binaries themselves
cargo test -p turul-mcp-framework-integration-tests --test reachability_guard

# Schema pin still names the released artifact
cargo run -p turul-mcp-protocol-2026-07-28 --bin mcp-compliance-2026-07-28 \
    --features compliance -- refresh
```

---

## 7. Curl, when you need the actual bytes

2026-07-28 — no session, capabilities in `_meta`, `Mcp-Method` header:

```bash
curl -sS http://127.0.0.1:8641/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/list' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{
        "io.modelcontextprotocol/protocolVersion":"2026-07-28",
        "io.modelcontextprotocol/clientCapabilities":{}}}}' | jq
```

Ask for SSE instead by sending `Accept: text/event-stream`. A request declaring a
`_meta.progressToken` is answered as a stream; one declaring neither a
progressToken nor a logLevel is answered as a single JSON object.

2025-11-25 — capture the session header first:

```bash
SID=$(curl -sS -D - -o /dev/null http://127.0.0.1:52950/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"curl","version":"0"}}}' \
  | tr -d '\r' | awk '/^[Mm]cp-[Ss]ession-[Ii]d:/{print $2}')
echo "session=$SID"

curl -sS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:52950/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'      # expect 202

curl -sS http://127.0.0.1:52950/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | jq
```

Negative checks worth doing by hand on the 2025 lane: omit `Mcp-Session-Id` →
**400**; send an unknown session id → **404**; send a terminated one → **404**.
On the 2026 lane, sending `Mcp-Session-Id` must be *ignored* — the server neither
mints nor echoes one.

---

## 8. Cleanup

Examples bind fixed ports and keep running. Between rounds:

```bash
pkill -f minimal-server; pkill -f client-initialise-server
ss -ltn | grep -E '8641|52950|8005'      # expect no output
```

---

## Known-noisy results, so you can tell them from regressions

| What you see | Verdict |
|---|---|
| `⚠️ Server did NOT echo progressToken` in B1 | **A regression, not noise** — `echo_sse` must echo the caller's token. §2 B1 |
| `LEG … SKIP peer exposed no …` from the probe | Expected against a minimal peer. Not a failure |
| `SKIP not exercisable: cargo lambda watch serves invocations serially` | Emulator limitation, stated in full by the script |
| `SKIPPED: DynamoDB unavailable` | No live table; build and boot still verified |
| `WARN Connection lost` at client exit | Streaming listener teardown |
| `features … are mutually exclusive` | The mutex working. Split the `-p` list by lane |
| `(no broadcast source here …)` from A1 | `minimal-server` has nothing to broadcast; use A2 |
