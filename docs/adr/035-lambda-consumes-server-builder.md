# ADR-035: Lambda consumes `McpServerBuilder` — one owner of handlers, capabilities and extensions

**Status:** Proposed
**Date:** 2026-11-14
**Crates:** `turul-mcp-server` (owner), `turul-mcp-aws-lambda` (transport only)
**Related:** ADR-004 (component extension), ADR-001 (Lambda integration), ADR-019 (Lambda task integration, 2025 lane), ADR-028 (extensions), ADR-033 (Skills)

Stays **Proposed** until the parallel registration code in `turul-mcp-aws-lambda` is deleted and
the gates in §Tests pass on the merged tree.

## Context

`LambdaMcpServerBuilder` (`crates/turul-mcp-aws-lambda/src/builder.rs`) is a parallel copy of
`McpServerBuilder`, not an extension of it (drift "D-L"), contrary to ADR-004. On main after
v0.4.22:

- 52 of about 65 public builder methods repeat `McpServerBuilder`, in 1,333 non-test lines;
  `LambdaMcpServer::handler()` (`src/server.rs:359-437`) re-assembles the dispatcher that
  `build_configured_http_builder` (`turul-mcp-server/src/server.rs:396`) already builds. The
  crate's own `tests/cross_builder_method_parity.rs` calls them "independent code paths".
- The copies have drifted:
  - `resources/read` is built `without_security()` at three sites (`builder.rs:188`, `:647`,
    `:1210`), so Lambda enforces no read policy;
  - `resource()` (`:404-428`) does no URI validation and silently overwrites duplicates;
  - `subscribe` is advertised `false` (`:1104`) where the server advertises `true`;
  - there is no `extension_capability` and no `ext_*` method.

**What breaks today:** a Lambda server cannot advertise or serve any extension
(`io.modelcontextprotocol/skills`, `/ui`, `/tasks`) through its builder, and its resource reads
are unchecked.

## Decision

### 1. One owner

`McpServerBuilder` is the only place handlers, capabilities, extension wiring, the
resource-security policy and build-time validation are defined. `turul-mcp-aws-lambda` owns
only transport: the Lambda adapter, CORS, response streaming, authorizer context and Lambda
storage defaults.

### 2. Thin wrapper, same public names

`LambdaMcpServerBuilder` keeps its public method names and signatures (downstream Lambda
servers build on it) and holds an inner `McpServerBuilder`. Every registration or capability
method delegates to it; no Lambda method re-implements one. `build()` consumes the core build
through one owner-side seam on `McpServer` (today `McpServer::new` is `pub(crate)`), so the
dispatcher and capabilities Lambda serves are the ones the HTTP server would serve for the
same configuration. Methods that exist only on the server (`extension_capability`,
`with_ext_skills`, the Tasks methods, `description`, `website_url`, …) become available on
Lambda as delegations. The parallel handler table, the three `ResourcesReadHandler` sites
and Lambda's own capability assembly are deleted in the same slice.

Feature passthrough: `turul-mcp-aws-lambda` features `ext-skills`, `ext-apps` (none needed:
Apps rides generic hooks) and `ext-tasks` map to the server's.

### 3. Behaviour changes on Lambda (patch, stated in the CHANGELOG)

- `resources/read` enforces the server's auto policy (ADR-007; blocklist `..`, `/etc/`,
  `/proc/`, derived allow patterns, size limit).
- Invalid resource URIs fail `build()` instead of being accepted.
- Capabilities equal the server's for the same configuration, except where Lambda cannot
  serve a feature truthfully: `resources.subscribe` stays `false` (no long-lived
  `subscriptions/listen` on buffered Lambda).

Removing or changing a `pub` item is formally semver-breaking; it ships as a patch under the
0.4 rule.

### 4. Skills and Apps

Skills (ADR-033) reach Lambda through `with_ext_skills` with no Lambda skills code. The
catalog is built once per container (at cold start), never per invocation. Version skew: during
a rollout `skills/list` and `resources/read` can be answered by different deployments, so a
host's digest check fails until the rollout completes — the same as behind any load balancer;
hosts recover by re-calling `skills/get` (ADR-033 §5).

### 5. Tasks on Lambda: deferred

The durable store exists: `turul-mcp-ext-tasks` feature `dynamodb`, with native TTL for
retention. What does not survive Lambda is execution: the worker is a `tokio::spawn` after the
response (`turul-mcp-server/src/ext_tasks.rs:190`), cancellation is an in-process abort map
(`:192-195`), and the retention sweep is a ticker (`builder.rs`). All freeze when the
invocation returns.

Options:

| Option | Mechanism | New infrastructure |
|---|---|---|
| T0 (ADR-019 parity) | The 2025-11-25 lane's accepted Lambda model, applied to the extension: durable `ext-tasks` store (DynamoDB, TTL retention), in-process worker that completes reliably only within the invocation, long-running work driven by an external system writing to the store, advisory cancellation | None |
| T1 | The invocation that creates a task persists its input in the `TaskStore`, then invokes its own function asynchronously (`InvocationType: Event`) with the task id; that invocation runs the worker to completion. Cancellation becomes a store state the worker checks between steps. Retention is DynamoDB TTL, not a ticker. | An IAM grant: `lambda:InvokeFunction` on the function's own ARN |
| T2 | Same, through an SQS queue the function also consumes | A queue, its event-source mapping and IAM |
| T3 | Keep the invocation alive until the worker finishes (response streaming) | None, but it defeats task semantics: the caller waits |

**Decision (2026-11-14): deferred by the owner.** This slice does not wire Tasks on Lambda,
and Lambda MUST NOT advertise `io.modelcontextprotocol/tasks`; a test asserts its absence.
Storage is not the gap. When Tasks on Lambda is picked up, T0 is the precedent: it needs no
new infrastructure. T1 and T2 each need the owner's yes on their infrastructure.

### 6. Owner-side seam

- `McpServer::transport_parts()` returns `TransportParts`: the dispatcher built by the one
  registration function `McpServer::mcp_dispatcher()` (also used by `run_http`), the
  capabilities, identity, session manager and storage, middleware, routes, tool fingerprint
  and tool-change notifier.
- `HttpMcpServerBuilder::register_dispatcher` lets the HTTP transport consume the same
  dispatcher.
- `McpServerBuilder::resource_subscriptions(bool)` is how a transport that cannot serve
  `subscriptions/listen` keeps `resources.subscribe` truthful.
- `McpServerBuilder::with_session_storage` takes `Arc<BoxedSessionStorage>`, so a
  type-erased store can be passed through; `Arc<Concrete>` arguments still coerce.
- `root()` and `with_roots()` are 2025-11-25-only on Lambda, as on the server: on
  2026-07-28 `roots/list` is not a client-to-server method.
- Lambda's session-timeout methods now take effect. They were stored but never read, so every Lambda server ran on a fixed 30 min / 60 s; `with_short_sessions()` uses the server's 15-minute preset.

## Tests (executable contract)

In `crates/turul-mcp-aws-lambda/tests/single_owner_2026.rs` unless noted:

1. `lambda_discover_matches_server`: for the same configuration (a resource, a vendor
   extension), Lambda's `server/discover` capabilities equal `McpServer`'s, except
   `resources.subscribe`. Method-set parity stays in `cross_builder_method_parity.rs`.
2. `lambda_read_enforces_server_policy` and `lambda_invalid_resource_uri_fails_build`.
3. `lambda_serves_skills_through_the_owner` and
   `lambda_q10_blocklisted_skill_file_refused_at_build` (feature `ext-skills`).
4. `lambda_does_not_advertise_tasks`.
5. `no_parallel_registration`: no `ResourcesReadHandler::new`, `ResourcesListHandler::new`,
   `ListToolsHandler::new`, `DiscoverHandler::new`, `SessionAwareToolHandler::new`,
   `PromptsListHandler::new` or `without_security` in `turul-mcp-aws-lambda/src`.
6. Live: `scripts/e2e-lambda-skills.sh` boots `lambda-skills-server` under
   `cargo lambda watch` and runs `scripts/ext-skills-conformance.sh` against it
   (`ci-gates.sh lambda`).
7. Downstream Lambda servers built on `LambdaMcpServerBuilder` build and smoke-test before release.

The builder unit tests that inspected Lambda's own resource maps now assert the same
outcomes through the built handler (`resources/list`, `resources/templates/list`).

## Consequences

- About 1,000 Lambda lines go; every future server feature reaches Lambda once.
- Existing Lambda servers may see refused reads or build errors they did not see before.
- Tasks on Lambda is deferred (§5); T0 needs no new infrastructure when it is picked up.

## Revision log

- **2026-11-14 (implemented)**: thin wrapper and seam landed; Lambda builder 2,641 → 2,120 lines (non-test 1,333 → 794) with no registration logic left, `server.rs` 529 → 208. Contract 7/7; Skills live on `cargo lambda watch`. Stays Proposed until merged and downstream Lambda servers pass their smoke test.
- **2026-11-14**: Proposed. Three independent design reviews rejected patching the copy
  (entrenches D-L) and patching it before the refactor (a compatibility copy).
