# MCP 2026-07-28 Final Readiness Audit

> **Note (2026-07-02): substantially superseded.** Most of this report's P1 findings have since closed (CI workflow now exists, ADR index covers 022–031, `turul-mcp-ext-tasks` is wired, `_meta` is enforced on the 2026 wire path, `notifications/roots/list_changed` is cfg-gated, crate README/lib.rs default-spec claims are corrected, `roots-server` is pinned to the 2025 lane). See `OUTSTANDING.md` at the repo root for the current schema re-pin (2026-06-10 → 2026-07-02) and its fix slice. Treat specific claims below as point-in-time, not current state.
>
> **Disposition (2026-07-29): all 33 §2 rows dispositioned.** Every row in the §2 table now carries a per-row **Status (2026-07-29)** column recording CLOSED / OPEN / NEEDS-MAINTAINER / PASS with file:line evidence and, where applicable, a live command re-run. Result: all 9 P1 rows CLOSED; of the 16 actual P2 rows (the header below said 13 — a stale self-count, itself now corrected), 14 CLOSED and 2 are explicitly **NEEDS-MAINTAINER** (ephemeral-session minting on sessionless 2026 POSTs; no tasks in the 2026 *default* build) rather than dispositioned by this pass; of 8 P3 rows, 3 CLOSED, 1 PASS (unchanged, stays fixed), and 4 OPEN (ADR 001-numbering collision and the lambda example dir/package-name mismatch — both explicitly optional/cosmetic in their own remediation text; modeled upstream-fixture coverage — improved 8→12 modeled but not complete; the DELETE auth-posture doc note — still not written). This branch's CI (`.github/workflows/ci.yml`) and `scripts/ci-gates.sh` now implement the gate list this report specified in §7; both still cite this document as their source, which remains accurate for the gate *shapes*, not the (now largely closed) finding list.

Branch: `feat/turul-mcp-protocol-2026-07-28` (side-branch of `2026-07-28-MCP-Specification`).
Reconciled from seven read-only auditors (ADR consistency, protocol/schema, feature-topology, examples, test-matrix, documentation/CI, security). Spec lanes referenced by full date: **2026-07-28** (default, stateless core) and **2025-11-25** (opt-in legacy).

Branch lock reminder: this branch does not merge to `main`, fast-forward, rebase-onto-main, squash, or open a release PR without the maintainer's express authority (AGENTS.md §Branch Lock; CLAUDE.md §Branch Lock). This report is a readiness assessment, not authorization to merge.

---

## 1. Executive verdict

**Ready with listed exceptions.**

No auditor found a P0. The default 2026-07-28 build, default tests (1015 pass), default clippy, and the protocol-2026-07-28 compliance suite are all green (PROVEN). The open items are P1/P2/P3: ADR/doc drift advertising 2025-11-25 as the default, fourteen default examples carrying stale 2025-11-25 handshake prose, no CI to enforce the dual-spec matrix, and three spec-enforcement gaps in the 2026 stateless core (`_meta` not enforced, ephemeral-session minting, removed `roots/list_changed` handler still registered) — all bounded and disclosed by the README WIP banner. None is a shipped-production wire defect; each has a named remediation and gate. Release/merge is contingent on the maintainer accepting the listed exceptions or closing the P1 set first.

---

## 2. Findings ranked P0/P1/P2/P3

P0: **0**. P1: **9**. P2: **16** (the original count of 13 undercounted the table below by 3 rows — corrected 2026-07-29). P3: **8**. Total: 33.

**2026-07-29 disposition summary:** P1 9/9 CLOSED. P2 14/16 CLOSED, 2/16 NEEDS-MAINTAINER (rows 10 and 14 below). P3 3/8 CLOSED, 1/8 PASS (unchanged), 4/8 OPEN (rows 29–32 below, each individually optional/cosmetic or in-progress in its own remediation text — none blocking). See the **Status (2026-07-29)** column added to each row for evidence.

| Sev | Area | Title | Files | Rule / schema invoked | Remediation + gate | Status (2026-07-29) |
|-----|------|-------|-------|-----------------------|--------------------|----------------------|
| P1 | ADR | README index omits ADRs 022–030 (entire cutover package undiscoverable) | docs/adr/README.md | CLAUDE.md §Source of Truth; §Data And Contracts (ADR drift is a defect) | Add index rows for 022–030 (+019 if present). Gate: `grep -cE '\[(02[2-9]\|030)\]' docs/adr/README.md` == 9. | CLOSED — README.md:44-51 indexes 022–030. (ADR-031 not yet indexed — new gap, not this row.) |
| P1 | ADR | ADR-009 records 2026 default wire string as `"DRAFT-2026-v1"`; code finalized to `"2026-07-28"` | docs/adr/009-protocol-based-handler-routing.md | CLAUDE.md §Data And Contracts (update ADR revision log in same slice) | Fix lines 143/162/165-166: default is `"2026-07-28"`, `"DRAFT-2026-v1"` is deserialize-only alias; add revision-log entry. Gate: `grep -n 'DRAFT-2026-v1' docs/adr/009*.md` shows alias-only. | CLOSED — ADR-009:146,208-209 name default `"2026-07-28"`, alias-only `DRAFT-2026-v1`; matches `MCP_VERSION` in code. |
| P1 | ADR | ADR-023 claims `listChanged` capability removed in DRAFT-2026; 2026 crate still carries it | docs/adr/023-tool-change-detection-and-notification.md | CLAUDE.md §Data And Contracts | Correct line 355 to scope removal to roots/list_changed notification, not the `listChanged` capability field (survives on prompts/tools/resources, initialize.rs:191/200/215). Gate: stateless-variant section no longer claims the key is removed. | CLOSED — ADR-023:355-361 now scopes removal to `roots/list_changed`; `listChanged` confirmed live at initialize.rs:189,198,213. |
| P1 | Protocol / Security / E2E | Server does NOT enforce schema-required per-request `_meta` (protocolVersion/clientInfo/clientCapabilities) — types strict, wire path loose (**reported by protocol, security, and e2e auditors**) | crates/turul-mcp-server/src/server.rs:1443; crates/turul-http-mcp-server/src/streamable_http.rs:166-212,1440; crates/turul-mcp-protocol-2026-07-28/src/json_rpc.rs:36; crates/turul-mcp-server/tests/discover_stateless_2026.rs:69 | AGENTS.md §Branch Lock (required per-request `_meta`; reject header/body disagreement); schema `RequestParams._meta: RequestMetaObject` REQUIRED; CLAUDE.md §Capability Truthfulness | (a) Deserialize `params._meta` into `RequestMetaObject` on the 2026 request path, reject -32602 on missing/incomplete; OR (b) record as deferred WIP in COMPLIANCE.md/ADR-027 with an owner. Gate: negative-path wire test in discover_stateless_2026.rs (no `_meta`; missing clientCapabilities; header≠body version) asserts -32602, with revert-and-fail. | CLOSED — streamable_http.rs:1553-1622 rejects missing/incomplete `_meta` and header/body mismatch. `--test discover_stateless_2026` 18/18 incl. `missing_meta_is_rejected_with_invalid_params`. |
| P1 | Examples | 14 of 35 default-2026 (category A) examples ship stale 2025-11-25 prose / initialize-handshake curl / Mcp-Session-Id under the stateless default | examples/{client-initialise-server,middleware-auth-server,middleware-auth-lambda,oauth-resource-server,icon-showcase,pagination-server,resources-server,zero-config-getting-started,notification-server,prompts-server,derive-macro-server,function-macro-server,minimal-server,stateful-server} | AGENTS.md §Critic Review Mode (spec-version drift; do not preserve removed 2025-11-25 contracts on draft branch); CLAUDE.md §Branch Lock | Rewrite README/doc-comment/runtime curl to 2026 stateless (server/discover, MCP-Protocol-Version: 2026-07-28, per-request `_meta`; no initialize/notifications/initialized/Mcp-Session-Id). Gate: per cat-A dir `grep -rniF -e '2025-11-25' -e 'Mcp-Session-Id' -e 'notifications/initialized' -e 'method":"initialize'` returns 0, OR remaining hits are explicitly labelled 2025-opt-in. | CLOSED — client-initialise-server/stateful-server moved to 2025 opt-in; grep of all 30 current default-member examples for stale-handshake tokens returns only correctly-labelled negative/opt-in mentions. |
| P1 | Tests | No CI workflow exists — entire feature-topology / opt-in matrix is manual-only | .github/workflows (absent); Cargo.toml | AGENTS.md §Proof Before Expansion; AGENTS.md §Build,Test (clippy/test gates not automated) | Add CI jobs: (1) default test + clippy -D warnings; (2) 2025 opt-in suites + integration-tests; (3) bilingual client; (4) protocol-2026 `--features compliance`; (5) 2026 acceptance `--no-default-features --features http,sse,protocol-2026-07-28`. Gate: red CI when any job removed/broken. | CLOSED — .github/workflows/ci.yml (165 lines): default-2026, opt-in-2025, spec-mutex, docs jobs. Matches ci.yml:6 / ci-gates.sh:7 citations. |
| P1 | Tests | No cross-process bilingual-client ↔ real-2026-server test — the two wire halves never validated against each other | crates/turul-mcp-client/tests/{bilingual_negotiation,bilingual_2026_operations}.rs; crates/turul-mcp-server/tests/discover_stateless_2026.rs | CLAUDE.md §Test Coverage Discipline #3 (wire-layer coverage at the consuming boundary) | Add one integration test spawning a real 2026 `McpServer` + bilingual `McpClient`: discover → list_tools/call_tool/read_resource/get_prompt. Gate: changing server `_meta` key namespace or discover envelope breaks the test. | CLOSED — crates/turul-mcp-client/tests/e2e_2026_real_server.rs spawns a real server + bilingual client. `cargo test -p turul-mcp-client --test e2e_2026_real_server` 7/7; runs in CI (ci.yml:75). |
| P1 | Docs | Published crate lib.rs headers advertise DEFAULT spec as 2025-11-25 (docs.rs-facing false claim) | crates/turul-mcp-server/src/lib.rs:4; crates/turul-mcp-client/src/lib.rs:16 | AGENTS.md §Critic Review Mode (incorrect defaults); §Branch-Conditional Spec Guidance | Reword headers to 2026-07-28 default stateless core, 2025-11-25 opt-in. Gate: `grep -rn 'comprehensive MCP 2025-11-25 specification support\|Complete MCP 2025-11-25 specification support' crates/*/src/lib.rs` == 0; `cargo doc --no-deps` green. | CLOSED — turul-mcp-server/src/lib.rs:6-8, turul-mcp-client/src/lib.rs:15-16 both state 2026-07-28 default. |
| P1 | Docs | turul-mcp-protocol README states "Currently aliases: turul-mcp-protocol-2025-11-25" — wrong after default flip | crates/turul-mcp-protocol/README.md:12,56-57,244,300-301 | AGENTS.md §Critic Review Mode (incorrect defaults); cutover ground truth | Update line 12 to default alias 2026-07-28 + 2025 opt-in; fix CURRENT_VERSION/MCP_VERSION example outputs and line 244. Gate: `grep -n '2025-11-25' crates/turul-mcp-protocol/README.md` only in opt-in context. | CLOSED — README.md:6,12-13,57-59,307-308 name 2026-07-28 as default alias. |
| P2 | Transport / Security | 2026 stateless POST mints + persists an ephemeral session per sessionless request — session inflation + unauthenticated session-creation amplification (**reported by feature-topology AND security auditors**) | crates/turul-http-mcp-server/src/streamable_http.rs:1445-1466; crates/turul-mcp-session-storage/src/in_memory.rs:40-49,157-159; crates/turul-http-mcp-server/src/server.rs:382-408 | AGENTS.md §Complexity Control (one authoritative path; no storage churn the requirement doesn't need); §Security & Configuration; CLAUDE.md §Session Management (TTL/isolation) | Carry an in-request-only (non-persisted) SessionContext for sessionless 2026 requests; OR document the bound in an ADR. Bound today: in-memory cap max_sessions=100_000, reaped by last_activity TTL (default 30 min, 60s tick); per-request row churn on durable backends. Gate: test firing N (≥50) sessionless 2026 POSTs asserts `storage.list_sessions()` stays ≤1 (or documented bound), revert-and-fail. | NEEDS-MAINTAINER (per dispatch — not dispositioned). streamable_http.rs:1672-1691 still calls `session_storage.create_session(...)` per request (never echoed/read back). Header-leak half is closed (next row); storage-churn/cost question unchanged. |
| P2 | Transport | 2026 stateless responses still emit the spec-removed `Mcp-Session-Id` header | crates/turul-http-mcp-server/src/streamable_http.rs:296-299,1550,~1886; crates/turul-mcp-client/src/transport/http.rs:325-326 | CLAUDE.md §Branch Lock (2026: Mcp-Session-Id removed); AGENTS.md §Branch Lock | Suppress `Mcp-Session-Id` in response_headers() and the 1550 notification path when active spec is 2026-07-28. Harmless intra-framework (own client skips it) but wrong to a strict third-party 2026 peer. Gate: discover_stateless_2026.rs asserts `headers().get("Mcp-Session-Id").is_none()` on a 2026 tools/call and notification response. | CLOSED — echo sites are `#[cfg(feature="protocol-2025-11-25")]`-gated; GET/DELETE now 405 under 2026. `--test stateless_2026_http_surface` 5/5 incl. `responses_never_mint_session_ids`. |
| P2 | Protocol / Security | Server reads protocol version from `MCP-Protocol-Version` header only; never consults `_meta.protocolVersion`; no header/body disagreement check; `Mcp-Method`/`Mcp-Name` unimplemented | crates/turul-http-mcp-server/src/streamable_http.rs:171-176 | AGENTS.md §Branch Lock (new required headers; reject header/body disagreement) | Read `_meta.protocolVersion`, cross-check the header, reject mismatch -32602; implement Mcp-Method/Mcp-Name validation (constants already exported, unused). OR record as WIP gap. Gate: wire test asserting header/body version mismatch is rejected. | CLOSED — streamable_http.rs:1553-1622 cross-checks `_meta.protocolVersion` vs header (-32020 on mismatch); Mcp-Method/Mcp-Name enforced. `--test mcp_headers_2026` 12/12. |
| P2 | Protocol | 2026 build still registers the REMOVED `notifications/roots/list_changed` handler (+ camelCase variant) unconditionally | crates/turul-mcp-server/src/builder.rs:198-201,215-218 | CLAUDE.md §Frozen Protocol Crates / Branch Lock (removed-from-core contracts absent in 2026); COMPLIANCE.md §SEP-2577 | Gate both inserts behind `#[cfg(feature = "protocol-2025-11-25")]` (as ping/logging/setLevel are). Inbound no-op route, not a wire defect, but contradicts removed-from-core. Gate: under default features `builder.handlers` contains neither key — unit test mirroring the `!contains_key("ping")` assertion at builder.rs:1908. | CLOSED — builder.rs:230-234,249-253 both `#[cfg(feature="protocol-2025-11-25")]`; builder.rs:2108-2112 asserts absence under 2026. Confirmed by direct read (maintainer-cited known-closed example). |
| P2 | Protocol / Tasks | 2026 task story is documentation-only: no `turul-mcp-ext-tasks-2026-07-28` crate; 2026 build has zero task support (core or extension) | crates/turul-mcp-task-storage/src/lib.rs:52; docs/adr/028-extensions-strategy.md:39; crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:195 | AGENTS.md §Branch Lock (Tasks → official extension, SEP-2663); ADR-028 | No code fix required IF maintainer accepts "no tasks in 2026 build" (accurately disclosed). Else scaffold the extension crate. Gate: maintainer sign-off OR ext crate compiles and advertises `extensions["io.modelcontextprotocol/tasks"]`. | "Doc-only" claim CLOSED (crate is real: `turul-mcp-ext-tasks`, opt-in `ext-tasks` feature, 9/9 in `--test ext_tasks_2026`, advertises the extension via discover). Whether tasks ship in the *default* build is NEEDS-MAINTAINER per dispatch. |
| P2 | Examples | roots-server (cat B) not pinned to 2025; builds against deprecated 2026 Root with 6-7 SEP-2577 warnings; README presents Roots as current | examples/roots-server/{Cargo.toml,src/main.rs,README.md} | AGENTS.md §Capability Truthfulness / §Critic Review Mode (no implied current support for deprecated features) | (a) Pin to `default-features=false, features=["protocol-2025-11-25"]` like the other deprecated-area examples; OR (b) add README/doc note that Roots is SEP-2577-deprecated. Gate: `cargo build -p roots-server` emits 0 deprecation warnings (a) OR README states the deprecation (b). | CLOSED — Cargo.toml pins `protocol-2025-11-25`, `default-features=false`. `cargo build -p roots-server` → 0 warnings. |
| P2 | Examples | README.md:27 claims "43 build on the 2026-07-28 default"; default-members has 35 examples | README.md:27; Cargo.toml | AGENTS.md §Workspace State Triage / §Critic Review Mode (doc behavior claims must be accurate) | Update to 35 examples (2026 default); 18-example 2025-11-25 regression set. Gate: README number equals `awk '/default-members/,/^\]/' Cargo.toml \| grep -c '"examples/'` (=35). | CLOSED — README.md no longer states any example count (claim removed, not left drifted). Current default-members count: 30. |
| P2 | Tests | Bilingual client excluded from default-members — default `cargo test` never compiles/runs it | Cargo.toml | CLAUDE.md §Test Coverage Discipline #2 (production-path coverage) | Add `turul-mcp-client` to default-members (builds clean per 611c957) OR mandatory CI job `cargo test -p turul-mcp-client`. Gate: `cargo test`/CI fails on a broken client test. | CLOSED via the row's CI-job alternative — still not in default-members, but ci.yml:74-75 runs `cargo test -p turul-mcp-client` as a mandatory job. |
| P2 | Tests | resources/read and prompts/get not proven statelessly server-side (only list is) | crates/turul-mcp-server/tests/discover_stateless_2026.rs | prompt's explicit gap question | Extend discover_stateless_2026.rs with sessionless `resources/read` + `prompts/get` against the real server. Gate: new cases return 200 with expected `contents`/`messages`, fail if a session requirement is reintroduced. | CLOSED — covered across discover_stateless_2026.rs, mcp_headers_2026.rs, wire_edges_2026.rs, mrtr_2026.rs (read/get + capability-gate + MRTR round-trip cases), all passing. |
| P2 | Tests | Upstream wire-fixture conformance (upstream_fixtures) gated behind non-default `compliance` feature; never runs in `cargo test` | crates/turul-mcp-protocol-2026-07-28/{Cargo.toml,tests/upstream_fixtures.rs} | CLAUDE.md §Test Coverage Discipline #3 (test exists but not in enforced path) | Run `--features compliance` in CI for the crate. Gate: upstream_fixtures runs in the enforced matrix. | CLOSED via the row's CI-job remediation — still feature-gated (unchanged ask), but ci.yml:72-73 runs it in the enforced default-lane job. Locally: 3/3. |
| P2 | Docs | Three more crate READMEs present 2025-11-25 as default/headline compliance posture (server, client["current default"], http-server, aws-lambda) | crates/turul-mcp-server/README.md:6,14,20; crates/turul-mcp-client/README.md:6,15,539; crates/turul-http-mcp-server/README.md:72,359,430-432; crates/turul-mcp-aws-lambda/README.md:15,467-468 | AGENTS.md §Reviewer Focus Areas (capability truthfulness; lifecycle 2025-only on draft branch) | Add a 2026-07-28 default-posture line to each headline; label initialize/Mcp-Session-Id curl blocks as 2025-11-25 opt-in. Gate: each README's first compliance bullet names 2026-07-28 default; 2025 snippets tagged opt-in. | CLOSED — server/client/http-server/aws-lambda READMEs all lead with 2026-07-28 default, 2025-11-25 labelled opt-in. |
| P2 | Docs | Client lib.rs rustdoc claims 2025-11-25-only handshake behavior (docs.rs-facing, default is bilingual) | crates/turul-mcp-client/src/lib.rs:17,56-79 | AGENTS.md §Critic Review Mode (docs advertising incorrect defaults) | Describe bilingual negotiation (default); note 2025 handshake applies only on the 2025-locked connection. Gate: lib.rs no longer states 2025-only handshake as THE transport behavior; `cargo doc -p turul-mcp-client` clean. | CLOSED — lib.rs:56-69 describes per-connection negotiation, not a single 2025-only handshake. |
| P2 | Docs | tests/README.md describes suite as "complete MCP 2025-11-25 specification compliance" without noting default is 2026 | tests/README.md:3,239,244,251,405; tests/Cargo.toml | AGENTS.md §Workspace State Triage | Add a top note: this crate is the pinned 2025-11-25 regression suite (opt-in); 2026-07-28 stateless E2E lives in the default-members server crate. Gate: README states both spec lanes and which crate covers each. | CLOSED — tests/README.md:3 opens with an explicit spec-lane note naming this crate as the 2025-11-25-opt-in regression suite and pointing to discover_stateless_2026.rs for the 2026 lane. |
| P2 | Docs / CI | Full-workspace 2025-11-25 opt-in CI matrix is an explicit, still-open coverage gap (self-declared in CHANGELOG) | CHANGELOG.md; Cargo.toml | CLAUDE.md §Test Coverage Discipline; AGENTS.md §Branch-Conditional Spec Guidance | Wire both spec matrices into CI / a just-target. Per-crate 2025 builds verified green; no aggregate gate exists. Gate: CI runs default + 2025-opt-in build+clippy+test for server/builders/http-server/derive/lambda/client, all green. | CLOSED — ci.yml `opt-in-2025` job (lines 87-138) runs the full per-crate 2025 matrix + regression E2E + every tests/Cargo.toml target. Locally: mcp-tools-tests 30/30. |
| P2 | ADR | ADR-030 "Internal module layout" sketch is stale and uses forbidden bare-year module names (v2025/v2026) | docs/adr/030-turul-mcp-client-bilingual-spec-coexistence.md:104,107,124,139,183,204 | CLAUDE.md §Spec-Version Naming (bare-year `v2026`/`v2025` forbidden) | Replace sketch with as-built full-date names (`v2026_07_28.rs`; 2025 inline) or annotate as illustrative-only. Revision log already records the divergence. Gate: `grep -nE 'mod v2026[^_]\|v2025\.rs\|v2026\.rs' docs/adr/030*.md` == 0 in the decision body. | CLOSED — ADR-030:124-127 marks the sketch "illustrative only — superseded", uses full-date `v2026_07_28.rs`. Bare-year grep in the decision body: 0 hits. |
| P2 | ADR | ADR-029 §Neutral still says default MCP_VERSION is `"DRAFT-2026-v1"` (now-satisfied "will flip" hedge) | docs/adr/029-spec-coexistence-via-cargo-features.md:188 | CLAUDE.md §Data And Contracts | Update line 188: default is `"2026-07-28"` now; drop "will flip" framing. Lower than ADR-009 because revision log (218-225) is correct. Gate: ADR-029 makes no claim the default is currently `"DRAFT-2026-v1"`. | CLOSED — ADR-029:186 states default is `"2026-07-28"`, alias-only `DRAFT-2026-v1`; revision-log 224 records reconciliation. |
| P3 | Tests | Default `cargo test` integration coverage is thin (only 3 integration binaries; heavy E2E is 2025-pinned/excluded) | Cargo.toml; tests/Cargo.toml | AGENTS.md §Proof Before Expansion | Add a 2026-native E2E crate to default-members (notifications/SSE/session-absence). Gate: a 2026 E2E suite runs in default `cargo test`. | CLOSED — turul-mcp-server (default-member) now ships ~20 2026-native integration test files exercised by plain `cargo test`. Full default run: 1274 passed, 0 failed (was 1015). |
| P3 | Docs | `cargo doc` emits ~9 pre-existing rustdoc warnings (CI denies warnings) | crates/turul-mcp-aws-lambda/src/lib.rs; crates/turul-http-mcp-server/src/lib.rs; crates/turul-mcp-derive/src/lib.rs | AGENTS.md §Coding Style (deny warnings in CI) | Escape generic-looking HTML tags (`` `<T>` ``), fix/relax private+unresolved intra-doc links. Pre-existing, not cutover-introduced. Gate: `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` exits 0. | CLOSED — `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` exits 0 clean in this verification run; enforced by ci.yml `docs` job. |
| P3 | ADR / Protocol | ADR-001 (protocol-alias) compile_error message labels protocol-2025-11-25 as "(default)"; real default is protocol-2026-07-28 | crates/turul-mcp-protocol/src/lib.rs:79 | CLAUDE.md §Spec-Version naming / diagnostic accuracy | Move "(default)" to protocol-2026-07-28 in the lib.rs:79 compile_error. Cosmetic (fires only under --no-default-features). Gate: neither-enabled diagnostic names protocol-2026-07-28 as default. | CLOSED — lib.rs:79-81 now labels `protocol-2026-07-28` as "(default)". |
| P3 | ADR | Three distinct ADRs share number 001 (numbering collision predates cutover) | docs/adr/001-{protocol-alias-usage,lambda-mcp-integration-architecture,session-storage-architecture}.md; docs/adr/README.md | CLAUDE.md §Source of Truth (navigability) | Optionally renumber the two non-canonical 001 files, or always cross-reference by full filename. No code impact. | OPEN — explicitly optional in the original row ("no code impact"). Confirmed still 3 files named `001-*.md`. |
| P3 | Examples | Lambda example dir/package-name mismatch (lambda-mcp-server dir = lambda-turul-mcp-server pkg) | examples/lambda-mcp-server/Cargo.toml; examples/lambda-mcp-client/Cargo.toml | none (consistency nit) | Optionally rename dirs to match pkg names. Gate: `cargo build -p <dirname>` succeeds for every examples/<dirname>. | OPEN — explicitly optional/cosmetic in the original row. Confirmed still mismatched (lambda-mcp-server dir → lambda-turul-mcp-server pkg, and client likewise). |
| P3 | Protocol | Modeled upstream-fixture coverage is 8 of 86 directories (9.3%); 78 cases `Kind::NotModeled` | crates/turul-mcp-protocol-2026-07-28/tests/upstream_fixtures.rs; COMPLIANCE.md:146 | AGENTS.md §Proof Before Expansion | Raise fixtures wave-by-wave (DiscoverResult, ReadResourceResult, notification params next). Honestly disclosed; floor test prevents regression. Gate: bump COVERAGE_FLOOR + add Case rows on the same pin. | OPEN, improved — modeled raised 8→12, fixtures 86→88 (pin 271ecc9a); floor/round-trip tests pass, but most upstream examples remain `NotModeled`. Wave-by-wave remediation in progress, not complete. |
| P3 | Security | DELETE/session-termination unauthenticated, acts on caller-supplied session id (pre-existing 2025 behavior, near-inert under 2026 stateless) | crates/turul-http-mcp-server/src/streamable_http.rs:959-1027 | AGENTS.md §Security & Configuration | No change required for 2026 (ephemeral sessions carry no authority). If 2025 opt-in is exposed without an upstream gateway, document DELETE is unauthenticated-by-design. Gate: one-line note in COMPLIANCE.md/ADR-030 on DELETE auth posture. | OPEN — no doc note found in COMPLIANCE.md/ADR-030 (grepped, no match). Code unchanged but now provably unreachable on a 2026 build (405 via stateless_2026_http_surface.rs); only the promised doc note is missing. |
| P3 | Security | Downgrade resistance correctly fail-closed and wired into real connect() (PASS, no defect) | crates/turul-mcp-client/src/{version.rs,client.rs}; crates/turul-http-mcp-server/src/streamable_http.rs | prompt success criterion (fail-closed downgrade, trusted -32601) | No fix required. Gate (stays-fixed): `cargo test -p turul-mcp-client --lib version::` stays 6/6. | PASS, stays fixed — `cargo test -p turul-mcp-client --lib version::` now 10/10 (was 6/6), no regression. |

---

## 3. Final checklist by area

### 3.1 ADR consistency

| Item | Result | Evidence |
|------|--------|----------|
| ADR-001 (protocol-alias): default-flip + client third-exception documented; matches code | pass | ADR-001 9,65-69; protocol/src/lib.rs:84-87 + Cargo.toml default=[protocol-2026-07-28]; cargo tree → 2026-07-28 v0.4.0. Minor in-code "(default)" mislabel (P3). |
| ADR-006 (streamable-http): DRAFT-2026 stateless addendum present + consistent | pass | ADR-006 418-487; uses "DRAFT-2026-v1" as a spec NAME (allowed), not a stale live default. |
| ADR-009 (handler routing): DRAFT-2026 wire string matches code | fail | ADR-009 143/162/165 say default is "DRAFT-2026-v1"; code MCP_VERSION="2026-07-28". (P1) |
| ADR-023 (tool-change): DRAFT-2026 fingerprint + capability claims match code | fail | ADR-023:355 claims `listChanged` removed; 2026 initialize.rs retains it (191/200/215 + subscribe 212). (P1) |
| ADR-025 (extract-turul-rpc): turul-rpc 0.1→0.2 cutover landed + recorded | pass | ADR-025 revision log 2026-06-07 (191): branch on turul-rpc 0.2.2. |
| ADR-027 (DRAFT-2026 target): wire-string finalization + cutover-landed recorded | pass | ADR-027 revision log 2026-06-07 (113-119) finalizes "2026-07-28", Phase 9.4 DONE; code matches. |
| ADR-028 (extensions): tasks-as-2026-extension matches code | pass | No ext-tasks crate on disk (matches ADR-028:171 "to be built"); server task_runtime `#[cfg(feature="protocol-2025-11-25")]`. |
| ADR-029 (cargo-feature coexistence): mechanism + default-2026 + cutover-landed matches code | pass | Alias compile_error mutex + feature-gated re-exports; revision log 218-225 records landed cutover. Minor §Neutral stale (P2). |
| ADR-030 (bilingual client): bilingual-implemented + as-built layout recorded | pass | Revision log 226 records as-built protocol/v2026_07_28.rs; code matches. Body sketch stale + bare-year (P2). |
| No forbidden bare-year spec tokens in cutover ADRs | fail | ADR-030 body sketch 104,107,124,139,183,204 use `v2025`/`v2026`. (P2) |
| ADR README index lists all current ADRs | fail | README stops at 021; 022–030 absent. (P1) |
| Alias default + framework crates default to protocol-2026-07-28 (ground truth) | pass | `cargo tree -p turul-mcp-protocol -e features` → 2026-07-28 v0.4.0 default; lib.rs test asserts MCP_VERSION=="2026-07-28". |

### 3.2 Protocol/schema compliance + tasks/extensions

| Item | Result | Evidence |
|------|--------|----------|
| DiscoverResult shape faithful (resultType, ttlMs/cacheScope, supportedVersions, capabilities, serverInfo, instructions) | pass | src/discover.rs:65-130 matches schema 584-609; CacheableResult mixin present. |
| CacheableResult mixin on tools/resources/prompts/discover list+read results | pass | caching.rs:63-79; all carry ttl_ms+cache_scope; wire test asserts cacheScope. |
| ResultType open union with verbatim round-trip of unknown | pass | result_type.rs:26-77 hand-written ser/de + Other(String); accepts_unknown + round_trips_verbatim pass. |
| RequestParams._meta required typed RequestMetaObject (protocol crate type) | pass | json_rpc.rs:36-46 not Option; test_request_params_rejects_missing_meta passes. |
| Server ENFORCES required `_meta` on the wire (or rejects missing/incomplete) | fail | DiscoverHandler ignores params (server.rs:1443); no RequestMetaObject deserialization/rejection in transport; wire test never asserts rejection. (P1) |
| server/discover present in 2026, answers without a session | pass | DiscoverHandler (server.rs:1414), registered (508/782); test server_discover_answers_without_a_session passes. |
| Removed-from-core methods absent in 2026 path, retained only behind protocol-2025-11-25 | pass | ping/logging/setLevel/sampling/elicitation `#[cfg(feature="protocol-2025-11-25")]` gated; schema lacks all; 2026 path mints session, no Mcp-Session-Id handshake. |
| notifications/roots/list_changed absent in 2026 build | fail | builder.rs:198 + :215 register it (and camelCase) unconditionally; schema removed it. (P2) |
| Deprecated-but-present (roots/list, sampling/createMessage, notifications/message) carry #[deprecated] and remain | pass | 17 #[deprecated]; roots.rs/sampling.rs/notifications.rs; COMPLIANCE.md §SEP-2577 enumerates all. |
| JSON-Schema-2020-12 ToolSchema (properties as Value, root type=object; outputSchema root-type-free) | pass | tools.rs:101-141 properties Option<HashMap<String,Value>>, type required; ToolOutputSchema no type field. |
| structuredContent may be any JSON value | pass | structured_content_accepts_any_json_value passes; schema unknown. |
| ServerCapabilities/ClientCapabilities carry extensions map; no core tasks field | pass | initialize.rs:163-182, 251-275; comment "tasks field is NOT present". |
| Core tasks 2025-only: task-storage compile_error under 2026, supports_tasks()==false for 2026 | pass | task-storage/src/lib.rs:52 compile_error; supports_tasks()=matches!(V2025_11_25). |
| 2026 extension-based task story is real (turul-mcp-ext-tasks crate/test exists) | fail | No ext crate; ADR-028:39 + COMPLIANCE.md:195 describe it as future. Documentation-only. (P2) |
| New required transport headers (Mcp-Method, Mcp-Name) + header/body disagreement rejection | fail | grep over transport src empty; version read from header only; `_meta.protocolVersion` never consulted. (P2) Disclosed by README WIP banner. |
| Protocol crate test gate green + clippy -D warnings clean | pass | 179 integration + 3 fixtures + doctests pass; clippy clean. |

### 3.3 Feature topology + client/server/transport behavior

| Item | Result | Evidence |
|------|--------|----------|
| Alias turul-mcp-protocol default = protocol-2026-07-28 | pass | Cargo.toml default=["protocol-2026-07-28"]. |
| Every framework crate defaults to protocol-2026-07-28 | pass | server/http-server/builders/derive/session-storage/aws-lambda default 2026. Exceptions: task-storage default 2025 (intended); server-state-storage no protocol feature; oauth inherits transitively. |
| protocol-2025-11-25 is opt-in AND builds | pass | `cargo build -p turul-mcp-server --no-default-features --features http,sse,protocol-2025-11-25` → Finished 6.29s. |
| Default (2026) server build is clean | pass | `cargo build -p turul-mcp-server` Finished; default-members workspace Finished 0 errors. |
| Mutex: both protocol features ⇒ compile_error | pass | `cargo build -p turul-mcp-protocol --features protocol-2025-11-25,protocol-2026-07-28` → mutually-exclusive error. Mirror mutex in client lib.rs. |
| Servers single-spec per build; client bilingual | pass | Alias re-exports exactly one versioned crate; client links both + negotiates per connection. |
| workspace default-members vs members coherence (no broken-in-limbo crate) | pass | members=76, default-members=45; held-backs all documented (frozen protocols, json-rpc shim, task-storage 2025-only, bilingual client, 2025-only examples). |
| Server advertises MCP-Protocol-Version: 2026-07-28 under 2026 | pass | discover_stateless_2026.rs:99-105 asserts header; passes. |
| server/discover served statelessly | pass | DiscoverHandler `#[cfg(feature="protocol-2026-07-28")]`; test passes (200, no Mcp-Session-Id). |
| 2026 mints no REQUIRED Mcp-Session-Id (sessionless requests succeed) | pass | tools_call_dispatches_without_session_handshake passes; 2026 path never returns 400 missing-session. |
| initialize/initialized gated 2025-only on server | pass | session-creation + missing-session-400 under `#[cfg(feature="protocol-2025-11-25")]`. |
| Client advertises negotiated version + drops Mcp-Session-Id on 2026 | pass | http.rs set_protocol_version clears session_id on "2026-07-28"; request builders add header only when Some. |
| Bilingual downgrade fail-closed (no silent downgrade on bare 4xx) | pass | classify_probe: only -32601 → FallbackTo2025; bare 4xx → Abort. bilingual_client_aborts_on_4xx present. |
| Fingerprint stale-detection unreachable on 2026 POST path | pass | Fingerprint check in validate_session_exists, called only on 2025 branch; 2026 branch never calls it. |
| Ephemeral-session bound characterized | pass | In-memory cap 100_000, reaped by TTL (default 30 min, 60s tick). See P2. |
| discover_stateless_2026 tests pass | pass | 4 passed, 0 failed. |
| bilingual_2026_operations tests pass | pass | 4 passed, 0 failed. |

### 3.4 Examples disposition

| Item | Result | Evidence |
|------|--------|----------|
| Enumerate all example crates | pass | 53 example crates in cargo metadata. |
| Enumerate all examples/ dirs on disk | pass | 54 dirs (53 crates + examples/archived/); set diff empty both ways. |
| Category A (default-2026) count | pass | 35 examples in default-members. |
| Category B (2025-regression, member not default) count | pass | 18 examples (enumerated). |
| Category C (archived/removed) count | pass | 24 under examples/archived/ (not members). |
| Category D (broken/stale among members) | pass | No member fails to build; 14 cat-A carry stale 2025 prose (P1); roots-server warns (P2). |
| Every cat-A example builds under default `cargo build` | pass | default-members Finished; 10 spot-built cat-A FINISHED 0 warnings. |
| Cat-A sources free of residual 2025 handshake/session/spec-date tokens | fail | 14 cat-A contain 2025-11-25 / Mcp-Session-Id / initialize curl. (P1) |
| Cat-B set small and builds under pinned 2025 config | pass | 18 build; 17 pinned via path-dep, roots-server unpinned warns. |
| Archived examples not referenced as workspace members | pass | `grep -nE 'examples/archived' Cargo.toml` no match. |
| README 2026 WIP banner present + example counts accurate | fail | Banner correct; README:27 claims 43 vs 35. (P2) |

### 3.5 Test matrix + coverage

| Item | Result | Evidence |
|------|--------|----------|
| Default `cargo test` (2026 path) passes | pass | 15 binaries, 1015 passed, 0 failed; `--no-run` warning sweep = 0. |
| 2026 stateless acceptance discover_stateless_2026 passes (real HTTP) | pass | 4 passed; real reqwest→server (discover, tools/call, resources/list, prompts/list). |
| Bilingual client tests pass | pass | 144 passed across 8 binaries. |
| Bilingual client tests exercise a REAL 2026 server (not wiremock) | fail | bilingual tests use `wiremock::MockServer::start()`; no real server spawned. (P1) |
| protocol-2026 compliance tests pass | pass | 153 unit + 179 compliance.rs. |
| Upstream wire-fixture conformance runs in default/enforced path | fail | upstream_fixtures requires `--features compliance`; not in default. (P2) |
| 2025 opt-in regression suite passes (real servers) | pass | mcp-tools-tests 30, mcp-resources-tests 83, tasks_e2e 7. |
| 2025 initialize/session/tasks lifecycle proven E2E | pass | tasks_e2e_inmemory.rs initialize→session→tasks transitions; 7 passed. |
| resources/read + prompts/get proven statelessly server-side | fail | discover_stateless_2026 covers only list + tools/call + discover. (P2) |
| CI guard exists so feature-topology/mutex cannot silently regress | fail | No `.github/workflows/`. (P1) |
| Bilingual client included in default `cargo test` | fail | turul-mcp-client in members, absent from default-members. (P2) |
| Mutex/concurrency contract has a regression guard | pass | transport_concurrency.rs asserts parallel &self transport; passes (runs only under -p, not default/CI). |

### 3.6 Documentation posture + CI/release gates

| Item | Result | Evidence |
|------|--------|----------|
| Root README banner+body present default as 2026-07-28 stateless | pass | README:1-19 WIP banner + 17/963-996 state 2026 default, 2025 opt-in; headline curl uses server/discover. |
| 2025-11-25 described ONLY as opt-in legacy across docs | fail | 5+ published artifacts present 2025-11-25 as headline/default (lib.rs ×2, protocol/server/client READMEs). (P1/P2) |
| Published crate lib.rs headers reflect 2026 default | fail | turul-mcp-server/src/lib.rs:4 + turul-mcp-client/src/lib.rs:16 say "MCP 2025-11-25 specification support". (P1) |
| CHANGELOG.md reconciled with the cutover | pass | [0.4.0] entry documents default flip, example migration, 2025 opt-in, branch-lock non-merge, self-flagged 2025 CI-matrix item. |
| Default build green (2026-07-28) | pass | cargo build Finished; server tests 159+4+14; protocol-2026 179. |
| Default clippy clean (0 warnings) | pass | clippy (default-members) grep warning/error == 0. |
| `cargo test --no-run` warning sweep (default) clean | pass | Finished, no warning/error lines. |
| Doctests pass on default (2026) | pass | server --doc 14; builders --doc 11. |
| 2025-11-25 opt-in build matrix (server/builders/http-server/derive/lambda) | pass | each `--no-default-features --features [...,protocol-2025-11-25]` Finished; server 2025 clippy 0. |
| Client bilingual + 2025-only + 2026-only build | pass | client-2025-11-25-only and client-2026-07-28-only both Finished; default client-bilingual. |
| 2025-pinned integration test crate compiles | pass | `cargo test -p turul-mcp-framework-integration-tests --no-run` Finished 17.83s. |
| `cargo doc` clean (no rustdoc warnings) | fail | ~9 rustdoc warnings (aws-lambda/http-server/derive). (P3) |
| Full-workspace 2025-11-25 opt-in matrix wired as release gate | unknown | No aggregate gate command; CHANGELOG self-flags it. Per-crate 2025 builds verified green. |
| Examples build on the 2026 default (claimed 43) | unknown | This auditor did not enumerate all examples; examples auditor verified 35 (see §3.4). Treated as the examples auditor's 35 (P2). |

### 3.7 Security posture

| Item | Result | Evidence |
|------|--------|----------|
| Bare HTTP 4xx from server/discover does NOT trigger downgrade (fail-closed) | pass | version.rs:101-104 HttpStatus→Abort; test http_4xx_aborts_by_default_no_downgrade 6/6. |
| Downgrade requires trusted JSON-RPC -32601 | pass | version.rs:64,82 only METHOD_NOT_FOUND→FallbackTo2025; all others→Abort. |
| classify_probe on real connect() path, not dead code | pass | client.rs:277 calls it; probe_discover maps transport HttpStatus + 200-body error.code. |
| 2025 server answers server/discover with HTTP 200 + -32601 (not 400) | pass | streamable_http.rs:1393-1409 returns OK + method_not_found before generic 400. |
| Bare-4xx-aborts covered by a regression test | pass | version.rs:149-164; verified passing. |
| Legacy-gateway 404/405 fallback hatch opt-in and off by default | pass | config.rs:26-30 default false; even on, 400/401/403/500 still abort. |
| 2026 server enforces required client _meta/capabilities/protocolVersion | fail | from_request unwrap_or_default() on MCP-Protocol-Version; no header/body check; no Mcp-Method/Mcp-Name; no missing-_meta rejection. (P1/P2) |
| Per-request ephemeral session creation not an unauthenticated flood/cost vector | fail | streamable_http.rs:1445-1466 persists a session per sessionless request, no auth gate; bounded only by 100k + 30-min TTL. (P2) |
| No risky per-spec auth/session divergence (DELETE / bearer rotation) | pass | DELETE unauthenticated both specs (pre-existing) but near-inert under stateless 2026; bearer rotation client-side, spec-agnostic. (P3 note) |

---

## 4. Example disposition matrix

Categories: **A** = default-2026 (in default-members, builds under default `cargo build`). **B** = 2025-11-25 regression (member, NOT default-members, pinned to the 2025 opt-in). **C** = archived/removed (not a workspace member, not built). **D** = broken/stale among members. Build command shown per default (category A) example.

| Category | Count | Members | Build command (per default example) | Status |
|----------|-------|---------|--------------------------------------|--------|
| A — default-2026 | 35 | client-initialise-server, derive-macro-server, function-macro-server, icon-showcase, middleware-auth-server, middleware-auth-lambda, lambda-authorizer, minimal-server, notification-server, oauth-resource-server, pagination-server, prompts-server, resources-server, stateful-server, zero-config-getting-started, + 20 more (tool/resource/prompt/middleware/lambda/notification kinds) | `cargo build` (default-members) → Finished 0 warnings; spot `cargo build -p <example>` per crate FINISHED 0 warnings | Build pass; **14 carry stale 2025-11-25 prose/handshake** (P1) |
| B — 2025-11-25 regression | 18 | client-initialise-report, dynamic-tools-server, elicitation-server, lambda-mcp-client, lambda-mcp-server, logging-test-client, logging-test-server, prompts-test-server, resource-test-server, roots-server, sampling-server, session-aware-logging-demo, session-logging-proof-test, session-management-compliance-test, streamable-http-client, tasks-e2e-inmemory-client, tasks-e2e-inmemory-server, tools-test-server | `cargo build -p <pkg> --no-default-features --features [...,protocol-2025-11-25]` (lambda dirs map to lambda-turul-* pkgs) → all FINISHED | 17 pinned via path-dep build 0 warnings; **roots-server unpinned → 6-7 SEP-2577 deprecation warnings** (P2) |
| C — archived/removed | 24 | examples/archived/* (+ README) | not built (not workspace members) | excluded; `grep -nE 'examples/archived' Cargo.toml` no match |
| D — broken/stale among members | 0 build-broken | (none fail to build) | — | No member fails compilation. "Stale" defects rolled into A (14 prose) + B (roots-server warnings). |

Naming nit (P3): examples/lambda-mcp-server dir → package `lambda-turul-mcp-server` (and -client); `cargo build -p lambda-mcp-server` fails — use `-p lambda-turul-mcp-server`.

---

## 5. Test matrix

| # | Suite / command | Spec lane | Result (PROVEN) | Notes |
|---|-----------------|-----------|-----------------|-------|
| 1 | `cargo test` (default-members) | 2026-07-28 | 15 binaries, 1015 passed, 0 failed; `--no-run` warnings = 0 | 3 integration binaries only (compliance.rs, discover_stateless_2026.rs, middleware_parity.rs); rest unit/doctests |
| 2 | `cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test discover_stateless_2026` | 2026-07-28 | 4 passed, 0 failed | Real HTTP stateless: discover, tools/call, resources/list, prompts/list |
| 3 | `cargo test -p turul-mcp-client` | bilingual | 144 passed across 8 binaries, 0 failed | bilingual_negotiation 4, bilingual_2026_operations 4, wire_compliance 14 — all wiremock |
| 4 | `cargo test -p turul-mcp-protocol-2026-07-28 [--features compliance]` | 2026-07-28 | 153 unit + 179 compliance.rs passed; +3 upstream_fixtures only with `--features compliance` | upstream_fixtures NOT in default path |
| 5 | `cargo test -p mcp-tools-tests` / `-p mcp-resources-tests` / `tasks_e2e_inmemory` | 2025-11-25 opt-in | 30 / 83 / 7 passed, 0 failed (real servers) | 2025 initialize→session→tasks lifecycle E2E proven |

**Marked coverage gaps:**
- **G1 (P1):** No bilingual-client ↔ real-2026-server cross-process test — both halves validated only against their own mocks.
- **G2 (P1):** No CI workflow — opt-in 2025 suites / bilingual client / upstream_fixtures run only on manual invocation.
- **G3 (P2):** Bilingual client excluded from default-members — default `cargo test` never compiles/runs it.
- **G4 (P2):** `resources/read` + `prompts/get` not proven statelessly server-side (only list ops are).
- **G5 (P2):** upstream_fixtures gated behind non-default `compliance` feature — richest wire-shape conformance invisible to default gate.
- **G6 (P3):** Default integration coverage thin (3 integration binaries); heavy 2026-native server E2E (notifications/SSE/session-absence) not yet in default-members.

---

## 6. Remaining work items (ordered)

**Blocks merge / release (P1):**

1. **owner: Tests/CI** — Add CI matrix. Files: `.github/workflows/*.yml`, `Cargo.toml`. Gate: jobs (default test+clippy -D warnings; 2025 opt-in suites + integration-tests; bilingual client; protocol-2026 `--features compliance`; 2026 acceptance `--no-default-features --features http,sse,protocol-2026-07-28`) all run and go red when any is removed/broken. (Unblocks enforcement of G2–G5.)
2. **owner: Protocol/Security/Tests** — Enforce or formally defer per-request `_meta`. Files: `crates/turul-http-mcp-server/src/streamable_http.rs`, `crates/turul-mcp-server/src/server.rs:1443`, `crates/turul-mcp-server/tests/discover_stateless_2026.rs` (or COMPLIANCE.md/ADR-027 if deferred). Gate: negative-path wire test asserts -32602 for missing `_meta` / missing clientCapabilities / header≠body version, with revert-and-fail; OR documented WIP gap with owner.
3. **owner: Tests** — Cross-process bilingual ↔ real-2026-server integration test. Files: `crates/turul-mcp-client/tests/` (+ dev-dep on turul-mcp-server) or root tests crate. Gate: changing server `_meta` key namespace or discover envelope breaks the test.
4. **owner: Examples** — Rewrite 14 cat-A examples to 2026 stateless. Files: the 14 example dirs in §2. Gate: per-dir `grep -rniF -e '2025-11-25' -e 'Mcp-Session-Id' -e 'notifications/initialized' -e 'method":"initialize'` == 0 (or remaining hits labelled 2025-opt-in).
5. **owner: Docs** — Fix published lib.rs headers + protocol README default-spec claim. Files: `crates/turul-mcp-server/src/lib.rs:4`, `crates/turul-mcp-client/src/lib.rs:16`, `crates/turul-mcp-protocol/README.md:12,56-57,244,300-301`. Gate: `grep -rn 'comprehensive MCP 2025-11-25 specification support\|Complete MCP 2025-11-25 specification support' crates/*/src/lib.rs` == 0; protocol README 2025-11-25 only in opt-in context.
6. **owner: ADR** — README index 022–030 + ADR-009 wire string + ADR-023 listChanged. Files: `docs/adr/README.md`, `docs/adr/009-*.md`, `docs/adr/023-*.md`. Gate: `grep -cE '\[(02[2-9]\|030)\]' docs/adr/README.md` == 9; ADR-009 names "2026-07-28" default; ADR-023 scopes removal to roots notification.

**Post-merge / maintainer-discretion (P2/P3):**

7. **owner: Transport/Security** — Stateless session handling: non-persisted SessionContext for sessionless 2026 requests + suppress `Mcp-Session-Id` on 2026 responses. Files: `crates/turul-http-mcp-server/src/streamable_http.rs`. Gate: N sessionless POSTs → `storage.list_sessions()` ≤1; 2026 responses assert no Mcp-Session-Id; revert-and-fail.
8. **owner: Protocol** — Gate `notifications/roots/list_changed` (+camelCase) behind `#[cfg(feature="protocol-2025-11-25")]`. Files: `crates/turul-mcp-server/src/builder.rs:198,215`. Gate: default-feature unit test asserts neither key in `builder.handlers`.
9. **owner: Protocol** — Header/body version cross-check + `Mcp-Method`/`Mcp-Name` validation (or WIP defer). Files: `crates/turul-http-mcp-server/src/streamable_http.rs:171`. Gate: wire test rejecting header/body version mismatch.
10. **owner: Tasks/Protocol** — Maintainer sign-off on "no tasks in 2026 build" OR scaffold `turul-mcp-ext-tasks-2026-07-28`. Files: `crates/turul-mcp-ext-tasks-2026-07-28/*` (new), COMPLIANCE.md. Gate: sign-off recorded OR ext crate advertises `extensions["io.modelcontextprotocol/tasks"]`.
11. **owner: Examples** — Pin roots-server to 2025 or annotate deprecation; fix README:27 count. Files: `examples/roots-server/*`, `README.md:27`. Gate: roots-server 0 deprecation warnings OR README states it; README count == 35.
12. **owner: Tests** — Stateless `resources/read` + `prompts/get` cases; bilingual client into default-members; upstream_fixtures into CI. Files: `discover_stateless_2026.rs`, `Cargo.toml`, protocol-2026 Cargo.toml. Gates per §5 G3–G5.
13. **owner: Docs** — Server/client/http-server/aws-lambda + tests READMEs default posture; client lib.rs rustdoc; rustdoc warnings. Files: those READMEs, `crates/turul-mcp-client/src/lib.rs:17,56-79`, aws-lambda/http-server/derive lib.rs. Gate: `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` exits 0; each README headline names 2026-07-28 default.
14. **owner: ADR** — ADR-030 bare-year sketch; ADR-029 §Neutral; ADR-001 compile_error "(default)"; 001 numbering collision. Files: `docs/adr/029,030-*.md`, `crates/turul-mcp-protocol/src/lib.rs:79`. Gates per §2 P2/P3 rows.

---

## 7. Final gate command list

```bash
# === DEFAULT 2026-07-28 LANE ===

# Default build (default-members, 2026-07-28)
cargo build                                                          # PROVEN: Finished, 0 errors

# Default test
cargo test                                                          # PROVEN: 15 binaries, 1015 passed, 0 failed

# Warning sweep on the default test build
cargo test --no-run 2>&1 | grep -c warning:                         # PROVEN: 0

# Default clippy (deny warnings)
cargo clippy --all-targets -- -D warnings                          # PROVEN: default-members clippy, 0 warning/error lines

# Doctests (default 2026)
cargo test -p turul-mcp-server --doc                               # PROVEN: 14 passed
cargo test -p turul-mcp-builders --doc                            # PROVEN: 11 passed

# 2026 stateless acceptance (real HTTP server)
cargo test -p turul-mcp-server --no-default-features \
  --features http,sse,protocol-2026-07-28 --test discover_stateless_2026   # PROVEN: 4 passed, 0 failed

# protocol-2026 compliance incl. upstream wire fixtures
cargo test -p turul-mcp-protocol-2026-07-28 --features compliance  # PROVEN: 179 integration + 3 upstream_fixtures + doctests pass
cargo clippy -p turul-mcp-protocol-2026-07-28 --features compliance -- -D warnings   # PROVEN: clean

# Bilingual client (currently NOT in default cargo test — must be run explicitly)
cargo test -p turul-mcp-client                                     # PROVEN: 144 passed across 8 binaries

# === 2025-11-25 OPT-IN LANE (per-crate verified; aggregate gate NEEDS-RUN) ===

# Per-framework-crate 2025 feature build (PROVEN green individually)
cargo build -p turul-mcp-server     --no-default-features --features http,sse,protocol-2025-11-25   # PROVEN: Finished 6.29s
cargo clippy -p turul-mcp-server    --no-default-features --features http,sse,protocol-2025-11-25 -- -D warnings  # PROVEN: 0 warnings
cargo build -p turul-mcp-builders       --no-default-features --features protocol-2025-11-25        # PROVEN: Finished
cargo build -p turul-http-mcp-server    --no-default-features --features sse,protocol-2025-11-25     # PROVEN: Finished
cargo build -p turul-mcp-derive         --no-default-features --features protocol-2025-11-25         # PROVEN: Finished
cargo build -p turul-mcp-aws-lambda     --no-default-features --features cors,sse,protocol-2025-11-25 # PROVEN: Finished
cargo build -p turul-mcp-client --no-default-features --features http,sse,client-2025-11-25-only     # PROVEN: Finished
cargo build -p turul-mcp-client --no-default-features --features http,sse,client-2026-07-28-only     # PROVEN: Finished

# 2025 opt-in regression suites (real servers)
cargo test -p mcp-tools-tests                                      # PROVEN: 30 passed
cargo test -p mcp-resources-tests                                 # PROVEN: 83 passed
cargo test -p turul-mcp-framework-integration-tests --test tasks_e2e_inmemory   # PROVEN: 7 passed
cargo test -p turul-mcp-framework-integration-tests --no-run      # PROVEN: Finished 17.83s

# Mutex contract (both protocol features => compile_error)
cargo build -p turul-mcp-protocol --features protocol-2025-11-25,protocol-2026-07-28   # PROVEN: mutually-exclusive error fires

# === EXAMPLES ===
# Default (2026) examples build
cargo build                                                        # PROVEN: default-members (35 examples) Finished, 0 warnings
# 2025-pinned examples build (sampled)
cargo build -p tasks-e2e-inmemory-server                          # PROVEN: Finished 4.94s
cargo build -p roots-server                                       # PROVEN: Finished WITH 6-7 SEP-2577 deprecation warnings (P2)

# === GATES THAT NEED-RUN BEFORE RELEASE ===
# Aggregate 2025-11-25 workspace matrix (no single command wired yet — CHANGELOG self-flags)
# NEEDS-RUN: wire as CI job / just-target

# Rustdoc deny-warnings (currently ~9 warnings — would FAIL)
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps                     # NEEDS-RUN: currently fails on ~9 rustdoc warnings (P3)

# CI matrix existence
# NEEDS-RUN: no .github/workflows/ — must be added (P1)

# Cross-process bilingual <-> real-2026-server test
# NEEDS-RUN: test does not exist yet (P1)
```

---

## 8. Drift watch

| Drift risk (codex flagged) | Status | Detail |
|----------------------------|--------|--------|
| **ADRs vs landed code** | **OPEN** | README index omits 022–030 (P1); ADR-009 wire string still "DRAFT-2026-v1" (P1); ADR-023 over-claims `listChanged` removal (P1); ADR-029 §Neutral + ADR-030 bare-year sketch stale (P2). Revision logs are correct but the decision bodies drifted from `MCP_VERSION = "2026-07-28"`. |
| **Examples vs stateless default** | **OPEN** | 14 of 35 default examples still document the removed 2025-11-25 handshake/Mcp-Session-Id/initialize curl (P1); roots-server unpinned, builds deprecated (P2). Builds pass; the wire contract advertised in prose does not match the default server. |
| **E2E coverage of the cutover** | **OPEN** | No bilingual ↔ real-2026-server cross-process test (P1); no CI to enforce the opt-in matrix (P1); resources/read + prompts/get not proven statelessly (P2); upstream_fixtures + bilingual client outside the default gate (P2). The dual-spec matrix is real but manual-only. |
| **README / published-doc claims** | **OPEN** | Root README reconciled (banner + body correct) but lib.rs headers (P1), protocol README alias claim (P1), and four crate READMEs (P2) still advertise 2025-11-25 as the default — the docs.rs landing surface is the drifted artifact. README:27 example count is stale (43 vs 35). |
| **What "2025 compatibility" means** | **PARTIALLY CLOSED** | The opt-in mechanism is sound and PROVEN: alias mutex, `--no-default-features --features protocol-2025-11-25` builds clean per-crate, 2025 regression suites pass, bilingual client speaks either spec per connection. **Closed** at the build/feature level. **Open** at the enforcement level: no aggregate workspace 2025 gate is wired (CHANGELOG self-flags it), so a 2025-only regression in an un-exercised crate could land undetected until CI exists. |
