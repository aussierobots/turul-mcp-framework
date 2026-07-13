# Draft-migration audit — live MCP draft vs vendored pin (2026-07-13)

**Status: AUDITED — unresolved MUST gaps recorded.** Implemented-and-verified fixes landed
in commits `87430c17` / `4690e007` / `a14d9f91` / `fdfcc2a9` / `509a3552` plus the
2026-07-14 F1/F2 slice (§7). The compliance matrix retains recorded MUST-level ❌ gaps —
notably BP-3 (JSON Schema dialect validation), GAP-CF-9 (sampling message-shape
enforcement), and the
server-sent cancellation/subscription-close obligation (row ~332) — a documented
limitation does not satisfy a MUST, and this branch must not be described as fully
latest-draft compliant until they are implemented. This document is the reviewable record
for the migration of `feat/turul-mcp-protocol-2026-07-28` onto the latest live MCP draft;
grades and dispositions follow the conventions of `2026-07-28-spec-compliance.md`
(the 541-row matrix), which this audit updates rather than replaces.

## 1. Immutable upstream basis

| What | Value | Verified |
|---|---|---|
| Live repo HEAD | `62811256f1aa73417c00a3c6dca262cde4ed09c5` (2026-07-13T08:22Z) | 2026-07-13 |
| Prior audit basis this session | `2807f9d6d8ae2012e09377908f47cff16a2b9489` (2026-07-11T19:41Z) | 2026-07-13 |
| Delta `2807f9d6..62811256` | 4 commits; `package.json` + `package-lock.json` only (dependabot). **No spec-surface change.** | 2026-07-13 |
| `schema/draft/schema.ts` | byte-identical to vendored pin, sha256 `6e4cba2d17f7156877357762b6b4b63cd790d8973f61ec35ab73cd61ad67017d`; last-touched upstream commit `93671a3f2bac3bc11b0eb6327c2d029e272b2871` | 2026-07-13 (full `diff` = 0 lines) |
| `LATEST_PROTOCOL_VERSION` | `"2026-07-28"` (stable) | 2026-07-13 |

**Stop-condition check:** stable canonical version identifier — PASS. No frozen-crate
(2025-06-18 / 2025-11-25) change required by any item below — PASS.

## 2. Normative prose deltas

### D1 — pin-commit → HEAD window (`93671a3f..2807f9d6`, verified via per-file patches)

Only one substantive normative change (both `draft/basic/transports/streamable-http.mdx`
and `docs/seps/2243-http-standardization.mdx`, same note):

> Clients **MUST** construct `Mcp-Param-*` headers using the most recently obtained
> `inputSchema` for the tool. A client that has never obtained the tool's `inputSchema`
> **SHOULD** send the request without `Mcp-Param-*` headers. If the server rejects the
> request because required `Mcp-Param-*` headers are missing **or do not match the body**,
> the client **SHOULD** call `tools/list` … then retry …

`draft/schema.mdx` (+237/−237) verified as TypeDoc HTML reformatting only (hunk-sampled).

### D2 — matrix-basis → pin-commit window (2026-06-12 → 2026-07-02, 30 commits enumerated by date)

Schema-side items were actioned in the 2026-07-02 re-vendor + OUTSTANDING.md burn-down.
Prose items requiring implementation-level disposition (codex review P2; §5 below):

| Upstream commit | Change | Disposition status |
|---|---|---|
| `f505a6c7`/`73ab7d2c`/`6bdff797`/`0bce6bce` | Error-code allocation policy + renumbering; `-32002` carve-out | Schema side done; **client regressions found — B1/B2 below**; matrix rows stale — M1 |
| `26dd54c0`/`c87328cc` | Mcp-Param emission decoupled from schema TTL | OPEN — audit client binding-cache behavior vs new wording |
| `b8809f54` | SSE comment-line keep-alive recommended for listen streams | OPEN — server `subscriptions/listen` stream: implement or explicit-deviation row |
| `201ee148` | `x-mcp-header` restricted to statically-reachable properties | OPEN — audit header-scan in `turul-mcp-protocol-2026-07-28/src/headers.rs` |
| `71d924e2` | Base64 sentinel extended to `Mcp-Name` header | OPEN — audit server validation + client emission |
| `9ede89ed` | `server/discover` version-selection bullet reframed | OPEN — prose-only? confirm no behavior delta |
| `fe74ef99` | Core client notifications do not occur over Streamable HTTP | Reflected in 2026-07-13 Lambda parity slice (T4 framing); matrix row check pending |
| `0b7f2e4c`/`380f1aff`/`43f8ea51` | Elicitation removals/filter changes | Believed covered by 2026-07-02 re-vendor; confirm rows |

### Full 31-page RFC-2119 re-extraction

In progress (resumed workflow `wf_451c493a-f7a`; 6 page-groups × extract+compare against the
490-row matrix). Results will be appended as §6 with every NEW/CHANGED requirement mapped to
code, test, or an explicit deviation row. Until §6 lands, D1/D2 above bound the
commit-verified change surface for the two windows.

## 3. Confirmed wire-contract defects (fixes in flight, no aliases)

**B1** — `crates/turul-mcp-client/src/client.rs:1063`: Mcp-Param mismatch auto-retry keys on
`-32001`; servers emit `-32020` (`ERROR_CODE_HEADER_MISMATCH`) since the renumbering. The
matrix's GAP-7 "FIXED 2026-06-11" silently regressed. Fix: match **only** `-32020`.
`-32001` is now implementation-defined and this framework itself emits `-32001` for
middleware `Unauthenticated` — an alias would turn auth failures into refresh-retries.
No backward alias (review decision, 2026-07-13).

**B2** — `crates/turul-mcp-client/src/version.rs:71`: era-classifier recognizes `-32004` as
UnsupportedProtocolVersionError; canonical is `-32022`. A current 2026 server's version
rejection is unrecognized → abort path instead of retry-with-supported-version.
Undetected because `tests/bilingual_negotiation.rs:154` mocked the stale code. Fix:
recognize **only** `-32022`; update mocks to current codes; no `-32004` alias; a stale-code
body may be asserted as *unrecognized* (negative case) only.

**B3** — `error.rs` `-32002` pre-2026 resource-not-found classification: upstream carved
`-32002` out of the legacy sub-range deliberately; classification is version-scoped compat,
not a draft alias. Audit wording; expected no behavior change.

## 4. HTTP era-decision table (codex review P1-3)

To audit and test as a complete table against the live Versioning + Streamable HTTP pages
(each row: implemented / tested / deviation):

| Rule (live prose) | Current state (matrix) | Action |
|---|---|---|
| `-32022` + `data.supported` → stay modern, retry mutually supported version | Broken until B2 | B2 + wire test mocking current code |
| Any recognized modern error → stay modern | Row 240 ✅ (but code set stale) | Re-verify recognized-set after B1/B2 |
| Unrecognized 400/4xx → identifies legacy, fall back | **Recorded deliberate deviation** — abort-by-default, opt-in `allow_legacy_gateway_fallback` (ADR-030 downgrade-resistance) | Keep deviation; re-affirm in ADR-030 revision log against current prose; maintainer may direct otherwise |
| Era cached per origin (HTTP) / process (stdio), re-probe on failure | Row 241 🟡 partial (per-instance lock only) | Keep 🟡 with rationale, or implement per-origin cache — maintainer call; SHOULD-level |
| Modern-only server names supported versions on any `initialize` error | ✅ both rejection paths (`-32020` header path, `-32601`+`data.supported` dispatch path); Lambda wire tests added 2026-07-13 | Done |

## 5. Matrix/doc corrections (M-items)

- M1: rows 395/664/926/928 — stale `-32001`/`-32004` literals and stale "retry NOT
  implemented" claims (contradicting row 926); re-disposition D1's new MUST (client binding
  cache = "most recently obtained inputSchema"; satisfied once B1/B2 land).
- M2: `crates/turul-mcp-protocol-2026-07-28/schema/README.md` — add re-verified-at HEAD
  `62811256` (2026-07-13).
- M3: CHANGELOG (0.4.0 Unreleased) + ADR-030 revision-log entry for the recognized-error-set
  change (B1/B2, no aliases).
- M4: disposition each OPEN row of §2-D2 with code/test/deviation mapping (feeds from §6).

## 6. Full requirement re-extraction results (completed 2026-07-13)

All 31 draft pages at commit `2807f9d6` (spec-surface-identical to HEAD `62811256`) were
swept for RFC-2119 requirements and compared against the 490-row matrix by six
extract+compare agent pairs. **Totals: 131 NEW (no matrix row), 4 CHANGED (row quotes
outdated text), 497 covered.** Full per-requirement detail: workflow `wf_451c493a-f7a`
journal (session records). Triage:

### 6.1 CHANGED rows (matrix-text staleness; code verified separately)

- Capability-gate error code: live prose says `-32021`; **13 matrix rows still quote
  `-32003`** (215, 143, 286, 474, 485, 507, 510, 574, 587, 606, 644, 748, 954). Server
  code and wire tests are CURRENT (`-32021`, `mrtr_2026.rs`); the middleware's
  `RATE_LIMIT_EXCEEDED = -32003` is a range-legal implementation-defined code, unrelated.
  → M1 scope extended to these rows.
- `x-mcp-header` statically-reachable restriction (row ~659) and HeaderMismatch `-32020`
  (row ~398), cancellation direction (row ~321): text refresh needed; behavior already
  current on the server side.

### 6.2 NEW-requirement clusters and their likely dispositions (to be row-authored)

| Cluster | Count | Likely disposition |
|---|---|---|
| Authorization sub-pages (discovery, client-registration, security-considerations; pages created 2026-06-04/05, before the audit but evidently swept thinly) | 62 | Server-role items (PRM issuer identity, discovery mechanisms, WWW-Authenticate content): mostly implemented in `turul-mcp-oauth` — author rows citing tests. Client-role items (WWW-Authenticate parsing, CIMD hosting/validation, issuer-identity checks): framework ships no OAuth *client* flow — author ➖ n/a rows with the role rationale, or record as roadmap. |
| `server/utilities/caching.mdx` TTL/cacheScope semantics | ~23 | Client-side caching heuristics (ttlMs default-0, stale handling, jitter/backoff MUST-if-polling): client implements no result caching — n/a-or-gap rows per item. Server-side (same `cacheScope` across pages MUST; MUST NOT rely on cacheScope for auth): audit `CacheableResult` producers, likely small fixes or n/a. |
| Transports (intermediary MUST/SHOULDs, SSE colon-comment MUST, Mcp-Name Base64 MUST, statically-reachable MUST, D1 Mcp-Param MUST) | 13 | Intermediary-role: n/a (framework ships no intermediary). SSE colon-comment: verify client SSE parser ignores comment lines (ties to D2 keep-alive item). Mcp-Name Base64 + statically-reachable: audit `headers.rs` (D2 rows). Mcp-Param MUST: satisfied by binding cache once B1 lands. |
| Cancellation/subscriptions patterns | 9 | stdio-server-only items (server-sent `notifications/cancelled` closing a listen stream): framework ships no stdio server binding — n/a rows. Subscription teardown SHOULDs: ties to the OUTSTANDING.md surviving graceful-close item. |
| Architecture/index meta-principles, elicitation/roots/sampling additions | rest | Mostly INFO/SHOULD design-principle rows; author with brief dispositions. |

### 6.3 Defect status after sweep

No additional CODE defects beyond B1/B2 (client) were confirmed by the sweep; the
`-32003` scare resolved as matrix-text-only (server verified current). The client's
recognized-modern-error set must include all three codes `-32022`/`-32021`/`-32020`
(folded into the B2 fix in flight).

### 6.3a Follow-up defect candidate (found during B1 test construction, deliberately out of slice-scope)

`HttpTransport::handle_response` (`crates/turul-mcp-client/src/transport/http.rs:332-351`)
treats any non-2xx as `TransportError::HttpStatus` without parsing a JSON-RPC error body;
only `probe_discover` has bespoke reparse-on-400 logic (`client.rs:435-444`). turul's own
server SSE-frames `tools/call` errors at HTTP 200 (client always sends
`Accept: text/event-stream`), so this is invisible against our stack — but a
spec-compliant server answering a plain-JSON 400 for a post-lock request would bypass
every `ServerError`-keyed client path (including the B1 retry). Needs its own slice:
generalize 400-body JSON-RPC reparse beyond the discover probe, with wiremock coverage.

### 6.4 Row-authoring outcome (completed 2026-07-13)

Applied: ~52 net new matrix rows + 19 changed-row edits + the M1 code-literal
corrections (13× `-32003`→`-32021`, 4× `-32001`→`-32020`, rows 395/664 and the gap
register refreshed against commit `4690e007`). Summary counts re-tallied two independent
ways: 541 total = 310 ✅ / 72 🟡 / 14 ❌ / 20 🧪 / 125 ➖. The sweep INCREASED the
recorded gap count (❌ 5→14, 🧪 12→20) — the matrix got more truthful, not greener.
Remaining `-32003` literals in the matrix are historical gap-register narration only
(lines ~523/534/563/864/868/1012), each dispositioned; the two live-row strays (ext-tasks
row, sampling row 818) were corrected during QA (`-32021`, verified against
`ext_tasks.rs` and `handlers/mod.rs` emission sites).

### 6.5 Grading judgment-call register

The row-authoring agents flagged 18 items as judgment calls rather than force-grading;
full notes in workflow `wf_36f39e2c-9a4` journal. The load-bearing ones:

- **B4 (real defect, confirmed during QA) — FIXED 2026-07-13.** `streamable_http.rs:1480`
  compared the `Mcp-Name` header to the body value with plain `!=`; the live draft
  (upstream commit `71d924e2`) requires servers to DECODE Base64-sentinel-encoded
  `Mcp-Name`/`Mcp-Param` values before comparing (`decode_param_value` exists in
  `headers.rs:221` and was used on the Mcp-Param path only). An encoded `Mcp-Name` was
  falsely rejected as a mismatch. `Mcp-Name` now routes through `decode_param_value`
  (streamable_http.rs:1474-1494), mirroring the Mcp-Param path including its
  decode-failure semantics. Tests: `mcp_headers_2026.rs::base64_encoded_mcp_name_decodes_and_matches`
  (revert-and-fail leg recorded), `::base64_encoded_mcp_name_mismatch_is_rejected` (-32020).
  Matrix row re-graded ✅ compliant (spec-compliance plan, 2026-07-13). Also corrected the
  stale source comment at `handlers/mod.rs:51` (said `-32003`; the code emits `-32021`).
- allOf-merge / statically-reachable `x-mcp-header` scope: grader's own reading of
  `walk()` recursion (headers.rs:159-181) — needs a follow-up verification test.
- MAY-permission rows (custom transports, 403 id-less body, ttlMs authoring freedoms)
  graded ➖ n/a as unexercised permissions; alternative graders might prefer INFO.
- Folded multi-quote rows (CIMD client-registration, elicitation §Understanding the
  Distinction) diverge from one-quote-per-row granularity — deliberate, per cluster
  guidance, noted here for future re-audits.
- Trailing-slash issuer SHOULD graded ❌ on "nothing implements it" — arguably ➖
  (operator-supplied URI); revisit if an issuer-normalization slice lands.

## 7. F-slice (2026-07-14, external-review round 2)

Codex review findings verified and dispositioned:

- **F1 (fixed)** — `version.rs` sent a bare `-32022` (no `data.supported` list) to
  `FallbackTo2025`. A recognized modern error identifies a modern server; with no
  server-named list there is no "mutually supported version from the supported list" to
  select, so falling back to `initialize` was inference — now Aborts. The
  `Some(list)`-naming-2025-11-25 sub-case is UNCHANGED and compliant (the spec's own
  error example carries `"2025-11-25"` inside `data.supported`; the reviewer's
  "retry another mutually supported **modern** version" inserted a word the spec does
  not contain). Test: `unsupported_protocol_version_with_no_list_aborts_without_downgrade`
  (revert-and-fail recorded).
- **F2 (fixed)** — closes §6.3a: `transport/http.rs` now rescues HTTP-400 responses whose
  body is a JSON-RPC error envelope into normal error classification
  (`rescue_400_jsonrpc_envelope`, applied ONLY to the two JSON-RPC request senders;
  404/auth statuses keep transport semantics — session-expiry recovery keys on 404).
  The discover probe uses a separate transport path and is untouched. Tests:
  `call_tool_recovers_from_plain_json_400_header_mismatch` (revert-and-fail recorded —
  the failure shows the exact buried envelope), `http_404_with_json_body_stays_a_transport_error`.
- **F3 (accepted)** — status header above corrected: "audited with unresolved MUST gaps",
  never "fully compliant" while ❌ MUST rows remain.
- **F4 (fixed)** — matrix reconciliation: 19 stale `-32004` literals in live rows →
  `-32022`; 10 dead test-name citations renamed; row 234's rotted "data.supported is
  never read" claim corrected; three stale provenance sites updated to the current pin
  (historical gap-register bullets and quoted superseded row text deliberately preserved).
- **F5 (half-accepted)** — the new `mcp_headers_2026.rs` comment's upstream commit hash
  removed (spec-name anchor kept). The reviewer's second citation
  (`builder.rs` "previously masqueraded" comment) is PRE-EXISTING (v0.3.41, verified via
  `git log -L`) — not new, left untouched per touch-only-what-you-must.

### MUST-gap worklist for full compliance (from the 541-row matrix, ❌ + MUST)

| Row | Gap | Scope |
|---|---|---|
| ~207 | JSON Schema dialect validation ($schema inspection + unsupported-dialect error) — BP-3 | server + client, own slice |
| ~332 | Server-sent `notifications/cancelled` / subscription-close emission (ties to the OUTSTANDING.md graceful-close item) | server, needs shutdown-signal infra |
| ~410 | ~~Client MUST Base64-sentinel-encode `Mcp-Name` values~~ **FIXED 2026-07-14** — `apply_request_metadata_headers` routes through `encode_param_value`; wire test `mcp_name_header_is_base64_sentinel_encoded_when_not_plain_ascii`, revert-and-fail recorded; matrix row re-graded ✅ | done |
| ~537 | Sampling message-shape enforcement (tool-result-only user messages, toolUseId pairing) — GAP-CF-9 | protocol validation helper + reject path |

MUST-level 🧪 rows (implemented, untested — need named tests to reach ✅ per the matrix
legend): UTF-8 body rejection (~345), SSE colon-comment handling (~392), x-mcp-header
static-reachability (~407), sampling capability declaration (~532), progress unknown-ID
tolerance (~572), per-connection tool-set stability (~696), request-ID uniqueness/echo
(~184/185).

## 8. F-slice round 2 (2026-07-14, external-review round 3)

Codex round-3 findings, each verified against code before acting:

- **F1 (fixed) — real.** `call_tool_or_task` (client.rs) passed a raw `Mcp-Name`
  extra-header while `apply_request_metadata_headers` also adds the encoded one;
  reqwest appends → two conflicting `Mcp-Name` headers. Removed the explicit one.
  Wire test inspects `received_requests` (a `.get()`-reading server hides the
  duplicate); revert-and-fail shows `["=?base64?…", "padded"]` on the wire.
- **F3 (fixed) — real.** `send_request_streaming` (both `subscriptions/listen`
  entry points) returned `HttpStatus` for any non-2xx without parsing the body.
  `classify_non_2xx` now applies the same status-400-only JSON-RPC-envelope
  rescue; a `400` + `-32021` surfaces as `ServerError(-32021)`. Revert-and-fail
  recorded.
- **F2 (fixed, and larger than codex's line numbers) — substantively right.**
  Codex's cited line 232 was wrong (that row is about `_meta`), but the matrix
  was materially stale on the pre-renumbering HeaderMismatch code: ~11 live rows
  cited `-32001` for the 2026 header/body-mismatch and missing-header paths, which
  the server now emits as `-32020` (verified: `header_body_protocol_version_mismatch_is_rejected_with_32020`
  asserts `-32020`; the only surviving legitimate `-32001` is the 2025 legacy
  "Missing Mcp-Session-Id" path). Corrected in the live rows; dated gap-register
  headlines (`VER-4`, `TX/GAP-4`) kept as history. The line-237 "ext-tasks/ext-apps
  crates are unscaffolded" claim was false — both crates exist — corrected. Line
  243's stale "no wire-level test for the 400-body fallback" self-note refreshed
  (F2 added it).
- **F2 line-243 grade — pushed back.** The row is a SHOULD graded ✅ with a
  prominent, ADR-030-recorded deliberate deviation (abort-on-unrecognized-400
  downgrade-resistance). RFC 2119 permits documented SHOULD deviation with weighed
  rationale, so ✅-with-inline-DEVIATION is defensible; not downgraded. Evidence
  refreshed to current codes/tests.
