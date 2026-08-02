# ADR-032: Adopt turul-jwt-validator; turul-mcp-oauth stops owning JWT validation

**Status**: Proposed
**Date**: 2026-08-02
**Related**: ADR-021 (oauth-resource-server-architecture), ADR-022 (oauth-compliance), ADR-025 (extract-turul-rpc — the precedent this follows)

> **Proposed, not Accepted.** This ADR stays Proposed until
> `crates/turul-mcp-oauth/src/jwt.rs` no longer contains a second
> implementation. An ADR that ratifies an intent while the alternative path
> still ships is a claim, not a decision.
>
> This is the first ADR in this repo to use `Proposed`; the template in
> [README.md](./README.md) previously listed only Accepted / Superseded /
> Deprecated, and has been extended to include it.

## Context

`crates/turul-mcp-oauth/src/jwt.rs` (472 lines, ~300 excluding tests) implements
JWT validation with JWKS caching and kid-miss refresh. That same implementation
now also ships from a sibling repository as
[`turul-jwt-validator`](https://github.com/aussierobots/turul-jwt-validator),
published to crates.io, currently `0.3.2`.

It is not a similar crate. It is this crate's code, extracted:

| Surface | `turul-mcp-oauth::jwt` | `turul-jwt-validator` 0.3.2 |
|---|---|---|
| Claims struct | `TokenClaims { sub, iss, aud, exp, iat, scope, extra }` | identical, field for field, same `#[serde(default)]` / `#[serde(flatten)]` |
| Constructor | `new(jwks_uri, audience)` | identical |
| Builders | `with_issuer`, `with_algorithms`, `with_refresh_interval` | identical, plus three more |
| Errors | `OAuthError::{InvalidToken, TokenExpired, InvalidAudience, InvalidIssuer, UnsupportedAlgorithm, JwksFetchError, KeyNotFound, DecodingError}` | `JwtValidationError` — same eight variants, renamed type |
| Module doc | "JWKS … kid-miss refresh" | same phrase, verbatim, as the crates.io description |

Two implementations of one algorithm is drift. The repo has already ruled on
this shape once: ADR-025 made `turul-mcp-json-rpc-server` a shim over the
extracted `turul-rpc` rather than maintaining both.

**Upstream has since diverged ahead of us.** Four capabilities exist there and
not here, and they are availability and revocation properties, not polish:

| Upstream API | Effect | State here |
|---|---|---|
| `with_max_age(Duration)` | On a `kid` **hit**, a cached key older than `max_age` is treated as a miss and re-fetched — a revocation safety-net | Absent. Once a `kid` hits, that key is served for the process lifetime |
| `with_stale_window(Duration)` | Stale-while-revalidate: keep validating through an AS outage | Absent. A failed refresh is a hard validation failure |
| `with_retry(attempts, base_delay)` | Bounded exponential backoff on JWKS fetch | Absent. Single attempt |
| `JwksFetchErrorKind` | Typed failure: `Timeout` / `Transport` / `HttpStatus(u16)` / `InvalidJson` / `NoSigningKeys` | One flat `JwksFetchError(String)` |

`with_max_age` is the one with a security argument: without it, a key revoked at
the authorization server stays usable here until the process restarts.

Upstream additionally uses `thiserror`, marks both error enums
`#[non_exhaustive]`, and documents RS384/RS512 alongside RS256/ES256/ES384.

**0.3.2 is specifically the release that makes adoption possible.** Its
changelog records the fix: the `jsonwebtoken` dependency previously had
hard-coded features that propagated to every workspace member. The crypto
backend is now selectable via the `aws_lc_rs` (default) / `rust_crypto`
features, so a consumer can keep the backend it already uses.

## Decision

`turul-mcp-oauth` delegates JWT validation to `turul-jwt-validator` and stops
carrying its own implementation.

1. **Ownership.** `turul-jwt-validator` is the sole owner of JWT signature
   verification, JWKS fetch/cache/refresh, and the `TokenClaims` shape.
   `crates/turul-mcp-oauth/src/jwt.rs` is deleted, not kept as a fallback.

2. **Names are preserved; `validate()`'s error type is not.**
   `turul_mcp_oauth::{JwtValidator, TokenClaims}` continue to resolve, as
   re-exports. But `JwtValidator::validate` is
   `pub async fn validate(&self, token: &str) -> Result<TokenClaims, OAuthError>`
   today ([jwt.rs:124](../../crates/turul-mcp-oauth/src/jwt.rs#L124)), and a
   re-exported upstream type returns `JwtValidationError` instead. **That is a
   breaking change to a public signature, and this ADR accepts it** rather than
   papering over it.

   A `From<JwtValidationError> for OAuthError` impl does **not** avoid this — it
   only smooths internal `?` call sites. Preserving the signature would require
   a local newtype wrapping the upstream validator and forwarding all seven
   builder methods, which is precisely the "compatibility copy" the migration
   rules forbid — and it would still leak `jsonwebtoken::Algorithm` through
   `with_algorithms`.

   The break is affordable because 0.4.0 is unpublished and the in-tree blast
   radius is one line: `middleware.rs:102` is the only caller of `validate()`,
   and it already converts via `.map_err(...)`. This is the single strongest
   reason the swap must happen before 0.4.0 rather than after.

3. **`OAuthError` remains this crate's error type at the middleware and
   metadata boundary.** It carries two variants with no upstream equivalent —
   `InvalidResourceUri` and `InvalidConfiguration`, used by `metadata.rs` and
   `oauth_resource_server` — so it is not replaced. It gains
   `impl From<JwtValidationError> for OAuthError`, which must destructure
   `JwksFetchError { kind, message }` as a struct variant (upstream 0.3.0
   changed it from a tuple). After this change both error types are public:
   `JwtValidationError` on `validate()`, `OAuthError` everywhere else.

### Full contract inventory

"Two call sites" counts constructor *invocations* and understates the contract.
The complete surface this change touches:

| Surface | Location | Effect |
|---|---|---|
| `JwtValidator` (type identity) | `lib.rs:70` re-export | Name preserved; becomes a foreign type |
| `TokenClaims` | `lib.rs:70` re-export | Name preserved; foreign type. Also read by `turul-mcp-server` at `ext_tasks.rs:66` |
| `JwtValidator::validate` error type | `jwt.rs:124` | **Breaks**: `OAuthError` → `JwtValidationError` |
| `OAuthResourceMiddleware::new` | `middleware.rs:33` | Public signature takes `Arc<JwtValidator>` — now an `Arc` of a foreign type |
| `OAuthResourceMiddleware.jwt_validator` | `middleware.rs:26` | Private field; no external impact |
| `with_algorithms(Vec<Algorithm>)` | upstream | Leaks `jsonwebtoken::Algorithm`; caller must match version exactly |
| Constructor call sites | `lib.rs:113`, `examples/oauth-resource-server/src/main.rs:136` | Source-compatible (`new(..).with_issuer(..)`) |
| Test constructors | `jwt.rs` ×6, `middleware.rs:308` | **Break**: no upstream injection point |

4. **Backend pinned to RustCrypto**, matching this workspace's existing choice:

   ```toml
   turul-jwt-validator = { version = "0.3.2", default-features = false, features = ["rust_crypto"] }
   ```

   A version pin from crates.io, not a `path` dep — same rule ADR-025 set for
   `turul-rpc`, so the branch builds without a sibling checkout.

5. **Workspace `jsonwebtoken` moves 10 → 11.** This is forced, not optional —
   see Constraints.

### Constraint: the jsonwebtoken major bump is not negotiable

`turul-jwt-validator` 0.3.2 requires `jsonwebtoken = "11"`; this workspace pins
`10.4.0` (resolved). Critically, **upstream does not re-export `Algorithm`** —
there is no `pub use jsonwebtoken::Algorithm` in its `lib.rs`. So
`with_algorithms(Vec<Algorithm>)` obliges the caller to depend on `jsonwebtoken`
directly at a version that matches upstream's exactly.

### Constraint: `Algorithm` must be re-exported, or `with_algorithms` is unusable downstream

A crate-local `jsonwebtoken` dependency makes `Algorithm` available *inside*
`turul-mcp-oauth` only. A downstream consumer calling
`validator.with_algorithms(vec![Algorithm::RS256])` needs the type in *their*
scope, which means adding their own `jsonwebtoken` dependency pinned to exactly
the version upstream uses — an invisible requirement that breaks the moment
either side moves.

**Decision: `turul-mcp-oauth` re-exports `Algorithm`** (`pub use
jsonwebtoken::Algorithm;`).

The coupling is not created by re-exporting — it already exists, because
`Algorithm` sits in the public signature of `with_algorithms` whether or not we
name it. Declining to re-export does not decouple anything; it only makes the
method uncallable without arcane knowledge. The alternatives were considered and
rejected: a wrapper enum duplicates a vocabulary type (and must be kept in sync
with upstream's variants), and "document that downstreams must pin
`jsonwebtoken`" pushes a version-matching obligation onto users.

The residual cost is honest and should be recorded: a `jsonwebtoken` major bump
becomes a breaking change for `turul-mcp-oauth`. The better long-term fix is for
`turul-jwt-validator` to re-export `Algorithm` itself — see Risks.

Adopting without bumping compiles two `jsonwebtoken` versions and makes
`with_algorithms` uncallable — `jsonwebtoken10::Algorithm` and
`jsonwebtoken11::Algorithm` are distinct types. The bump is contained:
`turul-mcp-oauth` is the only crate in the tree that depends on `jsonwebtoken`.

`reqwest` (workspace `0.13`) and `thiserror` (workspace `2.0`) already match
upstream's requirements. No other pin moves.

### Constraint: the existing unit tests cannot survive as written

Seven test entry points construct a validator by injecting a pre-loaded key
through `JwtValidator::test_with_key_async`, a `#[cfg(test)] pub(crate)` helper
that writes private fields directly:

- `jwt.rs` — 6 tests (`test_valid_jwt_accepted`, `test_expired_jwt_rejected_401`,
  `test_wrong_audience_rejected`, `test_wrong_issuer_rejected`,
  `test_alg_none_rejected`, `test_audience_always_validated`)
- `middleware.rs` — `hs256_validator()`, feeding four middleware tests

Upstream exposes no key-injection constructor, no `test-support` feature, and no
way to bypass the HTTP fetch. Its own suite uses `wiremock` + `axum` (both in its
dev-dependencies). These tests therefore move to a mock JWKS endpoint.

This is a cost, and it is the step most likely to be quietly skipped — which
would leave the crate with less coverage than it has today. It is called out
here so that outcome is visible rather than silent. The upside is real: a
wiremock-backed test exercises the actual fetch/parse/cache path, which the
injection helper bypasses entirely.

## Migration plan

Ordered; each step is independently verifiable.

1. **Bump `jsonwebtoken` 10 → 11** in root `Cargo.toml`
   `[workspace.dependencies]`, keeping `features = ["rust_crypto"]`. Build
   `turul-mcp-oauth` alone and fix any 10→11 API breakage before adding the new
   dependency. Isolates the bump from the swap.
2. **Add `turul-jwt-validator = { version = "0.3.2", default-features = false, features = ["rust_crypto"] }`**
   to `[workspace.dependencies]` and to `turul-mcp-oauth`.
3. **Add `impl From<JwtValidationError> for OAuthError`** in `error.rs`,
   destructuring `JwksFetchError { kind, message }`. Decide explicitly whether
   `JwksFetchErrorKind` is preserved into `OAuthError` or flattened to a string;
   preserving it is the point of the upgrade, so prefer widening `OAuthError`.
4. **Delete `crates/turul-mcp-oauth/src/jwt.rs`.** Replace the `pub mod jwt;`
   declaration with re-exports so `turul_mcp_oauth::{JwtValidator, TokenClaims}`
   still resolve.
5. **Rewrite the seven tests** onto a `wiremock` JWKS endpoint. Verify each
   against the pre-migration behaviour: an expired token must still yield
   `TokenExpired`, a wrong audience `InvalidAudience`, `alg:none`/HS256
   `UnsupportedAlgorithm`.
6. **Update the two production call sites** if the swap changes them —
   `lib.rs:113` (`oauth_resource_server`) and
   `examples/oauth-resource-server/src/main.rs:136`. Both use
   `new(...).with_issuer(...)`, which is source-compatible, so this step may be
   a no-op beyond imports.
7. **Apply the hardening policy from §"Policy: the new knobs must be set".**
   Adoption alone changes no runtime behaviour; this step is what delivers the
   benefit and is not optional.
8. **Reconcile every document that cites the local validator.** Deleting
   `jwt.rs` invalidates line-and-test citations across seven surfaces. Under
   AGENTS.md §"Schema pin governance", *"a row whose 'Verified by' cell names a
   test that no longer exists is a defect, not a stale doc"* — so the compliance
   registers are a correctness obligation of this slice, not follow-up tidying.

   | Surface | Exposure |
   |---|---|
   | `docs/compliance/base-protocol.md` | 5 rows cite `jwt.rs::test_*` in "Verified by"; more cite `jwt.rs:LINE` as Implementation. Every one of the six deleted tests is named here |
   | `docs/plans/2026-07-28-spec-compliance.md` | 3 rows cite `jwt.rs::test_*`, plus ~8 citing `jwt.rs:LINE` as compliance evidence |
   | `plugins/turul-mcp-skills/.../references/jwt-validator-reference.md` | A whole shipped API reference for `JwtValidator`. Asserts "construction, builder methods, `validate()`" are **stable** — which this ADR makes false |
   | `plugins/turul-mcp-skills/skills/auth-patterns/SKILL.md` | Multiple `JwtValidator::new` code samples taught to users |
   | `crates/turul-mcp-oauth/README.md` | Component list + usage example |
   | `examples/oauth-resource-server/README.md` | Describes the validator's behaviour |
   | ADR-021 (113–114, 156), ADR-022 (19, 24, 66, 72) | Describe `JwtValidator` as owned here |
   | Plugin routing/index surfaces — `plugins/turul-mcp-skills/README.md:160`, `middleware-patterns/SKILL.md:12`, `authorization-server-patterns/SKILL.md:391` | Skill trigger lists and cross-references naming `JwtValidator` / `TokenClaims` / JWKS. Lowest impact, since those names survive — but they must be re-read once `hardened_validator` exists, because the recommended entry point changes |
   | `authorization-server-patterns/references/oauth-endpoint-responsibilities.md:216` | Documents the RS fetching `http://localhost:9000/.well-known/jwks.json`. **Interacts with step 9** — it stays correct only because the TLS check exempts loopback |

   Line-number citations into a deleted file cannot be "updated" — they must be
   re-pointed at the upstream crate or replaced with test names that exist.

9. **Close the TLS gap in `hardened_validator`.** `base-protocol.md:117` grades
   *"TLS enforced on JWKS / issuer URIs"* as SHOULD / **Unknown**, citing "no
   scheme check in `JwtValidator::new`", with narrative at :320 noting ADR-021
   claims the posture while no code implements it.

   **Verified: upstream does not close this.** `turul-jwt-validator` stores
   `jwks_uri` unvalidated in `new()` and passes it straight to
   `http_client.get(&self.jwks_uri)` in `fetch_jwks_once`; the only `https`
   occurrences in its source are test fixtures. Adoption therefore *preserves*
   the gap exactly.

   Since `hardened_validator` is ours and is now the single entry point for both
   recommended paths, the check belongs there: reject a `jwks_uri` whose scheme
   is not `https`, **exempting loopback hosts** (`localhost`, `127.0.0.1`,
   `::1`) so local development and the documented
   `http://localhost:9000/.well-known/jwks.json` example keep working. Returns
   `OAuthError::InvalidConfiguration`. Re-grade the row to Implemented and cite
   the new test.

   **This is the one deliberate scope addition in this ADR**, and it is here for
   a timing reason rather than an opportunistic one: rejecting a plaintext JWKS
   URI is a breaking behaviour change for anyone running that configuration, so
   pre-release is the only free moment — the identical argument that governs the
   `validate()` break. Strike this step if you would rather carry the gap; then
   ADR-021's unimplemented claim must be corrected instead, since the slice
   cannot leave both the code and the ADR saying different things.

10. **Update ADR-021's revision log** — its lines 113–114 and 156 describe
    `JwtValidator` as owned here. Same for ADR-022.
11. **Update the publish order** in `CLAUDE.md` §Pre-Release Checklist and
   `AGENTS.md`: `turul-jwt-validator` becomes an external dependency of
   `turul-mcp-oauth`, so it is not in the workspace publish sequence, but the
   checklist's regeneration command output changes and should be re-derived
   rather than hand-edited.
12. **CHANGELOG entry** under the 0.4.0 release, recording the `validate()`
    error-type break explicitly — a breaking signature change that ships
    unannounced is worse than the break itself.
13. **Fix the stale spec wording** at
    `crates/turul-mcp-oauth/src/lib.rs:7`, which still reads "MCP draft,
    2026-07-28 era"; the spec has finalized. Same slice, since this crate is
    already open.

**Gate before flipping this ADR to Accepted:**

1. `jwt.rs` is deleted, not `#[cfg]`-disabled.
2. No **implementation** use of `jsonwebtoken` remains in
   `crates/turul-mcp-oauth/src/` — no `decode`, `decode_header`, `Validation`,
   `DecodingKey`, or JWKS parsing. Two uses are permitted and expected: the
   `pub use jsonwebtoken::Algorithm` re-export decided above, and dev-dependency
   use for minting test tokens. (An earlier draft of this gate said "no direct
   use outside dev-dependencies", which contradicted the re-export requirement.)
3. The rewritten tests fail when the delegation is reverted.
4. Every documentation surface in migration step 8 has been reconciled, and the
   TLS row in step 9 has been re-graded against the new implementation.

## Policy: the new knobs must be set, or the benefit is not claimed

**Upstream ships all three hardening features disabled by default.** `max_age`
is unset, retry is off, and the stale window is zero — the 0.3.1 changelog is
explicit that the default "preserv[es] existing behavior". Swapping the
dependency therefore delivers *no* revocation, outage-tolerance or retry
improvement on its own. The indefinite key-hit behaviour described in Context
survives adoption unchanged unless this section is implemented.

This ADR does not accept "the capability now exists upstream" as the benefit.
Either the framework sets a policy, or the Consequences below must drop the
security claim. Proposed defaults, applied in `oauth_resource_server` (the
opinionated constructor — a hand-built `JwtValidator` stays fully configurable):

| Knob | Default | Rationale |
|---|---|---|
| `with_max_age` | **15 min** | Bounds how long a revoked-at-the-AS key stays usable. One extra JWKS GET per 15 min per validator is negligible, so cost barely constrains the choice; 15 min is conservative against the 1 h–24 h caching many deployments run, without being chatty |
| `with_stale_window` | **5 min** | Survive a brief AS/JWKS outage instead of failing every request |
| `with_retry` | **3 attempts, 100 ms base** | Absorbs transient transport failures. Worst case adds ~300 ms before falling back to stale, and only when no usable cached key exists — a degraded state already |

**Worst-case revocation exposure is `max_age + stale_window` = 20 minutes**,
because stale serving extends past the age bound by design. An earlier draft of
this ADR claimed the stale window "cannot outlive the revocation bound" — that
was wrong; stale serving is precisely a bounded overrun of it. The number a
security reviewer should be given is 20 minutes, not 15.

### The policy binds manual construction too, via one exported function

Applying the policy only inside `oauth_resource_server` would harden the
*convenience* path and leave the *advanced* path on upstream's all-off defaults.
That is not a hypothetical gap: ADR-021, ADR-022 and the `auth-patterns` skill
all explicitly route multi-AS deployments to manual construction. Under a
convenience-only policy, **every multi-AS deployment would be unhardened**, and
nothing would say so.

**Decision:** `turul-mcp-oauth` exports

```rust
pub fn hardened_validator(jwks_uri: &str, audience: &str) -> Result<JwtValidator, OAuthError>
```

which applies the table above, and `oauth_resource_server` calls it internally.
The policy constants therefore have exactly one definition, and both the
convenience and manual paths route through it — the same one-owner rule this ADR
applies to JWT validation itself.

Rejected alternatives: pushing the defaults into `JwtValidator` is not ours to
do (the type is upstream's; it can only be a request), and a second *constructor*
would add a competing way to build the same thing. A free function that applies
a policy is not a parallel implementation — it has no validation logic of its
own.

Raw `JwtValidator::new` remains available and remains unhardened; that is
upstream's general-purpose type behaving as documented. What changes is that
every path this framework *recommends* is hardened, and the raw path is
documented as the opt-out rather than the silent default.

**Consequently, every `JwtValidator::new` snippet we ship must be updated** —
`examples/oauth-resource-server/src/main.rs:136` and the skills-plugin samples.
An example is a teaching artifact; shipping the unhardened form teaches the
unhardened pattern regardless of what the prose says.

**Required tests** (else the policy is unverifiable):

- A cached key older than `max_age` triggers a re-fetch on a `kid` **hit**, not
  just a miss. This is the test that proves the revocation claim; without it the
  claim is unfounded.
- With the JWKS endpoint failing, validation still succeeds inside
  `stale_window` and fails after it.
- A fetch that fails twice then succeeds yields a successful validation.

## Timing: this must land before 0.4.0 publishes

`turul_mcp_oauth::JwtValidator` and `TokenClaims` are public API. Once 0.4.0 is
on crates.io they are a compatibility surface, and converting them to re-exports
of a differently-versioned upstream type becomes a breaking change for
downstream consumers.

Today nothing depends on 0.4.0, so the swap costs nothing externally. The
`jsonwebtoken` major bump is likewise far cheaper pre-release. Deferring
converts a free change into a 0.5-cycle breaking change.

## Consequences

### Positive

- One implementation of JWT validation, one owner. Removes ~300 lines from this
  workspace.
- Bounded revocation exposure (20 min worst case, from unbounded today), outage
  tolerance and fetch retry — delivered on every recommended path via
  `hardened_validator`, not merely made available. Adoption *without* that
  function would be behaviour-neutral.
- Closes the standing TLS-on-JWKS gap (step 9), retiring an ADR-021 claim that
  no code has ever implemented.
- Typed JWKS failures (`JwksFetchErrorKind`) — this one *is* automatic, since it
  is the error type's shape rather than an opt-in setting.
- JWT validation becomes independently versionable and independently
  security-patchable, without a framework release.
- Tests move from private-field injection to a real HTTP fetch path.
- Matches the precedent already set by ADR-025.

### Negative

- **Breaks `JwtValidator::validate`'s public error type** (`OAuthError` →
  `JwtValidationError`), and leaves two public error types in the crate.
  Affordable only because 0.4.0 is unpublished.
- Forces `jsonwebtoken` 10 → 11 workspace-wide.
- Seven test entry points must be rewritten; `wiremock` joins dev-dependencies.
- A cross-repo dependency for a security-critical path: a CVE in JWT validation
  is now fixed upstream and pulled in, rather than fixed in-tree.
- `Algorithm` is not re-exported upstream, so `turul-mcp-oauth` must keep a
  direct `jsonwebtoken` dependency purely to re-export that type, and the two
  versions must be kept in lockstep. A `jsonwebtoken` major bump therefore
  becomes a `turul-mcp-oauth` breaking change.
- The documentation debt is larger than the code change: seven surfaces cite the
  local validator by file, line or test name (migration step 8), including a
  shipped skills-plugin API reference that currently asserts the API is stable.

### Risks

- **Version skew.** If upstream bumps to `jsonwebtoken` 12 and we do not,
  `with_algorithms` silently becomes uncallable again. Mitigation: the lockstep
  requirement is stated in this ADR and should be asserted by a build-time test
  that calls `with_algorithms`, so skew fails the build rather than surfacing at
  a call site.
- **Coverage regression.** Step 5 is the skippable one. Mitigation: the
  Accepted-gate above requires the rewritten tests to fail on reverted
  delegation.
- **Upstream availability.** A sibling repo becomes a release dependency for the
  framework. This risk is already accepted for `turul-rpc` under ADR-025.

## Alternatives considered

**Keep `jwt.rs`, port the four features by hand.** Avoids the major bump and the
test rewrite. Rejected: it accepts two implementations of one algorithm
permanently, and re-implementing `stale_window` / `max_age` / retry semantics by
hand is exactly the divergence this ADR exists to end. The features are the
symptom, not the reason.

**Adopt after 0.4.0.** Rejected on the timing argument above — it converts a
free change into a breaking one.

**Vendor the upstream source into the workspace.** Rejected: same drift, with
the provenance obscured.

## Revision log

- 2026-08-02: Proposed. Investigation established API equivalence with
  `turul-jwt-validator` 0.3.2, the four upstream-only capabilities, the
  `jsonwebtoken` 10→11 constraint (upstream does not re-export `Algorithm`),
  and the seven-test injection dependency. No code changed.
- 2026-08-02: Revised after external review, which found three defects in the
  first draft. All three were verified against the source and upheld:
  1. The draft claimed "downstream code compiles unchanged". False —
     `validate()`'s error type is public and changes. The ADR now decides that
     break explicitly instead of asserting compatibility it does not deliver.
  2. The draft listed a revocation safety-net under Positive consequences.
     Overclaimed — upstream defaults `max_age`, retry and stale-window to off,
     so adoption alone is behaviour-neutral. Added §"Policy: the new knobs must
     be set" with proposed defaults and the tests that would prove them.
  3. "Two call sites" understated the contract, which includes
     `OAuthResourceMiddleware::new`'s public signature. Added the full inventory
     table.

  A fourth finding — that this ADR did not exist — was an artifact of the
  reviewer reading a different checkout, and was withdrawn.
- 2026-08-02: Second review round; three further findings, all upheld.
  1. The hardening policy bound only `oauth_resource_server`, while the example
     hand-builds its validator — so the visible migration path would have
     shipped with every protection off. The manual-construction question is now
     recorded as an explicit OPEN decision instead of being settled by a
     parenthetical; the example half is resolved (an example must teach the
     hardened pattern).
  2. The migration plan omitted the documentation surfaces. The reviewer named
     two; the full inventory is seven, including 5 + 3 compliance rows whose
     "Verified by" cells name the six tests this ADR deletes, and a shipped
     skills-plugin API reference asserting the API is stable. Added as step 8,
     with the pre-existing TLS-enforcement row as step 9 — it grades deleted
     code and must be re-verified.
  3. The Accepted-gate demanded no direct `jsonwebtoken` use while the
     Consequences required keeping it for `Algorithm` — a flat contradiction.
     Resolved by deciding the re-export explicitly and rewriting the gate to
     distinguish implementation use from the re-export and test-token minting.

  Also confirmed by that review, independently of this workspace:
  `turul-jwt-validator 0.3.2` compiles against `jsonwebtoken 11` with
  `rust_crypto`, and `aws-lc-rs` can still enter via `reqwest`'s TLS stack
  without being the JWT crypto backend.
- 2026-08-02: Third review round, and the open decisions resolved on the
  maintainer's instruction to settle them.
  1. **TLS is now established, not unknown.** Read the upstream source: `new()`
     stores `jwks_uri` unvalidated and `fetch_jwks_once` GETs it directly, with
     no scheme check anywhere. Adoption preserves the gap exactly. Step 9 now
     closes it in `hardened_validator` with a loopback exemption, and is flagged
     as this ADR's one deliberate scope addition.
  2. **Manual construction is no longer an open question.** Convenience-only
     hardening would have left every multi-AS deployment unhardened — and the
     docs actively route multi-AS users to manual construction. Resolved with an
     exported `hardened_validator` that both paths call, so the policy constants
     have one owner.
  3. **Defaults decided**: `max_age` 15 min, `stale_window` 5 min, retry 3 ×
     100 ms. Corrected a wrong rationale in the process — the draft claimed the
     stale window "cannot outlive the revocation bound", but stale serving is by
     definition a bounded overrun of it. Worst-case exposure is `max_age +
     stale_window` = **20 minutes**, and the ADR now says so.
  4. **ADR status vocabulary reconciled.** The previous round's edit to
     README.md listed four statuses while the table used six, including
     `Mandatory` (5 ADRs) and `Closed` (1) — a defect introduced by editing the
     prose without checking the data. Now documents the live vocabulary with
     counts, and the template matches.
  5. Plugin routing/index surfaces added to the step-8 inventory. One of them
     documents an `http://localhost` JWKS endpoint and survives only because
     step 9 exempts loopback — recorded, since the two decisions are coupled.
