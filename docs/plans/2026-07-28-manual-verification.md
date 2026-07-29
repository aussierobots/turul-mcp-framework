# MCP 2026-07-28 — Manual Verification Checklist

> **Audience: the maintainer.** Every item here is something no automated test in this
> repo currently proves. Work top to bottom; each item states the exact command and the
> exact expected result. Tick a box only when you have seen the expected output with your
> own eyes.
>
> Companion document: [`2026-07-28-release-checklist.md`](./2026-07-28-release-checklist.md)
> (the engineering work items). This file is only the human-in-the-loop verification.
>
> Baseline recorded 2026-07-29 at HEAD `2958508`:
> `cargo test -p turul-mcp-protocol-2026-07-28 --features compliance` → **423 passed, 0 failed, 2 ignored**.

---

## A. Decisions only you can make

These block or reshape everything else. Answer them first — several checklist items below
are worded differently depending on your answer.

- [ ] **A1 — Branch-lock disposition.** The lock's stated rationale ("tracks adoption of the
      MCP 2026-07-28 **release candidate**", "the draft is still moving and is about to
      finalize") is now factually false: the spec is released. Three separable decisions:
      1. Correct the CLAUDE.md / AGENTS.md wording to "released / current" **now**,
         independently of any merge? (Safe, mechanical, reversible prose.)
      2. Does the branch lock stay in force after the wording change, or does finalization
         release it?
      3. If it stays, what is the new stated trigger for cutting over to `main`, and who
         records it?
      **Until A1 is answered, do not rename the branch, retitle ADR-027's file, or delete
      `OUTSTANDING.md`** — all three read as "this branch is done" signals.

- [ ] **A2 — Publish 0.4.0 now, or merge without publishing?** *The original argument against
      has been retired:* a third party (FastMCP 4) now completes the full stateless journey
      against a 2026 build — see D1, reproducible via `scripts/interop-fastmcp.sh`. What remains
      is narrower and worth a CHANGELOG sentence rather than a hold: interop is confirmed
      against FastMCP 4 (itself a beta) and *not* against the official TypeScript SDK, which has
      not shipped 2026-07-28 support. Publishing is still not forced by merging.

- [ ] **A3 — Is `plugins/turul-mcp-skills` currently listed or installable by third parties?**
      The staleness that made this urgent is being fixed regardless (task #15 rewrites the
      skills against the released spec), so this now only sets the *severity* if something is
      missed: blocker if publicly listed, should-fix if not. Note it does not gate a crates.io
      publish either way — `plugins/` is not packaged into any crate.

- [x] **A4 — RESOLVED 2026-07-29. Roots was a live spec defect, not a deprecation question.**
      Investigation against the schema found `roots/list` is absent from the 2026
      client-to-server request union entirely — `ListRootsRequest` is defined as sent *from the
      server to the client*, and roots now ride an MRTR input request. `.with_roots()` on a
      2026 build was answering `roots/list` with **HTTP 200** where the spec requires 404 with
      `-32601` (reproduced on a live server). The three builder surfaces are now cfg-gated to
      the 2025-11-25 lane, so the leak is a compile error. Not `#[deprecated]` — that implies
      "works, but migrate", and on 2026 it did not work. Removal trigger recorded in the
      CHANGELOG. The MRTR `ListRootsRequest` variant is untouched; gating it would have broken
      a spec-required path.

      *Superseded original wording:*
      **Roots/Sampling/Logging deprecation window.** SEP-2577 starts a 12-month clock
      (earliest removal 2027-07-28). Per CLAUDE.md §Active Development, temporary
      compatibility needs an owner, a removal trigger and a removal date. Decide what removal
      release to record, so the note can be written into `COMPLIANCE.md` and the CHANGELOG in
      the same slice.
      **Specific decision needed:** Sampling and Logging are feature-gated to the
      `protocol-2025-11-25` lane, so a 2026-default user cannot reach them. **Roots is not
      gated** and carries no `#[deprecated]` on the builder — a default-build user gets
      unwarned access to a feature with a death clock. Gate it to match, or mark it deprecated?
      Complicating factor: the same release routes **roots** through MRTR as a legal
      `InputRequest` variant while deprecating the Roots capability. Both cannot be the
      recommended path — say which is.

- [x] **A-neg — RESOLVED 2026-07-29. SSE vs JSON content negotiation.** The spec permits
      either framing and requires clients to support both, so our SSE answer was never a
      violation — but the choice is now made per request rather than by method name: SSE only
      when the request opted in via `_meta.progressToken` or
      `_meta."io.modelcontextprotocol/logLevel"`, plain JSON otherwise. Plain JSON is also the
      only path that can carry `-32020`/`-32021` on a 4xx, since chunked SSE commits 200 before
      dispatch. See ADR-006's 2026-07-29 revision. Verified by `scripts/interop-fastmcp.sh`.

- [ ] **A5 — Do you want a *network* schema-drift job** (weekly cron fetching upstream and
      diffing) in addition to the offline checksum gate? The offline gate is deterministic;
      the network one can flake on GitHub rate limits. Policy call, not mechanical.

---

## B. Smoke-test the 2026-07-28 server by hand

Run these in order. Each is a real server plus a real HTTP client — the path a user takes.

> All commands below were run against a live `minimal-server` on 2026-07-29 and produce the
> stated output. Every 2026-07-28 request needs **three** things: the `MCP-Protocol-Version`
> header, the `Mcp-Method` header, and a `params._meta` block — omit any one and the server
> rejects with `-32020` or `-32602`. Named calls (`tools/call`, `prompts/get`,
> `resources/read`) additionally need `Mcp-Name`, matching the name in the body.

- [ ] **B1 — Minimal server responds on the stateless path.**
      ```bash
      cargo run -p minimal-server -- --port 8641
      ```
      In a second terminal, list tools. Note there is **no `initialize` handshake** and no
      `Mcp-Session-Id` — that absence is the headline change and is what you are verifying:
      ```bash
      curl -s -X POST http://127.0.0.1:8641/mcp -H 'Content-Type: application/json' -H 'Accept: application/json' -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/list' -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' | jq
      ```
      **Expected (verified):** `"resultType":"complete"`, a `tools` array containing `echo`,
      `"ttlMs":0`, `"cacheScope":"public"`, and `_meta["io.modelcontextprotocol/serverInfo"]`
      naming the server. `ttlMs`/`cacheScope` are required fields on `tools/list` this spec.

- [ ] **B2 — `tools/call` with a correct `Mcp-Name` header.**
      **The README for this example is currently WRONG** (it sends `Mcp-Name: test-client`).
      `Mcp-Name` must equal the *item name being invoked*, not a client identifier. Note the
      argument is `text`, not `message`:
      ```bash
      curl -s -X POST http://127.0.0.1:8641/mcp -H 'Content-Type: application/json' -H 'Accept: application/json' -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: echo' -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"text":"Hello, MCP!"},"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' | jq
      ```
      **Expected (verified):** `structuredContent.result` = `"Echo: Hello, MCP!"`,
      `"isError":false`, `"resultType":"complete"`.

- [ ] **B3 — Header-mismatch rejection fires with the right code.** Re-run B2 with
      `-H 'Mcp-Name: wrong'`.
      **Expected:** JSON-RPC error **`-32020`** ("Header mismatch"), *not* `-32001`. This
      confirms the renumbered error-code partition is live on the wire. Dropping the
      `MCP-Protocol-Version` header instead also yields `-32020` (verified).

- [ ] **B4 — `server/discover` advertises truthfully.** It requires `params._meta` like every
      other request:
      ```bash
      curl -s -X POST http://127.0.0.1:8641/mcp -H 'Content-Type: application/json' -H 'Accept: application/json' -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' -d '{"jsonrpc":"2.0","id":3,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' | jq
      ```
      **Expected (verified):** `supportedVersions` = `["2026-07-28"]` (single-spec-per-build is
      correct and truthful), plus `capabilities` and `_meta` server identity.

- [ ] **B5 — Unsupported version is rejected with `-32022`.** Re-run B1 with
      `"io.modelcontextprotocol/protocolVersion":"2025-11-25"`.
      **Expected:** error **`-32022`** (`UnsupportedProtocolVersion`).

- [ ] **B6 — Resource-not-found uses the new code.** Against `cargo run -p resources-server`,
      request a URI that does not exist.
      **Expected:** **`-32602`** (Invalid Params), *not* the old `-32002`.

- [ ] **B7 — `subscriptions/listen` replaces the GET SSE endpoint.** Against
      `cargo run -p notification-server`, POST a `subscriptions/listen` opting into
      `toolsListChanged`.
      **Expected:** a long-lived POST-response stream; first event is
      `notifications/subscriptions/acknowledged`; every subsequent notification carries
      `_meta["io.modelcontextprotocol/subscriptionId"]`. Confirm a plain **HTTP GET** to
      `/mcp` is *not* a valid SSE endpoint any more.

- [ ] **B8 — MRTR round-trip.** Against `cargo run -p mrtr-elicitation-server`, invoke the
      tool that needs client input.
      **Expected:** first response is `"resultType":"input_required"` carrying
      `inputRequests`; retrying the *original* request with `inputResponses` completes it.
      There must be **no** server-initiated `elicitation/create` request and no
      `notifications/elicitation/complete`.

---

## C. Verification the automated suite cannot reach

- [ ] **C1 — MCP Inspector against a 2026-07-28 server.** Point the official Inspector at
      `cargo run -p minimal-server`. **Expected:** it connects with no `initialize` step,
      lists tools, and calls one successfully. *This is the single highest-value item in
      this document* — see D1.

- [ ] **C2 — `SubscriptionsListenResult` graceful teardown is NOT emitted.** This is a known,
      documented gap, not a bug to chase: the server has no shutdown-signal infrastructure
      to emit the terminal frame from. Verify only that an abrupt close does not corrupt the
      stream or panic the server. Spec-legal — the schema permits never emitting it.

- [ ] **C3 — Lambda deployment path.** Deploy `lambda-turul-mcp-server` and run one
      `tools/call` against the deployed URL. No automated test exercises real Lambda
      Runtime API wire bytes end to end against a live deployment.

- [ ] **C4 — OAuth flow against a real authorization server.** The RFC 9207 `iss` validation
      and the `application_type` DCR requirement are new this spec. No test in-tree drives a
      real AS.

- [ ] **C5 — The 2025-11-25 opt-in lane still works.** Build and smoke one example on the old
      lane to confirm de-drafting did not disturb it:
      ```bash
      cargo run -p client-initialise-server --no-default-features --features protocol-2025-11-25
      ```
      *(Cross-checked already: the frozen crate contains 0 draft literals and the bilingual
      client negotiates on full-date strings, so no breakage is expected — this is
      confirmation, not investigation.)*

---

## D. The uncomfortable one

- [x] **D1 — Third-party interop: MEASURED 2026-07-29. Result is decision-relevant for A2.**

      **The 2025-11-25 lane interoperates with the official SDK — verified, not asserted.**
      Ran the official `@modelcontextprotocol/sdk@1.30.0` (upstream's own TypeScript client,
      no turul code in the client path) against `client-initialise-server`:
      ```
      CONNECT: ok
      serverVersion: {"name":"client-initialise-server","version":"0.4.0"}
      TOOLS: echo_sse, get_session_data, get_session_events, get_table_info
      CALL echo_sse -> [{"type":"text","text":"{\"output\":{\"result\":\"Echo: hello\"}}"}]
      ```

      **The 2026-07-28 lane interoperates with FastMCP 4 — verified on the wire.**
      `fastmcp==4.0.0b1` (an independent Python implementation, no turul code in the client
      path) drove `minimal-server` on the 2026-07-28 default build. To reproduce:
      ```bash
      uv venv && uv pip install --prerelease=allow 'fastmcp==4.0.0b1'
      cargo run -p minimal-server -- --port 8654 &
      uv run --no-project python fastmcp_probe.py http://127.0.0.1:8654/mcp
      ```
      Captured through a logging proxy, so this is the actual byte-level exchange, not the
      client's self-report:
      ```
      MCP-Protocol-Version='2026-07-28'  Mcp-Method='server/discover'  rpc='server/discover'
      MCP-Protocol-Version='2026-07-28'  Mcp-Method='tools/list'       rpc='tools/list'
      MCP-Protocol-Version='2026-07-28'  Mcp-Method='tools/call'       rpc='tools/call'
      CALL echo -> TextContent(text='{"result":"Echo: hello"}')
      ```
      Note what is absent: **no `initialize`, no `notifications/initialized`, no
      `Mcp-Session-Id`** — the stateless journey exactly as the spec defines it, including the
      `server/discover`-first probe. This is the strongest external evidence available: it
      exercises the headline change of the release end to end against foreign code.

      **Caveat, measured not assumed — a Python-version-dependent client crash.** On
      **Python 3.12** the full client round-trip completed, printing the tool result. On
      **Python 3.14.4** the same client sends all three requests (confirmed at the proxy, and
      our server answers all three correctly) but segfaults while consuming our `tools/call`
      response, so the result never surfaces client-side.
      A control run — FastMCP client against **FastMCP's own server**, both on 3.14.4 —
      printed its result and *then* segfaulted at interpreter shutdown. So 3.14 + FastMCP
      4.0.0b1 is unstable in general, but the earlier crash point against us is explained by a
      real response-shape difference:

      | Server | Response to identical `Accept: application/json, text/event-stream` |
      |---|---|
      | FastMCP | plain JSON |
      | turul | **SSE-framed** (`data: {…}`) |

      Both are permitted by Streamable HTTP — the server MAY answer either way — but turul
      prefers SSE whenever the client advertises `text/event-stream`, which is the less
      exercised branch in client implementations. See the open question below.

      **The official TypeScript SDK does NOT yet support 2026-07-28.** `@modelcontextprotocol/sdk@1.30.0`
      (newest on npm) declares
      `SUPPORTED_PROTOCOL_VERSIONS = ['2025-11-25','2025-06-18','2025-03-26','2024-11-05','2024-10-07']`.
      Pointed at the 2026 server it fails with **HTTP 400** — our server behaving correctly by
      serving one spec and refusing a 2025-11-25 handshake. Not a defect on our side; the
      ecosystem is mid-transition.

      **What this means for the publish decision (A2):** the "zero external verification"
      objection no longer holds — one independent implementation completes the full stateless
      journey. The residual risk is narrower and worth stating in the CHANGELOG: interop is
      confirmed against FastMCP 4 (itself beta) and not yet against the official SDK, which
      has not shipped 2026-07-28 support.

      Re-run when the official SDK adds support: `npm view @modelcontextprotocol/sdk version`,
      check `SUPPORTED_PROTOCOL_VERSIONS` in `dist/esm/types.js`, then repeat C1.

---

## E. Post-fix regression confirmation

Run after the engineering checklist is applied.

- [ ] **E1** — `cargo test -p turul-mcp-protocol-2026-07-28 --features compliance`
      → expect **≥ 423 passed, 0 failed** (baseline above; count should rise if fixtures are
      newly modeled).
- [ ] **E2** — `cargo test -p turul-mcp-framework-integration-tests --test example_validation`
      → currently **fails to compile**; expect green after the `origin_policy` fix.
- [ ] **E3** — `cargo test -p turul-mcp-framework-integration-tests` (all binaries)
      → expect ~402 tests green once wired into CI.
- [ ] **E4** — `bash scripts/ci-gates.sh` → all gates pass, including the new schema-pin gate.
- [ ] **E5** — After re-pinning the schema, confirm the harness `modeled=N` count **moved**.
      A green run with an unmoved `modeled=N` proves nothing about the new commit — AGENTS.md
      §Schema pin governance is explicit that most fixture directories are `NotModeled`, so
      the harness reports `failed=0` for changes it never looked at.
