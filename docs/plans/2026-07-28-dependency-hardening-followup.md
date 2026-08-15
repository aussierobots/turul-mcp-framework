# Dependency-Hardening Follow-up (deferred from the 2026-07-28 readiness slice)

Branch: `feat/turul-mcp-protocol-2026-07-28`. Captured 2026-06-08.

These are **not** 2026-07-28 spec-compliance items and **not** gate failures. They are
dependency-posture decisions deferred out of the cutover/gate work, to be picked up as a
separate dependency-hardening slice (or folded into a future `cargo audit` release gate).

Source: the dependency freshness check on 2026-06-08 — every *direct* workspace pin is at
its latest compatible release (`cargo outdated --root-deps-only` = "all up to date"). The
items below are the exceptions worth a conscious decision.

## 1. `rsa` pinned to a pre-release (P3)

`[workspace.dependencies] rsa = "0.10.0-rc.17"` (latest published: `0.10.0-rc.18`). There is
no stable `0.10`; the last stable line is `0.9`. A published crate carrying a `-rc` dependency
propagates that pre-release constraint to downstream consumers.

- Decision needed: hold on the RC (track to `rc.18`+ / final `0.10`), or move back to stable
  `0.9` until `0.10` ships final.
- Used by: `turul-mcp-oauth` (JWT/JWKS RSA verification path). Re-verify signature behavior on
  any bump.

## 2. Unmaintained YAML crates (P3)

Two YAML dependencies are both flagged unmaintained:

- `serde_yml = "0.0.12"` (latest `0.0.13`, header now reads "DEPRECATED — unmaintained").
- `serde_yaml = "0.9"` (long unmaintained upstream).

Decision needed: consolidate onto a single maintained YAML crate, or drop YAML where it is not
load-bearing. Inventory the actual call sites first (`grep -rl serde_yml\|serde_yaml`).

## 3. Transitive `hyper 0.14` / `rustls 0.21` (informational, not actionable here)

The lockfile carries an old `hyper 0.14` / `http 0.2` / `rustls 0.21` / `h2 0.3` stack
**solely** via `aws-smithy-http-client 1.1.13` → `aws-smithy-runtime` → `aws-config 1.8.18`
(the AWS SDK's internal HTTP client). We are already on the latest `aws-config`/`aws-sdk-*`;
this resolves only when the AWS SDK migrates smithy off hyper 0.14. It coexists harmlessly with
our own hyper 1.x via Cargo multi-version resolution, and only enters through AWS-touching
crates (`turul-mcp-session-storage` dynamodb, the lambda crate). No action available on our side.

## Suggested gate (future)

If/when a release gate adds dependency security, add `cargo audit` (or `cargo deny advisories`)
as a CI job and resolve items 1–2 to clear it. Out of scope for the 2026-07-28 readiness slice.
