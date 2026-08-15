# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3] - 2026-08-15

**A security fix, and the compliance work that found it.** `turul-http-mcp-server`
0.4.0 → 0.4.1, `turul-mcp-ext-tasks` 0.1.0 → 0.1.1, `turul-mcp-server`
0.4.2 → 0.4.3. No other crate is republished; the frozen trio stays at `0.3.47`.

Release gate: `./scripts/ci-gates.sh all` → **92 gates, 0 failures** after
isolating one gate that lost a port race to a concurrent run and passes alone
(`ci-gates.sh lambda` → 10/0, 5/0, 3/0).

### SECURITY: a matching `Host` header no longer admits a foreign `Origin`

DNS-rebinding protection did not fire. `origin.rs` admitted any `Origin` whose
authority matched the request's `Host`, but `Host` is attacker-controlled: in
the rebinding attack the browser sends `Host` == the attacker's own name — the
URL host, rebound to loopback — so the two always agreed.

Measured against the fixture server with an otherwise-valid request:

| Request | Before | After |
|---|---|---|
| `Origin: http://evil.example.com` | 403 | 403 |
| `Host` **and** `Origin` both `evil.example.com` | **200** | **403** |

Same class as the TypeScript SDK's
[GHSA-w48q-cv73-mx4w](https://github.com/modelcontextprotocol/typescript-sdk/security/advisories/GHSA-w48q-cv73-mx4w).
Found by the upstream conformance scenario `dns-rebinding-protection`, then
reproduced by hand.

ADR-031 *specified* the `Host` match, so the code was faithful and the ADR was
the defect — its own Context section describes precisely the attack its
Decision then admitted. Revision entry added there.

**Operator impact — the one behaviour change to read before upgrading.** A
browser app served from a **non-loopback** origin must now declare that origin:

```rust
.origin_policy(OriginPolicy::AllowList(vec!["https://app.example".into()]))
```

Unchanged: absent `Origin` (curl and every SDK client), loopback origins,
`Disabled`, and the Lambda CORS derivation.

A legitimate same-origin deployment and a rebinding attack are
indistinguishable from `Origin` and `Host` alone, so `Host` cannot be the trust
anchor at any level of care. `matches_host_header` is deleted rather than
tightened, and it was redundant wherever it was safe — a loopback deployment's
origin already passes on the loopback branch.

### `SessionContext::client_capabilities()` — SEP-2322 was unimplementable

The spec requires a server to include `inputRequests` **only** for capabilities
the client declared. The framework enforced the negative half (`-32021` for an
undeclared capability) but never surfaced the declaration to a tool body, so a
tool could not degrade gracefully — ask for sampling when elicitation is absent
— and no server built on this framework could satisfy the requirement.
`server.rs` already parsed `_meta.clientCapabilities` for the tasks-extension
check; it now reaches `SessionContext` alongside `input_responses()`.

### Tasks extension (`io.modelcontextprotocol/tasks`, SEP-2663)

- `tasks/get` / `tasks/update` / `tasks/cancel` answered `-32602` for a client
  that never declared the extension. SEP-2663 makes that `-32021`: `-32602`
  told the client its task id was wrong when the real problem was that it had
  not negotiated the extension.
- Partial MRTR fulfilment left answered keys in `inputRequests`, so the next
  `tasks/get` asked the client to re-answer what it had just answered, with no
  way to tell the difference.
- `Mcp-Name` was never validated on `tasks/*`: the SEP-2243 dispatch table
  covered `tools/call`, `prompts/get` and `resources/read`, and everything else
  fell through unchecked. Extension methods validate the header **when
  present** rather than requiring it — this transport cannot see whether the
  extension is registered, so demanding it would answer `-32020` where
  `-32601`/404 is right, and would leak that the method is recognised.

### Builder: `experimental` and `extensions` are reachable

`.experimental_capability(key, value)` and `.extension_capability(id, value)`
reach two capability fields the schema models and nothing could set.
`extension_capability` is 2026-07-28 only — `capabilities.extensions` does not
exist in 2025-11-25, whose `ServerCapabilities` carries a concrete `tasks`
field instead.

Both advertise and nothing more: no handlers, no validation. Advertising a
capability you do not serve is a truthfulness violation the framework cannot
check for you.

### Tests that catch the class, not the instance

`every_modelled_server_info_field_is_builder_reachable` and
`every_modelled_capability_field_is_builder_reachable` serialize a fully
populated protocol struct, take its key set, and assert a configured builder
puts every key on the wire. Per-field tests could not catch this class:
`description` and `websiteUrl` were modelled and unreachable for a whole
release with nothing failing, because a field nobody wrote a case for has no
case to fail.

### Conformance

`examples/conformance-fixture-server` now passes **37 of 37 scored** scenarios
for `--requirements 2026-07-28`, plus both previously-failing `pending`
scenarios. Whole run 118 passed/52 failed → **179/11**; every remaining failure
is in the unscored tasks extension. Gated by `scripts/conformance-suite.sh`
(`ci-gates.sh conformance`), which treats a stale harness pin as fatal.

Known remaining, tracked and deliberately not blocking this release: MRTR under
task election creates the task before resolving the input round trip, where
SEP-2663 wants it resolved first. Unscored, and the extension is off by default
under SEP-2133.


## [0.4.2] - 2026-08-15

Two bug fixes found by running upstream's conformance suite, covering tool error
handling and resource serving. `turul-mcp-derive` 0.4.0 → 0.4.1 and
`turul-mcp-server` 0.4.1 → 0.4.2. No other crate is republished: the 0.4.x
crates keep their current versions, the frozen trio stays at `0.3.47`, and the
two extension crates stay at `0.1.0`.

Release gate: `./scripts/ci-gates.sh all` → **77 gates, 3623 tests, 0 failures**.

### A tool's own failure now returns `isError` instead of a JSON-RPC error

The spec (schema.ts:1828) says errors that originate from the tool SHOULD be
reported inside the result object with `isError` set to true, not as an MCP
protocol-level error response. Otherwise, a model driving the tool could not
see that an error occurred and self-correct. Both `#[mcp_tool]` and
`#[derive(McpTool)]` were mapping a tool's `Err` onto a JSON-RPC error
(`-32010`) instead.

The fix is narrow: ONLY `ToolExecutionError` becomes `isError: true`. `impl
From<&str>/From<String>` lands there, so the common `Err("...".into())` is
covered. Everything else keeps flowing as a JSON-RPC error, and that set is
load-bearing rather than cosmetic — a tool signals `input_required` by
returning `Err(McpError::InputRequired {..})` (MRTR), and
`MissingRequiredClientCapability` carries the spec's `-32021` gate. Both are
control-flow signals, not failures. A first version converted every `Err` and
broke MRTR completely; seven wire tests caught it. Both traps are written into
the macro comments so the rule does not widen back.

New test `macro_authored_tool_failure_is_also_is_error_not_a_json_rpc_error`
pins the macro path specifically. The pre-existing test only ever covered a
hand-written `McpTool`, so it proved the SERVER could carry `isError` while
saying nothing about whether the two recommended authoring paths could reach
it.

Included `examples/conformance-fixture-server` with 11 fixtures required by
upstream's suite. Measured against it, 14 of 15 runnable scenarios pass (43
checks). The one failure is a separate defect tracked below — `resources/read`
rejects a resource's own declared mimeType.

### A resource's own declared MIME type is now accepted

The resource-policy builder auto-derived allowed MIME types from file
extensions found in registered URIs. A resource declared as `test://static-binary`
+ `image/png` was answered `-32602`, because no other resource's URI happened
to end in `.png`. Whether a resource could be read depended on unrelated URIs'
cosmetics, and the derived list was strictly narrower than
`ResourceAccessControl`'s own default, which does include `image/png`. Non-file
schemes (test://, config://, ui://) are ordinary in MCP, so this was reachable
by normal configuration. There is no security value in validating a MIME type
the server itself authored — whoever controls the declaration controls the
content. The derivation and the now-dead `extensions_to_mime_types` /
`extract_extension` helpers were removed. URI allow/block patterns, traversal
blocking and size limits are untouched.

### `serverInfo.description` and `websiteUrl` are now reachable

The framework's `build()` method wired only `title` and `icons` to the
`ServerInfo` builder, leaving `description` and `websiteUrl` modelled,
serializable, and impossible to populate — with no error to notice. Two new
builder setters, `.description(…)` and `.website_url(…)`, expose them. Reported by a downstream consumer; a peer (FastMCP) already exposed
`website_url` while this framework silently dropped it.

### Two tests were corrected

`mcp_error_code_coverage::test_tool_execution_error` asserted the pre-fix
behaviour — that a tool's own failure produces a JSON-RPC error. It passed for
as long as the framework was wrong. A test that locks in a violation is worse
than no test: it converts a defect into a defended invariant. Corrected to
assert `isError: true`.

`session_context_macro_tests` exposed a wider effect of the tool-failure fix.
The macro previously wrapped every error variant in `McpError::tool_execution`,
so a `SessionError` arrived relabelled. Removing the blanket conversion means
it now reaches the caller as itself, which is the spec's split: `isError` is
for errors that originate from the tool, while any other exceptional
conditions stay on the JSON-RPC path. Corrected to expect `SessionError` as
itself.

Every corrected test was verified to FAIL without its fix, by reverting the fix
and re-running.

### FastMCP interop re-pinned and re-measured (4.0.0b2 → 4.0.0b3)

The pin was two releases stale in `scripts/interop-fastmcp.sh`, and the same
version was independently pinned in `scripts/interop-turul-client.sh` — one
fact, two homes. Both now b3 and both re-run: peer → turul 9/9 over the
stateless wire, turul → peer 9 driven / 8 answered (peer answers `-32601` for
`completion/complete`; R→R control passes, so the gap is theirs).

## [0.4.1] - 2026-08-15

Documentation only — no code, wire behaviour or API change. `turul-mcp-server`
is the sole republish, because its README is what renders on crates.io and that
README was wrong. Every other crate stays at `0.4.0`.

### The published Quick Start did not compile

Building a fresh `cargo new` project against the published 0.4.0 crates — the
first time the getting-started path had been walked the way a new user meets it
— produced **19 errors**. Two independent causes, both in the docs:

- **The dependency list was short by four crates.** `#[mcp_tool]` and
  `#[derive(McpTool)]` emit code naming `serde_json`, `async_trait`,
  `turul_mcp_builders` and `turul_mcp_protocol`. Generated code is compiled in
  the **consumer's** crate, so those names resolve against the consumer's
  dependency list; the macro crate having them is irrelevant. All 33 macro-using
  examples in this repo already declare the full six — the READMEs simply listed
  three.
- **The `main()` body had two type errors.** `.bind_address("…".parse()?)`
  inlines `?` where it must convert `AddrParseError` into `McpError`, which has
  no such `From`; and `server.run().await` as a tail expression yields
  `Result<(), McpError>` against a `Box<dyn Error>` signature. Corrected to the
  typed-`let` + `run().await?; Ok(())` shape all 39 examples already use.

Nothing about the framework changed. The code was correct; the instructions for
reaching it were not.

### New gate: `scripts/consumer-smoke.sh`

Builds the README's **own** Quick Start — extracted from `README.md` at run time,
not copied — in a crate **outside** this workspace, declaring only the
dependencies the README documents.

This is the check whose absence let the above ship. Every other gate runs inside
the workspace, where members inherit `[workspace.dependencies]`, share a lockfile,
and are written by people who already know the required dependency set. None of
them can observe a consumer-facing dependency or documentation error: **3617
tests passed over a Quick Start that did not compile.**

Verified to fail on the defect it exists to catch — reverting the README to its
pre-fix form makes the gate exit 1. Runs under `ci-gates.sh docs`; pass
`--published` to resolve from crates.io instead of the working tree.

## [0.4.0] - 2026-08-15

**Adopts MCP 2026-07-28 as the default specification.** The previous spec,
2025-11-25, remains fully supported as an opt-in build
(`--no-default-features --features protocol-2025-11-25`). A server binary
speaks one spec or the other — never both — because the two protocol features
are mutually exclusive at compile time; the client links both and negotiates
per connection (ADR-030).

Merged to `main` in [#18](https://github.com/aussierobots/turul-mcp-framework/pull/18)
at `6f35e04`. Release gate: `./scripts/ci-gates.sh all` → **76 gates, 3617
tests, 0 failures**. Externally verified by three stable peers — the reference
MCP Python SDK 2.0.0, the Go SDK v1.7.0 and the TypeScript SDK 2.0.0 — each
driving 9 methods plus 5 negative paths with no wire disagreement.

First publication of `turul-mcp-ext-apps` and `turul-mcp-ext-tasks` at `0.1.0`;
extension crates version independently of the framework per SEP-2133 §Evolution.

Entries below are newest-first.

### Pre-0.4.0 documentation accuracy pass (2026-08-15)

- **Every publishable crate now ships a `README.md` and declares
  `readme = "README.md"`** (was 13 of 15). `turul-mcp-ext-tasks` and
  `turul-mcp-schema-validation` had no README file at all, so crates.io would
  have shown them with none. The three crates still omitting the key are the
  frozen `0.3.47` trio, which correctly do not republish.
- **`turul-mcp-ext-tasks`'s README leads with its provenance caveat**, because
  it is the one a consumer most needs before depending on it: upstream
  `modelcontextprotocol/ext-tasks` is self-labelled *experimental*, publishes
  only the mutable `schema/draft/` path and cuts no tags. The crate therefore
  pins commit + content checksum — the strongest provenance a source with no
  tags admits — and says so where a consumer sees it rather than only in a
  document they may never open.
- **README:** the per-lane Task Support detail moved out of Quick Start into
  §Tasks Architecture, where the surrounding text explains why the lanes differ.
  In Quick Start it interrupted the five-line "hello world" with a lane caveat.
- **Seven documented commands named things that do not exist** and are fixed:
  three test files that are `#[path]`-included by an aggregate target rather
  than being `--test` targets (`autotests = false`), a schemars filter pointed
  at the wrong crate, a nonexistent `integration` test target, a nonexistent
  `integration` feature on `turul-mcp-builders`, and a `cd` into a path that is
  the *package* name rather than the directory. Lane coherence and port
  coherence were audited across ~155 documented commands and found **clean** —
  no doc drives a 2025 handshake at a 2026 server or the reverse.
- **`CLAUDE.md` pre-release step 2 described work that no longer exists.** It
  told the operator to edit `.version("x.y.z")` strings in `examples/*/src/main.rs`;
  all 45 examples that set a version use `env!("CARGO_PKG_VERSION")` and all 55
  manifests are already `0.4.0`. Replaced with the step that actually moves the
  wire value, plus a guard grep.
- **Release-checklist drift recorded.** Of 20 boxes still rendering unchecked,
  **18 were done and never ticked**, verified against the tree rather than
  against commit messages. The boxes are left as-is with a dated audit note, so
  the evidence survives; mass-ticking would have destroyed the trail.
- **Manual-verification doc corrected.** The load-bearing fix is the A2/D1
  interop premise, written 2026-07-29 when FastMCP 4 beta was the only peer and
  the official TypeScript SDK had not shipped 2026-07-28 support. Both claims
  are now false, so working that document top-to-bottom would have held the
  release on an objection that no longer exists.

### Single-era server posture, stated rather than implied (2026-08-11)

- The 2026-07-28 spec's §Versioning and Compatibility defines **dual-era** — one
  implementation serving modern and legacy clients, permitted to run
  "concurrently on the same endpoint or process" — and makes it a **MAY**.
  Nothing in the docs said which side of that this framework is on.
- **turul servers are single-era by construction** and now say so: the two
  protocol features are mutually exclusive via `compile_error!`, so a binary
  speaks one spec or the other. Serving both means two instances. The client is
  the exception and already links both, negotiating per connection (ADR-030).
- Stated in `README.md` (with the client-era × server-lane outcome table and the
  live `400` / `-32020` body a legacy client actually receives),
  `docs/compliance/base-protocol.md` §4, and `AGENTS.md`. The asymmetry worth
  knowing: per the spec's own matrix a **legacy client → modern server fails**,
  so a 2026-default server is unreachable to 2025-era clients.

### `turul-mcp-client` driven at a REAL 2025-11-25 server (2026-08-11)

- The 2026-07-28 half of ADR-030's bilingual contract was covered against a real
  server; the 2025-11-25 half was covered only by `wiremock` stubs. **A mock
  cannot disagree with the client, because the same author wrote both** — so if
  the server's 2025 lane and the client's 2025 lane had drifted apart, nothing
  in the repo would have noticed. `ci-gates.sh` only *built*
  `client-initialise-server` and never pointed the client at it.
- Three tests, all on mechanisms 2026-07-28 removed, so this doubles as the
  regression guard for the opt-in lane: the bilingual client locks 2025-11-25
  off a real server's `server/discover` rejection and the server mints an
  `Mcp-Session-Id`; that id survives real traffic rather than being re-handshaked
  per request; and tools round-trip with the returned payload asserted, not
  merely `Ok`.
- Hosted in the integration-tests crate by necessity — its `turul-mcp-server`
  dep is already pinned to `protocol-2025-11-25` while `turul-mcp-client`
  arrives bilingual. The client crate cannot host it: its own dev-dep on
  `turul-mcp-server` takes default (2026) features.

### Six workspace dependency bumps (2026-08-11)

- `syn` 2.0 → 3.0, `serial_test` 3.5 → 4.0, `jsonschema` 0.48 → 0.49 (a `0.x`
  minor is semver-major by cargo's rules), plus `hyper` 1.10 → 1.11, `http`
  1.4 → 1.5, `aws-config` 1.9 → 1.10.
- Verified rather than assumed, because three are major bumps and `syn`
  underpins `turul-mcp-derive`, which **both** spec lanes depend on: resolution
  checked with `cargo metadata`, default lane 29 PASS / 0 FAIL, opt-in lane
  36 PASS / 0 FAIL — the latter matters most, since it carries the derive-macro
  doctests.
- `jsonschema` stays `default-features = false`, so remote/local `$ref` fetching
  remains impossible to compile in; the bump does not widen that surface.

### `subscriptions/listen` driven from a peer (2026-08-08)

- `interop-fixture-server` gained `emit_changes`, broadcasting all four
  notification flavours. Emitting only the watched type would make "filtered
  correctly" and "emitted nothing else" indistinguishable.
- `scripts/interop-python-sdk.sh` opens a listen stream requesting
  `resourcesListChanged` only, then asserts on proxy-captured frames: the stream
  acknowledges **first**, delivers only the requested type, and every frame
  carries a `subscriptionId`.
- Two proxy defects fixed, the second being the dangerous shape: the buffering
  `read()` blocked forever on a long-lived stream (a deadline consulted only
  *after* a line arrives never fires once the server goes idle); and the listen
  handler's capture entry, appended at the end, raced the assertion pass and
  reported "no `subscriptions/listen` reached the server" while the client had
  demonstrably received notifications.

### MRTR and progress covered against a live peer — J3/J4 closed (2026-08-08)

- The two headline 2026-07-28 features were **self-verified only**: every check
  on MRTR and request-scoped progress had turul code on both ends of the wire.
  Both are now driven by the reference MCP Python SDK through a logging proxy,
  with assertions on captured bytes.
- Fixture `confirm` is an MRTR tool whose leg 2 re-derives the validation schema
  rather than persisting it — the stateless property J3 exists to prove — and
  rejects a tampered `requestState`. Fixture `count` emits progress carrying the
  *client's own* token and reports how many it actually sent, not how many it
  attempted.
- **J3a fired unprompted on the first run** against a probe that had simply
  forgotten to declare `elicitation`: `-32021` naming the capability it needed.
  The gate works. J3b then completed the round trip with the **SDK driving the
  retry itself**, so a foreign client finished MRTR unaided.

### Interop peers, toolchain pin, stale-doc reconciliation (2026-08-08)

- **New peer: the reference MCP Python SDK (`mcp==2.0.0`)**, cell P2→R, passing
  on its first run — 9 methods + 5 negatives, no `initialize`, no session header.
  Until now "Python interop" meant only FastMCP, a third-party framework; this
  is the implementation published by the protocol authors.
- **`interop-go-sdk.sh` had silently stopped running.** Its pin-currency check
  read `GO_SDK_VERSION` above the line assigning it, so under `set -u` the probe
  aborted every time while the matrix still recorded "pass". Fixed; the cell now
  passes for real.
- **Skips exited 0** in the Go and turul-client probes, making an absent peer
  indistinguishable from a green cell. Both now exit **77**.
- FastMCP pin `4.0.0b1` → `4.0.0b2`. R→P drives 9 methods and the peer answers 8
  (it returns `-32601` for `completion/complete`), recorded as "9 driven, 8
  answered" rather than as coverage that does not exist.
- **`rust-toolchain.toml` pins stable**, which is what every CI job uses. Without
  it the local toolchain floated: on nightly 2026-08-07 `clippy::double_must_use`
  fired on `async_trait` output and failed `ci-gates.sh opt-in-2025` twice while
  hosted CI stayed green. No code defect — a toolchain-parity defect.

### `OUTSTANDING.md` retired (2026-08-02)

- **The 0.4 compliance punch list is folded and deleted**, as the file itself
  instructed ("do not let it become a second, competing status authority
  alongside the driver doc"). It had one open item left —
  `SubscriptionsListenResult` graceful-close emission — now carried by
  `docs/plans/2026-07-28-spec-compliance.md` rows 336 and 356, which previously
  cited it by line number and are now self-contained. Earlier entries in this
  changelog that link to `OUTSTANDING.md` (including the 2026-07-13 burn-down
  below) are dated records and stand as written; the file's content remains in
  git history.
- Its second surviving line, a pagination test recorded as failing and
  "Untriaged", was verified closed: see the entry below.

### JWT validation moves to `turul-jwt-validator` (2026-08-02, ADR-032)

- **BREAKING (`turul-mcp-oauth`): `JwtValidator::validate` now returns
  `JwtValidationError`, not `OAuthError`.** The type is owned by the sibling
  [`turul-jwt-validator`](https://crates.io/crates/turul-jwt-validator) `0.3.2`
  and re-exported; `turul_mcp_oauth::{JwtValidator, TokenClaims}` still resolve.
  `turul-mcp-oauth` kept ~300 lines that had been extracted to that crate, which
  had since gained a revocation safety-net, stale-while-revalidate serving,
  fetch retry and typed JWKS failures — so the two had begun to diverge.
  Callers matching on `OAuthError` from `validate()` must switch; the only
  in-tree caller already mapped it.

- **BREAKING: `OAuthError::JwksFetchError` is now a struct variant**
  `{ kind, message }`, carrying `JwksFetchErrorKind`
  (`Timeout`/`Transport`/`HttpStatus`/`InvalidJson`/`NoSigningKeys`) instead of
  a flattened string. `OAuthError` also gained `#[non_exhaustive]`.

- **Added `turul_mcp_oauth::hardened_validator(jwks_uri, audience)`** — applies
  a signing-key max-age (15 min), stale window (5 min) and bounded fetch retry
  (3 × 100 ms), all of which upstream ships **disabled**. Worst-case revocation
  exposure goes from unbounded to `max_age + stale_window` = 20 minutes.
  `oauth_resource_server` routes through it, so the convenience and manual
  multi-AS paths share one definition of the policy. Prefer it over
  `JwtValidator::new`, which applies no hardening.

- **TLS is now enforced on `jwks_uri`** (loopback exempt for local development),
  closing a SHOULD that ADR-021 claimed but no code implemented. Tracked in
  `docs/compliance/base-protocol.md` as Implemented rather than Unknown.

- **Added `turul_mcp_oauth::Algorithm`** — re-exported so callers can use
  `with_algorithms` without adding a matching `jsonwebtoken` dependency of their
  own. Workspace `jsonwebtoken` moved 10 → 11, which upstream requires.

- The seven key-injection unit tests are replaced by wiremock-backed tests that
  assert through `OAuthResourceMiddleware` — the path a real request takes —
  and each negative case pins its rejection reason rather than asserting a bare
  `is_err()`.

### Documentation (2026-07-30, middleware error mapping, and a manual E2E matrix)

- **ADR-012's error table documented three arms that cannot execute.**
  `InvalidRequest`, `Internal` and `Custom` map to `-32600`/`-32603`, and the
  mapping builds every error through `JsonRpcErrorObject::server_error`, which
  asserts `-32099..=-32000`. Both codes fall outside it, so those three variants
  panic rather than returning. Verified by calling `server_error` with each of
  the five codes the mapping produces: `-32001`, `-32005`, `-32003` construct;
  `-32600` and `-32603` panic at `turul-rpc-core-0.2.3/src/error.rs:96`. Recorded
  in §Error Mapping with an owner and a removal trigger, and in the revision log.
  The defect itself is untouched — it needs a code change here or in the sibling
  `turul-rpc`, and this slice is documentation-only. It had been noted in
  ADR-027 but nowhere near the mapping it describes.

- **The `-32002` → `-32005` correction and the `HttpChallenge` variant were
  already in ADR-012** (landed in `78953a5`); no further change was needed there,
  and the section is consistent with `docs/compliance/base-protocol.md` §11 and
  §5. ADR-027's account of it was still written in the present tense as a live
  MUST NOT violation; both halves are closed, so it now reads as history and
  points at the compliance row for current state.

- **The shipped skills plugin still taught `-32002`.** Four sites across
  `middleware-patterns/SKILL.md` and its `middleware-error-guide.md` gave
  `Unauthorized` as `-32002` — the code 2026-07-28 reassigns to
  resource-not-found and forbids this version's implementations from emitting.
  Corrected to `-32005`. The same tables listed `Custom` as producing a "custom"
  wire code when it maps to `-32603` and discards the `code` string, and gave no
  hint that three of the six variants panic; both are now stated, since this is
  the document a user writing middleware reads.

- **New `docs/manual-e2e-matrix.md`** — runnable client × server combinations per
  lane and across lanes, plus interop, Lambda, curl and cleanup. Leads with the
  feature mutex, because putting a 2025-lane and a 2026-lane package in one
  `cargo` invocation fails and reads like a broken tree. Distinguishes the
  commands whose output was transcribed from real runs from those constructed
  from the repo but not executed, and carries a table of expected-noisy results
  so a known SKIP is not mistaken for a regression. Cross-linked from the README
  and from the existing manual-verification checklist.

### Fixed (2026-07-31, every failing leg in verify_client_examples.sh was silent)

- **Five legs discarded their own failure reason.** Each ran
  `cmd > log 2>&1` then `STATUS=$?`, but the file sets `set -e`, so a failing
  command ended the script *before* the assignment. The `FAILED:` branch and the
  log tail it was about to print were unreachable — the run exited non-zero with
  no indication of which leg broke or why. That is what made the earlier
  progress-token revert-and-fail take two attempts to read.

  All five now run through one `capture()` helper, which invokes the command
  inside an `if` — a position where the shell already tolerates failure — and
  records the status in `CAPTURED_EXIT`. The `wait $CLIENT_PID` leg uses the
  inline `if` form instead, since `wait` is a shell builtin operating on the
  current shell's jobs and does not survive dispatch through a function. The leg
  added on 2026-07-30 already had its own inline `if`; it now shares the helper,
  so there is one definition rather than six variations.

  Verified in both directions: 6/6 green unchanged, and pointing one leg at a
  closed port produces `FAILED: streamable-http-client did not complete a
  2026-07-28 round trip` **followed by the captured output**, where previously
  the script vanished at the command and printed neither.

  Note this is a shell-semantics fix, not an environment one: `set -e` is bash's
  `errexit`, evaluated by the interpreting shell, and is unaffected by which sudo
  implementation is installed. The `capture` form is correct regardless of
  `errexit`, shell version, or launcher.

### Fixed (2026-07-30, the 2025-lane test harness lost port races)

- **`TestServerManager` handed out ephemeral ports it no longer held.**
  `find_available_port()` bound `127.0.0.1:0`, read the number, dropped the
  listener, and returned. Between that drop and the child's own bind the port
  belonged to nobody — and tests within one binary run on parallel threads, so two
  could be handed the same number. The loser died on bind, and after 50 probes over
  ~15s surfaced as `Failed to start test server <name>`.

  This is the same defect class fixed for the 2026 wire tests, where
  `common::reserve_port()` holds a process-wide mutex across the handoff. The 2025
  harness never got that treatment. It now does: allocation and `spawn()` happen
  under a `PORT_HANDOFF` mutex, narrowing the window to the child's own bind. The
  `cargo build` step stays outside the lock — slow, and needs no exclusion.

- **The failure message could not distinguish the two causes.**
  `Failed to start test server <name>` said nothing about whether the child had
  died (lost port) or was merely slow, which is what made this expensive to
  diagnose. It now calls `try_wait()` and reports either "exited before becoming
  ready on port N (status …) — most likely lost the port" or "still running but
  never answered on port N after N probes over ~15s".

- `start()` also retries the whole allocate-and-spawn cycle up to three times. The
  window is narrowed, not eliminated — the real bind happens in the child, not
  under our lock — so a collision remains possible and is worth another roll.

  **Evidence, and its limit:** the prompts suite went 5/5 green and two further
  full-gate runs were clean (73 PASS, 0 FAIL each). That is not proof the flake is
  gone — the original failure appeared once in roughly four gate runs, so this
  sample cannot distinguish "fixed" from "got lucky". What is verifiable is that
  the mechanism is closed by construction, and that if it does recur the message
  now names which cause.

### Removed (2026-07-30, the two progress APIs that could not be used compliantly)

- **`SessionContext::notify_progress` and `notify_progress_with_total` are gone.**
  Both took a caller-chosen `progress_token`, and the spec leaves no compliant use
  for that: `ProgressNotificationParams.progressToken` is required and is defined
  as the token given in the initial request, and the progress pattern states as a
  MUST that notifications only reference tokens provided in an active request.
  Any token a tool invents violates it unless it coincidentally matches. This was
  not a weaker convenience API — it was a signature that produced MUST violations
  by construction, and every progress defect fixed this session came through it.
  `notify_request_progress` / `notify_request_progress_with_message` remain; they
  read the request's token and return `false` when none was declared, which is the
  correct outcome since the receiver "is not obligated to provide these
  notifications".

  Removed outright rather than deprecated: 0.4.0 is unpublished so nothing is
  owed a migration window, and `clippy -D warnings` would have failed the build on
  a `#[deprecated]` anyway, making deprecate-then-remove two steps for one result.

  The 14 call sites were all internal unit tests of the removed functions. Each
  now seeds `extensions["mcp:progressToken"]` — exactly what the `tools/call` and
  `resources/read` handlers inject on the real path — and asserts
  `notify_request_progress` returned `true`, so they verify delivery rather than
  merely not panicking. The "different progress tokens" case became one fixed
  token with varying progress values, which is what the wire contract actually
  describes; the over-100 case keeps its assertion, since the schema says progress
  "should increase every time progress is made, even if the total is unknown".

### Fixed (2026-07-30, the last invented progress tokens, and a guard for the example)

What the spec actually requires, since every progress defect this session traced
back to it. `ProgressNotificationParams.progressToken` is **required**, and the
schema defines it as "the progress token which was given in the initial request,
used to associate this notification with the request that is proceeding"
(`schema/schema.ts:1009-1013`). The spec's progress pattern states it as a MUST:
"Progress notifications MUST only reference tokens that: Were provided in an
active request; Are associated with an in-progress operation." Opt-in is the
caller's, and the receiver "is not obligated to provide these notifications"
(`schema.ts:65`), so sending nothing when no token was supplied is explicitly
allowed. There is therefore **no compliant call** to
`notify_progress(arbitrary_string, ..)` unless the string happens to equal the
request's token.

- **11 test fixtures across 6 files stopped inventing tokens** —
  `tests/{derive_examples, session_context_macro_tests, server_examples,
  http_server_examples, lambda_examples, lambda_streaming_real}.rs`. Each now
  calls `notify_request_progress(progress, total)` with a `total` the loop
  actually knows, or `None` for the open-ended counters. `examples/` and
  `tests/` are now both free of invented tokens.

- **The example's progress behaviour is now gated.**
  `crates/turul-mcp-server/tests/progress_token_match_2025_11_25.rs` pins the
  framework contract with a purpose-built tool, so no *example* was covered —
  `echo_sse` could regress silently. Two links of the chain were broken: nothing
  gated ran the 2025 client against the 2025 server, and the client **detected**
  a token mismatch and still exited 0, reporting a compliance failure as a pass.
  The client now errors and exits non-zero, and
  `verify_client_examples.sh::test_progress_token_echo` drives it against
  `client-initialise-server` inside `gate_examples` (that file's total went 5 → 6).
  Revert-and-fail: restoring `notify_progress("echo_processing", ..)` in
  `echo_sse` fails the leg with `progress notifications referenced
  ["echo_processing", "streamable-demo-1"] instead of the request's token`.

  That leg also captures its exit status in a condition rather than the bare
  `cmd; TEST_EXIT=$?` the file's other legs use — under `set -e` a failing
  command aborts the script before the assignment, so their `FAILED` reporting
  branches are unreachable and the reason is lost. Fixed for the new leg only;
  the pre-existing ones are noted, not swept.

### Fixed (2026-07-30, three middleware error variants aborted the request)

- **`InvalidRequest`, `Internal` and `Custom` panicked instead of answering.**
  All six variants were built through `JsonRpcErrorObject::server_error`, which
  asserts the code lies in the implementation-defined `-32099..=-32000`. `-32600`
  and `-32603` are standard JSON-RPC codes outside that range, so any middleware
  returning one of those three variants aborted the request. No `turul-rpc`
  change was needed — `server_error` was simply the wrong constructor;
  `invalid_request` and `internal_error` already exist and carry no range assert.
  `InvalidRequest` now answers `-32600` with the message in `data.reason`;
  `Internal` and `Custom` answer `-32603`.

  `Custom`'s application-level `code` string does not reach the wire. The enum
  docs claimed it became the JSON-RPC code, which was never true — there is no
  number to send, and inventing one would land in a range the spec governs.

- **The mapping had two verbatim copies and now has one owner.**
  `map_middleware_error_to_jsonrpc` was duplicated in `session_handler.rs` and
  `streamable_http.rs`, so the code a client received could drift by which
  transport served the request. It now lives in `middleware/error.rs` beside the
  enum and the codes it maps to; both transports call it.

  Guarded by `every_returnable_variant_maps_to_a_response_without_panicking`
  (all six variants, code asserted per variant) and
  `rate_limit_carries_retry_after_but_only_when_given`. Revert-and-fail:
  restoring `server_error` for `InvalidRequest` panics the first test at
  `turul-rpc-core-0.2.3/src/error.rs:96`. The pre-existing middleware test
  asserted only that the stack returned `Unauthenticated` and *commented* that
  the handler "would map this to -32001" — it never called the mapping, which is
  why three panicking arms survived. ADR-012 §Error Mapping and its revision log
  updated in the same slice.

- **`scripts/check-protocol-purity.sh` was run by nothing, and checked the wrong
  crates.** No gate, workflow or doc referenced it, and its list covered
  `turul-mcp-protocol` and `turul-mcp-protocol-2025-06-18` — not
  `turul-mcp-protocol-2025-11-25` and not `turul-mcp-protocol-2026-07-28`, the
  crate this branch exists to build. Both added, and the script is now a
  `gate_default` step. It passes.

  It warned on `traits.rs` in the 2026 crate, which was first read as an
  ADR-level purity question. It was not. The checker greps `^//.*Framework`, and
  the 2026 crate's module doc opened `//! Framework traits for JSON-RPC types`
  where 0.3's reads `//! Traits for JSON-RPC types as per MCP specification`.
  The warning was the word.

  The split CLAUDE.md prescribes has been in place since 2025-06-18 and is
  intact: `traits.rs` carries ~80 protocol traits in every generation
  (`HasMethod`, `HasParams`, `HasMeta` — the schema's `extends` relationships,
  which Rust cannot express directly), while the authoring traits a tool
  implements (`HasInputSchema`, `HasExecution`, `HasIcons`) live in
  `turul-mcp-builders/src/traits/`. No trait name is defined in both. Only the
  2026 crate's label had drifted, calling protocol traits "framework" ones —
  corrected to match 0.3's wording, with a note on why the traits exist. A stray
  "Framework trait impls" in `json_rpc.rs` fixed likewise, along with the
  "predates this slice" dev-log narration on the same comment. The check now
  passes with no warnings, by the label being accurate rather than the check
  being suppressed.

- **`scripts/quick_test_middleware.sh` deleted.** 47 lines of `echo` printing
  manual instructions, asserting nothing, referenced nowhere. Its content is in
  `docs/manual-e2e-matrix.md` §1 A5, which also states the HTTP-layering contract
  it did not mention.

### Fixed (2026-07-30, the E2E harness launched a binary it had not built)

- **`TestServerManager` rebuilt into `CARGO_TARGET_DIR` and then spawned from a
  hardcoded `<root>/target/debug`.** `cargo build` honours `CARGO_TARGET_DIR`;
  the spawn path did not, so with a custom target directory the rebuild landed in
  one place and the test launched whatever stale artifact sat in the other —
  including one built for the opposite spec lane. The comment above the rebuild
  said it existed "so the binary matches the fixture's current spec pin, not a
  stale artifact from a prior build", which is precisely what the path defeated.
  Both spawn sites now share one `debug_binary_path()` helper honouring
  `CARGO_TARGET_DIR` / `CARGO_BUILD_TARGET_DIR`.

  Found by editing `tools-test-server`, rebuilding, watching the suite pass, and
  noticing the wire frames were the *old* shape: `"progress":0,25,50,75,100` with
  no `total`, where the new code emits f64 progress and `total`. The suite was
  reporting on a binary that predated the edit.

  Not a CI false-pass: neither `ci-gates.sh` nor the workflows set
  `CARGO_TARGET_DIR`, so there the two paths coincided. It hits anyone using a
  per-lane target directory — which is the workflow `docs/manual-e2e-matrix.md`
  recommends, so the doc was steering readers into it. Noted there.

- **`test_progress_tracker_with_notifications` required non-compliant behaviour.**
  It called `progress_tracker` with no `_meta.progressToken` and then asserted
  progress notifications MUST arrive — i.e. it required the server to invent a
  token, since with no token there is nothing to reference and a compliant tool
  sends nothing. Once the harness ran the real binary, it failed. The request now
  declares a token and every notification is asserted to carry *that* token
  rather than merely to contain a `progressToken` field. A second assertion in
  the same test checked only that the result's `progress_token` key existed —
  true whether the tool echoed the caller's token, invented one, or reported
  nothing — and now asserts the empty string for the no-token call.

### Fixed (2026-07-30, `echo_sse` emitted a progress token no client could match)

- **`client-initialise-server`'s `echo_sse` invented its own progress token.**
  It called `notify_progress("echo_processing", …)`, so the 2025-lane client
  reported `Server did NOT echo progressToken 'streamable-demo-1' — saw
  ["echo_processing", "echo_processing"]`. 2025-11-25 requires a progress
  notification to reference the token from the originating request; an arbitrary
  string is noise the client cannot correlate. The framework gained the
  correlation API in the earlier progress-token slice, but this example was never
  migrated onto it, so the flagship 2025-lane demo still taught the wrong shape.

  Now uses `notify_request_progress()`, which reads the caller's
  `_meta.progressToken`. It returns `false` when the caller declared none — that
  means progress was never opted into, so the example sends nothing and logs why,
  rather than substituting a token of its own. Verified live end to end: both
  notifications now carry `token: Some("streamable-demo-1")` and the client
  prints `✅ Server echoed our progressToken 'streamable-demo-1'`.

  Coverage note: the framework contract is pinned by
  `crates/turul-mcp-server/tests/progress_token_match_2025_11_25.rs`, which uses
  its own tool. The *example* is covered only by the manual run in
  `docs/manual-e2e-matrix.md` §2 B1 — `scripts/verify_client_examples.sh` starts
  this server but never inspects progress, and is not in `ci-gates.sh` anyway.

- **The same defect in three more examples, each needing a different answer.**
  - `zero-config-getting-started` passed `self.message` — the human-readable
    text — as the correlation ID. Now `notify_request_progress_with_message`,
    which puts the text in `message` where it belongs and takes the token from
    the request.
  - `tools-test-server`'s `progress_tracker` minted a fresh `Uuid::now_v7()`,
    which reads as deliberate and is still uncorrelatable. Now uses the caller's
    token, and echoes it in `ProgressResult.progress_token` so a client can tie
    the notifications and the response to one request.
  - `stateful-server`'s four cart sites emitted `notifications/progress` with
    `cart_item_{n}`/`cart_clear` and `progress: 1`. These are not progress at
    all: a completed cart mutation is a state-change event, and progress is for
    tracking a long-running request. Converted to `notify_log`, which needs no
    token and says what actually happened.

  Also removed the now-unused `uuid::Uuid` import, and noted at the `as_str()`
  call that the frozen 2025-11-25 `ProgressToken` is a `String` newtype returning
  `&str`, not the string-or-number enum the 2026 binding uses — the two lanes need
  different code here.

### Fixed (2026-07-30, two orphan autobins the guard could not see, and dead dependency pins)

- **`tests/reachability_guard.rs` parsed `[[bin]]` blocks only, so it could not
  see an autobin.** `tests/prompts/src/main.rs` and
  `tests/resources/src/main.rs` are binary targets by virtue of their path —
  nothing in either manifest names them — so the guard added to catch
  never-launched test-crate binaries walked straight past two of them. Both
  duplicated an `examples/` server the harness actually spawns
  (`prompts-test-server`, `resource-test-server`, which are 947 and 1701 lines
  against the orphans' 240 and 168), and `TestServerManager` names neither.
  Deleted, and `nested_crate_bins()` now covers the autobin route as well as
  `[[bin]]`, honouring `autobins = false`. Revert-and-fail: reinstating one
  `src/main.rs` fails `every_nested_test_crate_bin_is_launched_by_the_harness`
  with `tests/prompts → mcp-prompts-tests (src/main.rs autobin)`. The failure
  message now names which of the two routes declared the target, since "go find
  the `[[bin]]`" is unactionable advice for a binary no manifest mentions.

- **The six deleted `bin/main.rs` targets left their dependencies behind.**
  `tests/{elicitation,prompts,resources,roots,sampling,tools}` between them
  declared 36 dependency entries across 16 distinct crates that no remaining
  source referenced —
  `clap`, `uuid`, `reqwest`, `hyper`, `hyper-util`, `http-body-util`, `tower`,
  `futures`, `tokio-stream`, `tempfile`, `chrono`, `base64`, `serde_yml`,
  `anyhow`, `async-trait`, `schemars`. Removed and verified by building and
  running all six suites (221 tests pass), not by grep: a grep-only pass had
  also flagged `serde`, which is required through the derive expansion and
  produced `E0463: can't find crate for serde` the moment it was dropped.

- **Six `[workspace.dependencies]` pins had no consumer at all.**
  `tokio-tungstenite`, `criterion`, `lambda-web`, `pin-project`, `serde_yaml`
  and `indicatif` were referenced by no crate manifest and no source file.
  Pre-existing rather than a consequence of the deletions above, and removed in
  the same pass because `serde_yaml` — abandoned upstream — sat in the pin table
  directly alongside the `serde_yml` fork that three crates do use, which reads
  as a choice rather than as debris.

- **`interop-client-probe` reported "nothing to test" as `FAIL` on two legs and
  `SKIP` on a third.** A peer exposing no resource and no prompt got
  `LEG resources/read FAIL` and `LEG prompts/get FAIL`, while the adjacent
  `completion/complete` leg called the identical condition `SKIP`. Since the
  probe exists to be pointed at servers this project did not write, and peers
  legitimately differ in what they expose, `FAIL` there reads as a defect in a
  peer that has simply not implemented that surface. All three now report
  `SKIP`, the three outcomes are defined in the module docs, and a skipped
  `tools/call` says why it still fails the run: the core claim is unproven, and
  `CORE ok` off an unexercised core would be a false pass.

### Fixed (2026-07-30, verification scripts and the forbidden `-32002`)

- **The legacy `≤2024-11-05` handler emitted `-32002`.**
  `session_handler.rs` returned the literal `-32002` for a missing
  `Mcp-Session-Id` on its GET SSE path. That file carries no `cfg` gate and is
  reachable through protocol-version routing, so the code applied on the
  2026-07-28 lane, which forbids implementations of this version from emitting
  it. Now `UNAUTHENTICATED` (`-32001`) — what `streamable_http.rs` already
  returned for the same condition. The existing per-constant guard could not
  catch it because the site bypassed `error_codes`, so
  `no_source_file_emits_the_forbidden_resource_not_found_code` now scans the
  crate's sources for the literal. Revert-and-fail: restoring `-32002` fails
  that guard, naming `session_handler.rs:896`. This also corrects a claim in
  `docs/compliance/base-protocol.md`, which had asserted no framework path
  emitted `-32002` on the 2026 lane.

- **`e2e-lambda-client-local.sh`: the 2025-11-25 client leg is now an explicit
  SKIP, and the 2026-07-28 legs were made deterministic.** `cargo lambda watch`
  builds the function lazily on first invoke while the readiness probe returned
  on its first success, so the probe could pass against a process the watcher
  then replaced — surfacing as `hyper::Error(IncompleteMessage)`, which names
  nothing about the cause. Both lanes are now pre-built into per-lane target
  dirs (they build the same binary name with mutually exclusive features and
  must not share one) and readiness needs three consecutive probes. A populated
  target dir hid all of this; `cargo clean` exposed it.
  The 2025-11-25 client-over-Lambda leg is not exercisable against this fixture
  at all: the emulator serves invocations serially with one instance, and on
  that lane the client holds a long-lived GET SSE stream open, so the stream and
  the following POST race for the instance — passing when idle, failing under
  load. Real Lambda scales out, so this does not describe production. The leg
  now prints SKIP with that reason instead of being a coin flip. Ruled out by
  experiment: connection reuse is fine (two requests on one connection report
  `Reusing existing http: connection` / `left intact`).

- **19 `verify_*.sh` / `test_*.sh` scripts audited, each one actually run.**
  8 deleted as superseded, orphaned, or assertion-free (`verify_example.sh`
  extracted a session id by grepping a body fetched without `-i`, which never
  worked; `test_rate_limit_debug.sh` asserted nothing; `verify_meta_examples.sh`
  targeted two examples that no longer exist). 11 ported to the 2026-07-28
  stateless wire via a new `scripts/lib/mcp2026.sh` helper and placed behind
  `gate_examples`, so none can rot unnoticed again. Several had been counting
  failure as success. (The squashed commit message for this batch says "10
  scripts deleted"; the true figure is 8 — `git diff --diff-filter=D` over
  `scripts/` is the authority.)


### Fixed (2026-07-30, sampling provider selection was HashMap-order roulette)

- **`ProvidedSamplingHandler` picked a sampling provider via `HashMap::values().next()`,**
  so with more than one provider registered, which one answered `sampling/createMessage`
  varied between process starts (`examples/sampling-server` registers three; across five
  restarts with identical input the creative sampler answered three times and the technical
  sampler twice). `SamplingProvider::can_handle` and `::priority` existed for exactly this
  and were called from nowhere. Dispatch now tries providers in `priority()` descending
  order, first `can_handle()` match wins, and equal priority breaks by registration
  order — carried by the provider `Vec` itself, never by `HashMap` iteration order. Verified across
  10 separate process invocations of a dedicated regression test with three equal-priority
  providers: same provider answered every time (was flaky ~60% of the time pre-fix across
  10 runs of the same test against the reverted code).
- **`examples/sampling-server`'s three samplers now claim requests via `modelPreferences.hints`**
  (`can_handle` matches the hinted model name against its own id; no hints means any may
  answer). This is what let `test_sampling_different_models` be corrected to assert which
  provider actually answered instead of only that the response text was non-empty and over
  20 characters long — an assertion that would have passed with a single hardcoded provider
  or fully random selection.

### Fixed (2026-07-29, client could not list tools from our own server)

- **`list_tools()` failed outright on a JSON Schema 2020-12 composition.** The client's
  public vocabulary is the frozen `turul-mcp-protocol-2025-11-25` types, whose `JsonSchema`
  is an internally-tagged enum on `"type"` with no fallback arm. A property written
  `{"oneOf": […]}` — legal on 2026 `inputSchema` per SEP-2106, and exactly what this
  framework's own server emits for a `#[serde(tag = "kind")]` tagged union — had no
  representation, and because the parser collected into a `Result`, **one** such tool errored
  the **entire** listing. Turul-client could not list tools from turul-server on the
  revision's headline schema change. The frozen crate cannot be widened, so the conversion is
  now infallible by construction: direct remap first, then a field-by-field rebuild that
  drops only the individual parts that cannot cross, each named in a `tracing::warn!`. The
  cost is stated rather than hidden — `Tool.input_schema` may be missing properties while
  `required` still names them — and it is recoverable: `McpClient::tool_input_schema(name)`
  returns the untruncated advertised schema, and the caveat is documented on `list_tools`,
  `refresh_tools` and `list_tools_paginated`. Dropping a valid, callable tool silently is the
  worse failure, because the caller cannot tell it happened. Exclusion stays reserved for
  definitions a client MUST NOT act on: an `x-mcp-header` placement violation (SEP-2243) or
  a dialect-invalid schema.
- **The client opened a GET SSE stream the revision deleted.** `connect()` spawned the
  listener before negotiation resolved, so every 2026 connection issued a GET, took HTTP 405,
  and logged two warnings naming `initialize` and session ids — concepts 2026-07-28 removed.
  The listener now starts after `negotiate_protocol()` and returns early on 2026-07-28, and
  the transport refuses independently. Deferring it also closed a **pre-existing 2025-lane
  race** the old ordering only compensated for with a compare-and-swap: reverting the fix
  fails in both directions, the 2026 connection issuing a GET and the 2025 GET going out
  without the session id the handshake had just produced.
- **`resources/read` mislabelled every text body `text/plain`.** `ResourceContent::text()`
  hardcodes that type and offered no way to set another, so a resource advertising
  `text/markdown` in `resources/list` reported `text/plain` on read — two contradictory
  answers about one property of one resource, with no way for a client to know which is
  authoritative. `ResourceContent::with_mime_type` (a builder method on a concrete spec type,
  implementing the schema's existing optional field) closes it, and the interop fixture server
  now drives both sides from one constant instead of documenting the mismatch as a known
  discrepancy a probe author was expected to work around.
- **Auth challenges were never checked for `Cache-Control: no-store`.** ADR-021 claimed it;
  no test looked, and the compliance register carried it as "Unknown — not located in code".
  The header was in fact always emitted by the shared challenge builder. Now asserted on the
  three statuses that builder reaches on the 2026 path (401 missing bearer, 401
  `invalid_token`, 400 `invalid_request`).
- **`turul-mcp-ext-apps` vendored the wrong spec artifact.** It shipped a byte-exact copy of
  upstream `specification/draft/apps.mdx` while declaring "Apps protocol version 2026-01-26"
  — misvendoring proven by re-fetching that path at that commit and hashing it, not inferred
  from a size difference. Replaced with `specification/2026-01-26/apps.mdx` at the commit that
  created it and which upstream has never touched since, so the pin is immutable by
  construction. The Rust types were correct at both commits; one **doc claim** was not —
  `UiResourceMeta` asserted a "hosts MUST check both locations" precedence rule that exists
  only in the draft's Metadata Location section, which the released spec does not have. The
  comment was importing draft normativity into a released binding.

### Removed (2026-07-29)

- **Phantom `stdio` and `all-transports` Cargo features on `turul-mcp-client`.** Both were
  declared in the manifest with no stdio module behind them; enabling either compiled and
  provided nothing. `detect_transport_type` already returned `Unsupported` for `stdio://`, so
  nothing regresses — the crate simply stops advertising a transport it does not have.

### Changed (2026-07-29, error-code policy: keep the legacy codes, name the deviation)

- **The error-code guard was asserting the opposite of the spec.** 2026-07-28 partitions the
  JSON-RPC server-error range: `-32020..-32099` is spec-reserved, and `-32000..-32019` is
  **legacy** — "New codes MUST NOT be allocated in this sub-range, and new implementations
  SHOULD NOT use codes from this sub-range at all." The existing test asserted every
  framework code *was inside* `-32000..-32019`, which would have passed a brand-new
  allocation there — the exact thing the MUST forbids. It is replaced by a frozen
  `LEGACY_ALLOCATIONS` set plus "outside the reserved range", so a new code in the legacy
  sub-range now fails. A matching guard freezes the three middleware codes.
- **The 14 pre-policy codes are retained, and that is recorded as a deviation rather than
  quietly kept.** Relocating them is *blocked*, not deferred: both
  `map_middleware_error_to_jsonrpc` sites build their object through
  `JsonRpcErrorObject::server_error`, whose `assert!` panics for any code outside
  `-32099..=-32000` — so the spec's recommended destination is unreachable through that
  constructor. Reproduced, not assumed: setting a code to `-33014` panics inside
  `turul-rpc-core 0.2.3`.
- **`-32002` is a live MUST NOT violation and is now registered as one.** The spec forbids
  implementations of this version from emitting it; `error_codes::UNAUTHORIZED` is `-32002`
  and maps `MiddlewareError::Unauthorized`. `turul-mcp-client::is_resource_not_found` matches
  `-32602 | -32002`, so turul-on-turul reports a permission denial as a missing resource. The
  earlier framing of this area as "`-32001`/`-32003` were vacated" was wrong on the facts:
  those renumbered to `-32020`/`-32021`, and `-32002` is the code the spec singles out.

### Added (2026-07-29, coverage for requirements that had none)

- **State Handle Hijacking audited for the first time.** The 2026-07-28 security page replaced
  2025-11-25's Session Hijacking section, and the original compliance sweep had no row for it.
  Three turul-issued identifiers were assessed. The Tasks extension's `taskId` is the only one
  that is a state handle in the spec's sense: unguessable v4 ids satisfy the RNG SHOULD, but
  there is **no owner binding at all** — every `TaskStore` method keys on `task_id` alone,
  `TaskState` has no owner field, and `tasks/get`/`update`/`cancel` implement a handler
  signature carrying no session or auth context. A turul server with OAuth wired *cannot*
  satisfy "MUST NOT treat possession of a state handle as authentication" even if the operator
  wants to. `subscriptions/listen`'s subscription id is argued not-applicable (it is the
  client's own request id, emit-only, never an inbound lookup key); MRTR's `requestState` is
  Unknown at framework level, because the framework is a pure conduit and nothing documents
  that binding is the tool author's obligation.
- **Eight wire tests for requirements that were previously structural, inferred, or covered
  only by an interop probe**: batch-array rejection; the handler-level HTTP 200 error path
  asserted against the 404 branch on the same server; `tools/call` domain failure returning
  `isError: true` with no `error` member; cursor walks for `resources/list`,
  `resources/templates/list` and `prompts/list` (each proving the walk reproduces the
  unpaginated listing *and* takes exactly `len()` pages, so a server ignoring `limit` fails);
  invalid-cursor `-32602` on all three; the `.well-known` Origin exemption asserted alongside
  the MCP endpoint still answering 403 to the same hostile header; a default build advertising
  no `extensions` and answering 404 for `tasks/*`; and prompt argument substitution on the
  2026 lane, which until now had no evidence outside two interop probes.
- **`completion/complete` and `notifications/cancelled` are reachable from `McpClient`.**
  `complete()` routes bilingually, building its result field by field because `total` is `f64`
  on the 2026 wire and `u32` in the public vocabulary — an integral `100.0` would not survive
  a serde round trip. `cancel_request()` is spec-neutral. `SubscriptionStream::request_id()`
  was added so a caller has at least one request id it can legitimately name; without it
  `cancel_request` would be decorative. This also makes the `completion/complete` interop cell
  fillable from our side for the first time — previously recorded UNSUPPORTED because no
  client method existed.
- **`scripts/check-schema-pin.sh` now covers `turul-mcp-ext-apps`** and rejects any `*.mdx`
  provenance row whose upstream source is not a dated `specification/<YYYY-MM-DD>/` path. The
  revert-and-fail run included a **well-formed** table honestly pointing at
  `specification/draft/` — the gate rejects it, which is the point: it catches the defect
  class, not the one instance. `ext-tasks` gets the checksum arm only, because upstream
  `modelcontextprotocol/ext-tasks` has no tags and only `schema/draft/`, so the dated-path
  rule genuinely cannot apply there.

### Fixed (2026-07-29, test-suite hygiene)

- **A port-binding race across 30 reservation sites in 21 server suites.** `build()` does not
  bind; the real bind happens later in `run()`, leaving the reserved port free in between and
  two tests able to claim it. A binary-wide mutex now spans the whole reserve→bind window.
  This is a workaround at the test layer: the window closes properly only with a
  `McpServerBuilder::listener(TcpListener)` API letting a test bind once and hand the live
  listener over, which is a framework change and is not made here.
- **Internal gap-register identifiers removed from test names and files.** Three test files
  carrying `bp3` / `gap_cf9` tracking IDs are renamed after their subject, 25 `verify_rN_*`
  function names are de-tagged, and the dev-log narration in their module docs is replaced by
  the spec requirement each asserts. Those IDs are how a fix is *tracked*, not what the code
  *is*; they belong in the compliance matrix, not in source. `docs/plans/2026-07-28-spec-compliance.md`
  still cites the three old filenames in its evidence cells and needs the same pass.

### Fixed (2026-07-29, found by cross-implementation interop)

- **`resources/templates/list` answered `-32601` when no templates were registered.** The
  handler was registered only if template resources existed, so a server that declares the
  resources capability told clients the method does not exist rather than that there are
  none — a different claim, and the one a capability-driven client acts on by abandoning
  templates entirely. Registration is now unconditional on both the local and Lambda
  builders, matching `resources/list` and `resources/read`; `build()` still swaps in the
  populated handler when templates were configured. This changes the 2025-11-25 lane too,
  deliberately: the method is standard on that spec as well and gating it would mean two
  code paths for one contract. Default handler counts move to 22 (2025-11-25) and 13
  (2026-07-28).

### Fixed (2026-07-29, false claims in shipped crate artifacts)

- **`turul-mcp-protocol-2026-07-28`'s own docs asserted things untrue of the crate**, and
  `cargo package --list` confirms they ship in the tarball. The README carried a "Re-pin
  outstanding" banner claiming the vendored schema came from `schema/draft/` — it does not;
  the bytes hash to the released `schema/2026-07-28/schema.ts` at the pinned commit, and the
  banner outlived the re-pin. `COMPLIANCE.md` stated 8 `@see` block-tags where the schema
  carries 13, marked five JSON-RPC anchors as mirrored when none are, and described the
  fixture tree as 86 directories / 124 files while `coverage.rs` asserts 88 in the same crate.
  `Cargo.toml` named the pre-release fixture path.
- **Three rustdoc links pointed at a fragment that does not resolve.** `basic/index#meta` is
  `#_meta` on the live page. Upstream's own `@see` writes `#meta`, so the mirroring rule was
  faithfully reproducing an upstream typo into broken docs.rs links; the mirrors now use the
  working anchor, with a note so a re-pin does not revert them.
- **Stale pre-renumbering error codes in 2026 contexts.** `examples/header-bound-tools-server`
  is on the 2026 default lane and documented its header-mismatch contract as `-32001` across
  `main.rs`, its README and `EXAMPLES.md`; that value is now the framework's own
  `UNAUTHENTICATED` constant, so the example described one contract using a code meaning
  something else. `ci-gates.sh` printed `-32001/-32004` and `-32003` as gate labels while the
  tests those gates run assert `-32020` and `-32021`.
- **New `tests/docs_consistency.rs`**, wired into the default gate, recomputes each figure
  stated in prose from the artifact it describes, so owner and documentation cannot drift
  apart silently again.

### Added (2026-07-29, test and interop surfaces)

- **A dedicated streaming end-to-end suite** (`streaming_e2e_2026.rs`). The existing 2026
  streaming tests read `data:` lines leniently and assert on the JSON inside, leaving the
  framing itself unasserted — a server could emit malformed field lines, drop the blank-line
  terminator, or cut mid-frame and every one of them would still pass. The new suite asserts
  the bytes: event-stream grammar, unbuffered response headers, frame ordering, the result
  frame terminating the stream, and the JSON counterpart staying distinguishable on the wire.
- **A Lambda end-to-end gate driven by `cargo lambda watch`** (`scripts/e2e-lambda-local.sh`).
  The in-process Lambda tests construct a handler and call it directly, skipping the AWS
  Runtime API entirely. This drives the real control-plane emulator, so all 10 assertions are
  on bytes that crossed a Function URL request/response cycle.
- **A shared interop fixture server** (`examples/interop-fixture-server`) exposing tools,
  resources, prompts and completion, so every peer probe hits one surface. Probes previously
  ran against `minimal-server`, whose single tool capped interop at 3 of 22 methods.
- **An interop client probe** (`examples/interop-client-probe`) that drives a *foreign* server
  with `turul-mcp-client` and reports per-leg results without aborting.
- **Four interop probes, with measured results.** FastMCP 4.0.0b1: 9 methods and 5 negative
  paths (`interop-fastmcp.sh`), and 8 methods driven by our client against a FastMCP server
  with an R→R control (`interop-turul-client.sh`). **MCP Go SDK v1.7.0** (`interop-go-sdk.sh`)
  and **MCP TypeScript SDK 2.0.0** (`interop-typescript-sdk.sh`): 9 methods and 5 negatives
  each, no wire disagreement from either. Both are stable releases, so their agreement is not
  qualified by "the peer may still move".
- **Corrected two published claims about the outside world, and the mechanism that produced
  them.** This entry previously recorded the TypeScript cell as failing at `connect()` and
  called the Go SDK the only peer that is not a pre-release. Both were wrong. The v2 line
  ships on npm as `@modelcontextprotocol/{core,client,server}` — `@modelcontextprotocol/sdk`,
  which the freshness watch pointed at, carries only the 1.x line — so the probe pinned a
  superseded `v2.0.0-beta.1` git tag while `2.0.0` was already published, and the watch was
  structurally incapable of noticing. Re-run against the published build, the cell passes and
  identity in `_meta` is accepted. `mcp==2.0.0` (PyPI) is a further stable peer, still
  untested. All four probes now compare their pinned peer version against the registry — npm,
  PyPI and the Go module proxy — and warn when it has fallen behind.
- **`docs/compliance/`** — per-spec-area records naming, for each requirement, the test that
  asserts it and which independent implementation has exercised it. Self-verified and
  externally verified totals are kept apart on purpose, and "not exercised" is a distinct
  value from "pass". AGENTS.md now requires these to be reconciled in the same slice as a
  schema re-pin.

### Changed (2026-07-29, adopt the released MCP 2026-07-28 spec)

- **Schema re-pinned from the pre-release draft path to the released one.** Upstream published
  2026-07-28 and moved the schema from `schema/draft/schema.ts` to the immutable dated
  `schema/2026-07-28/schema.ts`. The vendored copy is now `schema/schema.ts` (renamed from
  `draft-schema.ts`, whose prefix survived finalization and stopped describing the file), taken from commit
  `271ecc9accafdd9b83a3c869fa67c22953b2af80` (content sha256
  `742750af0bb8c716e7030c4977c992b55d1adc4407e9e66997db5846baedc2cd`, blob `9b55feeb…`);
  `PIN` in `src/compliance/fetch.rs` and `schema/EXAMPLES_PIN.md` moved with it to
  `subpath: "schema/2026-07-28/examples"`.
  **The pin is the content-bearing commit, not the release tag.** Tag `2026-07-28` is merge
  commit `5f5440bb…`; `resolve_subpath_head` filters history by subpath and never returns it,
  so pinning the tag would leave `fetch.rs` and `schema/README.md` permanently disagreeing
  with what `refresh` computes. `refresh` still probes `main`, which is now *correct* rather
  than hazardous: with the subpath naming the dated directory it detects post-release errata
  while never reaching next-cycle draft content.
- **No wire-format change.** The released schema differs from the prior pin only by TypeDoc
  `@see` anchors (`/specification/draft/…` → `/specification/2026-07-28/…`), the interface
  rename `SubscriptionsListenResultMeta` → `SubscriptionsListenResultMetaObject`, and one new
  type `SubscriptionsListenResultResponse`. Both deltas are applied; the rename was confined
  to `turul-mcp-protocol-2026-07-28` (no external consumers). Upstream fixture directories
  87 → 88; modeled cases 10/87 → 12/88, with 24/24 fixtures passing.
- **`turul-http-mcp-server` had two enums named `McpProtocolVersion`.** The one exported from
  `lib.rs` and `prelude.rs` lacked `V2026_07_28` and could not parse `"2026-07-28"` — the
  crate's own default spec version — while a second definition inside `streamable_http.rs`
  could. Collapsed to the single definition in `protocol.rs`; `LATEST` is now cfg-selected to
  the enabled lane instead of hardcoded to 2025-11-25. The inherent
  `to_string(&self) -> &'static str` was removed: it shadowed `ToString` (which returns
  `String`), so `let s: String = v.to_string()` did not compile. Use `as_str()`; `Display`
  supplies `to_string()` correctly. Guarded by `tests/public_protocol_version.rs`, one of
  whose tests compiles only if the root and prelude paths name a single type.
- **`notifications/initialized` no longer takes the synchronous lifecycle path on the 2026
  lane.** The ordering constraint exists only for the 2025-11-25 handshake; it is now
  cfg-gated. Acking an unrecognised notification with 202 is unchanged and deliberate —
  JSON-RPC notifications carry no response. `error_mapping_2026.rs` previously probed removed
  *notification* methods with an id-carrying *request* envelope and so proved nothing about
  the notification path; it now sends the real envelope.
- **Publish order re-derived from the actual dependency graph.** The documented order in
  CLAUDE.md would have failed: `turul-mcp-server` depends non-optionally on
  `turul-mcp-oauth` but was published first, and four crates added since it was written were
  missing. `tests/` is now `publish = false` — a test-only crate was publishable.

### Fixed (2026-07-29, Roots is no longer hostable on the 2026 lane)

- **`McpServerBuilder::with_roots()` registered a removed method on a 2026-07-28 build.**
  `roots/list` is not a member of the 2026 client-to-server request union — the schema defines
  `ListRootsRequest` as sent *from the server to the client*, and the release routes roots
  through an MRTR input request instead. Calling `.with_roots()` on a 2026 server nevertheless
  installed an inbound handler, so `roots/list` answered **HTTP 200** where the spec requires
  404 with `-32601` (verified on a live server before the fix). `with_roots()`, `root()` and the
  `roots` field are now `#[cfg(feature = "protocol-2025-11-25")]`, matching the treatment
  Sampling and Logging already had — the leak is now a compile error rather than a wire defect.
  `examples/roots-server` is 2025-pinned and unaffected.

  Deliberately *not* `#[deprecated]`: that would say "works, but migrate", and on 2026 these
  did not work correctly. The SEP-2577 deprecation of the Roots *feature* is expressed where it
  belongs — on the protocol types, which keep `ListRootsRequest` as a valid MRTR `InputRequest`
  variant for as long as the spec does. Removal trigger for the gated surfaces: retirement of
  the `protocol-2025-11-25` lane; removal date no later than the release adopting the first MCP
  revision on or after 2027-07-28.

### Changed (2026-07-29, response framing chosen per request)

- **A combined `Accept: application/json, text/event-stream` no longer forces SSE on every
  `tools/call`.** The spec lets the server answer a request with either a single JSON object
  or an SSE stream and requires clients to support both, so this was always a choice rather
  than a conformance issue — but the old rule chose by method name, so every simple call paid
  SSE framing on the branch client implementations exercise least. The choice is now made from
  the request: SSE when it opted into request-scoped notifications via `_meta.progressToken`
  or `_meta."io.modelcontextprotocol/logLevel"`, plain JSON otherwise. A request declaring
  neither cannot legally be sent progress or log notifications, so it loses nothing.
  Side benefit: plain JSON is the only path that can carry `-32020`/`-32021` on HTTP 400 as
  their schemas require, since chunked SSE commits `200 OK` before dispatch. `subscriptions/listen`
  is unaffected — it is served on its own path and still requires `Accept: text/event-stream`.
  **2026-07-28 lane only**: the frozen 2025-11-25 lane keeps its previous method-name heuristic,
  since its clients were built against always-SSE `tools/call` and gain nothing from the change.
  See ADR-006's 2026-07-29 revision.

### Added (2026-07-29)

- **`scripts/interop-fastmcp.sh`** — third-party interoperability probe. Drives a turul server
  with FastMCP through a logging proxy and asserts on the bytes the client actually sent:
  `server/discover` → `tools/list` → `tools/call`, every request carrying
  `MCP-Protocol-Version: 2026-07-28`, with no `initialize` and no `Mcp-Session-Id`. It is the
  only check in the repo whose client half this project did not write. Not in CI (needs network
  and a pre-release FastMCP); run it before a release.


- **`scripts/check-schema-pin.sh`** — offline gate asserting the vendored schema's checksum
  matches its provenance block, that `fetch.rs` / `schema/README.md` / `EXAMPLES_PIN.md` name
  one commit, and that the pinned subpath is the dated directory rather than the floating
  `schema/draft/`. Nothing previously recomputed the checksum, so a tampered schema or a
  half-applied re-pin stayed invisible: the compliance suite validates Rust types against
  whatever bytes are on disk. Wired into CI and `scripts/ci-gates.sh`. Verified by injecting
  three drifts (tampered schema, mismatched pin, subpath regressed to `schema/draft/`) — each
  fails the gate; the clean tree passes.
- **CI now runs the whole integration-test crate.** `tests/Cargo.toml` declares 13 `[[test]]`
  targets; CI invoked two. `example_validation` had not compiled since `origin_policy` was
  added to `ServerConfig` — a full feature slice landed with no signal there. Fixed, and all
  13 targets are invoked with a check that the CI list matches the manifest.
- **Rustdoc coverage for the 2025-11-25 lane's derive examples.** Five doctests in
  `turul-mcp-derive` are `rust,ignore` under the default lane and were compiled by no job.
  `--doc` cannot be used (doctests build the dev-dependency graph and trip the alias mutex),
  so the job runs `cargo doc` on the opt-in lane instead.

### Fixed (2026-07-29, documentation accuracy)

- **De-drafted the repo.** CLAUDE.md/AGENTS.md described a moving draft and a "release
  candidate", and directed the drift check at `schema/draft/schema.ts` — the path that is now
  the *next* spec cycle. 94 `/specification/draft/…` citation URLs repointed after verifying
  the dated sub-paths resolve. All six `.claude/agents/*.md` targeted 2025-11-25 and two
  directed edits into the frozen crate; retargeted with explicit do-not-edit guards and a
  lane banner. AGENTS.md documented `cargo build --workspace` / `cargo test --workspace`,
  both of which fail on this branch via the protocol-alias mutex. CLAUDE.md's
  §"Reviewing Agents" pointed every spawned reviewer at a `/Users/nick/…` path that does not
  exist here, so reviewers silently read no rules at all.
- **Removed a false compliance claim.** Three published crate docs stated the
  `DRAFT-2026-v1` literal was "still accepted on deserialize for back-compat";
  `src/version.rs` asserts it *fails* to parse. Two contradictory CHANGELOG entries about the
  same alias were reconciled.
- **Compliance-matrix self-accounting.** Minor 12 of the final changelog (the error-code
  allocation policy and `HeaderMismatchError`) had no row despite being implemented; the gap
  register held 78 checked entries against a claimed 73, with `UTIL/COMP-3` reused for two
  unrelated gaps (the second is now `UTIL/COMP-4` — earlier CHANGELOG references to that
  second meaning predate the rename). Stale fixture counts re-graded.
- **ADR drift.** ADR-006 and ADR-023 told implementers that 2026 core has no vehicle for
  server→client notifications and advised running in legacy 2025-11-25 mode — `subscriptions/listen`
  is core and shipped. ADR-028 cited `-32003` where the schema now exports `-32021`. ADR-030
  misquoted ADR-027's wire string. ADR-005 marked Superseded. Dated revision-log entries were
  preserved verbatim throughout.
- Two example READMEs documented `tools/call` smoke tests that fail as written (`Mcp-Name`
  must equal the invoked item's name); `scripts/README.md` taught the removed `initialize`
  handshake; `scripts/test_all_examples.sh` hard-coded an absolute path from another machine.

### Verified (2026-07-29, third-party interoperability)

- **FastMCP 4.0.0b1 completes the stateless journey against a 2026-07-28 build.** Captured at
  a logging proxy, so this is the wire exchange rather than a client's self-report:
  `server/discover` → `tools/list` → `tools/call`, every request carrying
  `MCP-Protocol-Version: 2026-07-28`, with no `initialize`, no `notifications/initialized` and
  no `Mcp-Session-Id`. This is the first external verification of the 2026 lane; every prior
  green result was this project's own code on both ends.
- **The official TypeScript SDK cannot yet reach this lane.** `@modelcontextprotocol/sdk@1.30.0`
  lists `SUPPORTED_PROTOCOL_VERSIONS` through 2025-11-25 only and receives HTTP 400 from a
  2026 server — correct behaviour for a single-spec build. It interoperates with the
  2025-11-25 opt-in lane, verified the same way.
- Known caveat: given `Accept: application/json, text/event-stream`, this server answers with
  SSE framing while FastMCP answers with plain JSON. Both are permitted by Streamable HTTP.


> **Release status.** This entry tracks the in-progress 0.4.0 cut on the
> `2026-07-28-MCP-Specification` branch (and its current sub-branch
> `feat/turul-mcp-protocol-2026-07-28`). The workspace `[workspace.package].version`
> was already bumped to `0.4.0` in commit `064733e` (with the turul-rpc isolation
> fix in `c0737fb`). **The branch has not been merged to `main` and 0.4.0 has not
> been published.** Per the branch lock, that requires explicit maintainer
> authorization. The footer compare-link will be added at the release tag.
> `main` continues to ship at the 0.3.x line (currently `0.3.47`).

### Changed (2026-07-28, re-pin to upstream `71e30695` — `clientInfo` optional, `serverInfo` moves to `_meta`)

- **Re-pinned the vendored schema AND the example fixtures to one immutable commit, `71e306956a4959c9655e5036be215d41986596e6`** (content sha256 `c56f0ad2…`, was `6e4cba2d…`). They were previously on *different* commits (`93671a3f` for `draft-schema.ts`, `60dc69e9` for the fixtures) via two independent mechanisms, so the two artifacts could disagree; `schema/README.md` now regenerates by commit SHA instead of by `main` (a mutable ref cannot be reproduced later). 46 additions / 13 deletions since the 2026-07-02 pin, all from upstream #3002 plus one docs-only ordering clarification.
- **`RequestMetaObject.clientInfo` is now optional (`Option<Implementation>`).** Upstream made the field optional and told servers not to key behavior or security decisions on it — it is self-reported and unverified. `turul-http-mcp-server` no longer rejects a request whose `_meta` omits `clientInfo` (that arm returned `-32602`/HTTP 400, so a spec-current client configured not to identify itself had **every** request refused). A *present but malformed* value is still `-32602`, now via params deserialization rather than a presence check. Audited the 2026 lane for behavioral coupling and found none: nothing in `caching`, `turul-mcp-oauth`, `turul-mcp-session-storage` or `turul-mcp-ext-tasks` reads it, and the tool fingerprint is keyed on tools. The `client_info` uses in `session.rs`/`handlers/mod.rs` are the **2025-11-25** `InitializeRequest.clientInfo` — a different field on a frozen crate, deliberately untouched.
- **`DiscoverResult.serverInfo` removed; server identity moved to `_meta.io.modelcontextprotocol/serverInfo`.** New `meta::ResultMetaObject` (+ `META_KEY_SERVER_INFO`) models the schema's `Result._meta?: ResultMetaObject`, and **all 12 result types were retyped to `Option<ResultMetaObject>`** rather than left as loose maps. `HasMeta` was split accordingly — it returns `Option<&ResultMetaObject>` for results, and `NotificationParams` moved to a new `HasNotificationMeta` returning the loose `MetaObject`, because one trait returning one type cannot model two different schema carriers (results extend `MetaObject` with `serverInfo`, notifications with `subscriptionId`). `From<MetaObject> for ResultMetaObject` lifts a loose map into the typed carrier, moving a reserved `serverInfo` entry onto the typed field so it cannot be emitted twice. A value that does not parse as an `Implementation` is **dropped**: the typed field owns the reserved key and `Serialize` never emits it from `extra`, so re-homing it there would look preserved in memory while vanishing on the wire, and emitting it as-is would put a value under a reserved key whose declared shape it does not satisfy. The drop is silent — the protocol crate takes no logging dependency. Asserted on the serialized form, not the in-memory map. `SubscriptionsListenResultMeta` re-parented to it and its hand-written `Serialize` extended to treat `serverInfo` as reserved (so a caller-populated `extra` cannot emit the key twice). The bare top-level field was **deleted, not deprecated** — 0.4.0 is unpublished, so no compatibility window is owed.
- **One owner for the stamp.** `StreamableHttpHandler::run_middleware_and_dispatch` now wraps its inner implementation and stamps `_meta.serverInfo` at that single point, covering the JSON, SSE and Lambda paths (Lambda reuses the same handler) without per-tail duplication. Scope is deliberately narrow: top-level object results of *success* responses only — never nested `_meta` (plain `MetaObject` in the schema), never error responses, never client-produced results (sampling/elicitation/roots), and a handler that already set the key keeps its value. `DiscoverHandler` therefore no longer sets it itself. Gated to the 2026 lane; `session_handler.rs` (2025) has its own separate dispatch and is unchanged.
- **Lambda transport parity for the stamp.** `turul-mcp-aws-lambda` builds its `StreamableHttpHandler` directly rather than through `HttpMcpServerBuilder`, so wiring the identity on the local builder did not reach it — Lambda would have omitted `serverInfo` from every result while the local server emitted it. Two legacy Lambda constructors already accepted an `Implementation` and discarded it as `_implementation`; they now use it, and `LambdaMcpHandler::with_server_info` covers the production `with_middleware_and_fingerprint` path, which takes no identity. Guarded by `results_carry_server_info_meta_on_the_lambda_path`. Also dropped the now-dead `DiscoverHandler.implementation` field (the transport owns the stamp), which `clippy -D warnings` caught.
- **`turul-mcp-client` reads `serverInfo` from `_meta`.** `DiscoveredServer::from_result` read the now-removed top-level field, which would have silently reported no server identity against any spec-current server. Version negotiation was never at risk — the probe classifies on raw JSON, not the typed `DiscoverResult`.

### Fixed (2026-07-28, compliance-harness defects found during the re-pin)

- **`refresh` reported `main`'s tip while labelling it "last commit touching `schema/draft/examples`".** `resolve_upstream_head_for_subpath` did `git ls-remote refs/heads/main` and never walked the subpath; the printed label was simply false, and it resolved `7d6c7b86` (a merge for an unrelated docs PR) where the true subpath head was `71e30695`. Replaced by `compliance::fetch::resolve_subpath_head`, which fetches blobless-but-not-shallow (`--depth=1` leaves no history to walk) and runs `git log -1 -- <subpath>`. Moved into the library so it is testable, with a regression test that builds a local fixture repo whose last two commits do *not* touch the subpath — a tip-returning implementation fails it.
- **Modeled `DiscoverResult` + `DiscoverResultResponse`** (`Kind::NotModeled` → real bindings; modeled fixtures 8 → 10 of 87). Both directories were unmodeled, so the harness reported `failed=0` for the very change #3002 made — the drift went unnoticed for 12 days and `refresh` would have advanced the pin straight over it. Red-before-green confirmed: against the new pin the two fixtures fail with `missing field 'serverInfo'`, while the same bindings pass 22/22 against the old pin.
- **`refresh --write` silently rotted `EXAMPLES_PIN.md`.** It rewrites only the Commit SHA line, but the file also carried a hand-maintained subpath tree digest and capture date that it never updated — the tree SHA was still the one from a prior capture. Dropped the fields the tool cannot maintain and documented which single line it owns.

### Fixed (2026-07-28, stale pin claims in trackers and guidance)

- **The driver document asserted "NO re-pin trigger exists."** `docs/plans/2026-07-28-spec-compliance.md` recorded pin parity against `6e4cba2d…` / `93671a3f` as a standing property; it expired when #3002 landed on 2026-07-16. Corrected to the current pin and re-worded — parity is a claim about a moment, not a standing property. `OUTSTANDING.md` and `docs/plans/2026-07-28-draft-migration-audit.md` are dated audit records, so they carry a SUPERSEDED banner rather than being rewritten.
- **Guidance drift between the two instruction files.** The mandatory pin rules had been added only to `CLAUDE.md`, while `AGENTS.md` is the stated authority on conflict. The governing rules now live once, in `AGENTS.md` §Branch Lock → "Schema pin governance"; `CLAUDE.md` keeps only the runnable check and points at it.
- **`README.md` still said "ETag-pinned"** in the crate-layout listing (a second occurrence beyond the provenance bullet already corrected).

### Fixed (2026-07-28, 2025-11-25 lane was never actually tested)

- **The `server 2025-11-25` CI gate ran `cargo build` + `clippy` but never `cargo test`**, so every test gated behind `#[cfg(feature = "protocol-2025-11-25")]` — pagination, logging builders, session-aware logging — executed nowhere. `turul-mcp-builders` and `turul-http-mcp-server` had the same build-only gate. All three upgraded to `cargo test` in both `scripts/ci-gates.sh` and `.github/workflows/ci.yml` (the same upgrade `lambda 2025-11-25` already received on 2026-07-14). `turul-mcp-derive` deliberately stays build-only, now with a comment saying why: it dev-depends on `turul-mcp-server`, whose 2026 default unifies both protocol features and trips the alias mutex under `cargo test`.
- **`test_pagination_with_invalid_cursor` asserted the pre-spec contract.** Commit `3ad11118` made an unissued cursor return `-32602` — which the schema backs, naming "Pagination: Invalid or expired cursor values" as an `InvalidParamsError` context with an `invalid-cursor.json` example — but the lane-gated test still asserted a silent restart-from-the-beginning fallback, and no gate ran it. Migrated to assert the rejection. Silently restarting would hand a caller a full first page while it believed it was resuming, surfacing as duplicated items rather than an error.
- **Two `turul-mcp-builders` doctests failed to compile on the 2025 lane.** `prompt_traits.rs` glob-imports both `turul_mcp_protocol::prompts::*` and `turul_mcp_builders::prelude::*`; on that lane both export a `PromptAnnotations` — different types (wire-level vs the framework display-hint type the 2026 schema no longer has), so the doctests hit `E0659` plus an `E0053` signature mismatch. Disambiguated with an explicit import. Per the doctest rule these must compile, and build-only gating had hidden it.

### Changed (2026-07-28, retire the pre-finalization `DRAFT-2026-v1` literal)

- **150 stale `DRAFT-2026-v1` usages replaced with `2026-07-28`** across the 2026 protocol crate (138) and the server/lambda/builders/schema-validation crates (12). These were a mix of wire-value literals — the compliance suite was largely negotiating with the *legacy* version string rather than the current one — and prose that still named the spec revision by its pre-finalization draft label, which the spec-version naming rule now spells as the full date. ADRs and this changelog deliberately keep the old literal: they are the historical record of when it applied.
- **The `DRAFT-2026-v1` deserialize alias was removed entirely** — serde `alias`, `FromStr` arm, explanatory comment, and the back-compat test. Nothing at 0.4 ever shipped emitting it, and it carried no owner, removal trigger, or removal date, which the active-development policy forbids. A negative test now asserts the literal is rejected.

### Fixed (2026-07-28, stale documentation)

- **Deleted the orphaned `docs/schema.ts`** — 1534 lines of 2025-11-25 schema with stripped interface bodies, referenced by nothing, duplicating what the frozen `turul-mcp-protocol-2025-11-25` crate already models.
- **`turul-mcp-protocol-2026-07-28/README.md` still described the pre-finalization draft** — it stated the wire-version string *is* `DRAFT-2026-v1` and that finalization was pending, while the crate emits `"2026-07-28"` and accepted the draft literal only as a deserialize alias. Also corrected the stale test count (322 → 420) and "ETag-pinned" (now commit-pinned). *(Superseded within this same unreleased version by the "retire the pre-finalization `DRAFT-2026-v1` literal" entry above, which removed the deserialize alias entirely — the crate now rejects the literal.)*
- **COMPLIANCE.md claimed the schema types `NotificationParams._meta` as `MetaObject`** — it is `NotificationMetaObject`, and the row was marked fully compliant. Re-stated as the same structural-only deviation as `Result._meta`.

### Added (2026-07-14)

- **Lambda ↔ local builder method-registration parity (`registered_methods()` + cross-builder test).** `turul-mcp-aws-lambda`'s `LambdaMcpServerBuilder` and `turul-mcp-server`'s `McpServer` builder are independent code paths; a JSON-RPC method registered on one but not the other means that transport silently 404s a spec method — this is the `server/discover`-missing-on-Lambda bug (fixed 2026-07-13 in `87430c17`, but the two builders had no test tying them together). Exposed `registered_methods()` on `HttpMcpServer`, `McpServer`, and `LambdaMcpHandler` (the last promoted `pub(crate)` → `pub`) and added a cross-builder parity test (`turul-mcp-aws-lambda/tests/cross_builder_method_parity.rs`) that builds **both** through their production paths and asserts identical registered method sets per protocol lane (2026-07-28 and 2025-11-25). Revert-and-fail proven: registering a bogus method on the Lambda side in place of `server/discover` fails the test with the exact divergence (`only in local: [server/discover] / only in Lambda: [__parity_break__]`). Also de-duplicated the method-registration block that was copy-pasted between `McpServer::run_http` and `run_with_sse_access` (the internal-divergence source of the same bug class) into a single `build_configured_http_builder()` — behaviour-preserving (the two blocks were byte-identical modulo a stray comment; full both-lane gate suite green). The `lambda 2025-11-25` CI gate was upgraded from `cargo build` to `cargo test` (in both `scripts/ci-gates.sh` and `.github/workflows/ci.yml`) so the 2025-lane lambda + parity tests actually run.
- **BP-3: JSON Schema 2020-12 dialect validation for tool `inputSchema`, both trust boundaries.** New dedicated crate `turul-mcp-schema-validation` (deps: `serde_json`, `jsonschema` 0.47 with `default-features = false` — no `reqwest`/`resolve-http`/`resolve-file`/`tls-*`, `thiserror`; no protocol-crate dependency, no cargo-feature plumbing). Exports `validate_tool_input_schema`/`SchemaValidationError`. Satisfies the spec MUST (basic protocol row 207: "Clients and servers MUST validate schemas according to their declared or default dialect and MUST handle unsupported dialects gracefully") via dialect detection + 2020-12 meta-validation. Additionally enforces, as **framework security policy layered on top of the spec MUST, not mandated by it**: remote-`$ref` rejection (SSRF hardening) and `MAX_SCHEMA_BYTES = 256 KiB` / `MAX_COMPOSITION_DEPTH = 32` bounds (DoS hardening) — every error message names the specific value that failed (byte count, limit, kind, or `$ref` URI + "policy" wording), asserted in tests via message content, not just the error variant. The bounds walk is cycle-safe: it traverses only the literal JSON document tree and never resolves/follows a `$ref` (an earlier draft did follow local `$ref` chains with a depth cap and was corrected during review — a genuinely cyclic local `$ref`, the legal recursive-schema case, would have been wrongly rejected as `TooDeep` after hitting the cap); a unit test with a recursive local-`$ref` tree-node schema asserts it is ACCEPTED. Server: `McpServerBuilder::build()` rejects registration of any tool whose `inputSchema` fails validation (a server MUST NOT advertise an invalid schema). Client: `parse_list_tools` (2026-07-28 wire path) excludes any tool whose `inputSchema` is invalid and logs a warning. Both crates link `turul-mcp-schema-validation` as a plain, unconditional dependency — it was NOT placed in `turul-mcp-builders` (an earlier draft did this and was corrected during review: `turul-mcp-client`'s `src/` uses `turul_mcp_builders` zero times, and `turul-mcp-builders` unconditionally depends on the `turul-mcp-protocol` alias crate, so linking it from the client would reintroduce the alias's `protocol-2025-11-25`/`protocol-2026-07-28` feature mutex into the client's graph — confirmed by spiking the dependency, it fails to compile with neither feature selected — exactly the coupling ADR-030 removed the alias from this crate to avoid). `cargo tree -p turul-mcp-derive -e normal,build -i jsonschema` is empty — the derive crate's published proc-macro artifact does not link `jsonschema` (only its own `[dev-dependencies]` on `turul-mcp-server`, test-only, surfaces it in the default dev-edges-included `cargo tree -i`). See ADR-003 revision log (2026-07-14).
- **SEP-2243: detect `x-mcp-header` annotations misplaced outside a `properties` chain.** New `turul-mcp-protocol-2026-07-28::headers::find_misplaced_x_mcp_header` closes a gap the existing `scan_x_mcp_headers` walk could not see on its own: an `x-mcp-header` reachable only through `items`, a composition keyword (`oneOf`/`anyOf`/`allOf`/`not`), a conditional (`if`/`then`/`else`), or `$ref` indirection. `scan_x_mcp_headers` silently skips these positions (by design — it only recurses via a property's own `properties` map for the positive binding scan), so a schema author who puts the annotation there previously got neither the mirrored header nor a rejection. The client's `parse_list_tools` now also excludes a tool flagged by this detector from `tools/list` and logs a warning, per SEP-2243's "client MUST exclude the invalid tool from tools/list" / SHOULD warn.
- **GAP-CF-9: sampling message-shape validation (client/sampling MUSTs).** New `CreateMessageRequestParams::validate_message_shape` in `turul-mcp-protocol-2026-07-28::sampling` enforces: (a) a user-role message whose content contains a `ToolResult` block must contain ONLY `ToolResult` blocks; (b) an assistant-role message containing a `ToolUse` block must be immediately followed by a user-role message consisting entirely of `ToolResult` blocks whose `tool_use_id`s match the preceding `ToolUse` ids. Enforced server-side in `input_required_to_result` (`turul-mcp-server/src/handlers/mod.rs`, a real reachable production path for a tool-originated `InputRequest::CreateMessage` on the 2026 lane, since `sampling/createMessage`'s inbound RPC handler is 2025-11-25-only per SEP-2577's deprecation): an invalid message shape is rejected with `McpError::InvalidParameters` (`-32602`) before the request is packaged into an `InputRequiredResult` and sent to the client, rather than forwarded as-is.
- **Wire/unit tests, all with revert-and-fail evidence**: `turul-mcp-schema-validation/src/lib.rs` (9 unit tests, including a recursive-local-`$ref`-accepted test and message-content assertions for `RemoteRef`/`TooLarge`/`TooDeep`), `turul-mcp-server/tests/schema_validation_2026.rs` (2, `build()` accept/reject), `turul-mcp-client/tests/bilingual_2026_operations.rs` (+2: misplaced-header exclusion, BP-3 exclusion), `turul-mcp-protocol-2026-07-28/src/headers.rs` (+8 `find_misplaced_x_mcp_header` unit tests), `turul-mcp-protocol-2026-07-28/src/sampling.rs` (+3 `validate_message_shape` unit tests), `turul-mcp-server/tests/sampling_shape_2026.rs` (2, reject/accept).

### Removed

- **0.4 docs purge (2026-06-12).** 23 stale 0.3-era / executed-plan documents deleted rather than archived in-tree — git history and the v0.3.x tags carry them. Root: `WORKING_MEMORY.md`, `TODO_TRACKER.md`, `HISTORY.md`, `TESTING_GUIDE.md`, `DOCUMENTATION_TESTING.md`, `GEMINI.md`, `EXAMPLE_VERIFICATION_LOG.md`. docs/: `architecture/GLOBAL_FANOUT_ARCHITECTURE.md`, `dynamodb-testing-notes.md`, `testing/MCP_E2E_COMPLIANCE_TEST_PLAN.md`, `superpowers/plans/*`. docs/plans/: the executed/superseded snapshots (`2026-03-07-oauth-compliance-v0.3.10`, `PARKED`, `codex-review-summary`, `spec-vs-website-audit`, `schema-coverage-matrix`, `release-readiness-review`, `compliance-plan`, `feature-gating-rollout`, `example-fixture-compliance`, `spec-4source-reconciliation`, `examples-review`). KEPT: the spec-compliance driver, `final-readiness-audit` (ci gates cite its §7), `migration-diff` (designated symbol map), `architecture-review` (maintainer-locked decision record), `dependency-hardening-followup` (still-actionable). Every reference in live docs/ADR bodies/code comments retargeted or annotated "(deleted in the 0.4 docs purge — see git history)"; dated revision-log entries, gap-register citations, and CHANGELOG history left verbatim; the two FROZEN 2025 crate reports untouched per the frozen-crates rule. ADRs themselves: none deleted.

### Added

- **`ext-tasks-server` example pair (2026-06-12, SEP-2663 — slice C).** Task-electing server (port 8645: `crunch` long-runner + `deploy` with mid-task elicited approval) and a client walkthrough bin driving the full lifecycle — task handle → `task_wait` polling at the server's `pollIntervalMs` → `input_required` → `tasks/update` → completed, plus the progressive-enhancement contrast (the same tool blocks ~2s and answers synchronously for an undeclared client). Live-verified end to end; EXAMPLES.md pairing row + counts (54 active); built by the client-using-examples gate step.
- **Tasks-extension client surface (2026-06-12, SEP-2663 — slice B).** New opt-in `ext-tasks` feature on `turul-mcp-client`: `declared_capabilities.ext_tasks = true` declares `io.modelcontextprotocol/tasks` in every 2026 request's `_meta` `clientCapabilities.extensions`; `call_tool_or_task(name, args)` returns `ToolCallOutcome::Completed | Task` (the server is the sole decider — the typed union replaces guessing); `task_get`/`task_update`/`task_cancel` bind the lifecycle methods; `task_wait` polls to a terminal status honoring the server's `pollIntervalMs` (clamped 50ms–30s). The strict BP-1 parser is untouched — plain `call_tool` still rejects unknown `resultType`s; only the ext-aware path accepts `"task"`. 4 real-server e2e tests (`ext_tasks_e2e_2026.rs`, gates + CI): task→poll→completed, sync fallback for undeclared clients, `task_update` resuming an `input_required` task, cancel-to-cancelled; revert-and-fail recorded (suppressing the capability emission fails the task-outcome test with a sync result).
- **Tasks-extension server runtime (2026-06-12, SEP-2663 — closes driver gap G1).** New opt-in `ext-tasks` feature on `turul-mcp-server` (off by default per SEP-2133): `.with_ext_tasks(store)` advertises `io.modelcontextprotocol/tasks` in `server/discover`'s `capabilities.extensions` and registers `tasks/get`/`tasks/update`/`tasks/cancel`; `.ext_task_tool(tool)` / `.ext_task_tool_required(tool)` mark tools for task election — a request whose per-request `_meta` `clientCapabilities.extensions` declares the extension gets a durable `CreateTaskResult` (UUIDv4 bearer-grade id, store written BEFORE the response) with a spawned worker; undeclared requests run synchronously (progressive enhancement) or, for `_required` tools, get `-32003` with the upstream `data.requiredCapabilities.extensions` shape. **MRTR bridge**: a task tool returning `McpError::InputRequired` parks its task in `input_required`; `tasks/update` validates response keys against outstanding requests (partial delivery keeps it parked) and resumes the worker with the responses injected through the same session-extension keys as the sync retry leg — tool code is identical under both execution models. `tasks/cancel` is cooperative (aborts the worker, drops input waiters, acks terminal tasks). `notifications/tasks` rides `subscriptions/listen`: the transport honors a `taskIds` filter iff the extension is advertised (keyed off the capability map — no transport dependency on the ext crate), echoes it in the ack, and delivers per-taskId. `turul-mcp-ext-tasks` gains the `TaskStore` trait + `TaskState` + `InMemoryTaskStore` (no tokio in the public API). 9 wire e2e tests (`ext_tasks_2026.rs`, wired into gates + CI); revert-and-fail recorded. Dispatcher design recorded in ADR-028 (2026-06-12 entry).
- **`turul-mcp-ext-apps` 0.1.0 scaffold (2026-06-12, SEP-1865).** Spec-neutral extension crate binding the MCP-side Apps surface: extension identifier `io.modelcontextprotocol/ui` (the ADR-028 table's `/apps` guess corrected against upstream), client capability (`UiClientCapabilities.mimeTypes` + the `text/html;profile=mcp-app` HTML-views gate), tool `_meta.ui` (`UiToolMeta`: `resourceUri`, `visibility` model/app), and UI-resource `_meta.ui` (`UiResourceMeta`: CSP domain lists, sandbox permissions, dedicated origin, `prefersBorder`). The host↔view iframe protocol is deliberately not bound (app/host SDK scope). Vendored spec pinned at `modelcontextprotocol/ext-apps@ca1d2989`; 5 wire-shape compliance tests.
- **Versioning/cancellation/elicitation P2 trio (2026-06-12).** (1) **VER-4**: the headerless-`initialize` rejection (400 + `-32001`) now carries `error.data.supported` naming this build's protocol versions — a true legacy client's only diagnostic; wire test `headerless_initialize_rejection_names_supported_versions`, red-phase recorded. (2) **PAT/G10**: dedicated `CancelledNotificationHandler` extracts `requestId` + `reason` from inbound `notifications/cancelled` into a structured log line ("Both parties SHOULD log cancellation reasons"); accepted-and-ignored semantics unchanged. (3) **CF/GAP-CF-8**: new `turul_mcp_builders::validate_elicit_content(schema, content)` validates elicited form content against the requesting schema (required/unknown keys, primitive types, string-length/numeric bounds, integer-ness, enum membership across the 2026 enum-union shapes; format assertions annotation-only by design) — central enforcement is impossible on the stateless lane (leg-1 schema not retained), so tools call it on the MRTR retry; wired into `mrtr-elicitation-server` and live-verified. Plus **BP-5** (COMPLIANCE.md §"Supported JSON Schema dialects" — the documentation the SHOULD asks for) and **UTIL/COMP-3** (relevance/fuzzy/rate-limit completion SHOULDs dispositioned: provider semantics + middleware rate limiting). Driver summary now 305 ✅ / 68 🟡 / 5 ❌ / 12 🧪 / 100 ➖ — the 5 remaining ❌ all carry recorded dispositions.
- **`turul-mcp-ext-tasks` 0.1.0 scaffold (2026-06-12, SEP-2663).** New spec-neutral extension crate per ADR-028 (2026-06-07 amendment): the `v2026_07_28` module carries the redesigned Tasks-extension surface — status-tagged `DetailedTask` (working/input_required/completed/failed/cancelled with variant fields inlined), `CreateTaskResult` (`resultType: "task"`, flat `Result & Task`), `tasks/get`/`tasks/update`/`tasks/cancel` bindings, `notifications/tasks`, `taskIds` subscription-filter fields, and capability negotiation helpers including SEP-2133 identifier validation. Upstream schema vendored from `modelcontextprotocol/ext-tasks@8966bea9` with a provenance README; 13 wire-shape compliance tests (explicit-null `ttlMs`, snake_case status strings, flat task discriminator). `protocol-2026-07-28` is the default feature; `--no-default-features` compiles empty. Server dispatch wiring and the 2025-11-25 reconciliation module are tracked as separate slices (ADR-028 revision log 2026-06-12). Partially closes driver gap **G1** (SEP-2663 row stays 🟡 until dispatch lands).
- **Driver-doc re-grade pass (2026-06-12, docs).** All 123 then-non-green rows of `docs/plans/2026-07-28-spec-compliance.md` were verified against post-P2-batch HEAD by an 11-agent sweep with spot-checked claims: 35 rows had been fixed by the P2 batches without being re-graded (now ✅ with **RE-GRADED 2026-06-12** citations — e.g. client MRTR retry triple, invalid-cursor -32602, initialize-names-supported-versions, conditional `completion/complete` registration), 17 improved to/confirmed 🟡, 2 implementation-only claims were demoted back to 🧪 during review, 3 got refreshed evidence pointers. Summary corrected to the true row count (490) and re-tallied: 302 ✅ / 66 🟡 / 10 ❌ / 12 🧪 / 100 ➖.

- **Server wire-edges P2 batch — the FINAL open driver gaps (2026-06-11).** All 73 audit gaps are now closed (52 fixed, 6 dispositioned with recorded rationale). Behavior: null request ids → 400 + -32600 pre-dispatch (MCP forbids them; turul-rpc's base-JSON-RPC Null variant stays); invalid pagination cursors → -32602 at all five list sites; `completion/complete` no longer a default handler (unconfigured server → 404 + -32601); blob resource contents validated as base64 before shipping; prompts/list carries title/icons/_meta; the initialize rejection names supported versions in error.data; `X-Accel-Buffering: no` on streaming responses; tool-name format warnings at registration; Mcp-Param message whitespace runs collapsed; `notify_elicitation_complete` + `notify_request_progress_with_message` session helpers; `PromptAnnotations` moved protocol→builders (no schema counterpart — purity). Tests: `wire_edges_2026.rs` (10), numeric Mcp-Param compare, roots/sampling -32003 arms, SEP-2577 marker tripwire (reverting Slice A'' now fails CI). Dispositions: schema-dialect validation (documented limitation), progress rate-limiting (middleware layer per ADR-012), sampling message-shape constraints (deprecated surface), tool-Err-vs-isError (deliberate AGENTS.md-documented contract: `Err` = protocol error, `CallToolResult::error` = model-visible). EXAMPLES_PIN capture date corrected. Closes **CHG/G4, CHG/G6, DEP-GAP-3, BP-2/3/4, VER-2, PAT/G5/G9, TX/GAP-3/4/5, CF/GAP-CF-6/7/9, DISC-4, PRM-2026-01/04/05, RES-G3/G6/G7, TOOLS-G3/G4/G6/G7, UTIL/COMP-2, UTIL/PAG-1/2, UTIL/LOG-2, SCHEMA/G2/G3/G5**.
- **Client capability/discovery P2 batch (2026-06-11).** (1) The `server/discover` body is now retained for the connection: `DiscoveredServer` (capabilities, instructions, serverInfo, supportedVersions) with `discovered_server()`/`server_capabilities()`/`server_instructions()` accessors. (2) `-32004` negotiation honors `error.data.supported`: fallback only when 2025-11-25 is mutually supported, otherwise the error names the server's list ("select a mutually supported version … or surface an error"). (3) Era detection no longer keys on one code: structured `-32602` also classifies as a legacy-server fallback signal per "commonly -32601 or -32602" (the prior -32602→abort unit pin migrated WITH the contract). (4) `DeclaredCapabilities` gains `elicitation_url`/`sampling_tools`/`sampling_context`, mapped into the spec's sub-capability shapes in every request `_meta`. (5) `call_tool` auto-recovers from `-32001` Mcp-Param rejections: one `tools/list` refresh + one retry per the SEP-2243 client-behavior note. (6) New `call_tool_with_progress(name, args, token, on_progress)`: SSE-framed request with `_meta.progressToken`, progress params delivered to the callback before the final result (real-server e2e). (7) `McpClientError::is_resource_not_found()` accepts `-32602` and the backwards-compat `-32002`. (8) First-page contract documented on the convenience list APIs (use `*_paginated` for full walks). structuredContent validation dispositioned (apps bring their own validator — no 2020-12 validator dependency for a SHOULD). Closes driver gaps **ARCH/GAP-ARCH-1, ARCH/GAP-ARCH-2, DISC-1, VER-3, CF/GAP-CF-5, TX/GAP-6, TX/GAP-7, RES-G4, UTIL/PAG-3, PAT/G4, TOOLS-G2**.
- **Subscriptions/cancellation P2 batch (2026-06-11, mostly tests).** New wire coverage: concurrent `subscriptions/listen` streams each receive exactly their filtered subset stamped with their own `subscriptionId`, and `notifications/message` never rides a listen stream (MUST NOT); dropping one subscription leaves siblings delivering; progress notifications stop at the final response (MUST); MRTR negative paths (neither-field `InputRequired` → server error; `InputRequired` escaping `completion/complete` → error, never `input_required`); unrecognized `logLevel` → `-32602`. Code: `notifications/cancelled` is now an explicitly registered notification on both lanes (202, never 404 — note: the 202 wire contract for true notifications pre-existed via the transport's fire-and-forget path; the registration adds sibling parity and the request-shaped consistency). Cancellation of in-flight work on Streamable HTTP remains the stream-close mechanism; cross-request correlation by id is impossible without sessions on the stateless lane, so inbound cancelled notifications are accepted and ignored per "Invalid cancellation notifications SHOULD be ignored". Server-shutdown stream teardown dispositioned (socket-close; no graceful-shutdown API exists). Closes driver gaps **PAT/G6, PAT/G7, PAT/G8, TOOLS-G5, UTIL/LOG-1, SCHEMA/G1**.
- **OAuth/security P2 batch (2026-06-11).** (1) *Malformed Authorization → 400*: a present-but-unparseable `Authorization` header (wrong scheme, empty/multi-token Bearer) now answers 400 + `error="invalid_request"` (RFC 6750 §3.1) instead of the missing-credentials 401 — `RequestContext::authorization_malformed` is set by both transports; wire-tested. (2) *Runtime scope enforcement*: `OAuthResourceMiddleware::with_required_scopes` rejects tokens missing a required scope with 403 + `error="insufficient_scope"` per Authorization §Insufficient Scope; unit-tested with minted HS256 tokens. (3) *offline_access guard*: `ProtectedResourceMetadata::with_scopes` filters `offline_access` with a warning (resource servers SHOULD NOT advertise it). (4) *Sessionless-ping auth*: on the 2025-11-25 lane, the `allow_unauthenticated_ping` bypass now runs AFTER the pre-session auth phase — it waives the session requirement only, matching its documented contract ("the full middleware stack still runs"); new 2025-lane wire test `tests/ping_auth_2025.rs` wired into the gates. (5) *Session-user binding (AUTH-7)*: dispositioned by design — claims stay request-scoped per ADR-021 D2; deployments needing binding implement it via middleware + session state; moot on the sessionless 2026 lane. Closes spec-compliance driver gaps **AUTH-2, AUTH-3, AUTH-5, AUTH-6 (fixed) + AUTH-7 (dispositioned)** — the OAuth/security P2 theme is closed.
- **Transport deprecation markers (2026-06-11, SEP-2596 + 2026 lane).** The client's `SseTransport` (HTTP+SSE, ≤ 2024-11-05) now carries `#[deprecated]` with migration notes in the crate docs and README — the transport is deprecated upstream (SEP-2596, 2025-03-26: "new implementations SHOULD NOT adopt it"); it remains functional for unmigrated servers. The server's legacy `session_handler` module documents the same. `ServerConfig.enable_get_sse` and the `get_sse()` builder setter are deprecated on the 2026-07-28 lane only (`cfg_attr`): the stateless endpoint is POST-only (GET = 405) and the long-lived stream is `subscriptions/listen`; stateful GET SSE remains first-class on the `protocol-2025-11-25` opt-in. Closes spec-compliance driver gap **DEP-GAP-1 (P2)**.
- **Client disconnect now cancels the in-flight request (2026-06-11).** Streamable HTTP §Cancellation: "Closing the SSE response stream MUST be treated by the server as cancellation of that request. The server SHOULD stop work … and MUST NOT send any further messages for it." The streaming dispatch task previously ran detached to completion after a disconnect; it now races the dispatch future against the response channel's `closed()` signal — on disconnect the future is dropped (the handler stops at its next await point), the progress task is shut down, and nothing further is sent. Wire test: a slow tool's completion flag stays unset when the client drops mid-execution (`cancellation_2026.rs`; control test pins the connected path). Closes spec-compliance driver gaps **PAT/G1 + TX/GAP-2 (both P1)** — the final open P1s.
- **Request-scoped progress on the 2026 path (2026-06-11).** Tools and resources can now emit spec-compliant `notifications/progress`: the request's `_meta.progressToken` is surfaced through the session extensions (`SessionContext::progress_token()`), and the new `notify_request_progress(progress, total)` references exactly that token — no-op when the request declared none ("Progress notifications MUST only reference tokens that were provided in an active request"). Numeric tokens now round-trip as JSON numbers end-to-end; the session→StreamManager bridge previously dropped non-string tokens (`as_str()`) and stringified the rest. Wire tests in `progress_2026.rs` (string echo, numeric round-trip, no-token-no-notifications); revert-and-fail recorded. Closes spec-compliance driver gap **PAT/G2 (P1)**.
- **Real-HTTP OAuth acceptance on the 2026 default transport (2026-06-11, tests + manifest).** New `crates/turul-mcp-server/tests/oauth_2026.rs`: missing/garbage bearers → 401 with the RFC 9728 `WWW-Authenticate` challenge (`resource_metadata=`, `error="invalid_token"`), 401 outranks the missing-`_meta` 400 (auth before validation), and both RFC 9728 well-known routes (root + path form) serve the metadata unauthenticated — all through Builder → `server.run()` → wire. To ride `turul-mcp-server`'s dev-deps without tripping the ADR-029 spec mutex, `turul-mcp-oauth` is now spec-neutral: its transport/storage deps drop default features and it gains its own `protocol-2025-11-25`/`protocol-2026-07-28` forwarding features (default 2026 standalone; unification supplies the spec when used with `default-features = false`). Closes spec-compliance driver gap **AUTH-1 (P1)**.
- **Regression nets for two MUST-level client behaviors (2026-06-11, tests only).** (1) `bilingual_client_falls_back_on_400_with_32004_body` pins the Versioning §Backward Compatibility wire path: HTTP 400 whose body carries structured `-32004` + `data.supported` → fall back to 2025-11-25 through the real probe (a bare 4xx still aborts). (2) `invalid_x_mcp_header_tools_are_excluded_from_tools_list` pins Tools §x-mcp-header: a tool definition with a constraint-violating `x-mcp-header` value MUST be excluded from `tools/list` while valid tools survive. Both revert-and-fail proven against their pre-existing implementations. Closes spec-compliance driver gaps **VER-1 + TOOLS-G1 (both P1)**.
- **`completion/complete` now dispatches to registered `McpCompletion` providers (2026-06-11).** Providers registered via `.completion_provider(...)` were stored but never consulted — the handler always answered with hardcoded placeholder values and ignored its input. The handler now parses typed `CompleteRequestParams` (malformed input → `-32602`, including reference-type literals `"ref/prompt"`/`"ref/resource"` that the untagged union would otherwise accept open-ended), routes deterministically (exact reference match first, `can_handle` fallback; priority desc then insertion order — provider storage moved from `HashMap` to `Vec` to make the tiebreak stable), runs the provider's `validate_request`, and enforces the spec's 100-item `completion.values` cap (truncation sets `total`/`hasMore`). No matching provider → empty values (the placeholder junk is gone). The same gap existed verbatim in `LambdaMcpServerBuilder` (providers stored, static handler answered) — mirrored fix there. Closes spec-compliance driver gaps **UTIL/COMP-1 (P1)** and **UTIL/COMP-3 (P2)**; red-phase wire tests in `discover_stateless_2026.rs`.
- **Mode-aware MRTR capability gating (2026-06-11).** The server's `-32003` gate on `InputRequiredResult` now enforces sub-capabilities, not just top-level presence: URL-mode elicitation requires the client's `elicitation.url` declaration ("Servers MUST NOT send elicitation requests with modes that are not supported by the client"; an empty `elicitation: {}` declares form-only), and tool-enabled sampling (`tools`/`toolChoice` present) requires `sampling.tools` ("Servers MUST NOT send tool-enabled sampling requests to Clients that have not declared support"). Closes spec-compliance driver gaps **CF/GAP-CF-1 + CF/GAP-CF-2 (both P1)**; red-phase wire tests in `mrtr_2026.rs`.
- **`roots/list` removed from the 2026 default surface (2026-06-11).** On 2026-07-28, roots is a client feature: the server requests roots via MRTR input requests and never hosts an inbound `roots/list` RPC; `notifications/roots/list_changed` has no binding in the pinned schema. The builder's `roots/list` + roots-notification registrations are now gated to the `protocol-2025-11-25` opt-in; on the 2026 default they answer 404 + `-32601` like every other non-2026 method. Closes spec-compliance driver gap **CF/GAP-CF-4 (P1)**; red-phase recorded in `error_mapping_2026.rs`.
- **Client MRTR completion for `resources/read` + `prompts/get`, and `resultType` discipline (2026-06-11, ADR-030 revision log).** The bilingual client's `parse_read_resource`/`parse_get_prompt` now surface `InputRequiredResult` as `McpClientError::InputRequired` (previously a serde "missing field" error that discarded `inputRequests`/`requestState`), with retry APIs `read_resource_with_input_responses` / `get_prompt_with_input_responses` mirroring `call_tool_with_input_responses`. All 2026 result parsers now enforce basic §Responses ("a resultType of any value unrecognized by the client MUST be considered invalid"): unknown discriminators are `ProtocolError::InvalidResponse` instead of being treated as complete results. Closes spec-compliance driver gaps CF/GAP-CF-3, PRM/PR-2026-02, RES-G1, PAT/G3, BP-1 (all P1 except PAT/G3). Real-server e2e round-trips added; revert-and-fail recorded.
- **Origin-header validation (DNS-rebinding protection) on the HTTP transport (2026-06-11, ADR-031).** Streamable HTTP §Security requires: "Servers MUST validate the `Origin` header … If the `Origin` header is present and invalid, servers MUST respond with HTTP 403 Forbidden." New `OriginPolicy` on `ServerConfig` — default `SameOriginOrLoopback` (Origin absent → allowed; loopback or Host-matching origins → allowed; anything else → 403), `AllowList(Vec<String>)` additive allowlist, `Disabled` opt-out for upstream-enforced deployments. Enforced at both transport handler entries (streamable + legacy session path), so hyper and Lambda deployments inherit it; OPTIONS preflight and `.well-known` routes exempt. Builder knobs: `HttpMcpServerBuilder::origin_policy` and `McpServer::builder().origin_policy(...)`. Wire tests in `crates/turul-mcp-server/tests/origin_validation_2026.rs` (revert-and-fail proven: disabling the gate fails 3 tests). On the Lambda builder, an explicit CORS configuration derives the policy (`cors_allow_all_origins()` → `Disabled`, an origin list → `AllowList`) unless `.origin_policy()` overrides it — `turul-http-mcp-server`'s blanket `enable_cors` flag deliberately does NOT derive (see ADR-031 §CORS-derived policy). Closes spec-compliance driver gap **TX/GAP-1 (P0)**. **Behavior change:** cross-origin browser clients now require an explicit `AllowList` (previously implicitly admitted via CORS `*`).
- **New crate `turul-mcp-protocol-2026-07-28` at 0.4.0** — first standalone binding for the MCP DRAFT-2026-v1 release candidate (see [https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)). Stateless protocol core (`initialize` / `notifications/initialized` removed, `Mcp-Session-Id` header removed, per-request capability negotiation in `_meta`), new `server/discover` method, multi-round-trip `InputRequiredResult` (SEP-2322), `CacheableResult` mixin (`ttlMs`, `cacheScope`), W3C Trace Context in `_meta`, JSON Schema 2020-12 on tool input schemas, MCP Apps templates, RFC 9207 auth hardening, error code `-32002 → -32602`. Schema pinned to upstream commit `c3e3f09eb5d271407afac0f0bb6ee2dae5813d1d`. Compliance harness with bidirectional wire-format gate against the upstream's 86 canonical example fixtures (8 modeled cases / 20 fixtures bound at this cut; remainder marked `Kind::NotModeled` for wave-by-wave migration). 343 tests pass under `--features compliance` (160 lib + 179 integration + 3 fixture + 1 doctest), 333 default; `clippy -D warnings` clean. See `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md`.
- **First-party 2026-07-28 stateless server (2026-06-07).** `turul-mcp-server` / `turul-http-mcp-server` gained a `server/discover` handler plus a stateless 2026 request path: capabilities, client info, and protocol version travel in per-request `_meta` on every call (no `initialize` / `notifications/initialized` handshake, no `Mcp-Session-Id` header). The transport advertises `MCP-Protocol-Version: 2026-07-28` on the wire. Wire-level acceptance tests cover the stateless core; `server/discover` returns a `CacheableResult` (`ttlMs`/`cacheScope`). The 2025-11-25 stateful core (handshake + session header + GET SSE) remains available under the `protocol-2025-11-25` opt-in.
- **Schema re-pinned to finalized upstream `2026-07-28` (2026-06-07).** Re-vendored `schema/draft-schema.ts` from `modelcontextprotocol/modelcontextprotocol@main` (HTTP ETag `0eeaed15…`, content sha256 `20df36f9…`; was `8bdd4ae5…` at the 2026-05-24 cut). The 159-symbol export surface and 22 method strings are **identical** to the prior pin (stateless core intact — no `initialize`/`ping`/`resources/subscribe`/`tasks/*` reintroduced; verified against live `main`). Exactly three substantive wire changes applied: (1) `LATEST_PROTOCOL_VERSION` / `MCP_VERSION` / `McpVersion::V2026_07_28` serde rename flipped `"DRAFT-2026-v1"` → `"2026-07-28"` (draft literal still accepted on deserialize via serde `alias` for back-compat); (2) `ResultType` became an open union `"complete" | "input_required" | string` — modeled as `ResultType::Other(String)` with custom serde so unknown discriminators round-trip instead of being rejected; (3) `DiscoverResult` now `extends CacheableResult` (`ttlMs`/`cacheScope` required fields added). Also fixed the `clippy::large_enum_variant` gate on the deprecated MRTR `InputRequest`/`InputResponse` unions (scoped `#[allow]` with rationale). Contract-change tests migrated; revert-and-fail verified for the version and `ResultType` deltas. Crate stays at **0.4.0** (unreleased) — completing the spec line 0.4.0 was created to target, not a new version. See `docs/adr/027` revision log (2026-06-07).
- **Standardized the branch on published `turul-rpc` 0.2 (crates.io); `turul-rpc` 0.1 is no longer referenced anywhere on this branch** (`Cargo.lock` resolves only `0.2.2`). The workspace pin moved `0.1 → 0.2` and the `turul-mcp-protocol-2026-07-28` per-crate `0.2.2` override collapsed onto the workspace pin. `turul-rpc` 0.2 split inbound `JsonRpcMessage` (now parse-only: `Request`/`Notification`) from the outbound `JsonRpcResponse` union (`Success`/`Error`); `turul-http-mcp-server` was ported accordingly — its private dispatch path now produces `JsonRpcResponse` and converts to `JsonRpcMessageResult` via the canonical `Success→Response`/`Error→Error` mapping, and `JsonRpcResponse::success` takes `ResponseResult` (result values `.into()`-converted). Wire format is byte-identical (`{jsonrpc,id,result}` / `{jsonrpc,id,error}`) — **zero test expectations changed**. The frozen `2025-06-18` / `2025-11-25` protocol crates compile unchanged on 0.2 (freeze intact). `cargo build --workspace` clean; `turul-http-mcp-server` 92 unit + 11 doc tests pass; `clippy -D warnings` clean. (0.1 was maintained only for the 0.3 framework, which lives on `main`.)
- **`turul-mcp-client` is now bilingual (2025-11-25 + 2026-07-28) by default (2026-06-07).** A single client negotiates the wire spec per connection at `connect()` (`server/discover` → 2026-07-28; JSON-RPC `-32601` → fall back to `initialize` → 2025-11-25; HTTP 4xx and all other JSON-RPC errors abort WITHOUT downgrade; opt-in `allow_legacy_gateway_fallback` broadens the fallback to 404/405) and locks `McpVersion` for the connection. The client links both versioned protocol crates directly (a recorded exception to the Protocol Re-export Rule — CLAUDE.md + ADR-001), gated by mutually-exclusive features `client-bilingual` (default) / `client-2025-11-25-only` / `client-2026-07-28-only` with a `compile_error!` mutex. On a 2026 connection every core operation routes through `protocol/v2026_07_28` with the required per-request `_meta` and 2026 result parsing: `tools/list`/`tools/call`, `resources/list`/`read`/`templates/list`, `prompts/list`/`get`, and the `*_paginated` variants. Methods removed from the 2026 core (`ping`, `tasks/*`) are rejected on a 2026 connection and retained on 2025-11-25. Acceptance: `bilingual_negotiation.rs` + `bilingual_2026_operations.rs` (per-op `_meta` wire enforcement against a mock 2026 server + removed-method rejection). 143 client tests; `clippy -D warnings` clean on all three feature configs. Still pending: MRTR `InputRequiredResult`, `completion/complete` client op, server-initiated elicitation. See ADR-030 revision log (2026-06-07). The bilingual client builds under the 2026-default workspace and the framework alias cutover has landed (see the cutover entry below).
- **ADR-027** — *Targeting MCP DRAFT-2026-v1*. Records the wire-string choice (`"DRAFT-2026-v1"` until the upstream RC ships its `2026-07-28` literal), the schema-pin regeneration trigger, the per-crate versioning policy, and the consequences for downstream consumers. Revision log captures the 2026-05-24 initial cut, the 2026-05-31 per-crate-versioning adoption, Slice A' schema-fidelity corrections, Slice A'' SEP-2577 deprecation annotations, and the Slice C status update (§Consequences replaced; Phase 9.4 moves *into* 0.4.0).
- **ADR-029** — *Spec-version coexistence via mutually-exclusive cargo features (default 2026-07-28)*. The load-bearing 0.4.0 architecture decision. Default = 2026-07-28; opt-in `protocol-2025-11-25` feature on `turul-mcp-protocol`; `compile_error!` mutex; Phase 9.4 flip-all-at-once (landed — see the cutover entry below).
- **ADR-030** — *turul-mcp-client spec coexistence — bilingual default*. Client diverges from server's single-spec strategy because a client has no process-wide state-machine lock — it talks to whatever's on the wire. Per-connection version detection via try-`server/discover`-then-fallback-to-`initialize`; opt-in `client-2025-11-25-only`/`client-2026-07-28-only` narrowing features for binary size.
- **8 existing-ADR amendments** documenting the default-2026 cascade through ADR-027 (consequences replaced + status update + revision log), ADR-006 (stateless variant; GET SSE is 2025-only), ADR-009 (`McpProtocolVersion` becomes feature-exclusive), ADR-023 (per-request fingerprint persistence), ADR-001-lambda (stateless 2026 Lambda variant — ~50 vs ~200 LOC), and revision-log entries on ADR-025, ADR-026, ADR-028.
- **`docs/plans/2026-07-28-architecture-review.md`** — doc-form persistence of the 5-pattern architecture-review workflow that recommended Pattern A (cargo-feature gating). Persists what was previously in `/tmp` so the analysis is permanently in the repo.
- **`docs/plans/2026-07-28-feature-gating-rollout.md`** — phase-by-phase implementation plan for wiring `#[cfg(feature = "protocol-...")]` through the framework crates, examples, and test crates. This plan was the verification artifact for the cutover rollout that has since landed (see the cutover entry under §Changed).
- **`docs/plans/2026-07-28-codex-review-summary.md`** — self-contained codex-review-ready summary covering the decision, files-touched inventory, technical risks, and codex focus areas.
- **SEP-2577 deprecation annotations** on Roots / Sampling / Logging types and traits (`#[deprecated(since = "0.4.0", note = "...")]` with migration-path guidance and 2027-07-28+ earliest-removal date). Annotation-only this revision; types remain fully functional during the 12-month migration window. `LoggingLevel` (the value type for the non-deprecated `RequestMetaObject.log_level` replacement) is intentionally NOT deprecated.
- **ADR-028** — *Extensions strategy* (SEP-2133 / SEP-2663). Documents how the framework will host out-of-tree extensions — originally as schema-version-suffixed crates; superseded by the spec-neutral `turul-mcp-ext-tasks` / `turul-mcp-ext-apps` names (ADR-028 amendments 2026-06-07 and 2026-06-12).

### Added (2026-06-10, release-prep sweep)

- **Client-side `subscriptions/listen`** — `McpClient::subscriptions_listen(filter)` (2026 connections only) opens the long-lived stream via a new additive `Transport::send_request_streaming` (Streamable HTTP implements it; other transports default to unsupported). The client consumes and validates the mandatory acknowledgement first and returns a `SubscriptionStream` exposing the honored filter subset and the subscription id, then yields each notification; dropping the stream closes it (= cancellation per Streamable HTTP). e2e: real client ↔ real server — open stream, trigger server-wide broadcasts from a second request, receive only the opted-in type stamped with the subscription id.
- **ADR-025 framework shim cut landed** — `turul-mcp-server`/`turul-http-mcp-server`/`turul-mcp-builders`/`turul-mcp-aws-lambda` now depend on `turul-rpc` directly (146 path swaps; the shim mirrors `turul-rpc` paths 1:1 and re-exports the same types, so no public API changed). The shim remains in-workspace solely for the frozen 2025 protocol snapshots and 2025-pinned test/example crates, with its manifest restored to the terminal `0.3.47` (the mechanical 0.4.0 sweep value was never publishable). ADR-025/ADR-027 revision logs updated; the 2025 regression lane is recorded as the per-crate matrix (a workspace-wide flag sweep trips the spec mutex by design).

- **MRTR on `resources/read` and `prompts/get`** — completes the SEP-2322 triple (the only methods permitted to return `input_required`). The conversion + client-capability gate is now one shared helper (`handlers::input_required_to_result`, also adopted by `tools/call`). Resources surface the retry's `inputResponses`/`requestState` via the session extensions (same as tools); prompts receive them in the render args under reserved `io.modelcontextprotocol/*` keys, because `McpPrompt::render` has no session parameter and changing it would break the public trait (documented on the trait; reserved-namespace keys cannot collide with wire prompt arguments, which are plain strings). Tests: two-leg round trips for both methods + a `-32003`/400 capability-gate case on `resources/read` (all real-HTTP; the handlers previously leaked the sentinel to `-32603`).
- **`resources.subscribe` capability truthfulness** — with `subscriptions/listen` serving per-URI `resources/updated`, both capability-construction sites now advertise `subscribe: true` on the 2026 lane (still `false` on 2025, which has no `resources/subscribe` handler). Wire test asserts the `server/discover` advertisement.
- **`completion/complete` e2e coverage** — sessionless dispatch + `CompleteResult` wire shape (`completion.values`) + capability advertisement, closing the last zero-e2e core method on the 2026 path.

### Fixed (2026-06-10, release-prep sweep)

- **Stale crate-doc versions**: 41 dependency-snippet strings across 15 non-frozen crate READMEs/lib-docs said `"0.3"`; all now `"0.4"` (the frozen 2025 protocol snapshots and the terminal-0.3.x shim correctly keep `"0.3"`).
- **Legacy prose labeled in default-lane docs**: `turul-http-mcp-server` README's session/SSE-resumability features and curl examples, and `turul-mcp-server` README's strict-lifecycle note, are now explicitly marked *2025-11-25 opt-in lane* (the 2026 default is POST-only, non-resumable, handshake-free).
- **Comment hygiene in touched 2026 paths**: internal-phase tags, fix-history phrasing, and the `subscriptions.rs` module narrative replaced with present-tense, spec-anchored descriptions.

### Fixed (2026-06-10)

- **`turul-mcp-oauth` CIMD/DCR posture dispositioned (docs/tests-only by design).** Audited against the live draft authorization spec: Client ID Metadata Documents are a SHOULD for *authorization servers and MCP clients*; Dynamic Client Registration is deprecated upstream (MAY, AS back-compat; not removed — earliest removal 2027-07-28). This crate implements the resource-server role only — RFC 9728 Protected Resource Metadata and OAuth 2.1 §5.2/RFC 8707 token validation, both unchanged — so no CIMD or DCR surface belongs in it and none was invented. The role posture is now documented in the crate header, and a wire-shape test pins that the published RFC 9728 document carries no client-registration keys (`registration_endpoint`, `client_id*`, `redirect_uris`, …). Client-side CIMD belongs to a future full MCP OAuth client flow.
- **Builders/derive schema pipeline is lossless on the 2026 path.** Two defects destroyed JSON Schema 2020-12 fidelity between a tool author's types and `tools/list`: (1) `ToolSchema::from_schemars` stripped `$defs`/`definitions` from the root while passing properties through verbatim — every `#/$defs/X` pointer dangled; the 2026 root now RETAINS `$defs`/`definitions`/`$schema` (the 2025 typed lane keeps its inline-resolution and stripping). (2) The derive macros funneled schemars-generated parameter and output schemas through the typed-enum converter, silently collapsing data-bearing unions (`oneOf` + `const` tags → bare `{"type":"object"}`) and other 2020-12 compositions. New lane-aware `turul_mcp_builders::schemars_param_schema`: on 2026 it inlines local `$ref`s (cycle-guarded `resolve_local_refs`; `$ref` siblings compose via `allOf`) and carries the result verbatim via the new transparent `JsonSchema::Raw` variant (untagged escape hatch on the 2026 typed enum — also the deserialize fallback for subschemas the structured variants reject); on 2025 it is the status-quo typed conversion. **Documented limitations with rejection tests** (not silent loss): cyclic `$ref`s cannot be inlined into a property subschema (error names the cycle; restructure the type or use a root `from_schemars` document), and non-local/network `$ref`s are rejected per the spec's no-auto-deref rule. Tests: 7 builders fidelity tests (nested `$defs` inlining with enum/required intact, tagged-union `oneOf` survival, composition-keyword verbatim round-trip, cycle/non-local rejection, `$ref`-sibling `allOf` composition, root `$defs` retention) + 2 real-HTTP e2e (`schema_fidelity_2026.rs`: a derived tool's tagged-union param and schemars output reach `tools/list` undamaged with no dangling `$ref`; `tools/call` `structuredContent` satisfies the ADVERTISED `outputSchema` wrapper field discovered from `tools/list`). Revert-and-fail: with the 2026 arm forced through the old converter, the tagged-union test fails showing the exact loss (`"shape":{"type":"object"}`) — recorded. No public macro/builder API shape changed.
- **Protocol-fidelity sweep, part 2 — `ttlMs` as a schema `number` + SEP-2577 marker absorption.** (a) `CacheableResult.ttlMs` (and its embeddings in the tools/resources/prompts/discover results) is now `f64` per the schema's `number` type: fractional values are accepted on deserialize and survive round trips, negative/non-finite values reject (`@minimum 0`), and whole values keep the compact integer wire form (byte-stable for the common `ttlMs: 0` case). (b) The re-pinned schema's SEP-2577 deprecations are now fully absorbed as `#[deprecated]` markers: `LoggingLevel` (+ `LogLevel` alias), the per-request `_meta` `logLevel` key and `RequestMetaObject.log_level`/`with_log_level`, `ServerCapabilities.logging`, `ModelHint`/`ModelPreferences`/`ToolChoice`, the `ContentBlock::ToolUse`/`ToolResult` variants and constructors, and the sampling trait surface (`HasCreateMessageRequestParams`/`CreateMessageRequest`/`CreateMessageResult`/`HasLevelParam`). The earlier rustdoc claim that `LoggingLevel`/`logLevel` were "the non-deprecated replacement" was wrong against the re-pin and is corrected — the whole Logging surface (including the per-request opt-in this branch implements) is deprecated-but-normative through the migration window. Framework-internal use sites carry scoped `#[allow(deprecated)]` (the framework intentionally serves the surface through the window); downstream consumers now get compiler nudges.
- **Protocol-fidelity sweep, part 1 (wire/type drift vs the pinned schema).** (a) `ToolChoice` no longer carries a non-spec `name` field on the wire (the `specific()` constructor is gone) and `mode` is optional per schema (`{}` parses; absent means `"auto"`; `effective_mode()` helper). (b) `PromptReference` is `BaseMetadata`-shaped: gains `title`, drops the non-spec `description`. (c) `Annotations.audience` is the closed `Role[]` union instead of `Vec<String>` — wire-invalid values like `"system"` are now rejected at parse time; the builders' `annotation_audience` takes `Role` (converted to strings on the frozen 2025 lane). (d) The duplicate `Role` binding is gone — `sampling::Role` re-exports the single `prompts::Role`. (e) `LoggingCapabilities`/`CompletionsCapabilities` match the schema's opaque `JSONObject`: the invented `enabled`/`levels` keys are removed from the bindings and from both server builders' capability advertisements (presence of the object is the signal). 5 new wire-shape contract tests in `compliance.rs`; existing tests migrated with the contract (e.g. the empty `ToolChoice` parse fails against the pre-fix required-`mode` binding).

- **2025 opt-in lane build regression (same day, pre-push).** The elicitation enum-union slice used the 2026-only union accessors in `turul-mcp-builders` code that also compiles under `protocol-2025-11-25`, breaking the opt-in lane builds (caught by `scripts/ci-gates.sh all`). The validation is now `#[cfg]`-split per lane.
- **`tools/list` now advertises `outputSchema` on the 2026 path.** The 2026 `ToolDefinition::to_tool()` hardcoded `output_schema: None` (a type-bridge gap: the trait returns the object-rooted `ToolSchema`, the 2026 wire type is the free-form `ToolOutputSchema`), so no tool could ever advertise its output contract and clients had no way to know `structuredContent` conformance applied. Bridged via a lossless serde round-trip. Real-HTTP test asserts the derive-declared `output = String` schema appears in `tools/list` (failing pre-fix).
- **Elicitation enum schemas no longer lose constraints through the untagged unions.** `PrimitiveSchemaDefinition`'s untagged deserialize matched `{type:"string", enum:[...]}` against `StringSchema` first, silently DROPPING the `enum` (and `enumNames`/numeric bounds in the analogous cases). The primitive structs (`StringSchema`/`NumberSchema`/`BooleanSchema`) and untitled select shapes now carry `deny_unknown_fields`, so each payload lands on its precise variant. The schema's enum union is now bound faithfully: new `EnumSchema` = `SingleSelectEnumSchema | MultiSelectEnumSchema | LegacyTitledEnumSchema` (upstream order); the old struct misusing the `EnumSchema` name is renamed `LegacyTitledEnumSchema` and gains its missing `default` field; union helpers (`new` → spec-pure untitled single-select, `allowed_values()`, `is_multi_select()`) keep the builders API working, and the elicitation builder validates multi-select array submissions. 7 new round-trip fidelity tests (incl. through `ElicitationSchema.properties` — the previously untested path); revert-and-fail: 5 of 7 fail with `deny_unknown_fields` removed from `StringSchema` alone.
- **`resources/read` results default to `cacheScope: "private"`.** Read contents routinely depend on the authenticated user; the previous blanket `public` default invited shared caches to serve one user's resource to another (the caching guidance's exact warning). List results keep `public`; user-independent read results opt back in via `with_cache()`. Contract test pins the default.

### Added (2026-06-10)

- **Per-request log gating (2026-07-28).** `notifications/message` is now opt-in per request: a `tools/call` whose `_meta` lacks `io.modelcontextprotocol/logLevel` gets NO message notifications (spec MUST), and the declared level is the severity threshold (replaces the removed `logging/setLevel` session threshold, which remains the filter on the 2025 lane). The tools/call handler surfaces the declared level to the session context; `notify_log` gates emission. **Also fixes a pre-existing POST-SSE ordering race**: the final response frame could beat (and the shutdown path silently DROP) request-scoped notifications already queued on the progress channel — the progress task now flushes queued events on shutdown and the final frame is sent only after the flush handshake, so notifications precede the final response on the wire. Tests: `log_gating_2026.rs` (real-HTTP SSE: suppressed without `logLevel`, delivered with `"info"`, filtered below an `"error"` threshold; the opt-in case failed against the pre-fix ordering — revert-and-fail evidence), wired into the default CI lane.
- **SEP-2243 `Mcp-Param-*` custom-header mirroring (client emission + server validation).** Completes the deferred remainder of the request-metadata headers work. Protocol crate: pure SEP-2243 logic in `headers.rs` — `scan_x_mcp_headers()` (annotation discovery at any nesting depth with the full constraint set: non-empty, `tchar` syntax, case-insensitive uniqueness, string/integer/boolean only), `encode_param_value()` / `decode_param_value()` (string/integer/boolean conversion, JS safe-integer range, `=?base64?…?=` sentinel incl. the self-matching-sentinel re-encode rule; unit tests reproduce all five spec encoding examples). Server: the tools/call handler validates every annotated parameter's mirrored header against the body argument (sentinel-decoded; integers compared numerically) — value-without-header, header-without-value, or decoded mismatch → `-32001 HeaderMismatch` at HTTP 400 (the transport surfaces request headers to handlers via the rpc session metadata; the inline 2026 JSON path now maps `-32001` to 400 alongside `-32003`). Client: `tools/list` rejects tool definitions with invalid `x-mcp-header` values (excluded + warning, per spec) and captures per-tool bindings BEFORE the 2025-vocabulary remap (which cannot carry the annotation); `tools/call` mirrors annotated arguments into `Mcp-Param-{name}` headers via a new `Transport::send_request_with_extra_headers` (default delegates to `send_request` — non-HTTP transports MAY ignore the annotations). Tests: `mcp_param_2026.rs` (4 real-HTTP server cases; revert-and-fail recorded — both negative cases fail with validation disabled) and a closed-loop client e2e (the validating server rejects missing mirrors, so the green client call proves emission; covers plain ASCII and Base64-sentinel values). Not implemented (acceptable per spec): the client's optional schema-stale auto-retry (`tools/list` + retry on rejection is left to the application).
- **Schema pin re-vendored (content sha256 `1bf94a60…`, fixture pin `1304c8fe`).** One substantive upstream change: `ElicitationCompleteNotificationParams` extracted into a named interface extending `NotificationParams` (surface 159 → 160). The Rust binding already modeled the optional `_meta`, so the previously recorded deviation resolved upstream. ADR-027 revision log + COMPLIANCE.md/schema README hash records updated.
- **Docs/ADR reconciliation to implemented behavior**: ADR-029 §CI surface rewritten to the as-built lanes (the prescribed `cargo test --workspace` matrices never compiled — spec-pinned workspace members trip the ADR's own mutex; per-crate matrix is the operative shape); ADR-025 revision entry recording the shim's branch reality (still consumed by four non-frozen crates; manifest carries an unpublishable 0.4.0 from the version sweep; the framework-wide cut remains 0.4.0 release-prep work); `docs/plans/2026-07-28-schema-coverage-matrix.md` marked STALE/superseded (authoritative coverage = COMPLIANCE.md + the compliance harness); COMPLIANCE.md elicitation-union and extension-crate-name notes refreshed.
- **MRTR (SEP-2322): `InputRequiredResult` production (server) and consumption (client).** Server: a tool returning the new `McpError::InputRequired { input_requests, request_state }` sentinel (2026 protocol crate; NOT a wire error — the only return channel available to tool impls) is converted by the `tools/call` handler into a successful `InputRequiredResult` (`resultType: "input_required"`), after enforcing that every input request targets a capability the client declared in that request's `_meta` `clientCapabilities` — undeclared → `-32003 MissingRequiredClientCapability` at HTTP 400 (the 2026 JSON-framed response path now dispatches inline so the HTTP status can reflect the JSON-RPC outcome; SSE-framed responses inherently stay 200). On the retry leg, `CallToolRequestParams.inputResponses`/`requestState` are surfaced to tools via `SessionContext::input_responses()` / `mrtr_request_state()` (requestState documented as attacker-controlled). Client: `call_tool` surfaces `resultType: "input_required"` as the new `McpClientError::InputRequired` carrying `inputRequests`/`requestState`; the application gathers inputs and retries via `call_tool_with_input_responses(name, args, responses, request_state)` (fresh JSON-RPC id; 2026 connections only). New `ClientConfig.declared_capabilities` (elicitation/sampling/roots, all off by default) feeds both the 2026 per-request `_meta` and the 2025 `initialize` capabilities — previously hardcoded empty. Tests: `mrtr_2026.rs` (real-HTTP two-leg round trip + `-32003`/400 capability rejection; the suite does not compile against the pre-slice tree — the sentinel variant did not exist) and a full client-driven MRTR e2e in `e2e_2026_real_server.rs`. Limitations (tracked): MRTR production is wired for `tools/call` only (`resources/read`/`prompts/get` handlers have no input hooks yet); the framework does not generate or verify `requestState` integrity (HMAC is the tool author's concern).
- **Unknown-method mapping on the 2026 path: HTTP 404 + JSON-RPC `-32601`.** A request for a method the server does not implement now returns `404 Not Found` with a `-32601` body (the body distinguishes this from a legacy HTTP+SSE server's 404), checked pre-dispatch against the dispatcher's registered methods (the streaming architecture commits the status before dispatch completes, so the check cannot ride the dispatch result). Methods absent from the pinned 2026-07-28 schema — `ping`, `initialize`, `tasks/*`, `logging/setLevel`, `resources/subscribe` — are never registered on a 2026 build and land here. The 2025-era sessionless-ping bypass is now `protocol-2025-11-25`-only (it previously let `ping` dodge header validation and answer 200/`-32601` on the 2026 path). Tests: `error_mapping_2026.rs` (3 real-HTTP cases incl. a sweep over the absent methods; failing pre-fix — revert-and-fail evidence), wired into the default CI lane. With this, the 2026 error/status contract is: 401/403 auth (middleware) → `-32004` unsupported version (400) → `-32001` header mismatch (400) → `-32602` missing/incomplete `_meta` (400) → 404/`-32601` unknown method → dispatch.
- **SEP-2243 request-metadata headers enforced (server) and emitted (client) on the 2026 path.** Server (`turul-http-mcp-server`, §Server Validation): every POST must carry `MCP-Protocol-Version` (a 2026-only build supports no pre-2025-06-18 clients, so an absent header is rejected) and `Mcp-Method` matching the body method; `tools/call`/`prompts/get` (`params.name`) and `resources/read` (`params.uri`) additionally require a matching `Mcp-Name`. Failures → HTTP 400 + JSON-RPC `-32001 HeaderMismatch` (id-less for notifications). A requested version this build does not implement → HTTP 400 + `-32004 UnsupportedProtocolVersionError` with `data.supported`/`data.requested` (previously never emitted); the header/body `_meta` protocolVersion disagreement moved from `-32602` to `-32001` (it is a header-validation failure). The 2026 build now routes ALL requests to the streamable handler — a legacy version header can no longer detour into the 2025-era session handler around version validation (`server.rs`). Client (`turul-mcp-client`): 2026 connections mirror `method` into `Mcp-Method` and `params.name`/`params.uri` into `Mcp-Name`; the `server/discover` probe now advertises `MCP-Protocol-Version: 2026-07-28` (header must match its 2026 `_meta` — the old legacy-header probe is rejected by a validating 2026 server) and the fallback arm restores the 2025 header before `initialize`; a 400 whose body is a JSON-RPC error surfaces its code to the negotiation classifier, and `-32004` now triggers the 2025 fallback (structured negotiation signal; bare 4xx still aborts). `headers.rs` (protocol crate) rewritten to the live-draft wire shape: `x-mcp-header` is a schema annotation key, the wire header is `Mcp-Param-{name}` (+ `=?base64?…?=` sentinels), and `-32001` gets a named constant — replacing the incorrect `x-mcp-header-<name>` wire-prefix constant. Tests: `mcp_headers_2026.rs` (9 real-HTTP enforcement cases, 7 failing pre-fix), `e2e_2026_real_server.rs` (bilingual client ↔ real in-process 2026 server: negotiation + tools round-trip, failing pre-fix on the probe header), strengthened wiremock matchers (stubs now require the headers), `-32004` classifier unit test. Existing 2026 suites migrated to send the now-mandatory headers. Not yet done (tracked): `Mcp-Param-*` emission/validation (requires `x-mcp-header` inputSchema scanning), client-side `subscriptions/listen` API.
- **`turul-mcp-client` no longer depends on the `turul-mcp-protocol` alias** — closing the ADR-030 drift. The frozen `turul-mcp-protocol-2025-11-25` crate is now an unconditional dependency serving as the public type vocabulary; `turul-mcp-protocol-2026-07-28` stays feature-gated. This is load-bearing beyond hygiene: the alias pin (`protocol-2025-11-25`) made any dependency graph containing both the client and a 2026-default server trip the ADR-029 spec mutex — which is exactly what blocked the new real-server e2e test. Narrowing features now control which wire paths compile (`client-2025-11-25-only = []`). See ADR-030 revision log (2026-06-10).
- **Server-side `subscriptions/listen` (2026-07-28).** The stateless transport now serves the Subscriptions pattern that replaced the GET notification stream and the `resources/subscribe` RPC: a `subscriptions/listen` POST opens a long-lived SSE stream whose first message is `notifications/subscriptions/acknowledged` echoing the honored filter subset (requested types without a corresponding server capability section are omitted); only opted-in types are delivered (`toolsListChanged`/`promptsListChanged`/`resourcesListChanged` plus per-URI `resourceSubscriptions` filtering of `notifications/resources/updated`); every delivered notification is stamped with `io.modelcontextprotocol/subscriptionId` in `_meta`, set to the JSON-RPC id of the listen request. Delivery is gated at the broadcast layer (subscription registry entry created even for an empty filter) with a per-URI + type filter at the stream layer; the client cancels by closing the stream. Real-HTTP acceptance suite `subscriptions_listen_2026.rs` (3 tests: ack-first + cross-request broadcast delivery with filtering, SSE-Accept required, unsupported-type omission in the ack; all failing pre-implementation — revert-and-fail evidence) wired into the default CI lane. Not yet done: capability advertisement reconciliation (`resources.subscribe`) and the client-side listen API.

### Fixed (2026-07-13)

- **Bilingual client: duplicate `Mcp-Name` header on `call_tool_or_task`; streaming-path 400 rejections buried.** (F1) `call_tool_or_task` (Tasks extension) passed an explicit *raw* `Mcp-Name` extra-header while the transport also derives and Base64-sentinel-encodes one from `params.name` — reqwest appends, so two conflicting `Mcp-Name` headers hit the wire (a `.get()`-based server silently took the first, hiding it). The explicit header is removed; the transport's encoded one is authoritative. Wire test `call_tool_or_task_emits_exactly_one_encoded_mcp_name_header` inspects `received_requests` (a real server can't observe the duplicate); revert-and-fail recorded (the failure shows both `["=?base64?IHBhZGRlZCA=?=", "padded"]` on the wire). (F3) `send_request_streaming` (used by `subscriptions/listen`) converted every non-2xx into a transport error without parsing the body, so a spec-compliant `400` + JSON-RPC error (e.g. `-32021` MissingRequiredClientCapability) never surfaced its code. New `classify_non_2xx` applies the same status-400-only JSON-RPC-envelope rescue as the ordinary request path. Test `subscriptions_listen_400_surfaces_jsonrpc_error_not_transport_error`; revert-and-fail recorded. See the draft-migration audit §8.
- **Client now Base64-sentinel-encodes `Mcp-Name` header values that are not safely plain ASCII** (SEP-2243 §Value Encoding MUST; the client-side mirror of the server-side decode fix below). `apply_request_metadata_headers` previously set `Mcp-Name` from the raw `params.name`/`params.uri` string — a padded or non-ASCII tool name either failed the server's header validation or was rejected by the HTTP layer outright. Now routed through `encode_param_value` (verbatim pass-through for plain values). Wire test `mcp_name_header_is_base64_sentinel_encoded_when_not_plain_ascii` matches only the encoded form on the wire; revert-and-fail recorded.
- **Bilingual client: bare `-32022` no longer triggers legacy fallback; plain-JSON 400 error envelopes now reach JSON-RPC error classification.** (1) `classify_probe` sent an `UnsupportedProtocolVersionError` carrying NO `data.supported` list to `FallbackTo2025` — but a recognized modern error identifies a modern server, and with no server-named list there is no "mutually supported version from the supported list" to select; inferring a downgrade to `initialize` was exactly the Versioning §Backward Compatibility anti-pattern. It now aborts with the reason. The structured sub-case (`data.supported` naming `2025-11-25` → fall back and lock 2025-11-25) is unchanged and compliant — the spec's own error example lists `"2025-11-25"` inside `data.supported`. Test: `unsupported_protocol_version_with_no_list_aborts_without_downgrade`; revert-and-fail recorded. (2) `HttpTransport` treated every non-2xx as a transport failure without reading the body, so a spec-compliant plain-JSON `400` carrying `-32020`/`-32021`/`-32022` never reached `ServerError`-keyed logic — including the Mcp-Param refresh-retry. New `rescue_400_jsonrpc_envelope` surfaces a parseable JSON-RPC error envelope from status 400 (only) on the two JSON-RPC request senders; 404/auth statuses keep transport semantics (session-expiry recovery keys on 404 — regression-guarded). The discover probe uses a separate transport path and is untouched. Tests: `call_tool_recovers_from_plain_json_400_header_mismatch` (revert-and-fail recorded), `http_404_with_json_body_stays_a_transport_error`. See the draft-migration audit §7.
- **Server-side `Mcp-Name` header validation now decodes Base64-sentinel values before comparing.** Streamable HTTP §Server Validation MUST: "servers MUST decode an encoded `Mcp-Name` or `Mcp-Param-{Name}` value before comparing it to the corresponding request body value." The `Mcp-Param-*` half already did this; `Mcp-Name` compared the raw header to the body value with plain `!=` (streamable_http.rs ~1477-1494), so a Base64-sentinel-encoded `Mcp-Name` was falsely rejected as a mismatch. `Mcp-Name` now routes through the same `decode_param_value` path as `Mcp-Param-*`, including its decode-failure semantics. New wire tests in `crates/turul-mcp-server/tests/mcp_headers_2026.rs`: `base64_encoded_mcp_name_decodes_and_matches` (revert-and-fail leg recorded) and `base64_encoded_mcp_name_mismatch_is_rejected` (asserts `-32020`). Ride-along fix: a stale comment at `handlers/mod.rs:51` said `-32003`; corrected to the code's actual `-32021`.
- **Bilingual client: two wire-contract regressions from the upstream 2026-07-02 error-code renumbering.** (1) `McpClient::call_tool`'s 2026-lane Mcp-Param mismatch auto-retry was still keyed on the pre-renumbering `-32001`; it now keys solely on `ERROR_CODE_HEADER_MISMATCH` (`-32020`). New wire test `call_tool_recovers_from_header_mismatch_with_one_refresh_and_retry` (wiremock) asserts exactly one `tools/list` refresh plus one retry; revert-and-fail recorded. (2) The bilingual era-classifier's `UnsupportedProtocolVersionError` constant was still the pre-renumbering `-32004`; it now recognizes `-32022` only, with **no legacy alias** — `-32001`/`-32004` are implementation-defined-range codes post-renumbering, and this framework's own middleware already emits `-32001` for `Unauthenticated`, so aliasing either would misclassify a real auth failure as a version signal. New negative tests `pre_renumbering_32004_is_unrecognized_and_aborts` (unit) and `bilingual_client_treats_pre_renumbering_32004_as_unrecognized_and_aborts` (wire); the existing fallback test renamed to `bilingual_client_falls_back_on_400_with_32022_body`; revert-and-fail recorded. (3) `classify_probe`'s recognized-modern-error set is now written out explicitly — `-32022` (unsupported version), `-32021` (`MissingRequiredClientCapabilityError`), `-32020` (header validation) — per the live Streamable HTTP prose; the `-32020`/`-32021` arms are behavior-preserving (the catch-all already aborted on them, verified before adding the named arms) and are not claimed as bug fixes — they name the contract so a future edit can't accidentally add a downgrade arm for them. Root cause of non-detection: `bilingual_negotiation.rs` mocked the pre-renumbering codes (a stale "faithful mock"), and the real-server e2e suite never exercised a version mismatch. See ADR-030 revision log (2026-07-13).
- **OUTSTANDING.md compliance punch-list burned down** (schema pin `6e4cba2d…` re-verified byte-identical upstream first — findings actioned as written). Wire/contract fixes in `turul-mcp-protocol-2026-07-28`: `RequestMetaObject` gained the hand-written `Serialize` collision guard (a caller-populated `extra` entry can no longer duplicate `progressToken` or an `io.modelcontextprotocol/*` reserved key on the wire — same pattern as `SubscriptionsListenResultMeta`); `ElicitResult.content` values retyped from `Value` to `ElicitResultValue` (`string | number | boolean | string[]` per schema, object values now rejected on deserialize; builders keep their `Value`-based public signatures via an infallible feature-gated conversion); `CompletionReference` deserialize now dispatches strictly on the `type` discriminator (`ref/resource` / `ref/prompt`; unknown or missing → rejected, previously silently matched structurally); `CompleteResult.completion.total` `u32`→`f64` (schema `number`; `CompletionHandler` cfg-split since the frozen 2025-11-25 crate keeps `u32`); `RootsCapabilities` reduced to the schema's empty object (removed the no-op `listChanged` field); `CreateMessageRequestParams.metadata` retyped `Option<Value>`→`Option<HashMap<String, Value>>`; dead non-spec `SamplingRequest`/`SamplingResult` removed. Server: under `protocol-2026-07-28`, `notifications/progress` and `notifications/message` are no longer registered as inbound client-notification handlers (both are absent from the schema's `ClientNotification` union; HTTP 202 semantics unchanged, outbound server→client progress untouched, 2025-11-25 lane unchanged — `notifications_2026.rs`). Tests: prompt-shape compliance assertions deepened to full field coverage; `DRAFT-2026-v1` fixture strings modernized to `"2026-07-28"`; new round-trip/rejection/collision suites, each behavior change with revert-and-fail evidence. Comment/doc corrections across content.rs / caching.rs / tools.rs / meta.rs / notifications.rs / completion.rs per the repo comment rules. `completion.values` `@maxItems 100` closed by disposition: enforcement already lives (tested) in the server dispatch layer; constructor-level truncation would lossily pre-empt the `total` count. Sole surviving punch-list item: `SubscriptionsListenResult` graceful-close emission (needs shutdown-signal infrastructure; spec-legal as-is). See OUTSTANDING.md §Addendum 2026-07-13.
- **Lambda dispatcher method-registration parity sync.** `turul-mcp-aws-lambda`'s registered JSON-RPC method set now mirrors the non-Lambda `McpServerBuilder` authority (`crates/turul-mcp-server/src/builder.rs`, `server.rs`) per feature lane. Added `server/discover` (`protocol-2026-07-28`) and `notifications/cancelled` (both lanes, accept-and-ignore parity per "Invalid cancellation notifications SHOULD be ignored"). Gated to `protocol-2025-11-25`: `ping`, `roots/list`, `notifications/message`, `notifications/progress`, `notifications/roots/list_changed` (+ legacy camelCase alias), and the `notifications/initialized` dispatcher registration. Removed the unconditional `completion/complete` mock from `new()` — an unconfigured Lambda server now answers `-32601`, matching the existing build()-time provider-backed registration. New crate-private `registered_methods()` parity probe plus wire tests assert the registered method set against an explicit spec-derived expectation per lane. Also reviewed the live draft's newly formalized dual-era Backward-Compatibility guidance (Modern/Legacy/Dual-era terminology, compatibility matrix, "a dual-era server MAY serve both eras concurrently"); single-spec-per-build remains intentional — the MAY is declined. Upstream basis re-verified: commit `93671a3f2bac3bc11b0eb6327c2d029e272b2871`, schema sha256 `6e4cba2d17f7156877357762b6b4b63cd790d8973f61ec35ab73cd61ad67017d` (byte-identical to the vendored pin). See ADR-001 / ADR-029 revision logs (2026-07-13).
- **Lambda transport routed the 2026-07-28 lane to the sessionless legacy handler — tools received `session: None`.** `LambdaMcpHandler::handle()` (buffered, non-streaming Lambda runtime) delegated every request to `SessionMcpHandler`, and `handle_streaming()` trusted the request's `MCP-Protocol-Version` header for routing — but on a `protocol-2026-07-28` build only `StreamableHttpHandler` mints the stateless core's internal per-request session and enforces SEP-2243 Server Validation. Result: a middleware-authenticated `tools/call` over Lambda reached the tool with no `SessionContext` (field symptom: `-32602 "session required"` even though `SessionInjection::set_state` had run), and the buffered lane bypassed header validation, the POST-only surface, and unknown-method 404 mapping entirely. Both entry points now mirror `server.rs`: under `protocol-2026-07-28` all requests go to the streamable handler (the JSON-framed 2026 path returns buffered bodies, so the non-streaming Lambda runtime is unaffected); the `protocol-2025-11-25` lane keeps its existing unconditional `SessionMcpHandler` delegation for `handle()`, and header-based routing for `handle_streaming()`, unchanged. Closes the ADR-029 §Consequences "Lambda simplification" drift (one Lambda = one feature = one protocol). Tests: `stateless_session_2026_07_28.rs` (production path `LambdaMcpServerBuilder → handler() → handle()`: middleware-injected state readable via `SessionContext::get_typed_state` in the tool, missing `MCP-Protocol-Version` → 400/`-32020`, middleware `Unauthenticated` short-circuit → `-32001`; all three fail with the routing fix reverted — revert-and-fail recorded). `middleware_parity.rs` (drives the `initialize`/`Mcp-Session-Id` lifecycle) is now gated `protocol-2025-11-25` — it only passed on the 2026 default because of the wrong legacy routing.

### Changed

- **2026 HTTP surface gated to POST-only (2026-06-10).** Under the `protocol-2026-07-28` default, the MCP endpoint now answers legacy-era traffic per the Streamable HTTP binding's Backward Compatibility rules: HTTP GET (the removed standalone SSE stream) and DELETE (the removed session termination) return `405 Method Not Allowed` with `Allow: POST, OPTIONS`; an inbound `Mcp-Session-Id` header is ignored at parse time (never honored as a session, never echoed — internal per-request sessions are no longer client-pinnable); `Last-Event-ID` is ignored (streams are not resumable in this revision); notification `202 Accepted` responses no longer carry a session header. The 2025-11-25 opt-in lane keeps the full stateful GET-SSE/session surface unchanged. New real-HTTP acceptance suite `stateless_2026_http_surface.rs` (5 tests, all failing pre-fix — revert-and-fail evidence) wired into the default CI lane (`ci.yml` + `scripts/ci-gates.sh`).
- **Default spec flipped to 2026-07-28 — the 0.4 cutover landed (branch-scoped, 2026-06-07).** `crates/turul-mcp-protocol/Cargo.toml` now declares `default = ["protocol-2026-07-28"]`; the alias re-exports the 2026-07-28 crate by default and `protocol-2025-11-25` is the opt-in escape hatch (`--no-default-features --features protocol-2025-11-25`). The protocol feature topology cascaded through every framework crate (`turul-mcp-session-storage`/`-task-storage`/`-builders`/`-derive`/`turul-http-mcp-server`/`turul-mcp-server`/`turul-mcp-aws-lambda`), each forwarding the spec choice. `ToolBuilder` and dynamic-tools were adapted to the 2026 result types (`resultType` + `CacheableResult` `ttlMs`/`cacheScope`) and work under the 2026 default. The bilingual `turul-mcp-client` builds under the 2026-default workspace while still speaking either spec per connection. Tasks are gated to the 2025-11-25 opt-in (`#[cfg(feature = "protocol-2025-11-25")]`) — under the 2026 default tasks are an extension (ADR-028). Example fleet migrated: **43 examples on the 2026 default**, 8 redundant duplicates removed (builders-showcase, comprehensive-server, sampling-with-tools-showcase, task-types-showcase, client-task-lifecycle, dynamic-tools-test-client, performance-testing, lambda-mcp-server-streaming), and a small 2025-11-25 regression suite pinned (tasks-e2e pair + logging/sampling/elicitation/client/lambda examples held at the 2025 opt-in); the integration-test crates are pinned to the 2025-11-25 opt-in. Default-members build is green at 0 warnings; the `compile_error!` mutex fires correctly under both feature configurations. Not merged to `main`. See ADR-027 / ADR-029 revision logs (2026-06-07).

- **Per-crate independent versioning policy adopted.** Every non-frozen crate's `Cargo.toml` migrated from `version.workspace = true` to a literal `version = "0.4.0"`. After this cut, individual crates may patch and publish independently — bump only the crate that changed, not the whole workspace. `[workspace.package].version` remains for tooling compatibility but is no longer authoritative. `[workspace.dependencies]` pins each internal crate path to its current literal version.
- **Frozen historical protocol crates** `turul-mcp-protocol-2025-06-18` and `turul-mcp-protocol-2025-11-25` received a one-time literal `version = "0.3.47"` pin in their respective `Cargo.toml` files. Without this they would inherit the new `[workspace.package].version = "0.4.0"` and silently bump the published version of crates that are explicitly frozen against historical spec snapshots. No source files were touched in either frozen crate. See ADR-027 §"Revision log" entry **2026-05-31** for the one-time-exception record.
- **`turul-mcp-json-rpc-server` is now a compatibility shim** re-exporting `turul-rpc 0.1`. New code should depend on `turul-rpc` directly (the 2026-07-28 protocol crate already does, isolated to `0.2.2` via a per-crate dep override). The shim continues to satisfy the rest of the framework on 0.1 through the 0.3.x line; framework-wide cutover is deferred to a later slice. See ADR-025.

### Notes for downstream consumers

- `turul-mcp-protocol` (the active-spec re-export alias) is now a **feature-gated re-export defaulting to `protocol-2026-07-28`**, with `protocol-2025-11-25` as the opt-in (`--no-default-features --features protocol-2025-11-25`). Phase 9.4 (the flip + every consumer crate migrating to the forwarding-feature topology) **has landed on this branch** across `turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`, `turul-mcp-builders`, the derive macros, and the migrated example fleet (43 examples on the 2026 default; 8 redundant examples removed; a small 2025-11-25 regression suite pinned). Publication to crates.io is still gated per ADR-027 (upstream final-spec publication + maintainer go-ahead); a full workspace `--no-default-features --features protocol-2025-11-25` CI matrix is the remaining coverage item.
- Branch lock: the `2026-07-28-MCP-Specification` branch remains unmerged from `main`. Pulling `main` against 0.4.0 gives a working tree that still ships MCP 2025-11-25 on the wire.

## [0.3.47] - 2026-05-23

### Fixed

- **`turul-http-mcp-server` returned HTTP 401 for missing `Mcp-Session-Id` instead of the spec-required HTTP 400.** MCP 2025-11-25 § Session Management states: *"Servers that require a session ID SHOULD respond to requests without an MCP-Session-Id header (other than initialization) with HTTP 400 Bad Request."* Two code paths were affected:
  - **Streamable HTTP POST non-initialize, non-allowed-ping**: `crates/turul-http-mcp-server/src/streamable_http.rs:1347-1373` was returning `StatusCode::UNAUTHORIZED`. Now returns `StatusCode::BAD_REQUEST`. The pre-init ping bypass at line 1174 is preserved; with `allow_unauthenticated_ping=false`, sessionless ping rejection also lands in this path and correctly returns 400 (same missing-header contract). Stale comment at line 296 documenting the bug as if it were spec is corrected.
  - **Legacy `session_handler.rs` GET SSE (protocol ≤ 2024-11-05)**: `crates/turul-http-mcp-server/src/session_handler.rs:864-870` was returning HTTP 200 with a JSON-RPC error body via `jsonrpc_error_to_unified_body` (which hardcodes 200). The JSON-RPC error body shape is preserved, but the response is now wrapped in a 400 status instead of 200. Cross-transport consistency with the Streamable HTTP path.
- **Streamable HTTP GET and DELETE** were already returning 400 for missing session (`streamable_http.rs:546` and `:1083`); no code change needed on those paths — only their test assertions were tightened (the GET test was tolerant of either 400/401; now requires 400).
- **Test compliance**: per CLAUDE.md §"Test Compliance" ("Tests validate the MCP spec — never change tests to preserve buggy behavior"), four test files were updated from asserting `401` to asserting `400`:
  - `tests/session_id_compliance.rs` (6 assertions + 2 test renames + header comment)
  - `tests/mcp_behavioral_compliance.rs` (sessionless-non-ping-rejected assertion, sessionless-ping-with-flag-off assertion, plus a new regression test for the legacy GET SSE missing-session path that pins both HTTP status and JSON-RPC envelope body)
  - `tests/streamable_http_e2e.rs` (POST hard assertion + GET tightened-tolerant assertion + stale comments)
  - `tests/phase5_regression_tests.rs` (line 136 assertion)
- **CLAUDE.md §"Session Status Codes" table** updated to reflect the spec-correct mapping, including the ping/`allow_unauthenticated_ping` interaction and an explicit row for the legacy SSE path.

### Versioning rule override

This is an MCP transport contract correction. By the prior versioning rule ("Minor bumps cover A2A/MCP/schema contract changes") it would have been a minor (`0.4.0`) bump. We ship it as a patch (`0.3.47`) because:

1. The change brings the framework into compliance with an existing spec, not adoption of a new spec revision; existing-spec compliance corrections are bug fixes by nature.
2. The user-global versioning rule has been updated to: patch bumps cover bug fixes, contract corrections, and spec-compliance fixes; minor bumps are reserved for new MCP spec adoption or explicit instruction.
3. Observable client impact is minimal: any conforming MCP 2025-11-25 client already handles 400 for missing session per spec; the prior 401 was a server-side defect that clients should already have been tolerant of (treating either 400 or 401 as "session is gone, restart `initialize`").

### Revert-and-fail evidence

After applying both fixes, reverting them via `git stash` and re-running the targeted tests produces:

```
test_sessionless_non_ping_rejected                                  left: 401, right: 400
test_legacy_handler_get_sse_without_session_returns_400             left: 200, right: 400
test_unauthenticated_ping_disabled_rejects_sessionless_ping         left: 401, right: 400
test result: FAILED. 0 passed; 3 failed
```

Restoring the fix returns all 11 targeted tests to GREEN (8 `feature_tests` + 3 `compliance`). The test net catches both bug classes.

## [0.3.46] - 2026-05-17

### Fixed

- **`turul-mcp-session-storage` failed to compile with `--features postgres` alone** (without `sqlite`). The `From<sqlx::Error> for SessionStorageError` impl in `crates/turul-mcp-session-storage/src/traits.rs` was gated `#[cfg(feature = "sqlite")]`, but `postgres.rs` contains a bare `?` on a `sqlx::Result` inside the expiration-cleanup transaction (`crates/turul-mcp-session-storage/src/postgres.rs:772`), which requires that `From` impl to exist. Enabling only the `postgres` feature therefore yielded 18 `E0277: the trait \`From<sqlx::Error>\` is not implemented` errors across the postgres module. Fix is a single feature-gate change to `#[cfg(any(feature = "sqlite", feature = "postgres"))]`, matching the gate already used in `turul-mcp-task-storage/src/error.rs:47`. Revert-and-fail evidence: `cargo check -p turul-mcp-session-storage --no-default-features --features postgres` fails with the 18 errors before the change, succeeds after. Verified clean across the four feature subsets users actually combine: `--features postgres`, `--features sqlite`, `--features dynamodb`, and `--features sqlite,postgres,dynamodb`. **Consumer impact**: Anyone depending on `turul-mcp-session-storage = { version = "0.3.45", features = ["postgres"] }` without also enabling `"sqlite"` could not build at all on 0.3.34–0.3.45; this is unblocked on 0.3.46. **Scope check confirming this is the only instance**: `turul-mcp-task-storage` already had the correct `any(...)` gate; `turul-mcp-server-state-storage` (the tool-fingerprint backend) has no `From<sqlx::Error>` impl and doesn't need one — its postgres backend uses `.map_err(...)` consistently and compiles cleanly under each single-feature combo.

## [0.3.45] - 2026-05-16

### Changed

- **`turul-mcp-client` migrates to `turul-rpc` directly, ahead of the rest of the framework** (scoped 0.3.x exception per [ADR-025](docs/adr/025-extract-turul-rpc.md) §"Revision log" 2026-05-16 entry). The client crate's `Cargo.toml` no longer depends on the `turul-mcp-json-rpc-server` shim — it depends on `turul-rpc` directly. The remaining framework crates (`turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`, etc.) continue to depend on the shim through the rest of 0.3.x; the framework-wide cutover lands at 0.4.0 per the original ADR-025 lifecycle. **Consumer impact**: `turul-mcp-client` users do not need to add `turul-rpc` to their own `Cargo.toml` — the dep is internal. Public API surface of `turul-mcp-client` is unchanged. Anyone explicitly importing types via `turul_mcp_json_rpc_server::*` from inside their own application code is unaffected (the shim crate still ships).

### Refactor

- **`turul-mcp-client` JSON-RPC envelopes now flow through `turul-rpc`'s typed constructors instead of 20+ hand-rolled `json!({"jsonrpc": "2.0", ...})` literals**. Two new private helpers in `crates/turul-mcp-client/src/client.rs` — `build_request(method, params)` and `build_notification(method, params)` — route every outbound MCP method (initialize, tools/list ×2, tools/call ×2, resources/list ×2, resources/read, resources/templates/list ×2, prompts/list ×2, prompts/get, ping, tasks/get, tasks/list ×2, tasks/cancel, tasks/result, notifications/initialized) through `turul_rpc::JsonRpcRequest::new` / `JsonRpcNotification::new`. The JSON-RPC 2.0 envelope shape (`jsonrpc` version, field ordering, `params` present-vs-absent semantics) now lives in one place rather than being copy-pasted across the file. **Wire bytes are semantically equivalent** to the prior hand-rolled form; this is a maintainability slice, not a behaviour change.
  - **Empty-params preservation**: `Value::Object(empty)` is intentionally preserved as `"params":{}` on the wire (not omitted via `skip_serializing_if`), matching the prior hand-rolled form so any MCP server that distinguishes `params: {}` from a missing `params` field continues to see the same envelope. `Value::Null` is correctly omitted (no `params` field).
  - **Defensive scalar handling**: `value_to_request_params(Value)` panics with `unreachable!()` for scalar `Value` inputs — no MCP client call site passes a scalar, and silently wrapping in a positional-array `RequestParams::Array` would be a wire-format change masking misuse rather than surfacing it.

### Test

- **17 new tests guarding the typed-envelope refactor**, totalling 130 client tests on the slice (was 113):
  - **5 unit tests** for `value_to_request_params` (Null → None; empty Object → `Some(Object(empty))`; Object preserves entries; Array preserved; scalar `#[should_panic]`).
  - **5 unit tests** for `build_request` (envelope shape with `jsonrpc/method/id/params`; nested object params; ID monotonic increment per call; `Value::Null` params omits the field on the wire; semantic JSON-envelope equality with the prior hand-rolled `json!({"jsonrpc": "2.0", ...})` form).
  - **3 unit tests** for `build_notification` (envelope shape with no `id` per JSON-RPC 2.0 §4.1; nested object params; `Value::Null` omits both `id` and `params`).
  - **1 unit test** `test_build_request_preserves_nested_array_values_in_arguments` — array values inside `params.arguments` (numeric, string, nested-array) round-trip through `RequestParams::Object(HashMap<String, Value>)` intact, distinct from JSON-RPC envelope-level positional params.
  - **3 wire-layer tests** in `tests/wire_compliance.rs` exercising `JsonRpcRequest` / `JsonRpcNotification` directly through `HttpTransport` against wiremock — typed request envelope on wire; empty-Object params preserves `"params":{}` on wire (not omitted); typed notification omits `id` field on wire.
  - **1 wire-layer test** `test_mcp_client_ping_sends_typed_jsonrpc_envelope_through_full_stack` — end-to-end production-path coverage walking `McpClient::connect()` + `ping()` against a wiremock server, capturing the `ping` POST body via `received_requests()`, asserting the JSON-RPC 2.0 envelope shape, AND asserting `notifications/initialized` POST has no `id` field.
  - **1 wire-layer test** `test_mcp_client_call_tool_preserves_array_argument_values_on_wire` — end-to-end `McpClient::call_tool("compute_stats", json!({"values": [1,2,3,4,5], "tags": [...], "matrix": [[1,2],[3,4]]}))` against wiremock, capturing the `tools/call` POST body, asserting `body["params"]["arguments"].is_object()` (proves MCP uses named args, not JSON-RPC positional) and that all three array values survive intact at `body["params"]["arguments"].{values, tags, matrix}` with no flattening, coercion, or stringification.

### Cleanup

- **`MockTransport` and `StatefulMockTransport` test fixtures now advertise `tools.listChanged: false`** (was `true`). Both fixtures previously claimed the capability during initialize but never emitted `notifications/tools/list_changed` from the mock itself, violating MCP capability truthfulness ("server MUST NOT claim a capability it does not actually deliver"). The three `test_*_list_changed_notification_invalidates_cache` tests inject the notification out-of-band via `MockTransport::event_sender()` and continue to pass — confirmed by the cache-invalidation handler at `client.rs:175-193` which processes the notification unconditionally rather than gating on the capability flag. No production-code change.

## [0.3.44] - 2026-05-15

### Added

- **`McpClient::set_bearer()` / `Transport::update_auth_header()`** — rotate the `Authorization` header on a live transport without rebuilding the underlying `reqwest::Client` (which would invalidate the HTTP/2 connection pool and force a fresh TLS handshake per rotation). Per-request `RequestBuilder::header(...)` overrides any same-named entry in `default_headers`, so existing `ConnectionConfig::headers`-baked bearers remain the initial value and become the fallback after `set_bearer(None)`. `Transport::update_auth_header` has a default no-op impl, so non-HTTP transports (stdio, SSE) are unchanged. Wired through `send_request`, `send_request_with_headers`, `send_notification`, `send_delete`, and the SSE GET listener task, so every outbound surface honours the live override.

### Fixed

- **`McpClient::disconnect()` could send DELETE under a stale bearer after OAuth `client_credentials` rotation** (`turul-mcp-client`). Discovered while investigating downstream consumer logs (sv-common / sw-common) that showed `HTTP 403 Forbidden` returned in ~15 ms from two unrelated upstream MCP servers fronting Lambdas (`st.aussierobots.com.au/mcp`, `sd.aussierobots.com.au/mcp`) on every `disconnect()` DELETE that followed a rotation event. Root cause was **not** server-side principal pinning — code inspection confirms the framework's DELETE handler in `turul-http-mcp-server` does not authenticate at all (both `streamable_http.rs` and `session_handler.rs` route DELETE around `MiddlewareStack::execute_before_session`, and `turul-mcp-oauth` only returns 401, never 403). The 403 originated upstream (API Gateway authorizer / ALB OIDC / equivalent) evaluating the bearer the client actually put on the wire — which was the *old* one. Reason: `HttpTransport` injected the `Authorization` header via `reqwest::ClientBuilder::default_headers()` at construction, with no API to mutate it thereafter. Callers that rotated the M2M token by creating a fresh `McpClient` were left holding old clients with bearers baked into the connection-pool-owning `reqwest::Client`; their cleanup `disconnect()` therefore sent DELETE under a bearer the AS had typically already revoked. Fix: `HttpTransport` now holds an `Arc<RwLock<Option<String>>>` auth override applied per-request via `RequestBuilder::header()`, with `Transport::update_auth_header()` / `McpClient::set_bearer()` as the rotation API. Callers rotate the bearer immediately before `disconnect()`, and the DELETE flies under the fresh token. Regression coverage: three wire-layer tests in `tests/wire_compliance.rs` exercising the actual reqwest pipeline against a wiremock server — `test_send_delete_uses_overridden_bearer_after_rotation` (the headline contract), `test_send_request_uses_overridden_bearer_after_rotation` (parity for POST), and `test_clearing_override_falls_back_to_default_headers` (confirms `None` removes the override). Revert-and-fail check recorded: with `apply_auth_override` removed from `send_delete` and `send_request`, the wire shows `authorization: Bearer OLD` and wiremock's `expect(1).matching(Authorization: Bearer NEW)` fails; the clearing test correctly stays green (it asserts the OLD-bearer fallback, which the unmodified code path still produces). Wire-layer rule per CLAUDE.md §"Test Coverage Discipline" #3 satisfied: tests assert what reqwest actually puts on the wire, not framework-internal state.

## [0.3.43] - 2026-05-15

### Fixed

- **`McpClient::disconnect()` followed by `Drop` no longer fires a second doomed DELETE** (`turul-mcp-client`). `SessionManager::terminate()` previously flipped state to `Terminated` but left `session_id` populated; the `Drop` impl then read `session_id_optional()`, observed `Some(_)`, and spawned a second `transport.send_delete(...)` against a session the server had already torn down — typically arriving after the originating bearer had expired in OAuth deployments, surfacing as a 401/410 noise event in server logs and prompting confused investigations on the server side. Fix: `terminate()` now clears `session_id` after logging it, establishing the invariant "a terminated session has no ID". Both production callers (`disconnect()` and `Drop`) already route through `terminate()`, so the single-point fix makes the whole lifecycle idempotent without any public API change or new method. The `Drop`-without-`disconnect()` path is unchanged — bare drop still fires exactly one DELETE, preserving server-side cleanup for callers that don't disconnect explicitly. Regression coverage: `test_disconnect_clears_session_so_drop_is_noop` (locks in DELETE-count == 1 across disconnect+drop) and `test_drop_without_disconnect_still_fires_delete` (regression guard for the implicit-cleanup path); `test_session_lifecycle` extended to assert `session_id_optional()` is `None` after `terminate()`. Revert-and-fail check recorded: with the one-line `session_id = None` clear removed, the new tests fail with `left: 2, right: 1` (double-DELETE) and the lifecycle assertion fails on the cleared-id check; the regression guard correctly stays green (it asserts an orthogonal invariant). Fix discovered by downstream consumers (sv-common / sw-common) hitting the second DELETE after explicit disconnect with a near-expired bearer — they will additionally adopt proactive disconnect at 95% bearer lifetime as a belt-and-suspenders measure on the consumer side.

### Note on v0.3.43 numbering

A previously-planned v0.3.43 (Lambda empty-body streaming) was investigated and **closed as documented limitation** rather than published — see the v0.3.42 entry below for the full reasoning. The version number v0.3.43 is therefore reused here for an **unrelated** client-side disconnect/Drop fix. There is no Lambda streaming behavior change in v0.3.43; the empty-body limitation continues to require APIGW MOCK on OPTIONS as documented in ADR-026.

## [0.3.42] - 2026-05-11

### Note (post-release): v0.3.43 Lambda empty-body investigation closed as documented limitation

Production verification by the downstream consumer (sd-mcp v0.7.12) confirmed that v0.3.42's `EnsureOneFrame` adapter does not actually fix the empty-body Lambda streaming `IncompleteMessage` / 60s timeout / APIGW 502 case it claimed to solve. A wire-level diagnostic harness on branch `park/wire-level-test-harness` (retained on origin) replicates `lambda_runtime-1.2.0/src/requests.rs` serialization verbatim and confirms `BodyDataStream` yielding a zero-byte data frame does not satisfy the AWS Lambda Runtime API wire contract. Three resolution paths were considered: (a) sentinel-byte fix in `EnsureOneFrame`, (b) reject empty bodies with a clear error, (c) document the limitation; APIGW MOCK on OPTIONS is the permanent pattern. **Decision: (c).** Reasons in ADR-026 §"Resolution 2026-05-11". v0.3.42 stays published; framework code is unchanged; fleet deployments (sd-mcp v0.7.13, plus sv-track/gps-trust-mcp/gps-trust-agent-mcp port wave) use APIGW MOCK on all OPTIONS methods. CLAUDE.md "Test Coverage Discipline" gained rule 3 (wire-layer coverage for transport-protocol boundaries) as a permanent gate improvement — this is the recurrence prevention for the class of failure mode v0.3.42 hit, regardless of how this specific bug resolved. The v0.3.43 version number was subsequently used for an unrelated client-side disconnect/Drop fix — see the v0.3.43 entry above.



### Fixed

- **Lambda streaming response with zero-data-frame body caused `IncompleteMessage` / 60 s timeout / API Gateway 502** (`turul-mcp-aws-lambda`). `into_lambda_stream_response` accepted any `B: http_body::Body + Unpin + Send + 'static`, but when `B` produced zero `Frame::Data` frames (e.g. `http_body_util::Empty::<Bytes>::new()`), the resulting `BodyDataStream` yielded zero items. The Lambda Response Streaming multipart envelope wrote the prelude + metadata JSON + trailer separator and then closed the body stream without ever writing a body chunk. Lambda's Runtime API client (hyper) requires at least one chunk before EOF for the framing to terminate cleanly; without one, the connection closed mid-frame with `hyper::Error(IncompleteMessage)`. The function appeared to hang for its full timeout, AWS reported `Status: timeout` (not `Status: error`, no `Errors` metric increment), and API Gateway emitted 502 to the client after the timeout. Common trigger: `.well-known/oauth-protected-resource` OPTIONS short-circuits in `run_streaming_with` dispatch closures returning `Response<UnsyncBoxBody<Bytes, hyper::Error>>` with `Empty::new()` body. **This is a pre-existing latent bug, not a v0.3.39 → v0.3.40 regression** — `f6438cb` does not touch any code path affected by it; consumer dispatch closures simply began exercising the empty-body path. Fix: internal `EnsureOneFrame<B>` body adapter wraps `B` in `into_lambda_stream_response` and emits a single zero-length `Frame::data` if the underlying body would otherwise yield no data frames. Bodies that natively produce ≥1 data frame are unaffected (first frame forwarded as soon as `B` yields it; no buffering, no pre-polling, streaming semantics preserved). The zero-length frame is invisible at the HTTP layer — no `Content-Length` header added, no response bytes visible to the client. Contract documented in ADR-026. Revert-and-fail recorded in commit message.

## [0.3.41] - 2026-05-11

### Fixed

- **`LambdaMcpServer::handler()` silently dropped builder-configured CORS** (`turul-mcp-aws-lambda`). `LambdaMcpServerBuilder::cors(...)` / `.cors_allow_all_origins()` / `.cors_allow_origins(...)` / `.cors_from_env()` populated `LambdaMcpServer.cors_config`, but `handler()` constructed the `LambdaMcpHandler` via `with_middleware_and_fingerprint(...)` (which initializes `cors_config: None`) and never chained `.with_cors(self.cors_config.clone())` onto the result. Every `if let Some(ref cors_config) = self.cors_config` branch inside `LambdaMcpHandler` — preflight short-circuit, custom-route injections, final-response injection — was therefore unreachable through the documented builder entry point. Pre-existing since `5d4bdd3` (2025-10-05, "feat: add middleware support for Lambda"); silently broken across all releases from then through v0.3.40. The injection logic added in v0.3.40 for streaming custom-route branches was functionally correct but unreachable for builder-constructed handlers, which is what surfaced the bug. Fix: `handler()` now chains `.with_cors(self.cors_config.clone())` after the notifier/registry wiring, before returning. Three new regression tests assert builder-path CORS coverage explicitly: streaming OPTIONS preflight, streaming 401 challenge (non-preflight, with `WWW-Authenticate` preserved + exposed), and negative-path (no CORS config → no CORS headers). The previous `test_cors_configuration` only smoke-checked `stream_manager`, which is why the bug slipped past CI for ~7 months — that smoke check is retained but no longer the sole guard.

## [0.3.40] - 2026-05-11

### Fixed

- **Lambda streaming custom-route CORS asymmetry** (`turul-mcp-aws-lambda`): `LambdaMcpHandler::handle_streaming()` returned route-matched and route-validation-error responses **without** injecting the configured `CorsConfig`, while the buffered `handle()` injected CORS for the equivalent branches. Browser-facing custom routes registered via `LambdaMcpServerBuilder::route(...)` (e.g. `.well-known/oauth-protected-resource` for RFC 9728 OAuth discovery) therefore returned `200 OK` with the correct body but no `Access-Control-Allow-Origin`, even when CORS was configured. Streaming now matches buffered: both branches inject CORS before returning. Regression tests added covering matched-route CORS, validation-error CORS, no-CORS-config-no-injection, and a 401 challenge end-to-end through the streaming transport with `WWW-Authenticate` preserved and exposed.

### Changed

- **Default `Access-Control-Expose-Headers` now includes `WWW-Authenticate`** in both `turul-mcp-aws-lambda::CorsConfig::default()` and `turul-http-mcp-server::cors::CORS_EXPOSE_HEADERS`. Browser OAuth clients cannot read non-safelisted response headers unless they appear in `Access-Control-Expose-Headers`; RFC 9728 discovery requires clients to parse `WWW-Authenticate` on `401` responses, so it must be exposed by default for any browser-fronted MCP server. Behavioural impact: a previously-not-exposed response header is now exposed to browser JS by default. Consumers passing a custom `expose_headers` list retain full control and must add `WWW-Authenticate` explicitly if they want browser OAuth to work. No change to non-browser clients.

### Added

- **Public re-exports for CORS helpers** in `turul-mcp-aws-lambda`: `inject_cors_headers` and `create_preflight_response` are now re-exported from the crate root and `prelude`, alongside the existing `CorsConfig` re-export. This is the supported escape hatch for `run_streaming_with` dispatch closures that short-circuit before calling `handler.handle_streaming()` — call the helper before returning to keep CORS behaviour consistent with the framework's built-in routing path. **Boundary**: the framework guarantees CORS on every response built inside `LambdaMcpHandler::handle_streaming()`; the consumer is responsible for CORS on responses built before `handle_streaming` is called, including custom `run_streaming_with` short-circuits — use the re-exported `inject_cors_headers` / `create_preflight_response`.

## [0.3.39] - 2026-05-10

### Changed

- **`turul-mcp-json-rpc-server` is now a re-export shim over [`turul-rpc`](https://github.com/aussierobots/turul-rpc)** (a new sibling repository / crate family). All implementation moved to `turul-rpc-core`, `turul-rpc-jsonrpc`, and `turul-rpc-server`; the framework crate `turul-mcp-json-rpc-server 0.3.39` is now ~50 lines of `pub use turul_rpc::*` plus module re-exports. Public API surface is preserved at every original path with identical nominal types — existing `turul_mcp_json_rpc_server::*` imports continue to compile and behave identically. Internal framework crates (`turul-mcp-protocol-2025-{06-18,11-25}`, `turul-mcp-builders`, `turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`) continue to depend on `turul-mcp-json-rpc-server` through 0.3.x; framework 0.4.0 will migrate those imports to `turul-rpc` directly and drop the shim crate. **There is no planned 0.4 release of `turul-mcp-json-rpc-server`** — 0.3.39 is the terminal shim. Existing 0.3.x consumers may continue depending on it indefinitely; new code should depend on `turul-rpc` directly. See [ADR-025](docs/adr/025-extract-turul-rpc.md).

### Added

- **JSON-RPC 2.0 batch processing** (via `turul-rpc-jsonrpc`): the original `turul_mcp_json_rpc_server::dispatch::parse_json_rpc_messages` was a stub claiming "JSON-RPC 2.0 removed batch support" (it didn't). `turul-rpc-jsonrpc 0.1.0` ships a spec-conformant batch implementation — `parse_json_rpc_batch` returns a `BatchOrSingle` discriminator; `JsonRpcDispatcher::handle_batch` dispatches per-member with notification-response suppression, all-notifications no-response semantics, and empty-batch → single `Invalid Request` (`-32600`) per [JSON-RPC 2.0 §6](https://www.jsonrpc.org/specification#batch).
  - **Reachable through the shim**: `JsonRpcDispatcher::handle_batch` (the dispatcher type is re-exported and methods come with the type — listed here per ADR-003 §"additive items reviewed").
  - **Not reachable through the shim**: `parse_json_rpc_batch` and `BatchOrSingle` live in `turul_rpc::batch`, a module the shim does **not** re-export. Users who want them depend on `turul-rpc` directly. This preserves the v0.3.38-surface discipline.

### Note on JSON-RPC 2.0 compliance posture

`turul-rpc 0.1` advertises JSON-RPC 2.0 with **one documented departure**: incoming requests with `"id": null` are rejected as `Invalid Request` (`-32600`). The spec permits null id with a discouragement note. The strict posture is **inherited from `turul-mcp-json-rpc-server 0.3.38`** — relaxing it in the shim release would be a behaviour change. A v0.2 candidate is to surface a permissive codec-level type for callers who need null-id requests. See `turul-rpc/docs/adr/002-json-rpc-2-compliance.md`.

### Documentation

- **Lambda eager handler init** (`turul-mcp-aws-lambda`): documented and exemplified building `LambdaMcpHandler` eagerly in `main()` before the Lambda runtime hand-off, avoiding request-path lazy initialization for fan-out-sensitive workloads. Removed `static OnceCell<LambdaMcpHandler>` patterns from `examples/lambda-mcp-server`, `examples/lambda-mcp-server-streaming`, and `examples/middleware-auth-lambda`; each example now builds the handler once in `main()` and `move`-captures it into a small `service_fn(move |req| lambda_handler(handler.clone(), req))` wrapper. Updated `crates/turul-mcp-aws-lambda/README.md` quick-start and "Custom Dispatch with `run_streaming_with()`" sections to match. The existing `LambdaMcpServerBuilder::build().await?` followed by `server.handler().await?` already performs full eager init (DDB session storage, server-state-storage, server build, tool/resource registration, session cleanup spawn, dynamic-tools sync, cold-start task recovery) — no new API is added. Per-request `info!` logging in example handlers dropped to `debug!` to avoid CloudWatch flood at production traffic. Closes #15. See [ADR-024](docs/adr/024-lambda-eager-handler-init.md).

## [0.3.38] - 2026-05-03

### Fixed

- **SSE GET 4xx hot-loop on streamable HTTP transport** (`turul-mcp-client`): `HttpTransport`'s SSE listener task previously treated every non-2xx GET response identically — `warn!` then sleep 5s + `continue` — producing an infinite retry loop against servers that legitimately reject the request (e.g. an MCP server with `strict_lifecycle(true)` returning HTTP 400 for a GET issued after session termination). The listener now distinguishes status classes:
  - **4xx → terminal**: clear the cached `Mcp-Session-Id` (only if it still matches what the failing GET sent — see CAS note below), emit `ServerEvent::Error("SSE GET rejected with HTTP <status> — listener exiting")`, and exit the spawned task. The cache clear ensures the caller's next `initialize` POST goes out without a stale session header (mirrors the canonical POST-404 recovery in `McpClient::send_request_raw`).
  - **5xx and other non-2xx → transient**: existing `warn!` + 5s sleep + retry behavior is preserved.

  Caller contract: on terminal SSE GET 4xx the listener exits cleanly; the caller may then re-run its normal `initialize` / `start_event_listener` flow. No new `ServerEvent` variants, no public API additions, no extension of `is_session_expired()` (which remains 404-only). The legacy `transport/sse.rs` is unchanged in this slice.

### Note

Surfaced by a Lambda MCP client logging repeated `"SSE stream error: error decoding response body"` followed by `"SSE connection lost, attempting to reconnect..."` against an API Gateway-fronted server with `strict_lifecycle(true)`. The 29s API GW idle timeout killed the SSE stream; the listener re-issued GET, which 400'd, and the loop never terminated. Four regression tests in `tests/sse_terminal_4xx.rs` lock in: terminal-on-400 with cache clear, transient-on-503, listener-works-without-session-id (stateless mode guard), and compare-and-swap on cache clear (does not clobber a fresher session).

The CAS detail matters because `McpClient::connect()` spawns the SSE listener **before** running `initialize_session()` (see `client.rs:135-229`). The two race: the listener may build a GET while `session_id` is still `None`, then `initialize` writes a real session ID into the cache, then the in-flight GET 4xx's. An unconditional cache clear would clobber the just-initialized session and break every subsequent POST. The fix snapshots the session header sent at request-build time and only clears the cache if it still matches that snapshot — preserving the strict-lifecycle bug-fix semantics (snapshot==current==Some(stale) → clear) while leaving a fresher value alone (snapshot=None, current=Some(new) → no-op).

## [0.3.37] - 2026-04-24

### Fixed

- **HTTP/2 connection drop detection** (`turul-mcp-client`): `HttpTransport` now configures `reqwest`'s h2 keepalive PINGs (`http2_keep_alive_interval = 30s`, `http2_keep_alive_timeout = 10s`, `http2_keep_alive_while_idle = true`) on both `new()` and `with_config()` construction paths. Without these, a connection silently dropped by the server or an intermediary (API Gateway ~350s idle, NAT, ALB) looks alive to the client pool until the next request — which then pays the full reconnect cost. PING keepalives surface the drop proactively so idle pooled connections either stay alive or fail fast and reconnect before a user-facing request uses them. No-op on h1-only backends (ALPN-negotiated h1 connections don't engage h2 keepalive state).

### Note

Values chosen as conservative defaults: 30s interval detects drops well before typical intermediary idle windows without being wasteful (~10 bytes per PING). 10s timeout halves reqwest's default 20s for faster fail-over on flaky paths. `while_idle = true` is the load-bearing bit — it keeps pooled idle connections being probed, which is precisely where silent-drop bimodality manifests. No new `ConnectionConfig` fields were added; if tuning becomes necessary it will land alongside other pending 0.4 surface changes.

## [0.3.36] - 2026-04-24

### Changed

- **`turul-mcp-client` now compiled with `reqwest/http2` feature**: reqwest auto-negotiates HTTP/2 via ALPN when the backend advertises `h2`. For servers that only speak HTTP/1.1, ALPN falls back to h1 — no behavior change. For h2-capable backends (AWS API Gateway, ALB, CloudFront, most modern HTTPS servers), concurrent `call_tool` invocations on one `Arc<McpClient>` are now multiplexed over a single TLS connection instead of opening N separate h1 connections. Resolves #13.

### Testing

- `tests/http2_feature.rs`: compile-time regression test that fails if a future `Cargo.toml` edit accidentally disables `reqwest/http2`.

### Note on validation

This change enables h2 at the dependency layer; the wire-level negotiation is handled entirely by reqwest + rustls ALPN. End-to-end validation (latency improvement on concurrent fan-out against h2-capable backends) is owned by downstream consumers — no specific latency claim is attached to this release. See #13 for the measurement plan and expected behavior.

## [0.3.35] - 2026-04-24

### Fixed

- **`ConnectionConfig` fields now honored** (`turul-mcp-client`): `HttpTransport::with_config` previously advertised six configuration fields but consumed only three (`user_agent`, `follow_redirects`, `headers`). `max_redirects`, `pool_settings.max_idle_per_host`, and `pool_settings.idle_timeout` were silent no-ops — callers set them and `reqwest` defaults applied instead. These three are now wired through to `reqwest::ClientBuilder` (`Policy::limited`, `pool_max_idle_per_host`, `pool_idle_timeout`).

### Deprecated

- **`ConnectionConfig::keep_alive`** and **`PoolConfig::max_lifetime`** (`turul-mcp-client`): no reqwest equivalent. `reqwest` exposes `tcp_keepalive(Option<Duration>)`, not a boolean, and has no per-connection max-lifetime API. Both fields will be removed in 0.4. Callers who do not reference them are unaffected.

### Changed

- **`PoolConfig::default().max_idle_per_host`** raised from 5 to 32 (`turul-mcp-client`): the previous default was silently ignored (reqwest's internal default `usize::MAX` applied). Now that the field is honored, the previous default would cap callers at 5 idle connections per host — a regression for fan-out workloads. 32 matches typical HTTP client sizing; callers can still set their own value.

### Note

This release fixes `ConnectionConfig` API truthfulness only. It does not change HTTP/2 support, connection protocol negotiation, or any other transport-layer behavior. A separate investigation (#13) is evaluating whether enabling `reqwest/http2` measurably affects cold-path tail latency; no decision has been made on that feature.

## [0.3.34] - 2026-04-21

### Fixed

- **DynamoDB read-your-writes on critical paths** (`turul-mcp-session-storage`, `turul-mcp-server-state-storage`): Added `consistent_read(true)` to the DynamoDB read sites that must observe just-written values across instances. Eventual-consistency reads on these paths could cause cold-start Lambda instances to miss sessions, session state, persisted events, or fingerprints written by other instances — breaking MCP SSE resumability and the `initialize` handshake.
  - `get_session`, `set_session_state` read-before-write, `store_event` session-exists check, and `store_event` max-eventId query (visibility; races still handled by the existing conditional `PutItem` + `MAX_RETRIES` loop).
  - `get_fingerprint` — cold-start instance must observe the latest fingerprint.

### Added

- **Storage contract regression tests** (`#[ignore = "requires DynamoDB"]`): `read_your_writes_contract` (session, state, event-replay) and `read_your_writes_contract_fingerprint`. Classified as storage contract regression tests; documented that DynamoDB-Local / LocalStack does not reliably reproduce AWS eventual reads, so passing locally does not prove AWS consistency correctness.

## [0.3.33] - 2026-04-21

### Changed

- **`Transport` trait — `&self` on hot-path methods** (`turul-mcp-client`): `connect`, `disconnect`, `send_request`, `send_request_with_headers`, `send_notification`, `send_delete`, `set_session_id`, `clear_session_id`, `start_event_listener`, and `health_check` now take `&self`. `McpClient::transport` is now `Arc<BoxedTransport>` — the outer `tokio::sync::Mutex` that serialized every request has been removed.

### Fixed

- **Concurrent client requests no longer serialize** (`turul-mcp-client`): N parallel `call_tool` / `list_tools` / etc. on one `Arc<McpClient>` now run in parallel through `reqwest`'s internal connection pool. Before: total wall time ≈ Σ per-call latency (Mutex-serialized). After: wall time ≈ max per-call latency.

### Breaking

- External implementors of `turul_mcp_client::transport::Transport` must change `&mut self` to `&self` on the listed methods and move any bare-mutable state into interior-mutable wrappers (`Atomic*` / `parking_lot::Mutex`). The stock `HttpTransport` and `SseTransport` already use interior mutability on all hot-path state.

## [0.3.32] - 2026-04-15

### Fixed

- **Client session retry on -32031**: `McpClient::call_tool()` (and all request methods) now detect JSON-RPC error code `-32031` ("Session not initialized") and automatically disconnect, reconnect, and retry once. Fixes cold-start race condition where `notifications/initialized` hasn't been processed before the first request arrives — especially visible on Lambda behind API Gateway.

### Added

- **`McpClientError::is_session_not_initialized()`**: Detects session-not-initialized errors by code (-32031) or message content.

## [0.3.31] - 2026-03-30

### Fixed

- **SSE replay**: No replay without `Last-Event-ID` — reverted bounded replay that caused duplicate notifications on API Gateway timeout reconnections. With `Last-Event-ID`: exact resume. Without: live events only.
- **Dead SSE connections**: Removed immediately on send failure, delivery falls back to next live connection. `has_connections()` now ignores closed senders.
- **DynamoDB event ID monotonic**: Conditional write (`attribute_not_exists`) with retry prevents duplicate event IDs across Lambda cold starts.
- **DynamoDB timestamp read**: Fixed numeric millis read (was parsing as RFC3339 string, always fell back to `Utc::now()`).
- **Distributed session targeting**: `broadcast_event()` enumerates targets from `storage.list_sessions()` for Custom events. `dispatch_custom_event()` for per-session delivery without cache dependency.
- **SessionEventDispatcher**: Guaranteed notification persistence on request path. `broadcast_event()` returns `Result` — dispatcher failures propagate.
- **Initialize live fingerprint**: Dynamic mode uses `ToolRegistry::fingerprint()` for new sessions, not build-time static.
- **DynamoDB `get_active_entities`**: Removed `entityId` from filter expression (DynamoDB rejects sort keys in filters).

### Added

- **`ToolChangeNotifier` trait**: Awaitable callback for restart/redeploy fingerprint mismatch notifications, backed by `SessionManager::dispatch_custom_event()`.
- **`dispatch_custom_event()`**: Storage-backed per-session event dispatch, not cache-gated.
- **`SessionEventDispatcher` trait**: Awaitable dispatcher on `SessionManager` for guaranteed Custom event persistence.
- **ADR-023 updates**: Distributed session targeting, session-backed event sequencing future consideration.

## [0.3.30] - 2026-03-29

### Fixed

- **DynamoDB `get_active_entities` filter** (`turul-mcp-server-state-storage`): Removed `entityId` (sort key) from `filter_expression` — DynamoDB rejects primary key attributes in filter expressions. Now uses application-level filtering.
- **Restart/redeploy notification persistence** (`turul-http-mcp-server`): Fingerprint mismatch in `validate_session_exists()` now emits `notifications/tools/list_changed` through the `ToolChangeNotifier` → `SessionManager` → dispatcher architecture. Failure propagates (500), not warn-and-continue.
- **DynamoDB TTL defaults** (`turul-mcp-session-storage`): Session and event TTL defaults increased from 5 to 30 minutes.

### Added

- **`ToolChangeNotifier` trait** (`turul-http-mcp-server`): Awaitable callback for restart/redeploy fingerprint mismatch notifications. Implemented by the server layer via `SessionManager::send_event_to_session()`.
- **`send_event_to_session()` with dispatcher** (`turul-mcp-server`): Per-session event dispatch with guaranteed persistence for Custom events. Retains NotFound error for missing sessions.

## [0.3.29] - 2026-03-29

### Added

- **SessionEventDispatcher** (`turul-mcp-server`): Awaitable dispatcher trait on `SessionManager` for guaranteed notification persistence on the request path. Custom events are persisted via `StreamManager::broadcast_to_session()` before `broadcast_event()` returns. Installed by the runtime (HTTP server, Lambda).
- **Mandatory persistence enforcement**: `broadcast_event()` returns `Result<(), String>` for Custom events. `broadcast_notification()` returns `Result<(), ToolRegistryError::NotificationFailed>`. `activate_tool()`, `deactivate_tool()`, `check_for_changes()` propagate dispatcher failures — no silent success when mandatory persistence fails.
- **Live registry fingerprint for new sessions**: In Dynamic mode, `SessionAwareInitializeHandler` reads `ToolRegistry::fingerprint()` instead of the build-time static value. New sessions after runtime tool mutations get the correct baseline — no spurious mismatch notification.

### Fixed

- **DynamoDB error observability** (`turul-mcp-server-state-storage`): `dynamo_err_debug()` uses `{:?}` (Debug) format instead of `{}`  (Display) for AWS SDK errors, surfacing error code, message, HTTP status, and request ID instead of generic "service error".
- **SSE bridge narrowed to observer-only**: The detached bridge task no longer persists or delivers `SessionEvent::Custom` events — the awaited dispatcher handles that on the request path. Eliminates duplicate persistence.

### Changed

- **BREAKING: `broadcast_event()` returns `Result`**: Callers that previously ignored the return value of `SessionManager::broadcast_event()` must now handle the `Result<(), String>` return for Custom events. Non-custom events always return `Ok(())`.

## [0.3.28] - 2026-03-29

### Fixed

- **Non-deterministic tool fingerprint** (`turul-mcp-server`): `compute_tool_fingerprint()` now canonicalizes JSON (recursive key sorting) before FNV hashing.

## [0.3.27] - 2026-03-29

### Changed

- **BREAKING: Default features reduced** (`turul-mcp-server`): Default features now `["http", "sse"]` only. SQLite, PostgreSQL, and DynamoDB backends are opt-in via `features = ["sqlite"]`, `features = ["postgres"]`, `features = ["dynamodb"]`. This significantly reduces compile time and binary size for projects that only need in-memory storage.
- **Backend features forward to all storage crates** (`turul-mcp-server`): `sqlite`/`postgres`/`dynamodb` features now forward to both `turul-mcp-session-storage` AND `turul-mcp-task-storage` (previously only session-storage).
- **Unified backend features** (`turul-mcp-server`): `sqlite`/`postgres`/`dynamodb` features use weak dependency forwarding (`?/`) to also enable backends on `turul-mcp-server-state-storage` when `dynamic-tools` is active. No separate compound features needed.
- **Lambda backend features** (`turul-mcp-aws-lambda`): Added `sqlite`, `postgres` forwarding features.

### Migration

If you previously depended on `turul-mcp-server` without specifying features and used SQLite, PostgreSQL, or DynamoDB backends, add the backend feature explicitly:

```toml
# Before (backends included by default)
turul-mcp-server = "0.3.26"

# After (backends opt-in)
turul-mcp-server = { version = "0.3.27", features = ["sqlite"] }
```

## [0.3.26] - 2026-03-29

### Fixed

- **Non-deterministic tool fingerprint** (`turul-mcp-server`): `compute_tool_fingerprint()` now canonicalizes JSON (recursive key sorting) before hashing. HashMap iteration order in `ToolSchema.properties`, `ToolSchema.additional`, and nested `JsonSchema.properties` caused different Lambda instances to compute different fingerprints for the same tool set, triggering spurious mismatch cycles on every cold start.

## [0.3.25] - 2026-03-29

### Added

- **Dynamic tool activation** (`turul-mcp-server`): `ToolChangeMode::Dynamic` enables runtime `activate_tool()`/`deactivate_tool()` with MCP-compliant `notifications/tools/list_changed`. Requires `dynamic-tools` feature.
- **ToolRegistry** (`turul-mcp-server`): Live registry for precompiled tools with `RwLock<ToolState>`, fingerprint tracking, and cross-instance coordination via `ServerStateStorage`.
- **ServerStateStorage** (`turul-mcp-server-state-storage`): New crate with InMemory, SQLite, PostgreSQL, DynamoDB backends for cross-instance tool state coordination.
- **Lambda dynamic tools** (`turul-mcp-aws-lambda`): `tool_change_mode()` and `server_state_storage()` on `LambdaMcpServerBuilder`. Request-time change detection with configurable TTL (`TURUL_TOOL_CHECK_TTL_SECS`, default 10s).
- **Client tool change notifications** (`turul-mcp-client`): `refresh_tools()`, cached tool lists, `notifications/tools/list_changed` auto-invalidation.
- **Dynamic tools example**: `examples/dynamic-tools-server` and `examples/dynamic-tools-test-client`.

### Fixed

- **POST SSE notification replay** (`turul-http-mcp-server`): Removed event replay from POST SSE responses — connection is registered before dispatch, so all events are delivered live. Prevents duplicate notification delivery.
- **Derive macro zero-config output preservation** (`turul-mcp-derive`): `#[tool(output = Type)]` without `name`/`description` now correctly preserves the output type via `extract_tool_meta_partial()`. Previously, the fallback path discarded all attributes.
- **OAuth dev-deps** (`turul-mcp-oauth`): Migrated to workspace dependency references. Updated `rsa` to 0.10, `jsonwebtoken` to 10 with `rust_crypto` feature.
- **Test suite MCP handshake** (tests): Added missing `notifications/initialized` to all E2E test suites (prompts, resources, elicitation, roots, sampling, session validation).

### Changed

- **Workspace dependency rule**: All crate dependencies must use `workspace = true` references (added to CLAUDE.md).
- **reqwest workspace default**: `default-features = false` at workspace level; crates opt-in to features individually.

## [0.3.24] - 2026-03-21

### Fixed

- **MCP client Accept header** (`turul-mcp-client`): POST requests now send `Accept: application/json, text/event-stream` per MCP spec. Notifications also include Accept header.
- **MCP client SSE POST responses** (`turul-mcp-client`): Client can now parse `text/event-stream` responses to POST requests instead of rejecting them.
- **MCP client session ID optional** (`turul-mcp-client`): Client no longer hard-fails when server doesn't return `Mcp-Session-Id` — stateless sessions are spec-valid.
- **MCP client protocol version enforcement** (`turul-mcp-client`): Client rejects servers that negotiate unsupported protocol versions.
- **MCP client 404 re-initialization** (`turul-mcp-client`): HTTP 404 triggers session reset, clears stale session ID from transport, and re-initializes.
- **MCP client JSON-RPC error preservation** (`turul-mcp-client`): Error frames pass through transport preserving code/message/data instead of flattening to opaque strings.
- **MCP client SSE double-routing** (`turul-mcp-client`): SSE path no longer duplicates events to both event channel and queue.
- **MCP client SSE data field parsing** (`turul-mcp-client`): Accepts `data:` with or without space after colon per SSE spec.

### Changed

- **`call_tool()` return type** (`turul-mcp-client`): Returns `CallToolResult` instead of `Vec<ToolResult>` — preserves `is_error`, `structuredContent`, `_meta` fields. **Breaking:** callers need `.content` to get the previous `Vec<ToolResult>`.
- **`get_prompt()` return type** (`turul-mcp-client`): Returns `GetPromptResult` instead of `Vec<PromptMessage>` — preserves `description`, `_meta` fields. **Breaking:** callers need `.messages` to get the previous `Vec<PromptMessage>`.
- **`Transport` trait** (`turul-mcp-client`): Added required `clear_session_id()` method. **Breaking** for custom `Transport` implementations.

### Added

- **GET SSE listener for HttpTransport** (`turul-mcp-client`): `server_events: true` enables server-initiated requests/notifications over GET SSE stream.
- **Server request routing** (`turul-mcp-client`): JSON-RPC frames with `method` + non-null `id` are routed as `ServerEvent::Request` (not `Notification`) in both SSE and JSON stream paths.
- **`HttpTransport::with_config()`** (`turul-mcp-client`): Constructor that applies `ConnectionConfig` (custom headers, user-agent, redirect policy).
- **`TransportError::HttpStatus`** (`turul-mcp-client`): Structured error variant preserving HTTP status code.
- **Builder transport detection** (`turul-mcp-client`): `McpClientBuilder` defers transport construction to `build()` so `with_config()` works regardless of call order.
- **21 behavioral tests** (`turul-mcp-client`): Protocol compliance, regression, and wire-level tests using `StatefulMockTransport` and `wiremock`.

## [0.3.23] - 2026-03-20

### Fixed

- **`after_dispatch` middleware mutations silently discarded** (`turul-http-mcp-server`): `DispatcherResult` was cloned into middleware, mutated, then the original `JsonRpcMessage` returned unchanged — mutations now applied back via `apply_dispatcher_result()`.
- **`after_dispatch` middleware errors silently ignored** (`turul-http-mcp-server`): `let _ = execute_after(...)` swallowed `Err(MiddlewareError)` — errors now propagated through `map_middleware_error_to_jsonrpc()` with correct semantic error codes.

## [0.3.22] - 2026-03-16

### Fixed

- **SSE wire-format test compliance** (`tests`): Replaced `strip_prefix("data: ").unwrap_or(...)` workaround in `session_id_compliance` test with explicit Content-Type assertion — tests now branch on the response's declared Content-Type instead of silently accepting both SSE and JSON formats.
- **DynamoDB events table check** (`turul-mcp-session-storage`): `ensure_events_table_exists()` now skipped when `verify_tables` is false (table assumed to exist via CloudFormation/Terraform).

### Added

- **Content-Type negotiation policy** (`turul-http-mcp-server`): `StreamableHttpContext::should_use_sse()` — conservative method-level heuristic for combined `Accept: application/json, text/event-stream`. Non-streaming methods (`tools/list`, `resources/list`, etc.) return `application/json`; streaming-capable methods (`tools/call`, `sampling/createMessage`, `elicitation/create`) return `text/event-stream`.
- **Content-Type negotiation tests** (`tests`): 4 new tests asserting wire-format consistency for JSON-only, SSE-only, combined+tools/call, and combined+tools/list Accept patterns.
- **Test Compliance rule** (`CLAUDE.md`): Tests must assert wire-format compliance — never silently accept multiple formats.
- **ADR-006 amendment**: Documented Content-Type negotiation policy, its architectural limitations, and the per-tool metadata improvement path.

## [0.3.21] - 2026-03-16

### Fixed

- **Lambda `resources/read` handler missing by default** (`turul-mcp-aws-lambda`): HTTP server registered it unconditionally; Lambda only added it when resources were configured. Now registered in `new()` matching HTTP parity.
- **Lambda `resources/templates/list` registered unconditionally** (`turul-mcp-aws-lambda`): Was registered even with no template resources, unlike HTTP which only adds it conditionally. Removed from `new()`, now only added in `build()` when templates exist.
- **Strict lifecycle tests made explicit** (`turul-mcp-aws-lambda`): `build_strict_streaming_handler()` now explicitly sets `.strict_lifecycle(true)` instead of relying on the default.

### Added

- **Lambda handler parity tests** (`turul-mcp-aws-lambda`): `resources/read` registered-by-default test and `resources/templates/list` absent-without-templates test.

## [0.3.20] - 2026-03-16

### Fixed

- **P0: Lambda missing `notifications/initialized` handler** (`turul-mcp-aws-lambda`): Lambda server never registered `InitializedNotificationHandler`, making `strict_lifecycle: true` (default since v0.3.19) non-functional — clients could never complete the MCP handshake. Now registered identically to the HTTP server path.
- **P1: Lambda `tools/list` not session-aware** (`turul-mcp-aws-lambda`): `ListToolsHandler` in Lambda was constructed without session manager, bypassing strict lifecycle checks. Now uses `new_with_session_manager()` consistent with the HTTP server.
- **P1: Streamable HTTP notification race** (`turul-http-mcp-server`): `notifications/initialized` was processed asynchronously via `tokio::spawn`, returning 202 before `is_initialized` was set. If the client sent `tools/list` immediately after, the session would be rejected. Now processed synchronously for `notifications/initialized` specifically; other notifications remain async.

### Added

- **Lambda strict lifecycle E2E tests** (`turul-mcp-aws-lambda`): 4 new tests over `handle_streaming()` with `MCP-Protocol-Version: 2025-11-25` — full handshake, rejection before initialized (with `-32031` error code assertions), immediate post-initialized race proof, and lenient mode fallback.

## [0.3.19] - 2026-03-15

### Changed

- **Strict MCP lifecycle is now the default** (`turul-mcp-server`, `turul-mcp-aws-lambda`): Both `McpServerBuilder` and `LambdaMcpServerBuilder` now default to `strict_lifecycle: true`, requiring clients to send `notifications/initialized` after `initialize` before any other operations. This matches the MCP 2025-11-25 spec. Use `.strict_lifecycle(false)` for legacy clients that skip the notification.

### Fixed

- **Integration tests now perform full MCP handshake** — `mcp_behavioral_compliance`, `session_id_compliance`, and `sse_progress_delivery` tests updated to send `notifications/initialized` after `initialize`.

## [0.3.18] - 2026-03-15

### Changed

- **`create_tables_if_missing` replaced with `verify_tables` + `create_tables`** (`turul-mcp-session-storage`, `turul-mcp-task-storage`): All 6 storage config structs (SQLite, PostgreSQL, DynamoDB × session + task) now use two granular flags. `verify_tables: false` (default) skips all startup verification — eliminates ~1,884 DynamoDB API calls/hour per Lambda server. `create_tables: true` creates tables when missing (only when `verify_tables: true`). **Breaking:** default changed from auto-create to skip-all. For first-time setup, use `verify_tables: true, create_tables: true`.

### Fixed

- **SQLite/PostgreSQL session storage now respect table verification flag** — previously called `migrate()` unconditionally, ignoring the config flag.

## [0.3.17] - 2026-03-15

### Added

- **Custom struct input parameter schema via schemars** (`turul-mcp-derive`): Unknown types in `#[mcp_tool]` parameters (e.g., `Vec<ObserverPoint>`, `MyStruct`) now use `schemars::schema_for!()` to generate correct JSON Schema at runtime instead of falling back to `"type": "string"`. Requires the parameter type to derive `schemars::JsonSchema`. This fixes `Vec<CustomStruct>` parameters generating `{"type": "array", "items": {"type": "string"}}` — they now correctly produce `{"type": "array", "items": {"type": "object", "properties": {...}}}`.

## [0.3.16] - 2026-03-15

### Added

- **Fixed-size array `[T; N]` support in `#[mcp_tool]` schema generation** (`turul-mcp-derive`): `type_to_schema` now handles `[f64; 3]`, `[String; 2]`, `[i32; 4]`, etc. — generating `{"type": "array", "items": ..., "minItems": N, "maxItems": N}` instead of silently falling back to `"type": "string"`. Also handles `Option<[T; N]>`.
- **`with_min_items()` / `with_max_items()` builder methods** (`turul-mcp-protocol-2025-11-25`): `JsonSchema::Array` now supports min/max item count constraints via builder chain.

### Fixed

- **E2E test expected 401 instead of 404 for nonexistent session** (`streamable_http_e2e.rs`): Updated `test_strict_lifecycle_enforcement_over_streamable_http` to expect 404 per MCP 2025-11-25 spec (regression from v0.3.14 session-404 fix).

## [0.3.15] - 2026-03-14

### Added

- **`.icons()` builder method** (`turul-mcp-server`, `turul-mcp-aws-lambda`): Both `McpServerBuilder` and `LambdaMcpServerBuilder` now support `.icons(vec![...])` for setting server icons displayed by MCP clients (e.g., Claude Desktop). Use `Icon::new("https://...")` for URL icons or `Icon::data_uri("image/svg+xml", "<base64>")` for embedded data URIs.
- **`Icon` in protocol prelude** (`turul-mcp-protocol-2025-11-25`): `Icon` is now re-exported via `turul_mcp_server::prelude::*` for convenience.

## [0.3.14] - 2026-03-14

### Fixed

- **Stale/terminated sessions now return 404 per MCP spec** (`turul-http-mcp-server`): `StreamableHttpHandler` previously returned 401 Unauthorized for nonexistent or terminated session IDs. MCP 2025-11-25 requires 404 Not Found so clients know to create a fresh session (not re-authenticate). Missing `Mcp-Session-Id` header (no session ID at all) still returns 401. Storage backend errors return 500.

## [0.3.13] - 2026-03-13

### Changed

- **CORS headers centralized behind constants** (`turul-http-mcp-server`): All CORS header values (`Allow-Methods`, `Allow-Headers`, `Expose-Headers`, `Max-Age`) are now defined as `pub(crate)` constants in `cors.rs`. Inline CORS headers removed from `options_response()`, `StreamableHttpHandler` OPTIONS handler, and `sse_response_headers()`. `CorsLayer::apply_cors_headers()` in `server.rs` is now the single source of truth.
- **`enable_cors = false` now fully respected** (`turul-http-mcp-server`): Previously, inline OPTIONS handlers leaked partial CORS headers even when CORS was disabled. Now `enable_cors = false` produces zero CORS headers on all responses.

### Removed

- **`CorsLayer::apply_cors_headers_for_origin()`** (`turul-http-mcp-server`): Removed — was never wired into the server request pipeline and would be overwritten by the wildcard `apply_cors_headers()` in `server.rs`. For origin-restricted CORS, configure at the reverse proxy layer.
- **`sse_response_headers()`** (`turul-http-mcp-server`): Removed — was never called by the framework. SSE responses are built inline by `StreamableHttpHandler` and `SessionMcpHandler`.
- **Orphan test files** (`turul-http-mcp-server`): Deleted `http_transport_tests.rs` and `sse_tests.rs` — not compiled (missing from `tests/mod.rs`) with 93 compilation errors against the current API.

## [0.3.12] - 2026-03-12

### Fixed

- **CORS: expose `Mcp-Session-Id` header for browser MCP clients** (`turul-http-mcp-server`): Browser-based MCP clients couldn't read the `Mcp-Session-Id` response header because CORS didn't expose it. Added `Access-Control-Expose-Headers: Mcp-Session-Id`, added `Mcp-Session-Id` to `Access-Control-Allow-Headers`, and added `DELETE` to `Access-Control-Allow-Methods` for session teardown. Applies to both wildcard and origin-specific CORS configurations.

## [0.3.11] - 2026-03-09

### Added

- **`run_streaming_with()` custom dispatch** (`turul-mcp-aws-lambda`): Accepts a custom `Fn(Request) -> Future<Response>` closure for Lambda streaming, with the same completion-invocation handling as `run_streaming()`. Use this when you need pre-dispatch logic (e.g., `.well-known` routing) that runs before the MCP handler. Fixes completion-invocation ERROR logs for custom dispatch patterns; does not claim to resolve all Lambda streaming timeout behavior.
- **Prelude re-exports**: `run_streaming` and `run_streaming_with` are now available via `turul_mcp_aws_lambda::prelude::*`.

### Changed

- **`lambda-mcp-server-streaming` example**: Refactored from raw `lambda_http::run_with_streaming_response(service_fn(...))` to `turul_mcp_aws_lambda::run_streaming()`, demonstrating the framework's recommended streaming entry point.

## [0.3.10] - 2026-03-07

### Changed

- **`JwtValidator::new()` now requires audience** (`turul-mcp-oauth`): `JwtValidator::new(jwks_uri, audience)` — audience is a mandatory parameter per MCP spec requirement that servers MUST validate token audience. The optional `with_audience()` method has been removed.
- **`ProtectedResourceMetadata::new()` is now fallible** (`turul-mcp-oauth`): Returns `Result<Self, OAuthError>`. Validates `resource` and `authorization_servers` URIs using `url::Url` — requires http/https scheme, authority present, no fragment. Empty AS list rejected.
- **`oauth_resource_server()` is now fallible** (`turul-mcp-oauth`): Returns `Result<..., OAuthError>`. Enforces exactly one authorization server in metadata (no silent `[0]` fallback). Auto-wires audience from `metadata.resource` and issuer from single AS.

### Added

- **Scope in `WWW-Authenticate`** (`turul-mcp-oauth`): When `scopes_supported` is configured on metadata, challenge responses include `scope="scope1 scope2"` per RFC 6750 §3.
- **`Cache-Control: no-store`** (`turul-http-mcp-server`): All 401/403 challenge responses include `Cache-Control: no-store` per OAuth 2.1 §5.3. Applied in both Streamable HTTP and legacy transports.
- **Canonical URI validation** (`turul-mcp-oauth`): `ProtectedResourceMetadata::new()` validates resource and AS URIs — absolute URI with http/https scheme, authority required, no fragment allowed. New error variants: `OAuthError::InvalidResourceUri`, `OAuthError::InvalidConfiguration`.
- **Single-AS issuer enforcement** (`turul-mcp-oauth`): `oauth_resource_server()` rejects metadata with multiple authorization servers, preventing misconfigured deployments.

## [0.3.9] - 2026-03-06

### Added

- **Lambda streaming event classification** (`turul-mcp-aws-lambda`): Three-way classification of raw Lambda runtime payloads via `classify_runtime_event()` — distinguishes API Gateway events, streaming completion invocations, and unrecognized payloads. Prevents ERROR logs and CloudWatch Lambda Error metrics from completion invocations.
- **`run_streaming()` public API**: Replaces `lambda_http::run_with_streaming_response()` for MCP Lambda servers. Gracefully acknowledges completion invocations (200 + `debug` log) and unrecognized payloads (200 + `warn` log) instead of failing deserialization.
- **Testable surfacing contract**: `handle_runtime_payload()` returns typed `HandleResult { response, event_type }` for observability; `event_log_level()` maps event types to tracing levels — both independently testable without log capture.
- **OAuth resource server foundation** (`turul-http-mcp-server`): Bearer token middleware, route registry, request-scoped extensions on `SessionContext` for auth claims propagation.
- 25 classification/action-path/contract tests with `include_str!` fixture files for API Gateway v1/v2, streaming completion variants, and precedence edge cases.

### Fixed

- **Benchmark compilation**: `SessionContext` struct initializers in `performance-testing` benchmarks updated for new `extensions` field.

## [0.3.8] - 2026-03-05

### Fixed

- **Client streaming response forwarding** (P1): Server-initiated requests (`sampling/createMessage`, `elicitation/create`) now receive JSON-RPC responses back from the client callback. Previously responses were logged and discarded, causing servers to hang indefinitely. Architecture: `StreamHandler` → response channel → consumer task → `transport.send_notification()`. See [ADR-020](docs/adr/020-client-response-forwarding-architecture.md).
- **HTTP transport event classification**: Server-initiated requests (with both `method` and `id`) were misclassified as notifications. Fixed classification order: `method+id` → Request, `method` only → Notification, `id` only → Response.
- **`json_schema_derive.rs` `Option<T>` type-schema**: `generate_field_schema()` now uses `segments.last()` instead of `get_ident()` to handle generic types. `Option<u32>` correctly generates `integer` schema (was falling through to `string`). `is_option_type()` fixed to use `segments.last()` for qualified path support (`std::option::Option<T>`).

### Added

- **Resource `title` attribute**: All three macro paths (`#[derive(McpResource)]`, `#[mcp_resource]`, `resource!{}`) now support `title = "..."` attribute. `HasResourceMetadata::title()` returns the configured value.
- `ServerEvent::Response` variant for distinguishing id-only SSE frames (responses to client-originated requests) from server-initiated requests. `StreamHandler` ignores these — they are handled by the normal request/response matching path.
- Null/missing `id` guard: Server requests without a valid `id` invoke the callback but do not emit a response (per JSON-RPC 2.0 spec).
- 11 new tests covering client response forwarding pipeline (unit + integration + mock transport).

## [0.3.7] - 2026-03-04

### Added

- **Tool annotations macro support**: `#[derive(McpTool)]`, `#[mcp_tool]`, and `tool!{}` now support `read_only`, `destructive`, `idempotent`, `open_world`, `title`, and `annotation_title` attributes — generates `ToolAnnotations` with camelCase JSON keys (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`) per MCP 2025-11-25
- `title` attribute on all three macro paths sets `Tool.title` (via `HasBaseMetadata`); `annotation_title` sets `ToolAnnotations.title` independently
- Boolean annotation type validation: `#[mcp_tool]` rejects wrong types (e.g., `read_only = "true"`) with a compile error

### Fixed

- Terminated sessions (after `DELETE /mcp`) now correctly reject subsequent POST and GET requests in both Streamable HTTP and legacy JSON transports

## [0.3.6] - 2026-03-03

### Fixed

- `#[mcp_tool]` and `#[derive(McpTool)]`: `Option<bool>`, `Option<u32>`, `Option<f64>`, `Vec<T>`, and `Option<Vec<T>>` parameters now generate correct JSON Schema types in `tools/list` input schemas (was incorrectly advertising `"type": "string"` for all generic-arg types)
- Fully-qualified paths (`std::option::Option<T>`, `std::vec::Vec<T>`) now correctly detected across all `is_option_type` checks

## [0.3.5] - 2026-03-03

### Added

- `McpClient::list_resource_templates()` and `list_resource_templates_paginated()` for `resources/templates/list` discovery

### Fixed

- `HttpTransport`: downgraded spurious session ID warning on `initialize` request from `warn!` to `debug!`

## [0.3.4] - 2026-03-03

### Fixed

- `HttpTransport::connect()` and `SseTransport::connect()` no longer send OPTIONS/HEAD pre-flight requests that fail with 405 (direct servers) or 502 (Lambda streaming servers) — connectivity failures now surface at `initialize` time instead of preflight time, matching MCP Inspector behavior
- `#[mcp_tool]` function-attribute macro: `Option<T>` parameters are now correctly excluded from the `required` array in the generated JSON schema (was incorrectly marking them as required unless `#[param(optional)]` was explicitly set)

### Changed

**DynamoDB Storage: camelCase Attribute Names (One-Way Migration):**
- New DynamoDB tables created by `turul-mcp-session-storage` and `turul-mcp-task-storage` now use camelCase attribute names (`sessionId`, `taskId`, `createdAt`, `lastActivity`, etc.) — aligning with DynamoDB convention
- Existing snake_case tables (`session_id`, `task_id`, `created_at`, etc.) are auto-detected via `describe_table()` key schema inspection and continue to work without any changes
- Per-table detection: session and events tables are detected independently, supporting mixed-convention deployments
- Read tolerance: non-key attributes written with either convention are readable via fallback lookup

**Rollback Contract (Breaking Storage Format):**
- This is a **one-way storage format change**. Once new tables are created with camelCase key schemas, pre-v0.3.4 code cannot read them (it has hardcoded snake_case key names)
- New code reads legacy snake_case tables: **Yes** (auto-detected)
- New code creates fresh tables with camelCase: **Yes**
- Old code reads legacy snake_case tables: **Yes** (unchanged)
- **Old code reads new camelCase tables: No — will fail**
- Rolling back to pre-v0.3.4 code after creating camelCase tables will break. Plan accordingly.

## [0.3.3] - 2026-03-01

### Fixed

- PostgreSQL task storage: `tasks.session_id` column type changed from `TEXT` to `VARCHAR(36)` to match `sessions.session_id` and `events.session_id`

## [0.3.2] - 2026-02-28

### Added

- `HasExecution` trait for per-tool task support declaration (follows `HasIcons` supertrait pattern)
- `task_support` attribute on `#[derive(McpTool)]` and `#[mcp_tool]` (`"optional"` | `"required"` | `"forbidden"`)
- `.execution()` builder method on `ToolBuilder`
- Build-time coherence guard rejects `taskSupport=required` without task runtime configured
- `tools/list` strips `execution` field when server has no tasks capability (truthful capability advertisement)
- `tools/call` with `params.task` returns `InvalidParameters` when server has no task runtime (was silent sync fallback)

### Changed

- **Breaking**: `HasExecution` added to `ToolDefinition` supertrait — manual tool impls must add `impl HasExecution for MyTool {}`

### Fixed

- `ToolDefinition::to_tool()` now populates `execution` field from trait (was hardcoded `None`)
- `tools/call` rejects task-augmented requests to tools that don't declare `task_support` (was silently accepted)

## [0.3.1] - 2026-02-28

### Fixed

- `ToolSchemaExt::from_schemars()` now handles schemars v1 nullable type arrays (`"type": ["string", "null"]`) and `anyOf`/null patterns for `Option<T>` fields
- `from_schemars()` enforces `type: "object"` root schema validation per MCP protocol requirements
- `from_schemars()` resolves `$ref` references through both `$defs` and `definitions` maps (merged, not first-hit)

## [0.3.0] - 2026-02-26

### Added

**MCP 2025-11-25 Protocol Support:**
- `turul-mcp-protocol-2025-11-25` crate with full spec compliance (127+ protocol tests)
- `turul-mcp-protocol` alias now re-exports 2025-11-25 types (ADR-015)
- `Icon` struct (`src`, `mime_type`, `sizes`, `theme`) on tools, resources, prompts, resource templates, and implementations
- `Task` struct with `task_id`, `TaskStatus` (`Working`/`InputRequired`/`Completed`/`Failed`/`Cancelled`), `created_at`/`last_updated_at`, `ttl`, `poll_interval`
- `ToolUse` and `ToolResult` content block variants
- `ToolExecution`, `ToolChoice`, `ToolChoiceMode` (`Auto`/`None`/`Required`)
- `TaskStatusNotification` and `ElicitationCompleteNotification`
- URL elicitation mode (`ElicitRequestURLParams`) alongside existing form mode
- `$schema` field on `ElicitationSchema`
- `tools` field on `CreateMessageParams` for sampling with tools
- `ModelHint { name }` struct (replaces closed enum)
- `Implementation` gains `description` and `website_url` fields
- Structured `TasksCapabilities` with `list`, `cancel`, `requests` sub-fields

**Task Storage (`turul-mcp-task-storage` crate):**
- `TaskStorage` trait with zero-Tokio public API
- `InMemoryTaskStorage` with state machine enforcement
- SQLite backend (`SqliteTaskStorage`) — optimistic locking, `julianday()` TTL, background cleanup
- PostgreSQL backend (`PostgresTaskStorage`) — `version` column optimistic locking, JSONB, partial index for stuck tasks
- DynamoDB backend (`DynamoDbTaskStorage`) — conditional writes, GSIs, native TTL, base64 cursors
- 11-function parity test suite shared across all backends
- Feature flags: `sqlite`, `postgres`, `dynamodb` (each opt-in with Tokio)

**Task Runtime & Executor:**
- `TaskExecutor` trait and `TokioTaskExecutor` in `turul-mcp-server`
- `CancellationHandle` for cooperative task cancellation
- `TaskRuntime` with `::new(storage, executor)`, `::with_default_executor(storage)`, `::in_memory()` constructors
- Server handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result` (blocks until terminal per spec)
- Auto-capability advertisement via `McpServer::builder().with_task_runtime()`

**Task Examples:**
- `tasks-e2e-inmemory-server` — task-enabled MCP server with `slow_add` tool
- `tasks-e2e-inmemory-client` — full task lifecycle client (create, poll, cancel, result)
- `client-task-lifecycle` — task API demonstration
- `task-types-showcase` — print-only demo of Task, TaskStatus, TaskMetadata, CRUD types

**Lambda Examples:**
- `lambda-authorizer` — API Gateway REQUEST authorizer with wildcard methodArn for MCP Streamable HTTP

**README Testing Infrastructure:**
- `skeptic` crate for automated markdown code block testing
- README.md files validated as part of `cargo test` suite

### Changed

**Protocol Types (Breaking):**
- `CreateMessageResult` flattened — `role` and `content` at top level (no `message` wrapper)
- `Role` enum: only `User` and `Assistant` (removed `System` variant; system prompts use `systemPrompt` field)
- `ProgressNotificationParams.progress`: `f64` (was `u64`)
- `icon` fields renamed to `icons: Option<Vec<Icon>>` (singular string → plural object array)
- `HasIcon` trait renamed to `HasIcons`; `HasSamplingTools` trait added
- Notification method strings use underscores (`notifications/tools/list_changed`) per spec; JSON capability keys remain camelCase (`listChanged`)
- Default protocol version is 2025-11-25 everywhere; backward-compat 2025-06-18 paths annotated with `// Intentional`

**Test Infrastructure:**
- 1,560+ workspace tests passing, 98 doctests, zero warnings
- Test binaries reduced from 155 to 43 via consolidation (Phase F)
- Root integration tests: 39 → 8 binaries (5 consolidated in `tests/consolidated/` + 3 standalone)
- Sub-crate integration tests: 24 → 7 binaries (`tests/*/tests/all.rs` with `#[path]` imports)
- Derive crate integration tests moved to workspace root (2 binaries eliminated)

**Examples:**
- 58 active examples (up from 42+ in v0.1.0), 25 archived
- 12 core crates in workspace

**Documentation:**
- README narrative updated to reflect spec-pure protocol crate design
- All 20+ protocol crate README code examples tested and verified
- Documentation accuracy fixes across READMEs, ADRs, and compliance reports (repo URL, config field names, notification method strings, version references, port numbers)
- CHANGELOG duplicate `[0.2.0]` sections merged
- ADR-009 updated with `V2025_03_26` and `V2025_11_25` protocol versions
- ADR-004 status updated from CRITICAL to Accepted (Implemented)
- Stale MIGRATION_0.2.1.md references removed workspace-wide

### Fixed

- Sampling server README: removed `System` role, fixed `ModelHint` to object form, corrected snake_case JSON fields to camelCase
- Session storage README: corrected config field names (`session_timeout_minutes`, `database_url`, `PostgresConfig`)
- Compliance reports marked as historical with accurate resolution status
- Client README compatibility list now includes 2025-11-25
- Protocol alias ADR updated from 2025-06-18 to 2025-11-25
- Notification method strings in ADR-005 and E2E test plan corrected to `list_changed`

## [0.2.1] - 2025-10-08

### Breaking Changes

**Schemars Integration (Detailed Schema Generation):**
- **BREAKING**: Tool output types MUST now derive `schemars::JsonSchema`
- **Impact**: Tools with custom output types generate detailed schemas with full property information
- **Migration**: Add `#[derive(JsonSchema)]` to all tool output types:
  ```rust
  use schemars::JsonSchema;

  #[derive(Serialize, Deserialize, JsonSchema)]  // Added JsonSchema
  struct MyOutput {
      result: f64,
      message: String,
  }
  ```
- **Benefit**: All tools now provide detailed schemas in `tools/list` with property names, types, and descriptions
- **Note**: `schemars` is already a workspace dependency - no Cargo.toml changes needed

**Framework Trait Reorganization (Protocol Crate Purity):**
- **BREAKING**: All framework traits moved from `turul-mcp-protocol` to `turul-mcp-builders::traits`
- **BREAKING**: `HasNotificationPayload::payload()` now returns `Option<Value>` (owned) instead of `Option<&Value>` (reference)
- **Impact**: Protocol crate is now 100% MCP spec-pure (no framework-specific code)
- **Migration**: Update imports to use preludes:
  ```rust
  // Before
  use turul_mcp_protocol::{ToolDefinition, ResourceDefinition};

  // After
  use turul_mcp_builders::prelude::*;  // or turul_mcp_server::prelude::*
  ```
- **Migration Guide**: See the breaking changes listed above for step-by-step migration instructions

### Fixed

**Critical Notification Payload Regression:**
- Fixed all notification types returning `None` for payloads (data loss bug)
- Base Notification now properly serializes `params.other` and `_meta`
- ProgressNotification now preserves progressToken, progress, total, message, _meta
- ResourceUpdatedNotification now preserves uri, _meta
- CancelledNotification now preserves requestId, reason, _meta
- All list-changed notifications now preserve _meta fields
- Added 18 comprehensive tests validating notification payload correctness

### Changed

**Framework Trait Locations:**
- Moved 10 trait hierarchies (~1200 LOC) from protocol to builders crate
- All protocol type implementations now in `turul-mcp-builders/src/protocol_impls.rs`
- Derive macros updated to generate correct trait signatures
- All examples and tests updated to use new import paths

## [0.2.0] - 2025-10-05

### Added

**MCP 2025-06-18 Specification:**
- Full compliance with MCP 2025-06-18 spec
- Session-Aware Resources: All resources now support `session: Option<&SessionContext>` parameter
- Sampling Validation Framework: `ProvidedSamplingHandler` for request validation
- SSE Streaming: Chunked transfer encoding with real-time notifications
- CLI Support: All test servers now support `--port` argument with dynamic binding
- Path Normalization: Traversal attack detection in roots validation
- Strict Lifecycle Mode: Optional strict session initialization enforcement

**Middleware System:**
- Complete middleware architecture for HTTP and Lambda transports
- `.middleware()` builder method on `McpServer` and `LambdaMcpServerBuilder`
- Transport-agnostic middleware execution (FIFO before dispatch, LIFO after)
- Session-aware middleware with `StorageBackedSessionView` and `SessionInjection`
- Error short-circuiting with semantic JSON-RPC error codes

**Middleware Examples:**
- `middleware-auth-server` - API key authentication (HTTP)
- `middleware-auth-lambda` - API key authentication (AWS Lambda)
- `middleware-logging-server` - Request timing and tracing
- `middleware-rate-limit-server` - Per-session rate limiting

**Testing Infrastructure:**
- Shared verification utilities (`tests/shared/bin/wait_for_server.sh`)
- Test server bin targets in all test packages (tools, prompts, resources, sampling, roots, elicitation)
- Comprehensive example verification suite (5 phases, 31 servers)
- Session lifecycle compliance: `notifications/initialized` in all e2e tests

### Changed

- **Resource Trait**: Updated `read()` signature to include session parameter
- **Tool Output**: Tools with `outputSchema` automatically include `structuredContent`
- **Error Handling**: Session lifecycle violations use `SessionError` type
- **Pagination**: Reject `limit=0` to prevent stalls
- **HTTP Transport**: Protocol-based routing (≥2025-03-26 uses streaming, ≤2024-11-05 uses buffered)
- SSE keepalives use comment syntax for better client compatibility
- DynamoDB queries use strongly consistent reads
- Lambda `LambdaMcpHandler` now cached globally (preserves DynamoDB client, StreamManager, middleware instances)
- Test packages updated to Rust edition 2024 and tokio version "1"
- Middleware stack execution order documented (FIFO/LIFO)

### Fixed

**Examples (4 bugs fixed):**
- pagination-server: Database unique constraint error (email generation duplicates)
- comprehensive-server: Missing resources and prompts registration
- audit-trail-server: SQLite connection URL missing protocol and create mode
- All 30/31 examples now verified working (96.8% passing, 1 skipped for PostgreSQL)

**Protocol & Core:**
- SSE resumability: Keepalive events preserve Last-Event-ID for proper reconnection
- MCP Inspector compatibility: Events use standard `event: message` format
- Lambda notifications: DynamoDB consistent reads fix race condition
- Lambda handler caching: Global `OnceCell` preserves handler instance (DynamoDB client, StreamManager, middleware) across invocations
- Tool output: Schema and runtime field names now consistent
- CamelCase: Proper acronym handling (GPS → gps, HTTPServer → httpServer)
- Lambda compilation: Fixed `LambdaError::Config` reference
- **TestServerManager**: Blocking wait for process termination, prevents zombie processes
- **Session Tests**: Correct response structure (`output` vs `value`)
- **Prompt Arguments**: Fix argument name mismatches in test expectations
- **MCP Inspector**: Enable compatibility with MCP Inspector and FastMCP clients
- **Zero-Config**: Correct output field expectations for derived tools
- **Borrow Checker**: Resolve errors in `roots_derive` macro

**Code Quality:**
- Fixed 14 collapsible_if clippy warnings using Rust 2024 let-chain syntax
- Fixed unused variable warnings in test suite
- Fixed useless type conversions in Lambda tests
- All clippy warnings addressed (100% clean workspace builds with `-D warnings`)

**Verification Infrastructure:**
- Scripts use deterministic 15s polling instead of fixed sleeps
- Pre-built binaries eliminate compilation timeouts
- SKIPPED tracked separately from PASSED (no hidden failures)
- Build errors properly diagnosed with detailed logs

### Examples
- Restored `roots-server` with clap CLI (108 lines, down from 512)
- Updated `elicitation-server` with multi-path data loading
- Updated `sampling-server` with dynamic port binding
- Updated `pagination-server` with proper SQLite URI (`?mode=rwc`)
- All 31 core examples verified and working

### Documentation

- README middleware section with examples and testing commands
- AGENTS.md middleware guidance with ADR 012 reference
- Doctests passing: turul-mcp-derive (25/25), turul-mcp-protocol (7/7)
- Complete verification run documented with bug fixes and runbook
- Middleware testing scripts: `test_middleware_live.sh` and Lambda examples
- Updated CLAUDE.md with session-aware patterns
- Updated EXAMPLES.md with validation results
- Added curl and jq to auto-approved commands
- Comprehensive test coverage documentation

### Tests

- 440+ unit tests passing (161 integration tests across 20 test suites)
- 30/31 examples verified (Phases 1-5: 100% passing)
- Middleware parity tests verify HTTP/Lambda consistency
- All critical functionality validated

## [0.1.0] - Initial Release

### Added
- Core MCP server framework
- Tool creation patterns (function, derive, builder, manual)
- Resource management with templates
- Prompt generation system
- Session management with multiple storage backends
- HTTP transport layer
- Client library
- Builder patterns
- AWS Lambda support
- 42+ working examples

[Unreleased]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.4.2...HEAD
[0.4.3]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.47...v0.4.0
[0.3.47]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.46...v0.3.47
[0.3.46]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.45...v0.3.46
[0.3.45]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.44...v0.3.45
[0.3.44]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.43...v0.3.44
[0.3.43]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.42...v0.3.43
[0.3.42]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.41...v0.3.42
[0.3.41]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.40...v0.3.41
[0.3.40]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.39...v0.3.40
[0.3.39]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.38...v0.3.39
[0.3.38]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.37...v0.3.38
[0.3.37]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.36...v0.3.37
[0.3.36]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.35...v0.3.36
[0.3.35]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.34...v0.3.35
[0.3.34]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.33...v0.3.34
[0.3.22]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.21...v0.3.22
[0.3.21]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.20...v0.3.21
[0.3.20]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.19...v0.3.20
[0.3.19]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.18...v0.3.19
[0.3.18]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.17...v0.3.18
[0.3.17]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.16...v0.3.17
[0.3.16]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.15...v0.3.16
[0.3.15]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.14...v0.3.15
[0.3.14]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.13...v0.3.14
[0.3.13]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.12...v0.3.13
[0.3.12]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.11...v0.3.12
[0.3.11]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.10...v0.3.11
[0.3.10]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.9...v0.3.10
[0.3.9]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.8...v0.3.9
[0.3.8]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aussierobots/turul-mcp-framework/releases/tag/v0.1.0
