# ADR-028: Extensions strategy — separate `turul-mcp-ext-*` crates, mirroring upstream `ext-*` repos

**Status:** Accepted
**Date:** 2026-05-24
**Crate(s):** `turul-mcp-protocol-2026-07-28` (host of the `extensions` capability map) + future `turul-mcp-ext-tasks-2026-07-28` (and siblings)
**Branch:** `2026-07-28-MCP-Specification` (and sub-branches)
**Related:** ADR-027 (DRAFT-2026-v1 wire-string target), SEP-2133 (extensions framework), SEP-2663 (tasks extension), SEP-1865 (MCP Apps extension)

## Context

DRAFT-2026-v1 introduces the extensions framework (SEP-2133) and immediately exercises it by demoting tasks from core to an extension (SEP-2663). Both SEPs are now `Final` upstream.

**SEP-2133 establishes** (verified against `github.com/modelcontextprotocol/modelcontextprotocol/seps/2133-extensions.md`):

- Each official extension lives in its **own repository** in the `modelcontextprotocol` GitHub org with the `ext-` prefix (e.g. `ext-auth`, `ext-apps`).
- Extensions are identified by reverse-DNS strings: `io.modelcontextprotocol/oauth-client-credentials`, `com.example/websocket-transport`.
- Extensions evolve **independently of the core protocol** — separate versioning, separate release cadence.
- Capability negotiation happens via an `extensions: { [identifier]: settings }` map on both `ClientCapabilities` and `ServerCapabilities`.
- SDKs **MAY** implement extensions, **MUST** disable them by default, and have full autonomy. Extension support is explicitly **NOT** required for "100% protocol conformance or upcoming SDK conformance tiers."

**SEP-2663 demotes tasks** (verified against `seps/2663-tasks-extension.md`):

- Identifier: `io.modelcontextprotocol/tasks`.
- Methods: `tasks/get`, `tasks/update`, `tasks/cancel` (no `tasks/list` — schema removed it).
- Server is the **sole decider** of when to materialize a task; clients do not request tasks per-call.
- Result discrimination via `resultType: "task"` (extending the schema's `ResultType` union — schema-level extension point).
- Server MUST return `-32003 MISSING_REQUIRED_CLIENT_CAPABILITY` if it cannot serve the request without producing a task and the client did not declare the extension capability.

The framework therefore needs to answer five questions about how it hosts extensions in Rust.

## Decision

### 1. Extensions live in their own crates, named `turul-mcp-ext-<name>-<schema-version>`

We mirror upstream's repo-per-extension separation at the crate level. Each official extension that the framework chooses to support gets its own crate:

| Upstream repo | Framework crate | Extension identifier | Initial version |
|---------------|------------------|----------------------|-----------------|
| `modelcontextprotocol/ext-tasks` | `turul-mcp-ext-tasks-2026-07-28` | `io.modelcontextprotocol/tasks` | `0.1.0` |
| `modelcontextprotocol/ext-apps` | `turul-mcp-ext-apps-2026-07-28` | `io.modelcontextprotocol/apps` (SEP-1865) | `0.1.0` |
| `modelcontextprotocol/ext-auth` | (already covered by existing `turul-mcp-oauth` crate; rebrand only if needed) | `io.modelcontextprotocol/oauth-client-credentials` etc. | inherits existing version |

**Why a separate crate per extension** (not a single `turul-mcp-extensions` blanket crate, not in-tree modules of `turul-mcp-protocol-2026-07-28`):

- **Matches SEP-2133 governance.** Upstream is explicit that extensions evolve independently. Crates can version, ship, and break independently of the protocol crate.
- **Honors the "disabled by default" rule.** A consumer that doesn't depend on `turul-mcp-ext-tasks-*` cannot accidentally import or expose task types. The Cargo dependency declaration is the opt-in.
- **Keeps the protocol crate spec-pure.** `turul-mcp-protocol-2026-07-28` only hosts core schema types. The `extensions: HashMap<String, Value>` capability map field is the ONLY extension surface in the protocol crate itself — the values are opaque `serde_json::Value` so the protocol crate doesn't need to know about any specific extension.
- **Allows non-core extensions.** Third-party extensions can publish `turul-mcp-ext-<vendor>-<name>-<version>` crates following the same naming pattern without touching `turul-mcp-protocol-*`.

**Why `-<schema-version>` suffix on the crate name** (not just `turul-mcp-ext-tasks`):

- The extension's wire shape is fixed against a specific protocol schema version. When the protocol moves to e.g. `2027-NN-NN`, the extension may need to update its types (the schema's `Result`/`InputRequiredResult` shapes etc. may shift). Suffixing the schema version lets multiple major versions coexist in the workspace, just like the protocol crates themselves (`turul-mcp-protocol-2025-11-25`, `turul-mcp-protocol-2026-07-28`).
- Mirrors the "frozen crate" rule (`CLAUDE.md`/`AGENTS.md` "Frozen Protocol Crates"): the `2026-07-28` extensions are frozen once their schema version is frozen, and new spec-version work happens in `-<new-version>` siblings.

### 2. Where the extension types and wire surface live

Per extension crate, the layout mirrors the protocol crate idiom:

```
crates/turul-mcp-ext-tasks-2026-07-28/
├── Cargo.toml
├── README.md
├── schema/
│   └── README.md       # provenance of any vendored upstream extension schema fragments
└── src/
    ├── lib.rs          # public re-exports + module doc
    ├── types.rs        # Task, CreateTaskResult, GetTaskRequest/Result, etc.
    ├── lifecycle.rs    # tasks/get, tasks/update, tasks/cancel method bindings
    ├── capability.rs   # extension capability shape, helper to detect + assert support
    └── compliance_test.rs   # wire-shape tests against vendored SEP-2663 fragments
```

Each extension crate:

- Depends on `turul-mcp-protocol-2026-07-28` (for `Result`, `ResultType`, `RequestMetaObject`, error types).
- Re-exports the extension identifier as a `pub const EXTENSION_IDENTIFIER: &str = "..."`.
- Provides a `capability()` helper returning the value to insert under the identifier in `ClientCapabilities.extensions` / `ServerCapabilities.extensions`.
- Has its own `compliance_test.rs` mirroring the protocol crate's approach (schema-line refs in doc comments, wire-shape tests, drift detectors).

### 3. How the `extensions` HashMap is used at runtime

`turul-mcp-protocol-2026-07-28::initialize::ClientCapabilities.extensions: Option<HashMap<String, Value>>` and the corresponding `ServerCapabilities.extensions` field stay typed as `serde_json::Value` for the per-extension settings. Each extension crate provides a typed helper:

```rust
// In turul-mcp-ext-tasks-2026-07-28/src/capability.rs:
pub const EXTENSION_IDENTIFIER: &str = "io.modelcontextprotocol/tasks";

pub fn capability() -> serde_json::Value {
    serde_json::json!({}) // empty object per SEP-2663 §Capability Negotiation
}

pub fn declared_by(caps: &ClientCapabilities) -> bool {
    caps.extensions
        .as_ref()
        .and_then(|m| m.get(EXTENSION_IDENTIFIER))
        .is_some()
}
```

The server-side `turul-mcp-server` framework dispatches extension methods (e.g. `tasks/get`) only when the extension crate is in scope AND the negotiating client has declared support. Reverse-DNS validation of keys: see §4.

### 4. Reverse-DNS key validation: runtime-only at the negotiation boundary

The protocol crate does NOT enforce reverse-DNS at parse time. Reasons:

- The schema (line 51) says implementations **SHOULD** use reverse-DNS, not **MUST**. A SHOULD violation is not a parse error.
- Strict parse-time enforcement would reject the upstream-reserved `dev.mcp/` and `io.modelcontextprotocol/` prefixes that don't match a literal reverse-DNS pattern (the second label is the meaningful one).
- Validation belongs at the negotiation boundary in `turul-mcp-server` and `turul-mcp-client`, where bad keys can be downgraded to a warning (server logs) or surfaced as a capability-negotiation error (client logs) without breaking deserialization of an otherwise-valid request.

Each extension crate provides a `validate_identifier(s: &str) -> Result<(), ExtensionError>` helper that enforces the SEP-2133 naming rules; the server crate calls it during capability registration.

### 5. Versioning: extensions follow their own semver, independent of protocol crate version

Per SEP-2133 §Evolution: "All extensions evolve independently of the core protocol... new version of an extension MAY be published without review by the core maintainers."

- The extension crate's `Cargo.toml` declares its own `version`, NOT `version.workspace = true` (same independent-versioning pattern established for `turul-mcp-protocol-2026-07-28 = "0.4.0"` in ADR-027).
- Breaking changes to an extension's types REQUIRE a major bump per the extension crate's own semver — independent of whether the protocol crate has bumped.
- Per SEP-2133's breaking-change rule, breaking changes to the extension's wire shape MUST also use a new extension identifier (e.g. `io.modelcontextprotocol/tasks-v2`). When this happens, the existing crate is frozen and a new crate is created at the next protocol-schema-version boundary (`turul-mcp-ext-tasks-v2-NNNN-NN-NN`).

## Consequences

### Good

- **Spec-pure protocol crate.** `turul-mcp-protocol-2026-07-28` stays a faithful 1:1 mapping of `schema/draft-schema.ts` with no extension-specific types leaking in.
- **Independent release cadence.** Tasks can ship a `0.1.x` patch tomorrow without bumping the protocol crate. The protocol crate can move to `2027-NN-NN` without forcing every extension to immediately migrate.
- **Opt-in by Cargo dependency.** Consumers add `turul-mcp-ext-tasks-2026-07-28 = "0.1"` to get task support. Consumers that don't need tasks have a smaller dependency tree and a smaller wire surface.
- **Matches upstream governance.** When MCP creates `modelcontextprotocol/ext-tasks` as a separate repo, we mirror with a separate crate. Easier for downstream readers to find the corresponding crate from a SEP link.

### Bad / accepted tradeoffs

- **More crates to publish and version.** Each official extension is a separate publish step. This is the cost of independent evolution.
- **Cross-crate consumer wiring.** A consumer using tasks needs to import the extension types from a different crate than where they import core types. The `turul-mcp-server` framework smooths this with `register_extension(ext_tasks::capability())` helpers, but downstream code is still explicit about the boundary.
- **No "everything in one crate" convenience.** A consumer that wants every extension must depend on every extension crate. We provide a meta crate `turul-mcp-extensions-2026-07-28` (purely a re-export, no logic) for convenience.

### Migration from 2025-11-25 task users

`turul-mcp-protocol-2025-11-25` still has `tasks.rs` in core because that version's spec did. **That crate is frozen** per the "Frozen Protocol Crates" rule and remains the migration source for downstream code.

For consumers moving from 2025-11-25 to 2026-07-28:

1. Drop the in-line `task: Some(TaskMetadata {..})` field from `CallToolParams` etc. — schema removed it.
2. Add the extension crate to `Cargo.toml`: `turul-mcp-ext-tasks-2026-07-28 = "0.1"`.
3. Declare extension support in client capabilities: `extensions.insert("io.modelcontextprotocol/tasks".into(), serde_json::json!({}))` (or via the helper).
4. Handle the new `CreateTaskResult` result variant returned by the server at server's discretion (per SEP-2663 — server is sole decider).
5. Use `tasks/get`/`tasks/update`/`tasks/cancel` instead of the removed `tasks/list`.

The lifecycle is now stateless (per SEP-2567/2575): no `tasks/list` because there's no session to scope it to.

## Implementation order

1. **Land ADR-028** (this document). ✅
2. **Scaffold `turul-mcp-ext-tasks-2026-07-28`** following the layout in §2. Vendor relevant SEP-2663 fragments under `schema/`. Implement types + capability + lifecycle + compliance tests. Schema-line refs in doc comments per the protocol-crate convention.
3. **Wire extension registration into `turul-mcp-server`** behind a Cargo feature flag (`ext-tasks`) so the protocol crate stays free of the dependency.
4. **Repeat for `turul-mcp-ext-apps-2026-07-28`** when SEP-1865 implementation work is prioritized.
5. **Auth**: existing `turul-mcp-oauth` is already aligned with `io.modelcontextprotocol/oauth-*` extensions — just confirm/update identifiers to match SEP-2133 reverse-DNS form.

Tracked as plan items §5.2 and §5.3 in `docs/plans/2026-07-28-compliance-plan.md`.

## Open items

- The meta-crate name `turul-mcp-extensions-2026-07-28` is provisional; revisit at first publish.
- Server-side dispatcher API for `register_extension(...)` to be designed when `turul-mcp-server` migration to 2026-07-28 lands (separate slice).
- Whether to publish `turul-mcp-ext-*` crates from the same workspace or as siblings (like `turul-rpc`) is a release-engineering decision to revisit before first publish.

## Revision log

- **2026-05-24**: Initial. Decision: separate `turul-mcp-ext-*` crates per official extension, mirroring upstream `ext-*` repos. Tasks extension (`io.modelcontextprotocol/tasks`) is the first one; MCP Apps and Auth are next. Reverse-DNS validation deferred to runtime at the negotiation boundary. Independent semver per extension crate.

- **2026-05-31**: Cross-reference with ADR-027 §"Status update (2026-05-31)". The 0.4.0 default-cutover to DRAFT-2026-v1 does not change this ADR's strategy. Extensions remain per-crate, schema-version-suffixed, opt-in by Cargo dependency. The scaffolding of `turul-mcp-ext-tasks-2026-07-28` (SEP-2663) and `turul-mcp-ext-apps-2026-07-28` (SEP-1865) remains tracked in `docs/plans/2026-07-28-compliance-plan.md` §5.2 and §5.3 — they are not blockers for 0.4.0 publication. The release notes for 0.4.0 will state that tasks and apps support require the respective extension crates (when scaffolded). No content change to this ADR.

- **2026-06-07 (amendment — drop schema-version suffix; build tasks)** — **Active extension crates are named without the schema-version suffix: `turul-mcp-ext-tasks` / `turul-mcp-ext-apps` (was `turul-mcp-ext-tasks-2026-07-28` / `-apps-2026-07-28`).** §1's `turul-mcp-ext-<name>-<schema-version>` convention and the §"Why `-<schema-version>` suffix" rationale are superseded for the active crates: a single `turul-mcp-ext-tasks` crate hosts the tasks extension and reconciles the cross-spec task representations (2025-11-25 carries tasks in core; 2026-07-28 moves them to the `io.modelcontextprotocol/tasks` extension), gated the same way as the protocol coexistence features rather than forked per schema version. Per SEP-2133, a breaking change to the extension's own wire shape still mints a new extension identifier, but that is versioned inside the crate (semver), not by a crate-name suffix. Decision: `turul-mcp-ext-tasks` is to be built (not merely scaffolded) so the bilingual client and server reach tasks against both specs; tracked as its own slice.
