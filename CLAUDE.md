# CLAUDE.md

1. Don't assume. Don't hide confusion. Surface tradeoffs.
2. Minimum code that solves the problem. Nothing speculative.
3. Touch only what you must. Clean up only your own mess.
4. Define success criteria. Loop until verified.

Production-ready Rust framework for Model Context Protocol (MCP) servers with zero-configuration design and MCP 2026-07-28 specification support (2025-11-25 available as an opt-in build).

> **Source of Truth**
> - **The MCP specification + vendored schema** — what the wire must be
> - **docs/adr/** — architectural decisions already taken
> - **The code** — what is actually true today
> - **AGENTS.md** — repo policy, compliance rules, full architecture
> - **CLAUDE.md** — concise operator playbook (this file)
>
> Between the two playbooks, AGENTS.md wins. But both are prose *about* the system,
> and prose generalises badly: where either contradicts the spec, an ADR, or working
> code, **the playbook is what is wrong** — correct the wording rather than the
> system. Cite the schema type, ADR number or file when invoking this; a preference
> is not a contradiction.

## Branch Lock: `feat/turul-mcp-protocol-2026-07-28`

**This branch is the 0.4 release in preparation**, adopting MCP 2026-07-28 — now the
released current specification (stateless core, `initialize`/`Mcp-Session-Id` removed,
Tasks moved to extension, error code `-32002` → `-32602`, JSON Schema 2020-12, MCP Apps,
caching headers, RFC 9207 auth, deprecations of Roots/Sampling/Logging).
See https://modelcontextprotocol.io/specification/2026-07-28.

**0.4 becomes the current release only when the maintainer opens the PR and merges it.**
Until then this branch is pre-release: it holds the 0.4 line, `main` holds 0.3 /
2025-11-25, and neither fact licenses merging. Write about this branch's contents as
**0.4** — every non-frozen crate is already `0.4.0`, so "0.3" in a *current-state*
claim is stale (see [docs/rules/crate-versioning.md](docs/rules/crate-versioning.md)
for what legitimately stays 0.3).

Confirm the name before relying on the rules below — they bind whatever branch holds
the 2026-07-28 work, and a stale name here would make them unenforceable as written:

```bash
git branch --show-current      # expect feat/turul-mcp-protocol-2026-07-28
```

- **DO NOT merge `feat/turul-mcp-protocol-2026-07-28` into `main` without the maintainer's express authority.**
- **DO NOT fast-forward, rebase-onto-main, squash-to-main, or open a merge PR for this branch** unless the maintainer (Nick) has explicitly authorized that specific action in the current session.
- **DO NOT delete the branch, force-push it, or treat it as "complete"** without express authority. "Tests pass" / "all SEPs implemented" is not sufficient — final disposition is the maintainer's call.
- All work for the 2026-07-28 spec lands on this branch. `main` continues to hold 2025-11-25 — now the *previous* spec, not the current one — until the maintainer chooses to cut over.

### Check the schema pins BEFORE any 2026-07-28 work

**2026-07-28 has finalized.** The released schema lives at the immutable upstream path
`schema/2026-07-28/schema.ts`; upstream `schema/draft/` is now the *next* spec cycle's
floating pointer and is no longer what this crate tracks. Anything still resolving against
`schema/draft/` or against `main` will silently drift onto next-cycle content. Verify the pin
still names the released artifact — before writing code, and before trusting a green suite:

```bash
# 1. Which commit last changed the fixtures, and does the harness still pass there?
cargo run -p turul-mcp-protocol-2026-07-28 --bin mcp-compliance-2026-07-28 \
    --features compliance -- refresh          # dry-run; --write only once green

# 2. Has the vendored schema itself drifted from its pinned commit?
shasum -a 256 crates/turul-mcp-protocol-2026-07-28/schema/schema.ts
#    compare against the Content sha256 in schema/README.md, then diff that
#    commit against the released tag 2026-07-28 — never against upstream main
```

Governing rules — including the one-immutable-commit requirement and the
`modeled=N` caveat — live in **AGENTS.md §Branch Lock → "Schema pin governance"**,
which wins on conflict. Do not restate them here; this section is the runnable
check only.

## Rules

Each rule lives in its own file under [docs/rules/](docs/rules/README.md) — one topic
per file, so a rule can be linked, cited by a reviewer, and updated without touching
the others. The precedence in §Source of Truth above applies to every one of them:
spec > ADR > code > rule text.

| Rule | Governs |
|---|---|
| [protocol-crate-purity.md](docs/rules/protocol-crate-purity.md) | What may live in `turul-mcp-protocol*` crates; frozen 2025-* crates |
| [protocol-reexport.md](docs/rules/protocol-reexport.md) | Always import via `turul-mcp-protocol`, never a versioned crate directly |
| [spec-version-naming.md](docs/rules/spec-version-naming.md) | Full `YYYY-MM-DD` spec dates, never a bare year |
| [zero-configuration-design.md](docs/rules/zero-configuration-design.md) | No method strings — framework derives them from types |
| [crate-versioning.md](docs/rules/crate-versioning.md) | Per-crate `version =`, what's stale vs. legitimately still 0.3, workspace deps |
| [comments.md](docs/rules/comments.md) | What a source comment may say; forbidden tags/citations; slice completion gate |
| [test-coverage-discipline.md](docs/rules/test-coverage-discipline.md) | Pre-publish test gate; what makes a check meaningless; reviewer-agent briefing |
| [notification-architecture.md](docs/rules/notification-architecture.md) | SessionManager as sole event bus; wire-complete notification envelopes; handler error rules |
| [wire-format-compliance.md](docs/rules/wire-format-compliance.md) | Streamable HTTP headers/status codes, camelCase JSON, structuredContent, 2025-11-25 opt-in lane |
| [scope-discipline.md](docs/rules/scope-discipline.md) | Minimal fixes only; stay inside the approved plan; core-crate change checklist |

Before spawning a reviewer agent (Explore, Plan, code-reviewer, devils-advocate,
etc.), point it at `AGENTS.md`, this file, and the relevant ADRs — see
[test-coverage-discipline.md § Briefing reviewer agents](docs/rules/test-coverage-discipline.md#briefing-reviewer-agents)
for why and exactly what to say.

## Quick Reference

### Tool Creation (macro-first)
```rust
// Recommended: Function macro
#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(a: f64, b: f64) -> McpResult<f64> { Ok(a + b) }

// Alternative: Derive macro
#[derive(McpTool)]
#[tool(name = "calc", description = "Calculate", output = CalcResult)]
struct CalcTool { a: f64, b: f64 }

// Runtime: Builder
let tool = ToolBuilder::new("calc").execute(|args| async { /*...*/ }).build()?;

// Manual trait implementation: reference-only — see examples/calculator-add-manual-server
// Manual impls MUST include: impl HasExecution for MyTool {}
```

### Task Support (per-tool) — **2025-11-25 lane only**

> **This section does not apply to the default build.** `TaskSupport` and
> `ToolExecution` exist only in `turul-mcp-protocol-2025-11-25` (0 occurrences in
> `turul-mcp-protocol-2026-07-28/src/`), and `.with_task_storage()` is
> `#[cfg(feature = "protocol-2025-11-25")]` (`builder.rs:1412`). The derive macros
> emit the attribute's code unconditionally, so a `task_support = "..."` tool on
> the 2026-07-28 default lane fails to compile with `cannot find TaskSupport in
> tools` / `cannot find type ToolExecution`. Removing the attribute clears both.
>
> On 2026-07-28, tasks moved to the **extension** `io.modelcontextprotocol/tasks`
> (SEP-2663) — see `crates/turul-mcp-ext-tasks` and `examples/ext-tasks-server`,
> and note SEP-2133 keeps it off by default. `tasks/list` was removed.

Tools can declare `task_support` to enable the "Run as Task" button in MCP Inspector:

```rust
// Function macro
#[mcp_tool(name = "slow_add", description = "Add with delay", task_support = "optional")]
async fn slow_add(a: f64, b: f64) -> McpResult<f64> { Ok(a + b) }

// Derive macro
#[derive(McpTool)]
#[tool(name = "slow_calc", description = "Slow calc", task_support = "optional")]
struct SlowCalcTool { a: f64 }
```

**Values**: `"optional"` (sync or async), `"required"` (must run as task), `"forbidden"` (never as task). Omit for no task support.

**Server requirement**: The server must have a task runtime configured (`.with_task_storage()`) for tools with task support. `task_support = "required"` without a runtime causes a build-time error.

**Manual impls**: Override `HasExecution::execution()` to return `Some(ToolExecution { task_support: Some(TaskSupport::Optional) })`.

**Capability truthfulness**: When no task runtime is configured, the server strips `execution` from `tools/list` responses and rejects task-augmented `tools/call` requests.

### Tool Annotations (per-tool)

MCP 2025-11-25 behavior hints. All attributes are optional — omit for `None`.

```rust
// Function macro
#[mcp_tool(name = "search", description = "Search the web",
           title = "Web Search", read_only = true, open_world = true)]
async fn search(query: String) -> McpResult<String> { Ok(query) }

// Derive macro
#[derive(McpTool)]
#[tool(name = "delete_file", description = "Delete a file",
       title = "File Deleter", read_only = false, destructive = true,
       idempotent = true, open_world = false)]
struct DeleteFileTool { path: String }

// Builder
let tool = ToolBuilder::new("delete")
    .annotations(ToolAnnotations::new()
        .with_read_only_hint(false)
        .with_destructive_hint(true))
    .build()?;
```

**Attributes**: `title` (→ `Tool.title`), `annotation_title` (→ `ToolAnnotations.title`, rare), `read_only` (→ `readOnlyHint`), `destructive` (→ `destructiveHint`), `idempotent` (→ `idempotentHint`), `open_world` (→ `openWorldHint`).

**Not `Annotations`**: Tool annotations (`ToolAnnotations`) are separate from resource/prompt `Annotations` (`audience`/`priority`).

### Output Types and Schemas

**IMPORTANT**: Tools with custom output types (including Vec<T>) MUST specify the `output` attribute:

```rust
#[derive(McpTool)]
#[tool(name = "search", description = "Search", output = Vec<SearchResult>)]
struct SearchTool { query: String }
// Without output attribute, schema shows tool inputs not output type!
```

**Why Required**: Derive macros cannot inspect the `execute` method's return type at compile time. The `output` attribute tells the macro what schema to generate.

**Schemars (automatic detection):**
If the output type derives `schemars::JsonSchema`, the framework automatically uses it for detailed schema generation — no additional schemars flag is needed. The `output = Type` attribute is still required on derive macros:
```rust
#[derive(schemars::JsonSchema, serde::Serialize)]
struct MyOutput { value: f64 }

#[derive(McpTool)]
#[tool(name = "calc", description = "...", output = MyOutput)]  // output = required
struct MyTool { a: f64 }

#[mcp_tool(name = "add", description = "Add numbers")]
async fn add(a: f64) -> McpResult<MyOutput> { Ok(MyOutput { value: a }) }  // auto-detected from return type
```

For manual `HasOutputSchema` implementation, see `examples/calculator-add-manual-server`.

### API Conventions
- **SessionContext**: Use `get_typed_state(key).await` and `set_typed_state(key, value).await?`
- **Builder Pattern**: `McpServer::builder()` not `McpServerBuilder::new()`
- **Error Handling**: Always use `McpError` types - NEVER create JsonRpcError directly in handlers
- **Session IDs**: Always `Uuid::now_v7().as_simple()` for temporal ordering (no-hyphen hex)

### Basic Server
```rust
use turul_mcp_server::prelude::*;

let server = McpServer::builder()
    .name("my-server")
    .tool(Calculator::default())
    .build()?;

server.run().await
```

### The two spec lanes cannot be built together

`protocol-2025-11-25` and `protocol-2026-07-28` are mutually exclusive features on
`turul-mcp-protocol`, so **`--workspace` and any `-p` list mixing lanes fail**:

```
error: turul-mcp-protocol: features `protocol-2025-11-25` and
       `protocol-2026-07-28` are mutually exclusive — a build re-exports
       exactly one MCP spec. Enable one.
```

That is the mutex working, not a broken tree. Split the `-p` list by lane. Give each
lane its own `CARGO_TARGET_DIR` (e.g. `target-2025`) or every switch triggers a full
rebuild. `scripts/ci-gates.sh` already separates them: `default` = 2026-07-28,
`opt-in-2025` = 2025-11-25, `mutex` proves they still refuse to co-compile.

Runnable per-lane commands and the client × server matrix live in
[`docs/manual-e2e-matrix.md`](docs/manual-e2e-matrix.md).

### Development Commands
```bash
cargo build     # 2026-07-28 default lane only — see the mutex above
cargo test
cargo run -p minimal-server

# Specific test suites
cargo test -p turul-mcp-server --features dynamic-tools     # Dynamic tools + registry tests
cargo test -p turul-mcp-framework-integration-tests --test event_dispatcher_persistence  # Notification persistence
cargo test -p turul-mcp-framework-integration-tests --test compliance  # MCP spec compliance

# MCP Testing
cargo run -p client-initialise-server -- --port 52935
cargo run -p client-initialise-report -- --url http://127.0.0.1:52935/mcp
```

### Debugging: Stale Build Issues
If behavior doesn't match code changes:
```bash
cargo clean  # Full workspace clean required for cross-crate changes
cargo test -p turul-mcp-framework-integration-tests --test e2e_tests
```

**Why**: Incremental compilation caches string literals/errors across crates.

## Architecture

Full crate list lives in `AGENTS.md` §Architecture Overview — don't duplicate it here.

### Session Management
- UUID v7 sessions with automatic cleanup
- Streamable HTTP with SSE notifications
- Pluggable storage (InMemory, SQLite, PostgreSQL, DynamoDB)

### HTTP Transport Routing
- **Protocol >= 2025-03-26**: `StreamableHttpHandler` — serves both 2026-07-28 (stateless, `server/discover`) and 2025-11-25 (session handshake); which one is compiled in is the feature mutex's decision, not a runtime branch
- **Protocol <= 2024-11-05**: `SessionMcpHandler` (buffered JSON, legacy compatibility)

Routing in `crates/turul-http-mcp-server/src/server.rs`

## Generally Safe Dev Commands

The following are considered safe for automatic execution during development:
- `cargo build/check/test/run/clippy/fmt/clean/doc/bench/metadata/expand` — including with `--package`, `--test`, `--bin`, `--example` flags, environment variables (`RUST_LOG`, `RUST_BACKTRACE`, `CI_SANDBOX`), and `timeout` wrappers
- `cd <dir> && cargo <command>` — including `cd examples/<name> && cargo run`
- `curl`, `jq` — HTTP testing and JSON parsing (all variations auto-approved)
- `timeout`, `pkill`, `killall` — process management for testing
- `git add` — staging changes (commit only when user explicitly requests)
- `rustc`, `sed`, `grep`, `find`, `awk`, `cat`, `tee`, `echo` — standard dev tools
- Background processes (`&`, `wait`, `jobs`)
- Shell control flow (`while`, `for`, `if`)

These commands do not require interactive permission prompts. Use normal judgment about context and timing.

### Commands requiring explicit user approval:
```bash
git checkout      # Discards uncommitted work
git restore       # Discards uncommitted work
git reset --hard  # Irreversible reset
git clean -f      # Deletes untracked files
cargo publish     # Pushes to crates.io (irreversible)
git commit        # Only when user explicitly requests a commit
```
**These commands destroy uncommitted work and are IRREVERSIBLE. Always ask the user first.**

### Commit Message Style
- **No `Co-Authored-By` attribution** — omit Claude/AI co-author trailers
- **Succinct** — one-line summary, optional body only if non-obvious

## Pre-Release Checklist

Before publishing a new version:

1. **Crate versions**: Bump the literal `version = "X.Y.Z"` in each changed crate's `Cargo.toml` AND its pin in `[workspace.dependencies]`. Per [docs/rules/crate-versioning.md](docs/rules/crate-versioning.md), `[workspace.package].version` is *not* authoritative — updating only it changes nothing that ships.
2. **Example server versions**: Update `.version("x.y.z")` strings in `examples/*/src/main.rs`
3. **Plugin skill versions**: Skills use the generic minor version (`v0.4`, not
   `v0.4.1`) — do NOT bump on patch releases, only when the minor changes. Bump
   *current-state* references only; see [docs/rules/crate-versioning.md § Version References](docs/rules/crate-versioning.md#version-references-what-is-stale-and-what-is-not) —
   since-markers and changelog entries stay put.
4. **CHANGELOG.md**: Add release entry with date and comparison links
5. **Stale version scan**: `grep -rn 'v0\.[0-9]\.[0-9]' plugins/ examples/ .claude/` — fix any outdated references
6. **Publish order** (dependency-first, derived from the actual non-dev dependency graph):
   ```
   protocol-2026-07-28 → protocol → session-storage → http-server → builders → derive* →
   ext-tasks → oauth → schema-validation → server-state-storage → task-storage →
   server → aws-lambda → client → ext-apps
   ```
   *`turul-mcp-derive` has circular dev-deps on `turul-mcp-server` — temporarily comment out dev-deps, publish with `--allow-dirty`, restore*

   External sibling crates are **not** in this sequence — they publish from their
   own repos and must already be on crates.io: `turul-rpc` (0.2) and
   `turul-jwt-validator` (>= 0.3.2, required by `turul-mcp-oauth` — the
   `rust_crypto` feature does not exist before 0.3.2).

   Frozen `turul-mcp-json-rpc-server`, `turul-mcp-protocol-2025-06-18` and
   `turul-mcp-protocol-2025-11-25` stay published at `0.3.47` — no republish step.
   `turul-mcp-framework-integration-tests` is `publish = false`.

   Regenerate this list rather than hand-editing it — `turul-mcp-server` depends on
   `turul-mcp-oauth` non-optionally, so an order that publishes the server first fails:
   ```bash
   cargo metadata --format-version 1 --no-deps   # then topo-sort on kind == null deps
   ```
7. **Git tag**: `git tag v0.x.y && git push origin v0.x.y`
