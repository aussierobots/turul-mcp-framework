# 2026-07-28 Release Readiness Review

Branch: `feat/turul-mcp-protocol-2026-07-28`. Reviewed 2026-06-10 against the **live**
MCP draft specification (https://modelcontextprotocol.io/specification/draft), the
vendored pin (`crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts`, sha256
`20df36f9…`, pinned 2026-06-07), and the governing ADRs (009, 014, 023, 025, 027,
028, 029, 030).

Method: three concurrent review tracks — (1) the `2026-compliance-review` multi-agent
workflow (16 domain auditors + adversarial verification: **116 findings upheld, 13
refuted**), (2) a live-draft validator that fetched the draft pages and byte-diffed the
upstream `schema.ts` against our pin, (3) an e2e test-coverage inventory. Three
late-phase workflow audits (rollout-gap, deprecations, purity/comments) and its
synthesis step aborted on an account spend limit; their scope is covered by tracks
2–3 and the journal-recovered findings below.

**Verdict: NOT release-ready.** The protocol crate is in good shape (343 tests green,
0 warnings, 20/20 upstream fixtures, pin one cosmetic diff behind upstream), but the
**transport and framework layers do not yet implement several prose-level MUSTs** of
the draft spec that have no schema symbol and were therefore invisible to
schema-fidelity auditing: MRTR, `subscriptions/listen`, required `Mcp-Method`/
`Mcp-Name` headers, the new HTTP status/error-code mapping, and per-request log-level
gating. One HIGH framework bug (outputSchema dropped from `tools/list` on the 2026
path) and two HIGH elicitation binding defects also block. The schema re-vendor (§1)
is incidental — a one-symbol cosmetic catch-up, not the work. The error-code/status
mapping items are P0 interop contracts, not cleanup: an implementation can pass every
serde/schema test and still fail interop on them.

---

## 1. Pin status vs live draft

Exactly **one** upstream `schema.ts` change since our 2026-06-07 pin (live sha256
`1bf94a60…`): `ElicitationCompleteNotificationParams` extracted into a named interface
that `extends NotificationParams` (formally gains the optional `_meta` carrier).
LOW wire impact. **Action: re-vendor** (`cargo run --features compliance -- refresh
--write`) and re-check the `ElicitationCompleteNotification` binding for `_meta`
acceptance. Everything else — `server/discover`, `subscriptions/listen`,
`InputRequiredResult`, `CacheableResult`, error codes `-32004`/`-32003`/`-32602`,
removals of `ping`/`tasks/*`/`logging/setLevel`/`resources/subscribe` — is already in
the pin, byte-identical.

Live-draft content **newer than the 2026-07-28 RC blog post** (not yet reflected in
this branch's planning docs):

- **Dynamic Client Registration (RFC 7591) is now Deprecated** (PR #2858); AS +
  clients SHOULD support **Client ID Metadata Documents** (CIMD,
  draft-ietf-oauth-client-id-metadata-document-00). `turul-mcp-oauth` has no CIMD
  surface.
- Feature-lifecycle policy (SEP-2596) + the deprecated-features registry page
  (includes HTTP+SSE transport and `includeContext` reclassifications).
- RFC 9207 forward-notice: `iss` inclusion expected to move SHOULD → MUST in a future
  revision.
- Versioning page now states a dual-era server **MAY** serve both eras concurrently on
  one endpoint — legitimizing the runtime-routing pattern ADR-029 rejected. ADR-029's
  single-spec-per-build choice remains conformant (MAY), but record this in ADR-029's
  revision log if re-litigated.
- Tasks extension detail: live extension has `tasks/update`; no `tasks/list`; no
  blocking `tasks/result`.

### Authority & wording discipline

Two distinct mechanisms must never be conflated in this branch's docs or claims, and
each claim must name its authority:

1. **Absent from the pinned schema** (stateless-core redesign): `InitializeRequest`/
   `initialize`, `ping`, `tasks/*` (core), `logging/setLevel` (RPC),
   `resources/subscribe`, `RootsListChangedNotification`. Authority: the vendored
   `schema.ts` byte-diff (and confirmed absent from the live Schema Reference page,
   checked 2026-06-10). These may be described as "absent from the pinned schema" /
   "replaced by the stateless redesign".
2. **Deprecated-but-present** (SEP-2577 / PR #2858 lifecycle): `ListRootsRequest`,
   `CreateMessageRequest`, `LoggingMessageNotification`, `LoggingLevel`,
   `ToolUse`/`ToolResult` content, `ModelPreferences`/`ModelHint`/`ToolChoice`, DCR.
   These remain in the schema and the Schema Reference with deprecation notices.
   **Never describe these as "removed"** — under the lifecycle policy no deprecated
   feature has been removed yet (earliest eligibility 2027-07-28).
3. **Framework lane-split choices**: when this branch's 2026 default declines to
   serve a surface (e.g. sessionful lifecycle, GET SSE), describe that as framework
   policy / the 2025 opt-in lane — not as a spec removal.

The website's prose pages and the schema file can be in tension (prose MUSTs with no
schema symbol — §2 P0 items — and rendering artifacts). Where they disagree, this
branch follows **the pinned `schema.ts` for wire shapes** and **the spec prose for
transport behavior**, recording any residual conflict here rather than silently
picking one.

---

## 2. Release checklist

> **Status update (final, 2026-06-10 release-prep sweep):** every P0 AND every
> P1 below is CLOSED (real-HTTP tests in the default-2026 CI lane,
> revert-and-fail recorded per slice). Functional completions beyond the
> original list: MRTR on all three permitted methods, client-side
> `subscriptions/listen` (`McpClient::subscriptions_listen` + e2e),
> `completion/complete` e2e, `resources.subscribe` truthfulness, the ADR-025
> framework shim cut (shim repinned to terminal `0.3.47`), CIMD/DCR posture
> dispositioned (RS-only crate), schema pin `1bf94a60`, lossless
> builders/derive schema pipeline. **True remaining exceptions:**
> (1) the residual P2 comment/doc lows from the audit journal
> (`wf_d1bd157b-617`) not covered by the hygiene sweeps;
> (2) `requestState` integrity (HMAC) is the tool author's concern — the
> framework does not sign it;
> (3) cyclic/non-local `$ref` tool schemas are documented rejections;
> (4) client-side CIMD belongs to a future full MCP OAuth client flow;
> (5) release/cutover/publish disposition remains the maintainer's call per
> the Branch Lock.

Status key: ☐ open ☑ done. P0 = spec MUST not met or HIGH defect (release blockers).
P1 = mediums (spec SHOULDs, fidelity drift, ADR/doc contract drift). P2 = lows.

### P0 — Base protocol (transport: `turul-http-mcp-server`)

- ☑ **Implement `subscriptions/listen`** (long-lived POST SSE): server MUST ack first
  with `notifications/subscriptions/acknowledged`, MUST NOT send unrequested
  notification types, every delivered notification MUST carry
  `io.modelcontextprotocol/subscriptionId` in `_meta`. Protocol types exist
  (`src/subscriptions.rs`); **no dispatch handler exists**.
- ☑ **Gate the legacy GET/DELETE surface out of the 2026 path**: GET/DELETE on a
  modern-only server SHOULD return **405**; `Mcp-Session-Id` and `Last-Event-ID` MUST
  be ignored ("Resumable SSE streams via Last-Event-ID are not supported"). Today
  `streamable_http.rs:521-538` routes GET→SSE and DELETE→session-delete with no
  `protocol-2026-07-28` gating.
- ☑ **MRTR server production**: servers MUST use the `InputRequiredResult` pattern
  (only on `tools/call`, `resources/read`, `prompts/get`; ≥1 of
  `inputRequests`/`requestState`; MUST NOT request undeclared client capabilities;
  `requestState` treated as attacker-controlled — HMAC/AEAD if it affects authz).
  Types exist; no production path in `turul-mcp-server`.
- ☑ **Enforce `Mcp-Method` (all POSTs) + `Mcp-Name` (tools/call, resources/read,
  prompts/get)**: missing/mismatch → HTTP 400 + `-32001 HeaderMismatch`. Constants
  exist (`protocol-2026-07-28/src/headers.rs:21,26`) but zero transport enforcement;
  the headers.rs comment claiming enforcement lives in `turul-http-mcp-server` is
  currently false.
- ☑ **Header/body protocolVersion mismatch must return `-32001 HeaderMismatch`**, not
  `-32602` (`streamable_http.rs:1459-1475`). Note the same code space already uses
  `-32001` for the 2025 missing-session error — disambiguate per lane.
- ☑ **Emit `-32004 UnsupportedProtocolVersionError`** (HTTP 400, `data.supported` /
  `data.requested`) for unsupported requested versions. Constant exists; zero
  emission sites.
- ☑ **Unknown method → HTTP 404 + `-32601`** (today JSON-RPC errors ride HTTP 200).
  Keep the deliberate 200/`-32601` for `server/discover` probes on the 2025 lane —
  that one is the negotiation contract.
- ☑ **Gate `notifications/message` on per-request `_meta`
  `io.modelcontextprotocol/logLevel`**: server MUST NOT emit it for requests lacking
  the key. Only reference today is the `_meta`-echo exclusion list
  (`handlers/mod.rs:56`).

### P0 — Server features / framework

- ☑ **HIGH: `ToolDefinition::to_tool()` hardcodes `output_schema: None` on the 2026
  path** (`turul-mcp-builders/src/traits/tool_traits.rs:145`) — `tools/list` can never
  advertise `outputSchema`, which also breaks the structuredContent contract.
- ☑ **Fix `x-mcp-header` wire-shape drift** (`protocol-2026-07-28/src/headers.rs:34`):
  live spec (SEP-2243) says `x-mcp-header` is a JSON-Schema **annotation inside
  `inputSchema`**, and the wire header is **`Mcp-Param-{name}`** with `=?base64?…?=`
  encoding. Our constant/doc says clients send `x-mcp-header-<name>` headers — wrong.
- ☑ **Review `cacheScope` defaults**: every result defaults `(ttlMs=0, Public)`;
  spec-valid, but `"public"` on `resources/read` results that depend on the
  authenticated user is the exact data-sharing risk the caching page warns about. No
  handler-level override path is exercised anywhere. Default `resources/read` to
  `private` (or force an explicit choice).

### P0 — Protocol crate (HIGH bindings, spec-compliance scope so purity-allowed)

- ☑ **`PrimitiveSchemaDefinition` untagged union silently destroys enum constraints
  on deserialize** (`elicitation.rs:265-272`).
- ☑ **Same union omits the four DRAFT-2026 single/multi-select enum schema variants**
  (`elicitation.rs:265-272`).
- ☑ Re-vendor the schema pin (§1) — picks up `ElicitationCompleteNotificationParams
  extends NotificationParams`.

### P0 — Client (`turul-mcp-client`, ADR-030 scope)

- ☑ **Emit `Mcp-Method` / `Mcp-Name` headers** on 2026 connections (MUST).
- ☑ **Handle `resultType: "input_required"`** (MRTR client arm): construct inputs,
  echo `requestState` opaquely, retry with a NEW JSON-RPC id. Flagged pending in
  ADR-030's revision log.
- ☑ **`x-mcp-header` → `Mcp-Param-{name}` mirroring** incl. rejecting tools with
  invalid annotation values (excluding them from `tools/list`).
- ☑ Record in ADR-030 that our `-32601`-only fallback is deliberately **narrower**
  than the live draft's "fall back on a 400 whose body is not a recognized modern
  error" — security-motivated, defensible, now a documented divergence.

### P1 — Protocol-crate fidelity mediums (workflow-verified)

- ☑ (2026-06-10 enum-union rework) Legacy `EnumSchema` missing `default?: string`; upstream `EnumSchema` union has
  no Rust binding and its name is reused for `LegacyTitledEnumSchema`
  (`elicitation.rs:62-80`); no test deserializes enum schemas through
  `ElicitationSchema.properties`.
- ☑ (2026-06-10) `ToolChoice`: non-spec `name` field serializes onto the wire; `mode` required in
  Rust but optional in schema (`sampling.rs:114-116`).
- ☑ (2026-06-10) `PromptReference`: omits `title`, carries non-spec `description`
  (`completion.rs:23-28`).
- ☑ (2026-06-10) `Annotations.audience` is `Vec<String>` where schema requires closed `Role[]`
  (`meta.rs:24`).
- ☑ (2026-06-10) `CacheableResult.ttlMs` bound as `u64` where schema declares `number` — floats
  rejected on deserialize (`caching.rs:70` + 6 embedding sites).
- ☑ (2026-06-10) SEP-2577 deprecation of `LoggingLevel` / per-request `logLevel` `_meta` key not
  absorbed from the re-pin; rustdoc asserts the opposite (`logging.rs:29-44`,
  `meta.rs:298`, `notifications.rs:373`). Same for `ServerCapabilities.logging`, and
  `traits.rs:358-452` has `#[allow(deprecated)]` without `#[deprecated]` while
  COMPLIANCE.md claims otherwise.
- ☑ (2026-06-10) `LoggingCapabilities`/`CompletionsCapabilities` invent named fields where the
  schema declares opaque objects (`initialize.rs:221`).
- ☑ (2026-06-10) JSON Schema 2020-12 pipeline: `ToolSchema::from_schemars` strips
  `$defs`/`definitions` but passes `$ref` through verbatim → dangling references for
  nested types (`schemars_helpers.rs:384`); derive macros still funnel through the
  lossy ADR-014 converter (`turul-mcp-derive/src/utils.rs:457`).

### P1 — ADR / docs contract drift (workflow-verified)

- ☑ (2026-06-10) ADR-025 shim: the dep is CUT — all four non-frozen framework
  crates depend on `turul-rpc` directly, and the shim's manifest is repinned
  to the terminal `0.3.47` (in-workspace only for the frozen 2025 snapshots
  and 2025-pinned test/example crates). See ADR-025 revision log.
- ☑ (2026-06-10, alias dep removed in a0ada8cf) ADR-030/ADR-001/CLAUDE.md claim the bilingual client doesn't import the
  `turul-mcp-protocol` alias — the client manifest pins it with 19 source imports.
- ☑ ADR-029 prescribes `cargo test --workspace` matrices that cannot compile (§CI surface rewritten 2026-06-10) (feature
  unification trips the alias mutex — empirically verified).
- ☑ `docs/plans/2026-07-28-schema-coverage-matrix.md` is stale (STALE banner added 2026-06-10) post Slice A'/A'' and
  the 2026-06-07 re-vendor.
- ☑ (2026-06-10, dispositioned as docs/tests-only) `turul-mcp-oauth`: absorb DCR
  deprecation / CIMD SHOULD (§1). **Finding:** the live draft assigns the CIMD
  SHOULD to *authorization servers and MCP clients*; DCR (deprecated, MAY) is
  likewise AS/client-side. `turul-mcp-oauth` implements the resource-server
  role only (RFC 9728 PRM + OAuth 2.1 §5.2 token validation, both unchanged) —
  no CIMD or DCR surface belongs in it, and none was invented. Posture recorded
  in the crate docs; a wire-shape test pins that the RFC 9728 document carries
  no client-registration keys. A future full MCP OAuth *client* flow (in
  `turul-mcp-client` or an app layer) is where client-side CIMD would land.

### P2 — Lows (64 upheld; do as one sweep slice)

Mostly comment/doc drift the Comments rule already forbids: stale "Fine-Grained
Traits" banners, `(A7)`/`(A8)` slice tags, schema-line-numbered test names
(`log_level_constant_matches_schema_line_106` — line moved at the re-pin), tombstone
phrasing, COMPLIANCE.md count mismatches (124 vs 127 fixture files; deviations list
out of sync), `caching.rs` documenting a flatten pattern no result uses, duplicate
`Role` enums, `includeContext` as open `String`, `RootsCapabilities.list_changed`
retained vs schema `{}`, result index-signature members dropped on re-serialize,
speculative `ParamExtractor` traits with zero impls, framework helper traits
(`JsonSchemaGenerator`/`ToJsonSchema`) living in the protocol crate against the
purity rule. Full machine-readable list: workflow run `wf_d1bd157b-617` journal
(116 upheld findings, per-finding file:line and rationale).

---

## 3. E2E testing — current state and required plan

### Today

- **2026 default path**: ONE real-HTTP acceptance suite —
  `crates/turul-mcp-server/tests/discover_stateless_2026.rs` (8 tests:
  `server/discover`, sessionless `tools/call` + list ops, `_meta` enforcement,
  header/body mismatch). Plus client wiremock suites (`bilingual_negotiation.rs`,
  `bilingual_2026_operations.rs`) and ~180 in-process serde compliance tests +
  20/20 upstream fixtures.
- **2025 opt-in lane**: carries nearly all real-wire coverage (~40 real-HTTP tests:
  tools/resources/prompts/roots/sampling/elicitation/tasks/session lifecycle/SSE
  resumability).
- Lanes run via `.github/workflows/ci.yml` (4 jobs) / `scripts/ci-gates.sh`.

### The gate

**E2E is the release gate, not a follow-up.** A P0 item does not close without a
real-HTTP positive + negative wire test landing in the default-2026 CI lane
(`ci.yml` + `ci-gates.sh`) in the same slice — per CLAUDE.md §Test Coverage
Discipline (production path, wire-layer bytes, revert-and-fail). The single existing
real-HTTP 2026 suite is not sufficient evidence of interop.

### Required for release (2026 path) — in dependency order

1. **A 2026 e2e server+client harness** mirroring `tests/tools` et al.: real HTTP
   server on the 2026 default features, driven by the bilingual client (not raw
   JSON), per CLAUDE.md §Test Coverage Discipline (production-path + wire-layer
   bytes, revert-and-fail).
2. **Negative transport tests** for every P0 item as it lands: GET/DELETE → 405;
   unknown method → 404/`-32601`; unsupported version → 400/`-32004`; missing/wrong
   `Mcp-Method`/`Mcp-Name` → 400/`-32001`; header/body mismatch → 400/`-32001`;
   `notifications/message` suppressed without `logLevel` `_meta`.
3. **`subscriptions/listen` e2e** once implemented: ack-first ordering, type
   filtering, `subscriptionId` `_meta` on every event, stream teardown.
4. **MRTR multi-round-trip e2e**: `tools/call` → `input_required` →
   client constructs inputs + echoes `requestState` → new-id retry → `complete`.
5. **Existing zero-coverage areas**: `completion/complete` (handler exists, zero
   e2e), `elicitation/create` on stateless semantics, client cursor/pagination
   round-trip on real HTTP, caching fields (`ttlMs`/`cacheScope`) asserted per list
   op, OAuth (RFC 9728 PRM / 9207 `iss` / audience) against `turul-mcp-oauth`.
6. **CI wiring**: each new suite added to the default-2026 lane in both
   `ci.yml` and `ci-gates.sh` in the same slice.

Out of scope until implemented in the framework: MCP Apps (SEP-1865), extensions
registry (SEP-2133), tasks extension crate (ADR-028), OTel trace-context `_meta`
passthrough.

---

## 4. Deprecated features (Roots / Sampling / Logging / DCR) — policy + decision

Live registry (SEP-2596 lifecycle, minimum twelve-month window):

| Feature | Deprecated in | Earliest removal | Spec migration target |
|---|---|---|---|
| Roots (SEP-2577) | 2026-07-28 | ≥ 2027-07-28 | tool params / resource URIs / config |
| Sampling (SEP-2577) | 2026-07-28 | ≥ 2027-07-28 | direct LLM provider APIs |
| Logging (SEP-2577) | 2026-07-28 | ≥ 2027-07-28 | stderr (stdio) / OpenTelemetry |
| DCR RFC 7591 (PR #2858) | 2026-07-28 | ≥ 2027-07-28 | Client ID Metadata Documents |
| HTTP+SSE transport | 2025-03-26 | 3 months after SEP-2596 Final | Streamable HTTP |

Normative: "remains part of the specification … new implementations SHOULD NOT adopt
it, and existing implementations SHOULD migrate before the feature's earliest
removal." Removal is a Core Maintainer decision at release prep, not automatic.

Key architectural nuance: in the 2026 redesign the deprecated client features only
exist **as MRTR input requests** (`ListRootsRequest`/`CreateMessageRequest` inside
`inputRequests`) and logging only via per-request `logLevel` `_meta` — the pinned
schema has no standalone server→client request channel (MRTR replaces it), so the
old delivery mechanism is absent from the schema independent of the types'
deprecation status (§1 Authority note, mechanism 1 vs 2).

**Framework decision (recommended):**

1. **Keep them, lane-split** (already the architecture): full Roots/Sampling/Logging
   support stays on the 2025-11-25 opt-in lane with its e2e suites; the 2026 default
   path serves them only in the forms the 2026 spec actually defines (MRTR input
   requests; `logLevel`-gated `notifications/message`).
2. **Carry the spec's own deprecation through the bindings**: add the missing
   `#[deprecated]` markers the re-pin introduced (LoggingLevel, `logLevel` key,
   `ServerCapabilities.logging`, ToolUse/ToolResult content, ModelPreferences/
   ModelHint/ToolChoice, traits.rs SEP-2577 surface) so downstream users get compiler
   nudges — and fix the rustdoc that currently asserts the opposite.
3. **No new feature work on deprecated surfaces**; per CLAUDE.md §Active Development,
   any temporary compatibility carries an owner + removal trigger. Trigger here:
   removal lands when the upstream spec removes the feature (≥ 2027-07-28) **or**
   when the maintainer retires the 2025-11-25 lane, whichever is first.
4. **OAuth**: treat DCR as legacy-supported, add CIMD support before claiming 2026
   auth compliance; keep RFC 9207 `iss` validation mandatory in the client (it's
   heading to MUST).

---

## 5. Recommended next slice (interop-first ordering)

Do not start with docs or the re-vendor. Close the externally visible wire contracts
first, each with its real-HTTP test in the same slice (§3 gate):

1. Gate the 2026 path's GET SSE / `Last-Event-ID` / DELETE surface (405; ignore
   `Mcp-Session-Id`).
2. Implement `subscriptions/listen` (ack-first, type filtering, `subscriptionId`
   `_meta`).
3. Enforce (server) and emit (client) `Mcp-Method` / `Mcp-Name`; implement
   `x-mcp-header` → `Mcp-Param-{name}`.
4. Fix the exact error-code/status mapping (`-32001` HeaderMismatch, `-32004`,
   404/`-32601`, `logLevel` gating) and the outputSchema HIGH.
5. MRTR production (server) + consumption (client).
6. **Then** re-vendor the schema pin and sweep docs/ADRs (P1/P2) so they describe
   implemented reality instead of planned behavior.

## 6. What was verified vs inferred

- Workflow findings: each upheld finding cites file:line and survived an independent
  refutation pass (13 were refuted and are excluded). Build/test reality measured,
  not assumed (343/333 tests, 0 warnings, clippy clean, fixtures 20/20).
- Live-draft drift: authoritative byte-diff of upstream `schema.ts` vs the pin;
  prose MUSTs grep-verified to emission/enforcement sites (or their absence) in
  `turul-http-mcp-server`, `turul-mcp-server`, `turul-mcp-client`.
- Live Schema Reference page checked 2026-06-10: `initialize`/`ping`/`tasks/*`
  absent (an external-review claim that the page still lists them did not hold);
  `LoggingLevel` shown with its SEP-2577 deprecation notice, consistent with the
  deprecated-but-present set in the Authority note.
- UNVERIFIED items intentionally left open: `-32003` emission semantics, Origin→403
  mapping, 202-no-body on the 2026 notification path, JSON-Schema validator
  behavior on network `$ref`, `turul-mcp-oauth` internals (out of scoped crates).
- Not audited (spend-limit abort): a dedicated rollout-gap pass, a dedicated
  deprecations pass (covered instead by track 2 §4 above), and a purity/comments
  pass (substantially covered by the 64 P2 lows).
