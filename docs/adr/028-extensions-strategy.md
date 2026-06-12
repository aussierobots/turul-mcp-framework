# ADR-028: Extensions strategy — separate `turul-mcp-ext-*` crates, mirroring upstream `ext-*` repos

**Status:** Accepted
**Date:** 2026-05-24
**Crate(s):** `turul-mcp-protocol-2026-07-28` (host of the `extensions` capability map) + `turul-mcp-ext-tasks` / `turul-mcp-ext-apps` (and future siblings)
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

### 1. Extensions live in their own SPEC-NEUTRAL crates, named `turul-mcp-ext-<name>`

(Revised 2026-06-07; original schema-version-suffixed convention recorded in the revision log.)

We mirror upstream's repo-per-extension separation at the crate level. Each official extension that the framework chooses to support gets its own crate, named WITHOUT a schema-version suffix — per-spec-line surfaces live in feature-gated modules inside the one crate (`v2026_07_28`, …), the same coexistence pattern the protocol features use:

| Upstream repo | Framework crate | Extension identifier | Initial version |
|---------------|------------------|----------------------|-----------------|
| `modelcontextprotocol/ext-tasks` | `turul-mcp-ext-tasks` | `io.modelcontextprotocol/tasks` | `0.1.0` |
| `modelcontextprotocol/ext-apps` | `turul-mcp-ext-apps` | `io.modelcontextprotocol/ui` (SEP-1865; upstream reserves this label — see the 2026-06-12 revision entry) | `0.1.0` |
| `modelcontextprotocol/ext-auth` | (already covered by existing `turul-mcp-oauth` crate; rebrand only if needed) | `io.modelcontextprotocol/oauth-client-credentials` etc. | inherits existing version |

**Why a separate crate per extension** (not a single `turul-mcp-extensions` blanket crate, not in-tree modules of `turul-mcp-protocol-2026-07-28`):

- **Matches SEP-2133 governance.** Upstream is explicit that extensions evolve independently. Crates can version, ship, and break independently of the protocol crate.
- **Honors the "disabled by default" rule.** A consumer that doesn't depend on a `turul-mcp-ext-*` crate cannot accidentally import or expose that extension's types. The Cargo dependency declaration is the opt-in.
- **Keeps the protocol crate spec-pure.** `turul-mcp-protocol-2026-07-28` only hosts core schema types. The `extensions: HashMap<String, Value>` capability map field is the ONLY extension surface in the protocol crate itself — the values are opaque `serde_json::Value` so the protocol crate doesn't need to know about any specific extension.
- **Allows non-core extensions.** Third-party extensions can publish `turul-mcp-ext-<vendor>-<name>` crates following the same naming pattern without touching `turul-mcp-protocol-*`.

**Why spec-NEUTRAL crate names + feature-gated lane modules** (not `-<schema-version>` suffixed crates):

- One crate can host and RECONCILE an extension across spec lines — tasks is the proving case (2025-11-25 carries tasks in core; 2026-07-28 moves them to the `io.modelcontextprotocol/tasks` extension). A per-spec crate fork would force the cross-spec story into a third place.
- Per-spec surfaces live in feature-gated modules (`#[cfg(feature = "protocol-2026-07-28")] pub mod v2026_07_28;`) with the protocol-sibling dependency optional behind the matching feature — exactly the coexistence pattern of the protocol features (ADR-029). `--no-default-features` compiles to an empty crate.
- Per SEP-2133, a breaking change to the extension's own wire shape mints a new extension IDENTIFIER (e.g. `io.modelcontextprotocol/tasks-v2`); that is versioned inside the crate (a new module + semver major), not by a crate-name suffix.
- A bare-name crate also satisfies the repo's spec-version naming rule: names without a date are deliberately spec-neutral (`CLAUDE.md` §Spec-Version Naming).

### 2. Where the extension types and wire surface live

Per extension crate, the layout mirrors the protocol crate idiom:

```
crates/turul-mcp-ext-tasks/
├── Cargo.toml          # features: protocol-2026-07-28 (default), later protocol-2025-11-25
├── schema/
│   └── README.md       # provenance of vendored upstream extension schema (pinned commit)
└── src/
    ├── lib.rs          # feature-gated lane modules + flat re-exports
    └── v2026_07_28/
        ├── mod.rs
        ├── types.rs    # Task, DetailedTask, CreateTaskResult, etc.
        ├── lifecycle.rs # tasks/get, tasks/update, tasks/cancel method bindings
        ├── capability.rs # extension capability shape + support detection
        └── compliance_test.rs # wire-shape tests against the vendored schema
```

Each extension crate:

- Depends on the protocol-schema-version sibling for each lane module (`turul-mcp-protocol-2026-07-28` for `v2026_07_28`), as an OPTIONAL dependency behind the matching `protocol-*` feature.
- Re-exports the extension identifier as a `pub const EXTENSION_IDENTIFIER: &str = "..."`.
- Provides a `capability()` helper returning the value to insert under the identifier in `ClientCapabilities.extensions` / `ServerCapabilities.extensions`.
- Has its own `compliance_test.rs` mirroring the protocol crate's approach (schema-line refs in doc comments, wire-shape tests, drift detectors).

### 3. How the `extensions` HashMap is used at runtime

`turul-mcp-protocol-2026-07-28::initialize::ClientCapabilities.extensions: Option<HashMap<String, Value>>` and the corresponding `ServerCapabilities.extensions` field stay typed as `serde_json::Value` for the per-extension settings. Each extension crate provides a typed helper:

```rust
// In turul-mcp-ext-tasks/src/v2026_07_28/capability.rs:
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
- Per SEP-2133's breaking-change rule, breaking changes to the extension's wire shape MUST also use a new extension identifier (e.g. `io.modelcontextprotocol/tasks-v2`). The new identifier's surface lands as a new module (and a semver-major bump) inside the SAME crate — crate names stay spec-neutral.

## Consequences

### Good

- **Spec-pure protocol crate.** `turul-mcp-protocol-2026-07-28` stays a faithful 1:1 mapping of `schema/draft-schema.ts` with no extension-specific types leaking in.
- **Independent release cadence.** Tasks can ship a `0.1.x` patch tomorrow without bumping the protocol crate. The protocol crate can move to `2027-NN-NN` without forcing every extension to immediately migrate.
- **Opt-in by Cargo dependency.** Consumers add `turul-mcp-ext-tasks = "0.1"` to get task support. Consumers that don't need tasks have a smaller dependency tree and a smaller wire surface.
- **Matches upstream governance.** When MCP creates `modelcontextprotocol/ext-tasks` as a separate repo, we mirror with a separate crate. Easier for downstream readers to find the corresponding crate from a SEP link.

### Bad / accepted tradeoffs

- **More crates to publish and version.** Each official extension is a separate publish step. This is the cost of independent evolution.
- **Cross-crate consumer wiring.** A consumer using tasks needs to import the extension types from a different crate than where they import core types. The `turul-mcp-server` framework smooths this with `register_extension(ext_tasks::capability())` helpers, but downstream code is still explicit about the boundary.
- **No "everything in one crate" convenience.** A consumer that wants every extension must depend on every extension crate. A re-export-only meta crate (`turul-mcp-extensions`) can be added if demand materializes.

### Migration from 2025-11-25 task users

`turul-mcp-protocol-2025-11-25` still has `tasks.rs` in core because that version's spec did. **That crate is frozen** per the "Frozen Protocol Crates" rule and remains the migration source for downstream code.

For consumers moving from 2025-11-25 to 2026-07-28:

1. Drop the in-line `task: Some(TaskMetadata {..})` field from `CallToolParams` etc. — schema removed it.
2. Add the extension crate to `Cargo.toml`: `turul-mcp-ext-tasks = "0.1"`.
3. Declare extension support in client capabilities: `extensions.insert("io.modelcontextprotocol/tasks".into(), serde_json::json!({}))` (or via the helper).
4. Handle the new `CreateTaskResult` result variant returned by the server at server's discretion (per SEP-2663 — server is sole decider).
5. Use `tasks/get`/`tasks/update`/`tasks/cancel` instead of the removed `tasks/list`.

The lifecycle is now stateless (per SEP-2567/2575): no `tasks/list` because there's no session to scope it to.

## Implementation order

1. **Land ADR-028** (this document). ✅
2. **Scaffold `turul-mcp-ext-tasks`** following the layout in §2. Vendor the upstream extension schema under `schema/` at a pinned commit. Implement types + capability + lifecycle + compliance tests. ✅ (2026-06-12)
3. **Wire extension registration into `turul-mcp-server`** behind a Cargo feature flag (`ext-tasks`) so the protocol crate stays free of the dependency.
4. **Repeat for `turul-mcp-ext-apps`** — MCP-side surface. ✅ (2026-06-12; identifier is `io.modelcontextprotocol/ui`)
5. **Auth**: existing `turul-mcp-oauth` is already aligned with `io.modelcontextprotocol/oauth-*` extensions — just confirm/update identifiers to match SEP-2133 reverse-DNS form.

Tracked as plan items §5.2 and §5.3 in `docs/plans/2026-07-28-compliance-plan.md`.

## Open items

- The meta-crate idea (`turul-mcp-extensions`, re-export only) is provisional; revisit at first publish.
- Server-side dispatcher API for `register_extension(...)` to be designed when `turul-mcp-server` migration to 2026-07-28 lands (separate slice).
- Whether to publish `turul-mcp-ext-*` crates from the same workspace or as siblings (like `turul-rpc`) is a release-engineering decision to revisit before first publish.

## Revision log

- **2026-05-24**: Initial. Decision: separate `turul-mcp-ext-*` crates per official extension, mirroring upstream `ext-*` repos. Tasks extension (`io.modelcontextprotocol/tasks`) is the first one; MCP Apps and Auth are next. Reverse-DNS validation deferred to runtime at the negotiation boundary. Independent semver per extension crate.

- **2026-05-31**: Cross-reference with ADR-027 §"Status update (2026-05-31)". The 0.4.0 default-cutover to DRAFT-2026-v1 does not change this ADR's strategy. Extensions remain per-crate, schema-version-suffixed, opt-in by Cargo dependency. The scaffolding of `turul-mcp-ext-tasks-2026-07-28` (SEP-2663) and `turul-mcp-ext-apps-2026-07-28` (SEP-1865) remains tracked in `docs/plans/2026-07-28-compliance-plan.md` §5.2 and §5.3 — they are not blockers for 0.4.0 publication. The release notes for 0.4.0 will state that tasks and apps support require the respective extension crates (when scaffolded). No content change to this ADR.

- **2026-06-07 (amendment — drop schema-version suffix; build tasks)** — **Active extension crates are named without the schema-version suffix: `turul-mcp-ext-tasks` / `turul-mcp-ext-apps` (was `turul-mcp-ext-tasks-2026-07-28` / `-apps-2026-07-28`).** §1's `turul-mcp-ext-<name>-<schema-version>` convention and the §"Why `-<schema-version>` suffix" rationale are superseded for the active crates: a single `turul-mcp-ext-tasks` crate hosts the tasks extension and reconciles the cross-spec task representations (2025-11-25 carries tasks in core; 2026-07-28 moves them to the `io.modelcontextprotocol/tasks` extension), gated the same way as the protocol coexistence features rather than forked per schema version. Per SEP-2133, a breaking change to the extension's own wire shape still mints a new extension identifier, but that is versioned inside the crate (semver), not by a crate-name suffix. Decision: `turul-mcp-ext-tasks` is to be built (not merely scaffolded) so the bilingual client and server reach tasks against both specs; tracked as its own slice.

- **2026-06-12 (scaffold landed)** — `crates/turul-mcp-ext-tasks` created per the 2026-06-07 amendment (spec-neutral name, feature-gated lanes): `v2026_07_28` module carries the SEP-2663 surface (status-tagged `DetailedTask`, `CreateTaskResult` with `resultType: "task"`, `tasks/get`/`tasks/update`/`tasks/cancel` bindings, `notifications/tasks`, `taskIds` subscription filter fields, capability negotiation helpers incl. SEP-2133 identifier validation) with 13 wire-shape compliance tests; upstream schema vendored from `modelcontextprotocol/ext-tasks@8966bea9` with provenance README. `protocol-2026-07-28` is the default feature; `--no-default-features` compiles to an empty crate. NOT yet wired into `turul-mcp-server` dispatch (step 3 of §Migration path) and the 2025-11-25 reconciliation module is not started — both remain their own slices.

- **2026-06-12 (apps scaffold landed; identifier corrected)** — `crates/turul-mcp-ext-apps` created (spec-neutral name per the 2026-06-07 amendment). **The extension identifier is `io.modelcontextprotocol/ui`, not `io.modelcontextprotocol/apps`** — the original table guessed `/apps` before upstream published; `modelcontextprotocol/ext-apps`'s spec (`specification/draft/apps.mdx`) states "This extension is identified as: io.modelcontextprotocol/ui" and reserves that label. Scope decision: the crate binds the **MCP-side** surface only (client capability `mimeTypes` incl. the `text/html;profile=mcp-app` gate, tool `_meta.ui` → `UiToolMeta` (`resourceUri`/`visibility`), UI-resource `_meta.ui` → `UiResourceMeta` (CSP/permissions/domain/prefersBorder)); the host↔view iframe protocol (`ui/*` over postMessage) belongs to app/host SDKs and is out of a server framework's scope. Vendored from `modelcontextprotocol/ext-apps@ca1d2989` with provenance README; 5 wire-shape compliance tests; `--no-default-features` compiles empty. The Apps protocol versions independently (`2026-01-26`); the crate's `v2026_07_28` module names the CORE lane it pairs with.

- **2026-06-12 (normative body rewritten to the spec-neutral strategy)** — §1 (naming + table), the suffix rationale, §2 layout, §5 breaking-change rule, Consequences, Migration step 2, and Implementation order now state the unsuffixed `turul-mcp-ext-<name>` convention with feature-gated lane modules as normative — the 2026-06-07 amendment had superseded the suffixed convention but only in this log, leaving the body contradicting the implemented crates (ADR drift). The original suffixed-name decision survives only in the dated entries above.

- **2026-06-12 (server dispatcher API designed; ext-tasks runtime slice)** — The §Open-items dispatcher question is resolved as a per-extension builder surface rather than a generic `register_extension(...)`: **(1) Store in the extension crate** — `turul-mcp-ext-tasks::v2026_07_28::store` hosts `TaskState`, the `TaskStore` trait (async-trait, no tokio in the public API — same discipline as `turul-mcp-task-storage`), and `InMemoryTaskStore`; durable backends implement the trait later. **(2) Runtime/worker in `turul-mcp-server`** behind a new `ext-tasks` feature (off by default per SEP-2133's disabled-by-default rule): `.with_ext_tasks(store)` advertises `io.modelcontextprotocol/tasks` in `ServerCapabilities.extensions` and registers `tasks/get`/`tasks/update`/`tasks/cancel` handlers; `.ext_task_tool(tool)` / `.ext_task_tool_required(tool)` mark tools for task election. **(3) Election semantics per the upstream overview**: client declared the extension in per-request `_meta` `clientCapabilities.extensions` → create the task durably, spawn the worker, answer `CreateTaskResult`; not declared → run synchronously (progressive enhancement) unless the tool is `_required`, which answers `-32003` with `data.requiredCapabilities.extensions` exactly as the upstream example shows. **(4) MRTR bridge**: a task-augmented tool returning `McpError::InputRequired` parks the task in `input_required` (the requests persisted to the store); `tasks/update` validates the response keys, and when all outstanding requests are answered the worker resumes with the responses injected through the SAME session-extension keys the synchronous MRTR retry leg uses — tool code is identical under both execution models. **(5) Task IDs are UUIDv4** (not the house v7): upstream §Security says IDs may serve as bearer tokens and must resist enumeration; v4's 122 random bits beat v7's timestamp-prefixed layout for unguessability. **(6) `notifications/tasks` over `subscriptions/listen`**: the transport honors a `taskIds` filter field iff the server's advertised `capabilities.extensions` contains the tasks identifier — keyed off the capability map, so `turul-http-mcp-server` needs no dependency on the extension crate. Worker-side emission is best-effort via the request-captured broadcaster ("Servers may also push status updates").
