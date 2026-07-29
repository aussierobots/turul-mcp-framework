# MCP 2026-07-28 — Release Checklist (engineering work items)

> **Status: APPLIED, uncommitted — 2026-07-29.** Every §1 release blocker, all of §2, and
> §3–§4 are implemented in the working tree; see the CHANGELOG's 2026-07-29 entries for what
> landed. Do not re-do checked items — verify against the diff instead.
>
> Two things closed that this document originally framed as open questions:
> **Roots** turned out to be a live spec defect rather than a deprecation choice —
> `.with_roots()` served `roots/list` with HTTP 200 on a 2026 build where the spec requires
> 404/`-32601`; the builder surfaces are now cfg-gated to the 2025 lane. **Third-party interop**
> is no longer unverified: FastMCP 4 completes the full stateless journey against a 2026 build
> (`scripts/interop-fastmcp.sh`), which retires the strongest argument against publishing.
>
> Verified at the time of writing: default lane **1276 passed / 0 failed**, protocol suite
> **423 / 0**, integration crate **400 / 0**, 2025 opt-in lanes **248** and **112 / 0**,
> `clippy --all-targets -- -D warnings` clean, schema-pin gate PASS.
>
> Genuinely still open: the maintainer decisions in the companion manual checklist §A
> (branch-lock disposition A1, publish-vs-merge A2, plugin listing status A3, network
> drift job A5).
>
> Originally produced 2026-07-29 at HEAD `2958508` by a 12-agent
> audit (six parallel auditors, adversarial verification of every blocker-severity claim,
> a completeness critic, and an independent devil's-advocate pass). 20 verification
> verdicts: **16 findings stood, 4 were refuted** — the refuted ones are recorded in
> §6 so they are not re-raised.
>
> Companion: [`2026-07-28-manual-verification.md`](./2026-07-28-manual-verification.md) —
> the human-in-the-loop checks, including the decisions only the maintainer can make.
>
> **Headline: no MUST-level spec gap remains, and there is no wire-format delta from the
> release.** What is broken is mostly the paperwork, the CI wiring, and the pin provenance —
> see §1.1, §1.6 and §2.3.
>
> **Read §0 first.** The finalization is *not* a re-implementation trigger, and the single
> highest-value item is not a code change.

---

## 0. What finalization actually changed

Verified against upstream, not assumed:

| Fact | Value |
|---|---|
| Upstream tag `2026-07-28` | `5f5440bb26a62e2cf3440b92da5a667efa03b267` — a **merge commit** (PR #3158, 2 parents) |
| Content-bearing commit for `schema/2026-07-28/` | **`271ecc9accafdd9b83a3c869fa67c22953b2af80`** |
| Released `schema.ts` sha256 | `742750af0bb8c716e7030c4977c992b55d1adc4407e9e66997db5846baedc2cd` |
| Released `schema.ts` blob sha | `9b55feeb412bc3ae877f2eac10b5c01ba29a2eed` (98426 bytes) |
| Our vendored pin | `71e30695…`, sha256 `c56f0ad2…`, at the pre-release path `schema/draft/` |
| **Wire-format delta** | **none** |

The entire content delta between our pin and the release is: `@see` anchor paths
(`/specification/draft/…` → `/specification/2026-07-28/…`), the interface rename
`SubscriptionsListenResultMeta` → `SubscriptionsListenResultMetaObject`, and one new type
`SubscriptionsListenResultResponse` with a new fixture directory.

**Pin the content-bearing commit `271ecc9a`, not the tag commit `5f5440bb`.** The repo's own
`compliance::fetch::resolve_subpath_head` filters history by subpath; because `5f5440bb` is a
merge, the path-filtered head resolves to `271ecc9a`. Pinning the tag would put `fetch.rs`
and `schema/README.md` permanently at odds with what `refresh` computes — the exact
two-different-commits split AGENTS.md §Schema pin governance forbids. Both commits serve
identical content.

---

## 1. Release blockers — ✅ APPLIED (see CHANGELOG 2026-07-29)

### 1.1 Re-point provenance to the immutable released path

`schema/draft/` is upstream's **floating next-cycle pointer**. It survived the release and is
byte-identical to `schema/2026-07-28/` *today* (differing only in four `@see` anchor lines) —
but the moment upstream opens the next draft cycle, `refresh` will silently walk our pin onto
next-spec content while the crate still claims to implement 2026-07-28. Today it is safe;
structurally it is a trap, and CLAUDE.md/AGENTS.md currently *mandate* running that
drift-check every slice.

- [ ] `crates/turul-mcp-protocol-2026-07-28/src/compliance/fetch.rs:27` — `subpath:
      "schema/draft/examples"` → `"schema/2026-07-28/examples"`
- [ ] `crates/turul-mcp-protocol-2026-07-28/src/compliance/fetch.rs:26` — `sha` → `271ecc9a…`
- [ ] `crates/turul-mcp-protocol-2026-07-28/src/bin/compliance.rs:91` — `resolve_subpath_head(&PIN,
      "main", …)` resolves against the floating `main`; repoint to the tag/commit
- [ ] Re-vendor the schema file **first**, then re-derive every provenance value from the file
      actually on disk (never copy a hash forward):
      ```bash
      curl -fsSL "https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/271ecc9accafdd9b83a3c869fa67c22953b2af80/schema/2026-07-28/schema.ts" -o crates/turul-mcp-protocol-2026-07-28/schema/schema.ts
      ```
- [ ] `schema/README.md` — update `Upstream source`, `Raw URL used`, commit pin, `Content
      sha256`, blob sha, ETag. **Rewrite only lines 18–25** (the `⚠ DRAFT-PATH WARNING`
      heading and its one draft-path paragraph). **Preserve lines 27–67 verbatim** — those are
      the re-vendor revision log, not draft-path prose.
- [ ] `schema/EXAMPLES_PIN.md:7` — still says `schema/draft/examples`
- [ ] Apply the two code deltas: rename `SubscriptionsListenResultMeta` →
      `…MetaObject` (`src/subscriptions.rs:193`, re-export `src/lib.rs:180`, doc refs
      `src/meta.rs:217,395`), and add the `SubscriptionsListenResultResponse` binding
- [ ] `src/compliance/coverage.rs` — add a case for the new
      `SubscriptionsListenResultResponse` fixture dir, or `assert_table_matches_upstream`
      fails with "missing". Modeling it is optional; 8 of its 9 `*ResultResponse` siblings are
      `NotModeled`.
- [ ] Update the ADR-027 revision log **and** CHANGELOG.md in the same slice

**Also worth doing while here** (found during verification, independent of the re-pin):
`src/compliance/coverage.rs:461` marks the **existing** `SubscriptionsListenResult` fixture
`NotModeled` even though the type is fully bound at `src/subscriptions.rs:263`. The fixture
is already vendored locally. Modeling it costs nothing and raises `modeled=N`.

### 1.2 Three public docs assert a back-compat alias that the code deliberately removed

`crates/turul-mcp-protocol-2026-07-28/src/version.rs:216` asserts
`"DRAFT-2026-v1".parse::<McpVersion>().is_err()` — there is no `FromStr` arm and no serde
alias. CHANGELOG.md's own entry confirms the alias "was removed entirely". Yet:

- [ ] `crates/turul-mcp-protocol-2026-07-28/README.md:8` — "still accepted on deserialize for
      back-compat, but is never emitted" → **false**, drop the claim
- [ ] `crates/turul-mcp-protocol-2026-07-28/schema/README.md:23` — same claim, same fix
- [ ] `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:14` — same claim, same fix

This is *not* the "keep deserialize back-compat" carve-out: the alias was deliberately
retired, so documenting it as accepted is a false compliance claim in published crate docs.
`version.rs:215-216` is the ground truth — leave it alone.

- [ ] CHANGELOG.md carries **two contradictory `[0.4.0]-Unreleased` entries** on this same
      alias (one says README was fixed to describe the alias, the other says the alias was
      removed entirely), both dated 2026-07-28 with no ordering. Reconcile before tagging.

### 1.3 `COMPLIANCE.md` — the branch's designated compliance authority — is stale both ways

AGENTS.md:44 names this file as the branch's current compliance state. Verified:

| Claim | File says | Reality |
|---|---|---|
| Schema content sha256 (`:12`) | `6e4cba2d…` | `c56f0ad2…` |
| Fixture pin (`:10`) | `60dc69e9…` | `71e30695…` (per `schema/README.md`) |
| `notifications/progress` | listed as an open gap | **already fixed** — cfg-gated at `builder.rs:192`, asserted at `builder.rs:2099` |

- [ ] Re-derive both hashes from disk after the §1.1 re-pin
- [ ] Remove the `notifications/progress` gap entry
- [ ] Re-check the remaining "Known gaps" entries the same way — a document wrong in both
      directions cannot be trusted in either without a pass

### 1.4 The operator playbooks still describe a moving draft

Every future agent session reads these first and will act on the false premise.

- [ ] `CLAUDE.md:27` — "The draft is still moving and is about to finalize as the current
      spec. Upstream keeps revising `schema/draft/schema.ts` in place…"
- [ ] `CLAUDE.md:18` and `AGENTS.md:204` — "release candidate"
- [ ] `AGENTS.md:227` — "is about to finalize as the current spec"
- [ ] `CLAUDE.md:35` — the runnable check still targets `schema/draft/schema.ts`
- [ ] `CLAUDE.md:8` — project description still claims "complete MCP **2025-11-25**
      specification support" (README.md:7 and the crate `lib.rs` docs are already correct)
- [ ] `CLAUDE.md:459` — the §Comments worked example itself cites "the DRAFT-2026-v1 schema";
      it is the pattern contributors copy

> Sequencing: §1.4 is prose-only and safe, **but** see manual-checklist item A1 — the
> maintainer decides whether the branch lock itself survives the wording change.

### 1.5 Two example READMEs document a broken smoke test

Live-verified by running the documented curl verbatim against a running server: both return
`-32020 Header mismatch`. `Mcp-Name` must equal the *item name being invoked*, not a client
identifier.

- [ ] `examples/minimal-server/README.md:75` — `Mcp-Name: test-client` → `Mcp-Name: echo`
- [ ] `examples/zero-config-getting-started/README.md:92` — `Mcp-Name: test` →
      `Mcp-Name: calculator`

Seven other example READMEs already use the correct pattern; this is an old copy-paste
template, not a systemic error.

### 1.6 An entire integration-test crate is orphaned from CI

`tests/Cargo.toml` declares 13 `[[test]]` targets (~402 tests). CI and `scripts/ci-gates.sh`
invoke exactly **two** of them (`tasks_e2e_inmemory`, `ping_auth_2025`). Verified:
`grep -n 'turul-mcp-framework-integration-tests' .github/workflows/ci.yml scripts/ci-gates.sh`
→ 2 hits each, both the same two tests.

This is the same defect class as commit `1ecbbea` ("actually run the 2025-11-25 lane's
tests") — a larger, older sibling instance that fix did not touch.

The proof it has had zero CI signal for at least one full feature slice — reproduced:
```
error[E0063]: missing field `origin_policy` in initializer of `ServerConfig`
  --> tests/consolidated/../http_server_examples.rs:31:19
```
`origin_policy` was added to `ServerConfig` for the 2026 origin-validation work; this call
site was never updated because nothing runs it.

- [ ] Fix `tests/http_server_examples.rs:31` — add `origin_policy: …::default()`
- [ ] Wire all 11 orphaned binaries into `.github/workflows/ci.yml` **and**
      `scripts/ci-gates.sh`

### 1.7 Nothing enforces the schema pin

`grep -rnE 'shasum|sha256sum|sha256|checksum' .github/ scripts/` → **0 hits**. The compliance
suite validates Rust types against the *vendored* schema, so it stays green no matter how far
upstream moves — which is exactly how the current drift accumulated undetected. AGENTS.md
requires pin-drift checks at the start of every slice; that is human discipline with zero
automated enforcement, and §1.1's fix has no gate preventing recurrence.

- [ ] Add a `schema-pin` CI job + matching `gate_schema_pin` in `scripts/ci-gates.sh` that
      (a) recomputes the vendored file's sha256 and fails unless it matches the `Content
      sha256` in `schema/README.md`, and (b) asserts that README's commit equals `PIN` in
      `fetch.rs`. Both checks are offline and deterministic — no network, no flake.

### 1.8 The published plugin teaches removed protocol mechanics as current

`grep -rn '2026-07-28' plugins/` → **0**. `grep -rn '2025-11-25' plugins/` → **36**.
`plugins/turul-mcp-skills` (manifest v0.6.3, 13 skills) ships to end users and currently
teaches the `initialize` / `notifications/initialized` / `Mcp-Session-Id` lifecycle,
`tasks/list` (removed by SEP-2663), and the pre-MRTR elicitation model as live contracts.

- [ ] **Minimum:** add a lane banner to the plugin README and all 13 `SKILL.md` files stating
      they target the 2025-11-25 opt-in lane and are not yet updated for the 2026-07-28
      default. Gate: `grep -rL '2026-07-28' plugins/turul-mcp-skills/skills/*/SKILL.md` empty.
- [ ] **Full:** rewrite `mcp-client-patterns`, `testing-patterns`, `task-patterns`,
      `elicitation-workflows` for the stateless core.

> Severity depends on manual-checklist item **A3** — blocker if the plugin is publicly
> listed, should-fix if unlisted. Note this is a *different* defect from the 46 cosmetic
> `v0.3` version strings in the same files (§3.3).

### 1.9 Subagent definitions target the wrong spec and direct edits into a frozen crate

`grep -rn '2026-07-28' .claude/agents/` → **1 hit**, and it is a plan-doc path, not a spec
target. All six definitions target 2025-11-25; `spec-compliance.md:8` and `architect.md:203`
direct review and edits into `crates/turul-mcp-protocol-2025-11-25/`, which AGENTS.md:20
declares **frozen** ("no code changes, no version bumps, no doc updates").

- [ ] Repoint all six to 2026-07-28 default / 2025-11-25 opt-in, and delete every instruction
      to modify or test the frozen crate. Gate: `grep -rn 'turul-mcp-protocol-2025-11-25'
      .claude/agents/` returns only lines explicitly labelled FROZEN / do-not-edit.

### 1.10 Publish order in CLAUDE.md would fail

`crates/turul-mcp-server/Cargo.toml:34` has a **non-optional** dependency on
`turul-mcp-oauth`, but CLAUDE.md's documented order publishes `server` *before* `oauth`.
Following it would fail exactly as the observed `cargo publish --dry-run -p turul-mcp-derive`
failure did. The list also omits all four crates added since it was written.

- [ ] Replace with the dependency-first order derived from actual `[dependencies]` graphs:
      `turul-mcp-protocol-2026-07-28 → turul-mcp-schema-validation →
      turul-mcp-server-state-storage → turul-mcp-ext-tasks → turul-mcp-ext-apps →
      turul-mcp-client → turul-mcp-protocol → turul-mcp-builders → turul-mcp-session-storage →
      turul-mcp-task-storage → turul-mcp-derive → turul-http-mcp-server → turul-mcp-oauth →
      turul-mcp-server → turul-mcp-aws-lambda`
      (Frozen `turul-mcp-json-rpc-server`, `…-2025-06-18`, `…-2025-11-25` stay published at
      0.3.47 — no republish step.)

---

## 2. Compliance status

**The header tally is accurate.** Re-derived independently: 322 ✅ / 72 🟡 / 9 ❌ / 12 🧪 /
126 ➖ = 541, matching the claimed summary exactly. (A naive emoji `grep -c` over-counts every
category by 1 — line 28 is the Summary table's own column-header row. Anyone spot-checking
this will get 546 and wrongly conclude the header is wrong. It isn't.)

**No MUST-level gap remains.** All 9 ❌ and all 12 🧪 rows are SHOULD/MAY. The ❌ rows cluster
in *client-side consumption* of the new 2026 features, and the most material is real:

| Row | Gap | Judgement |
|---|---|---|
| 503, 507 | Client reads and **discards** `ttlMs`/`cacheScope`; no cache store exists | The headline SEP-2549 caching feature is **emit-only end-to-end** in this stack. SHOULD-level, so not a blocker — but "complete 2026-07-28 support" would overstate it. |
| 731 | Client does not validate `structuredContent` against `outputSchema` | Deliberately dispositioned (`TOOLS-G2`): adding a JSON-Schema validator dep to a published client for a SHOULD. Reasonable call. |
| 316, 338 | No reconnect-on-abrupt-close; no progress-based timeout reset | MAY-level |
| 275, 418, 452, 491 | Progress-token rate limiting, header pre-seeding, trailing-slash normalization, session-to-identity binding | SHOULD/MAY; 491 is largely moot — the 2026 default has no client-visible sessions |

The 12 🧪 rows are lower-stakes than the grade suggests: two (116, 117) are untestable
design-philosophy SHOULDs ("servers should be extremely easy to build"), and most of the rest
are behaviours that are present but lack a *named* test.

### 2.1 One final-changelog item has no row anywhere in the 541-row matrix — ✅ APPLIED

- [ ] **Minor 12 — the error-code allocation policy** (`-32000..-32019` grandfathered /
      `-32020..-32099` spec-reserved) **and the new `HeaderMismatchError` schema type** are
      absent. Verified: `grep -c 'HeaderMismatchError' docs/plans/2026-07-28-spec-compliance.md`
      → **0**; `grep -c 'Minor 12'` → **0**.
      **The code already implements it** — `headers.rs:58` defines
      `ERROR_CODE_HEADER_MISMATCH: i64 = -32020` with a range assertion, and `lib.rs` carries
      the documented partition plus a test module. So this is a gap in the *driver document*,
      not in the code. Add the row citing both.

Reconciliation of the other 27 items: **25 of 28 have a dedicated row.** Three (Major 8
`resultType`, Major 9 SSE-resumability removal, Minor 11 `elicitation/complete` removal) are
substantively covered in other sections but are not cross-referenced from the "Key Changes"
table, so a reader scanning that table alone would conclude they were missed.

- [ ] **2.1a** Add stub rows in the Key Changes table pointing at the sections that cover
      Major 8, Major 9, and Minor 11. *(nice-to-have)*

### 2.2 Roots is the only deprecated feature reachable — and unwarned — on the 2026 default — **OPEN (maintainer decision A4)**

This is sharper than "the builder surfaces lack `#[deprecated]`". Verified in
`crates/turul-mcp-server/src/builder.rs` (`grep -c '#\[deprecated'` → **0** for the whole file):

| Feature | Builder surface | Gated to `protocol-2025-11-25`? |
|---|---|---|
| Sampling | `sampling_provider:981`, `sampling_providers:989`, `with_sampling:1271` | **yes** |
| Logging | `logger:1018`, `loggers:1026` | **yes** |
| **Roots** | `root_provider:1037`, `root_providers:1044`, `with_roots:1256` | **no** |

On a 2026-07-28 default build, Sampling and Logging **do not exist** — a user cannot reach
them regardless of the missing attribute. Roots is fully reachable, unconditional, and
carries no compile-time signal at all, for a feature the *same release* deprecated with an
earliest-removal of 2027-07-28. The protocol-crate types do carry
`#[deprecated(since = "0.4.0", …)]`; the gap is specifically the server builder API users call.

- [ ] **2.2** Either gate the three Roots builder methods behind `protocol-2025-11-25` to
      match Sampling/Logging's posture, **or** add `#[deprecated]` to them. Note the tension
      flagged in §5: the same release routes **roots** through MRTR as a valid `InputRequest`
      variant *and* deprecates the Roots capability — decide the framework's position before
      choosing.

### 2.3 The driver document's own self-accounting does not reconcile — ✅ APPLIED

- [ ] **2.3a** The header claims "**73** registered gaps ... all 73 closed or dispositioned
      (1/1 P0, 14/14 P1, 58/58 P2)". The register actually contains **78 checked entries /
      77 unique IDs** (verified: `sed -n '825,1159p' … | grep -cE '^\s*- \[x\]'` → 78). Even
      excluding the one `WITHDRAWN (obsolete)` entry the floor is 76. No documented basis for
      "73" was found. Correct the header, or document which entries are excluded and why.
- [ ] **2.3b** `UTIL/COMP-3` is used **twice** for two unrelated gaps (completion input
      validation; completion relevance-ranking/rate-limiting). Register-integrity defect
      independent of the count — rename the second.
- [ ] **2.3c** Row 797 cites "20/20 fixtures" against its own evidence source, which now says
      **22/22, with 10 of 87 directories (11.5%) modeled**. Re-grade.

> The 11.5% modeling figure is the one to keep in mind when reading any green result:
> AGENTS.md is explicit that the harness reports `failed=0` for the 88.5% of fixture
> directories it never looked at.

---

## 3. Should-fix

- [ ] **3.1** `docs/plans/2026-07-28-spec-compliance.md` — 94 `/specification/draft/…`
      citation URLs. Bulk-replace to `/specification/2026-07-28/`, then spot-check a sample of
      sub-paths actually resolve before trusting a blind `sed` across 94 URLs.
- [ ] **3.2** ADR-027 Status is still `Accepted (in-flight)` and its own trigger
      ("regenerate on final spec") has now fired with no revision-log entry recording it.
      `docs/adr/README.md:49` repeats the stale status independently. **Do not rename the ADR
      file** — ~15 cross-references cite it by exact path, and per manual item A1 a rename
      reads as a "branch is done" signal.
- [ ] **3.3** 46 `v0.3` references across 10 `SKILL.md` files → `v0.4` (CLAUDE.md
      §Pre-Release item 3: bump on minor changes, which this is).
- [ ] **3.4** `description = "… (BP-3)"` in `crates/turul-mcp-schema-validation/Cargo.toml:9`
      — an internal gap-register ID rendered verbatim on the crates.io page. Also in two
      manifest comments (root `Cargo.toml:183`, `turul-mcp-client/Cargo.toml:26`).
- [ ] **3.5** Both `ext-*` crates vendor from mutable `schema/draft/` paths at ~7-week-stale
      commits — the identical provenance defect as §1.1, unexamined. First check whether
      `modelcontextprotocol/ext-tasks` and `ext-apps` cut release tags; if not, state
      explicitly in each README and in the CHANGELOG that they track an upstream **draft** as
      of the 0.4.0 date, so the release does not imply the extensions are finalized.
- [ ] **3.6** `cargo doc` runs default-features only, so five `turul-mcp-derive` doctests that
      are `rust,ignore` under the 2026 lane are compiled by **no** job — violating CLAUDE.md
      §Before Modifying Core Crates ("every ```rust block MUST compile"). Add
      `cargo test -p turul-mcp-derive --no-default-features --features protocol-2025-11-25 --doc`
      to the opt-in-2025 job. *Try it locally first* — if the alias mutex fires, the `cargo doc`
      fallback is required and this drops to nice-to-have.
- [ ] **3.7** `docs/plans/2026-07-28-final-readiness-audit.md` is self-marked "substantially
      superseded" yet `.github/workflows/ci.yml:6` and `scripts/ci-gates.sh:7` name it as the
      source of the gates CI runs. Its 30 findings have no disposition column; two spot-checked
      P2s are in fact already closed. Add a Status column, then either promote surviving OPEN
      rows here or retire it — **fixing the two CI provenance comments first**, or the gates
      lose their recorded rationale.
- [ ] **3.8** Dead `ctx.method() == "ping"` auth bypass in two 2026-default examples
      (`middleware-auth-server/src/main.rs:67`, `middleware-auth-lambda/src/main.rs:150`).
      `ping` is removed this spec, so the branch is permanently dead and the comment misleads.
      Confirm with the maintainer whether an unauthenticated health-check path is still wanted.
- [ ] **3.9** `cargo check --workspace --all-targets` **fails** — not a code bug, but Cargo
      feature-unification forcing the mutually-exclusive `protocol-2025-11-25` and
      `protocol-2026-07-28` features onto the shared dependency. AGENTS.md:75 nonetheless
      documents `cargo build --workspace` as a primary command. Fix the documented command, or
      add a CI matrix step looping `cargo check -p <pkg>` over non-default members.

---

## 4. Nice-to-have

- [ ] **4.1** Rename the vendored `schema.ts` — now a misnomer. Blast radius is
      contained: 2 `include_str!` sites plus 42 prose references, all enumerated. Sequence
      *after* §1.1, which already rewrites the file.
- [ ] **4.2** Example `.version("x.y.z")` strings are inconsistent — 24 at `0.4.0`, 20 at
      `1.0.0`, 2 at `2.0.0`, 1 at `0.1.0`. Either sync all to the crate version or drop
      CLAUDE.md §Pre-Release item 2; currently neither is true.
- [ ] **4.3** No crate declares `readme = "README.md"` (0 of 18), though 16 have the file.
- [ ] **4.4** `OUTSTANDING.md` is self-flagged for deletion at "final release preparation".
      **Sequencing hazard:** `2026-07-28-final-readiness-audit.md:3` points to it by name, and
      CI points to *that*. Order: disposition the readiness-audit rows → repoint the two CI
      comments → only then delete. Also gated on manual item A1.
- [ ] **4.5** `docs/plans/2026-07-28-migration-diff.md` — title says `DRAFT-2026-v1`, and
      line 53's version-transition arrow is backwards relative to what shipped.
- [ ] **4.6** Bare `#[ignore]` with no reason at `turul-mcp-derive/src/macros/schema.rs:50`.
- [x] **4.7** ~~Ten stale `scripts/verify_phase*.sh` / `run_all_phases*.sh`~~ — renamed to
      `verify_*_examples.sh` and `verify_all_examples*.sh` (descriptive names per CLAUDE.md
      §Comments: "If any is still the documented way to verify something, rename it to what
      it verifies rather than which phase produced it").
- [ ] **4.8** `turul-mcp-client/src/session.rs:4` — unqualified `pub const PROTOCOL_VERSION:
      &str = "2025-11-25"` in a crate whose default is bilingual; reads as the crate's
      protocol version.

---

## 5. Open questions routed to the maintainer

All are in [`2026-07-28-manual-verification.md`](./2026-07-28-manual-verification.md) §A:
branch-lock disposition (A1), publish-vs-merge (A2), plugin listing status (A3), deprecation
removal trigger (A4), network drift job (A5). Plus two rows from the superseded readiness
audit that no agent can disposition: ephemeral session minting on sessionless 2026 POST, and
"no tasks in the 2026 default build".

Also unresolved, from the architecture page: the same release routes **roots** through MRTR
(`InputRequest = CreateMessageRequest | ListRootsRequest | ElicitRequest`) *and* deprecates
Roots under SEP-2577. Nothing in `COMPLIANCE.md` reconciles those for an implementer asking
"should I wire `roots/list` via MRTR or not?" — state the framework's position before writing
any example that demonstrates it.

---

## 6. Claims raised and REFUTED — do not re-raise

Recorded so a later pass does not resurrect them.

1. **"Pin to the tag commit `5f5440bb`."** Wrong — it is a merge commit; the path-filtered
   resolver returns `271ecc9a`. Pinning the tag recreates the two-commit split AGENTS.md
   forbids. *(Independently re-verified.)*
2. **"Every field in `schema/README.md`'s provenance block is stale."** False — the content
   sha256, blob sha and commit pin all match the artifact on disk exactly. **One** field is
   genuinely wrong: line 8's `blob/main/…` URL, which now serves content matching neither our
   pin nor the release. Fix that line alone; do not rewrite correct hashes.
3. **"Delete the `⚠ DRAFT-PATH WARNING` section."** Only lines 20–25 are draft-path prose;
   lines 27–67 are the re-vendor revision log. Deleting the section destroys 41 lines of
   durable provenance history.
4. **"`SubscriptionsListenResult` is the one genuinely new type in the released schema."**
   False — it is already in our vendored draft copy at `schema.ts:1349`. The genuinely
   new type is `SubscriptionsListenResultResponse`. The teardown gap is a pre-existing
   draft-era caveat, spec-legal (the schema permits never emitting it), already tracked in
   `COMPLIANCE.md` — informational, not should-fix.
5. **"Add `turul-mcp-ext-apps` to `[workspace.dependencies]`."** The pin table tracks crates
   consumed via `workspace = true`; `ext-apps` has zero consumers. Add the pin only in the
   slice that introduces the first consumer.
