# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - Unreleased (feature branch `feat/turul-mcp-protocol-2026-07-28`)

> **Release status.** This entry tracks the in-progress 0.4.0 cut on the
> `2026-07-28-MCP-Specification` branch (and its current sub-branch
> `feat/turul-mcp-protocol-2026-07-28`). The workspace `[workspace.package].version`
> was already bumped to `0.4.0` in commit `064733e` (with the turul-rpc isolation
> fix in `c0737fb`). **The branch has not been merged to `main` and 0.4.0 has not
> been published.** Per the branch lock, that requires explicit maintainer
> authorization. The footer compare-link will be added at the release tag.
> `main` continues to ship at the 0.3.x line (currently `0.3.47`).

### Added

- **Tasks-extension server runtime (2026-06-12, SEP-2663 — closes driver gap G1).** New opt-in `ext-tasks` feature on `turul-mcp-server` (off by default per SEP-2133): `.with_ext_tasks(store)` advertises `io.modelcontextprotocol/tasks` in `server/discover`'s `capabilities.extensions` and registers `tasks/get`/`tasks/update`/`tasks/cancel`; `.ext_task_tool(tool)` / `.ext_task_tool_required(tool)` mark tools for task election — a request whose per-request `_meta` `clientCapabilities.extensions` declares the extension gets a durable `CreateTaskResult` (UUIDv4 bearer-grade id, store written BEFORE the response) with a spawned worker; undeclared requests run synchronously (progressive enhancement) or, for `_required` tools, get `-32003` with the upstream `data.requiredCapabilities.extensions` shape. **MRTR bridge**: a task tool returning `McpError::InputRequired` parks its task in `input_required`; `tasks/update` validates response keys against outstanding requests (partial delivery keeps it parked) and resumes the worker with the responses injected through the same session-extension keys as the sync retry leg — tool code is identical under both execution models. `tasks/cancel` is cooperative (aborts the worker, drops input waiters, acks terminal tasks). `notifications/tasks` rides `subscriptions/listen`: the transport honors a `taskIds` filter iff the extension is advertised (keyed off the capability map — no transport dependency on the ext crate), echoes it in the ack, and delivers per-taskId. `turul-mcp-ext-tasks` gains the `TaskStore` trait + `TaskState` + `InMemoryTaskStore` (no tokio in the public API). 9 wire e2e tests (`ext_tasks_2026.rs`, wired into gates + CI); revert-and-fail recorded. Dispatcher design recorded in ADR-028 (2026-06-12 entry).
- **`turul-mcp-ext-apps` 0.1.0 scaffold (2026-06-12, SEP-1865).** Spec-neutral extension crate binding the MCP-side Apps surface: extension identifier `io.modelcontextprotocol/ui` (the ADR-028 table's `/apps` guess corrected against upstream), client capability (`UiClientCapabilities.mimeTypes` + the `text/html;profile=mcp-app` HTML-views gate), tool `_meta.ui` (`UiToolMeta`: `resourceUri`, `visibility` model/app), and UI-resource `_meta.ui` (`UiResourceMeta`: CSP domain lists, sandbox permissions, dedicated origin, `prefersBorder`). The host↔view iframe protocol is deliberately not bound (app/host SDK scope). Vendored spec pinned at `modelcontextprotocol/ext-apps@ca1d2989`; 5 wire-shape compliance tests.
- **Versioning/cancellation/elicitation P2 trio (2026-06-12).** (1) **VER-4**: the headerless-`initialize` rejection (400 + `-32001`) now carries `error.data.supported` naming this build's protocol versions — a true legacy client's only diagnostic; wire test `headerless_initialize_rejection_names_supported_versions`, red-phase recorded. (2) **PAT/G10**: dedicated `CancelledNotificationHandler` extracts `requestId` + `reason` from inbound `notifications/cancelled` into a structured log line ("Both parties SHOULD log cancellation reasons"); accepted-and-ignored semantics unchanged. (3) **CF/GAP-CF-8**: new `turul_mcp_builders::validate_elicit_content(schema, content)` validates elicited form content against the requesting schema (required/unknown keys, primitive types, string-length/numeric bounds, integer-ness, enum membership across the 2026 enum-union shapes; format assertions annotation-only by design) — central enforcement is impossible on the stateless lane (leg-1 schema not retained), so tools call it on the MRTR retry; wired into `mrtr-elicitation-server` and live-verified. Plus **BP-5** (COMPLIANCE.md §"Supported JSON Schema dialects" — the documentation the SHOULD asks for) and **UTIL/COMP-3** (relevance/fuzzy/rate-limit completion SHOULDs dispositioned: provider semantics + middleware rate limiting). Driver summary now 305 ✅ / 68 🟡 / 5 ❌ / 12 🧪 / 100 ➖ — the 5 remaining ❌ all carry recorded dispositions.
- **`turul-mcp-ext-tasks` 0.1.0 scaffold (2026-06-12, SEP-2663).** New spec-neutral extension crate per ADR-028 (2026-06-07 amendment): the `v2026_07_28` module carries the redesigned Tasks-extension surface — status-tagged `DetailedTask` (working/input_required/completed/failed/cancelled with variant fields inlined), `CreateTaskResult` (`resultType: "task"`, flat `Result & Task`), `tasks/get`/`tasks/update`/`tasks/cancel` bindings, `notifications/tasks`, `taskIds` subscription-filter fields, and capability negotiation helpers including SEP-2133 identifier validation. Upstream schema vendored from `modelcontextprotocol/ext-tasks@8966bea9` with a provenance README; 13 wire-shape compliance tests (explicit-null `ttlMs`, snake_case status strings, flat task discriminator). `protocol-2026-07-28` is the default feature; `--no-default-features` compiles empty. Server dispatch wiring and the 2025-11-25 reconciliation module are tracked as separate slices (ADR-028 revision log 2026-06-12). Partially closes driver gap **G1** (SEP-2663 row stays 🟡 until dispatch lands).
- **Driver-doc re-grade pass (2026-06-12, docs).** All 123 then-non-green rows of `docs/plans/2026-07-28-spec-compliance.md` were verified against post-P2-batch HEAD by an 11-agent sweep with spot-checked claims: 35 rows had been fixed by the P2 batches without being re-graded (now ✅ with **RE-GRADED 2026-06-12** citations — e.g. client MRTR retry triple, invalid-cursor -32602, initialize-names-supported-versions, conditional `completion/complete` registration), 17 improved to/confirmed 🟡, 2 implementation-only claims were demoted back to 🧪 during review, 3 got refreshed evidence pointers. Summary corrected to the true row count (490) and re-tallied: 302 ✅ / 66 🟡 / 10 ❌ / 12 🧪 / 100 ➖.

- **Server wire-edges P2 batch — the FINAL open driver gaps (2026-06-11).** All 73 audit gaps are now closed (52 fixed, 6 dispositioned with recorded rationale). Behavior: null request ids → 400 + -32600 pre-dispatch (MCP forbids them; turul-rpc's base-JSON-RPC Null variant stays); invalid pagination cursors → -32602 at all five list sites; `completion/complete` no longer a default handler (unconfigured server → 404 + -32601); blob resource contents validated as base64 before shipping; prompts/list carries title/icons/_meta; the initialize rejection names supported versions in error.data; `X-Accel-Buffering: no` on streaming responses; tool-name format warnings at registration; Mcp-Param message whitespace runs collapsed; `notify_elicitation_complete` + `notify_request_progress_with_message` session helpers; `PromptAnnotations` moved protocol→builders (no schema counterpart — purity). Tests: `wire_edges_2026.rs` (10), numeric Mcp-Param compare, roots/sampling -32003 arms, SEP-2577 marker tripwire (reverting Slice A'' now fails CI). Dispositions: schema-dialect validation (documented limitation), progress rate-limiting (middleware layer per ADR-012), sampling message-shape constraints (deprecated surface), tool-Err-vs-isError (deliberate AGENTS.md-documented contract: `Err` = protocol error, `CallToolResult::error` = model-visible). EXAMPLES_PIN capture date corrected. Closes **CHG/G4, CHG/G6, DEP-GAP-3, BP-2/3/4, VER-2, PAT/G5/G9, TX/GAP-3/4/5, CF/GAP-CF-6/7/9, DISC-4, PRM-2026-01/04/05, RES-G3/G6/G7, TOOLS-G3/G4/G6/G7, UTIL/COMP-2, UTIL/PAG-1/2, UTIL/LOG-2, SCHEMA/G2/G3/G5**.
- **Client capability/discovery P2 batch (2026-06-11).** (1) The `server/discover` body is now retained for the connection: `DiscoveredServer` (capabilities, instructions, serverInfo, supportedVersions) with `discovered_server()`/`server_capabilities()`/`server_instructions()` accessors. (2) `-32004` negotiation honors `error.data.supported`: fallback only when 2025-11-25 is mutually supported, otherwise the error names the server's list ("select a mutually supported version … or surface an error"). (3) Era detection no longer keys on one code: structured `-32602` also classifies as a legacy-server fallback signal per "commonly -32601 or -32602" (the prior -32602→abort unit pin migrated WITH the contract). (4) `DeclaredCapabilities` gains `elicitation_url`/`sampling_tools`/`sampling_context`, mapped into the spec's sub-capability shapes in every request `_meta`. (5) `call_tool` auto-recovers from `-32001` Mcp-Param rejections: one `tools/list` refresh + one retry per the SEP-2243 client-behavior note. (6) New `call_tool_with_progress(name, args, token, on_progress)`: SSE-framed request with `_meta.progressToken`, progress params delivered to the callback before the final result (real-server e2e). (7) `McpClientError::is_resource_not_found()` accepts `-32602` and the backwards-compat `-32002`. (8) First-page contract documented on the convenience list APIs (use `*_paginated` for full walks). structuredContent validation dispositioned (apps bring their own validator — no 2020-12 validator dependency for a SHOULD). Closes driver gaps **ARCH/GAP-ARCH-1, ARCH/GAP-ARCH-2, DISC-1, VER-3, CF/GAP-CF-5, TX/GAP-6, TX/GAP-7, RES-G4, UTIL/PAG-3, PAT/G4, TOOLS-G2**.
- **Subscriptions/cancellation P2 batch (2026-06-11, mostly tests).** New wire coverage: concurrent `subscriptions/listen` streams each receive exactly their filtered subset stamped with their own `subscriptionId`, and `notifications/message` never rides a listen stream (MUST NOT); dropping one subscription leaves siblings delivering; progress notifications stop at the final response (MUST); MRTR negative paths (neither-field `InputRequired` → server error; `InputRequired` escaping `completion/complete` → error, never `input_required`); unrecognized `logLevel` → `-32602`. Code: `notifications/cancelled` is now an explicitly registered notification on both lanes (202, never 404 — note: the 202 wire contract for true notifications pre-existed via the transport's fire-and-forget path; the registration adds sibling parity and the request-shaped consistency). Cancellation of in-flight work on Streamable HTTP remains the stream-close mechanism; cross-request correlation by id is impossible without sessions on the stateless lane, so inbound cancelled notifications are accepted and ignored per "Invalid cancellation notifications SHOULD be ignored". Server-shutdown stream teardown dispositioned (socket-close; no graceful-shutdown API exists). Closes driver gaps **PAT/G6, PAT/G7, PAT/G8, TOOLS-G5, UTIL/LOG-1, SCHEMA/G1**.
- **OAuth/security P2 batch (2026-06-11).** (1) *Malformed Authorization → 400*: a present-but-unparseable `Authorization` header (wrong scheme, empty/multi-token Bearer) now answers 400 + `error="invalid_request"` (RFC 6750 §3.1) instead of the missing-credentials 401 — `RequestContext::authorization_malformed` is set by both transports; wire-tested. (2) *Runtime scope enforcement*: `OAuthResourceMiddleware::with_required_scopes` rejects tokens missing a required scope with 403 + `error="insufficient_scope"` per Authorization §Insufficient Scope; unit-tested with minted HS256 tokens. (3) *offline_access guard*: `ProtectedResourceMetadata::with_scopes` filters `offline_access` with a warning (resource servers SHOULD NOT advertise it). (4) *Sessionless-ping auth*: on the 2025-11-25 lane, the `allow_unauthenticated_ping` bypass now runs AFTER the pre-session auth phase — it waives the session requirement only, matching its documented contract ("the full middleware stack still runs"); new 2025-lane wire test `tests/ping_auth_2025.rs` wired into the gates. (5) *Session-user binding (AUTH-7)*: dispositioned by design — claims stay request-scoped per ADR-021 D2; deployments needing binding implement it via middleware + session state; moot on the sessionless 2026 lane. Closes spec-compliance driver gaps **AUTH-2, AUTH-3, AUTH-5, AUTH-6 (fixed) + AUTH-7 (dispositioned)** — the OAuth/security P2 theme is closed.
- **Transport deprecation markers (2026-06-11, SEP-2596 + 2026 lane).** The client's `SseTransport` (HTTP+SSE, ≤ 2024-11-05) now carries `#[deprecated]` with migration notes in the crate docs and README — the transport is deprecated upstream (SEP-2596, 2025-03-26: "new implementations SHOULD NOT adopt it"); it remains functional for unmigrated servers. The server's legacy `session_handler` module documents the same. `ServerConfig.enable_get_sse` and the `get_sse()` builder setter are deprecated on the 2026-07-28 lane only (`cfg_attr`): the stateless endpoint is POST-only (GET = 405) and the long-lived stream is `subscriptions/listen`; stateful GET SSE remains first-class on the `protocol-2025-11-25` opt-in. Closes spec-compliance driver gap **DEP-GAP-1 (P2)**.
- **Client disconnect now cancels the in-flight request (2026-06-11).** Streamable HTTP §Cancellation: "Closing the SSE response stream MUST be treated by the server as cancellation of that request. The server SHOULD stop work … and MUST NOT send any further messages for it." The streaming dispatch task previously ran detached to completion after a disconnect; it now races the dispatch future against the response channel's `closed()` signal — on disconnect the future is dropped (the handler stops at its next await point), the progress task is shut down, and nothing further is sent. Wire test: a slow tool's completion flag stays unset when the client drops mid-execution (`cancellation_2026.rs`; control test pins the connected path). Closes spec-compliance driver gaps **PAT/G1 + TX/GAP-2 (both P1)** — the final open P1s.
- **Request-scoped progress on the 2026 path (2026-06-11).** Tools and resources can now emit spec-compliant `notifications/progress`: the request's `_meta.progressToken` is surfaced through the session extensions (`SessionContext::progress_token()`), and the new `notify_request_progress(progress, total)` references exactly that token — no-op when the request declared none ("Progress notifications MUST only reference tokens that were provided in an active request"). Numeric tokens now round-trip as JSON numbers end-to-end; the session→StreamManager bridge previously dropped non-string tokens (`as_str()`) and stringified the rest. Wire tests in `progress_2026.rs` (string echo, numeric round-trip, no-token-no-notifications); revert-and-fail recorded. Closes spec-compliance driver gap **PAT/G2 (P1)**.
- **Real-HTTP OAuth acceptance on the 2026 default transport (2026-06-11, tests + manifest).** New `crates/turul-mcp-server/tests/oauth_2026.rs`: missing/garbage bearers → 401 with the RFC 9728 `WWW-Authenticate` challenge (`resource_metadata=`, `error="invalid_token"`), 401 outranks the missing-`_meta` 400 (auth before validation), and both RFC 9728 well-known routes (root + path form) serve the metadata unauthenticated — all through Builder → `server.run()` → wire. To ride `turul-mcp-server`'s dev-deps without tripping the ADR-029 spec mutex, `turul-mcp-oauth` is now spec-neutral: its transport/storage deps drop default features and it gains its own `protocol-2025-11-25`/`protocol-2026-07-28` forwarding features (default 2026 standalone; unification supplies the spec when used with `default-features = false`). Closes spec-compliance driver gap **AUTH-1 (P1)**.
- **Regression nets for two MUST-level client behaviors (2026-06-11, tests only).** (1) `bilingual_client_falls_back_on_400_with_32004_body` pins the Versioning §Backward Compatibility wire path: HTTP 400 whose body carries structured `-32004` + `data.supported` → fall back to 2025-11-25 through the real probe (a bare 4xx still aborts). (2) `invalid_x_mcp_header_tools_are_excluded_from_tools_list` pins Tools §x-mcp-header: a tool definition with a constraint-violating `x-mcp-header` value MUST be excluded from `tools/list` while valid tools survive. Both revert-and-fail proven against their pre-existing implementations. Closes spec-compliance driver gaps **VER-1 + TOOLS-G1 (both P1)**.
- **`completion/complete` now dispatches to registered `McpCompletion` providers (2026-06-11).** Providers registered via `.completion_provider(...)` were stored but never consulted — the handler always answered with hardcoded placeholder values and ignored its input. The handler now parses typed `CompleteRequestParams` (malformed input → `-32602`, including reference-type literals `"ref/prompt"`/`"ref/resource"` that the untagged union would otherwise accept open-ended), routes deterministically (exact reference match first, `can_handle` fallback; priority desc then insertion order — provider storage moved from `HashMap` to `Vec` to make the tiebreak stable), runs the provider's `validate_request`, and enforces the spec's 100-item `completion.values` cap (truncation sets `total`/`hasMore`). No matching provider → empty values (the placeholder junk is gone). The same gap existed verbatim in `LambdaMcpServerBuilder` (providers stored, static handler answered) — mirrored fix there. Closes spec-compliance driver gaps **UTIL/COMP-1 (P1)** and **UTIL/COMP-3 (P2)**; red-phase wire tests in `discover_stateless_2026.rs`.
- **Mode-aware MRTR capability gating (2026-06-11).** The server's `-32003` gate on `InputRequiredResult` now enforces sub-capabilities, not just top-level presence: URL-mode elicitation requires the client's `elicitation.url` declaration ("Servers MUST NOT send elicitation requests with modes that are not supported by the client"; an empty `elicitation: {}` declares form-only), and tool-enabled sampling (`tools`/`toolChoice` present) requires `sampling.tools` ("Servers MUST NOT send tool-enabled sampling requests to Clients that have not declared support"). Closes spec-compliance driver gaps **CF/GAP-CF-1 + CF/GAP-CF-2 (both P1)**; red-phase wire tests in `mrtr_2026.rs`.
- **`roots/list` removed from the 2026 default surface (2026-06-11).** On 2026-07-28, roots is a client feature: the server requests roots via MRTR input requests and never hosts an inbound `roots/list` RPC; `notifications/roots/list_changed` has no binding in the pinned schema. The builder's `roots/list` + roots-notification registrations are now gated to the `protocol-2025-11-25` opt-in; on the 2026 default they answer 404 + `-32601` like every other non-2026 method. Closes spec-compliance driver gap **CF/GAP-CF-4 (P1)**; red-phase recorded in `error_mapping_2026.rs`.
- **Client MRTR completion for `resources/read` + `prompts/get`, and `resultType` discipline (2026-06-11, ADR-030 revision log).** The bilingual client's `parse_read_resource`/`parse_get_prompt` now surface `InputRequiredResult` as `McpClientError::InputRequired` (previously a serde "missing field" error that discarded `inputRequests`/`requestState`), with retry APIs `read_resource_with_input_responses` / `get_prompt_with_input_responses` mirroring `call_tool_with_input_responses`. All 2026 result parsers now enforce basic §Responses ("a resultType of any value unrecognized by the client MUST be considered invalid"): unknown discriminators are `ProtocolError::InvalidResponse` instead of being treated as complete results. Closes spec-compliance driver gaps CF/GAP-CF-3, PRM/PR-2026-02, RES-G1, PAT/G3, BP-1 (all P1 except PAT/G3). Real-server e2e round-trips added; revert-and-fail recorded.
- **Origin-header validation (DNS-rebinding protection) on the HTTP transport (2026-06-11, ADR-031).** Streamable HTTP §Security requires: "Servers MUST validate the `Origin` header … If the `Origin` header is present and invalid, servers MUST respond with HTTP 403 Forbidden." New `OriginPolicy` on `ServerConfig` — default `SameOriginOrLoopback` (Origin absent → allowed; loopback or Host-matching origins → allowed; anything else → 403), `AllowList(Vec<String>)` additive allowlist, `Disabled` opt-out for upstream-enforced deployments. Enforced at both transport handler entries (streamable + legacy session path), so hyper and Lambda deployments inherit it; OPTIONS preflight and `.well-known` routes exempt. Builder knobs: `HttpMcpServerBuilder::origin_policy` and `McpServer::builder().origin_policy(...)`. Wire tests in `crates/turul-mcp-server/tests/origin_validation_2026.rs` (revert-and-fail proven: disabling the gate fails 3 tests). On the Lambda builder, an explicit CORS configuration derives the policy (`cors_allow_all_origins()` → `Disabled`, an origin list → `AllowList`) unless `.origin_policy()` overrides it — `turul-http-mcp-server`'s blanket `enable_cors` flag deliberately does NOT derive (see ADR-031 §CORS-derived policy). Closes spec-compliance driver gap **TX/GAP-1 (P0)**. **Behavior change:** cross-origin browser clients now require an explicit `AllowList` (previously implicitly admitted via CORS `*`).
- **New crate `turul-mcp-protocol-2026-07-28` at 0.4.0** — first standalone binding for the MCP DRAFT-2026-v1 release candidate (see [https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)). Stateless protocol core (`initialize` / `notifications/initialized` removed, `Mcp-Session-Id` header removed, per-request capability negotiation in `_meta`), new `server/discover` method, multi-round-trip `InputRequiredResult` (SEP-2322), `CacheableResult` mixin (`ttlMs`, `cacheScope`), W3C Trace Context in `_meta`, JSON Schema 2020-12 on tool input schemas, MCP Apps templates, RFC 9207 auth hardening, error code `-32002 → -32602`. Schema pinned to upstream commit `c3e3f09eb5d271407afac0f0bb6ee2dae5813d1d`. Compliance harness with bidirectional wire-format gate against the upstream's 86 canonical example fixtures (8 modeled cases / 20 fixtures bound at this cut; remainder marked `Kind::NotModeled` for wave-by-wave migration). 343 tests pass under `--features compliance` (160 lib + 179 integration + 3 fixture + 1 doctest), 333 default; `clippy -D warnings` clean. See `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md`.
- **First-party 2026-07-28 stateless server (2026-06-07).** `turul-mcp-server` / `turul-http-mcp-server` gained a `server/discover` handler plus a stateless 2026 request path: capabilities, client info, and protocol version travel in per-request `_meta` on every call (no `initialize` / `notifications/initialized` handshake, no `Mcp-Session-Id` header). The transport advertises `MCP-Protocol-Version: 2026-07-28` on the wire. Wire-level acceptance tests cover the stateless core; `server/discover` returns a `CacheableResult` (`ttlMs`/`cacheScope`). The 2025-11-25 stateful core (handshake + session header + GET SSE) remains available under the `protocol-2025-11-25` opt-in.
- **Schema re-pinned to finalized upstream `2026-07-28` (2026-06-07).** Re-vendored `schema/draft-schema.ts` from `modelcontextprotocol/modelcontextprotocol@main` (HTTP ETag `0eeaed15…`, content sha256 `20df36f9…`; was `8bdd4ae5…` at the 2026-05-24 cut). The 159-symbol export surface and 22 method strings are **identical** to the prior pin (stateless core intact — no `initialize`/`ping`/`resources/subscribe`/`tasks/*` reintroduced; verified against live `main`). Exactly three substantive wire changes applied: (1) `LATEST_PROTOCOL_VERSION` / `MCP_VERSION` / `McpVersion::V2026_07_28` serde rename flipped `"DRAFT-2026-v1"` → `"2026-07-28"` (draft literal still accepted on deserialize via serde `alias` for back-compat); (2) `ResultType` became an open union `"complete" | "input_required" | string` — modeled as `ResultType::Other(String)` with custom serde so unknown discriminators round-trip instead of being rejected; (3) `DiscoverResult` now `extends CacheableResult` (`ttlMs`/`cacheScope` required fields added). Also fixed the `clippy::large_enum_variant` gate on the deprecated MRTR `InputRequest`/`InputResponse` unions (scoped `#[allow]` with rationale). Contract-change tests migrated; revert-and-fail verified for the version and `ResultType` deltas. Crate stays at **0.4.0** (unreleased) — completing the spec line 0.4.0 was created to target, not a new version. See `docs/adr/027` revision log (2026-06-07).
- **Standardized the branch on published `turul-rpc` 0.2 (crates.io); `turul-rpc` 0.1 is no longer referenced anywhere on this branch** (`Cargo.lock` resolves only `0.2.2`). The workspace pin moved `0.1 → 0.2` and the `turul-mcp-protocol-2026-07-28` per-crate `0.2.2` override collapsed onto the workspace pin. `turul-rpc` 0.2 split inbound `JsonRpcMessage` (now parse-only: `Request`/`Notification`) from the outbound `JsonRpcResponse` union (`Success`/`Error`); `turul-http-mcp-server` was ported accordingly — its private dispatch path now produces `JsonRpcResponse` and converts to `JsonRpcMessageResult` via the canonical `Success→Response`/`Error→Error` mapping, and `JsonRpcResponse::success` takes `ResponseResult` (result values `.into()`-converted). Wire format is byte-identical (`{jsonrpc,id,result}` / `{jsonrpc,id,error}`) — **zero test expectations changed**. The frozen `2025-06-18` / `2025-11-25` protocol crates compile unchanged on 0.2 (freeze intact). `cargo build --workspace` clean; `turul-http-mcp-server` 92 unit + 11 doc tests pass; `clippy -D warnings` clean. (0.1 was maintained only for the 0.3 framework, which lives on `main`.)
- **`turul-mcp-client` is now bilingual (2025-11-25 + 2026-07-28) by default (2026-06-07).** A single client negotiates the wire spec per connection at `connect()` (`server/discover` → 2026-07-28; JSON-RPC `-32601` → fall back to `initialize` → 2025-11-25; HTTP 4xx and all other JSON-RPC errors abort WITHOUT downgrade; opt-in `allow_legacy_gateway_fallback` broadens the fallback to 404/405) and locks `McpVersion` for the connection. The client links both versioned protocol crates directly (a recorded exception to the Protocol Re-export Rule — CLAUDE.md + ADR-001), gated by mutually-exclusive features `client-bilingual` (default) / `client-2025-11-25-only` / `client-2026-07-28-only` with a `compile_error!` mutex. On a 2026 connection every core operation routes through `protocol/v2026_07_28` with the required per-request `_meta` and 2026 result parsing: `tools/list`/`tools/call`, `resources/list`/`read`/`templates/list`, `prompts/list`/`get`, and the `*_paginated` variants. Methods removed from the 2026 core (`ping`, `tasks/*`) are rejected on a 2026 connection and retained on 2025-11-25. Acceptance: `bilingual_negotiation.rs` + `bilingual_2026_operations.rs` (per-op `_meta` wire enforcement against a mock 2026 server + removed-method rejection). 143 client tests; `clippy -D warnings` clean on all three feature configs. Still pending: MRTR `InputRequiredResult`, `completion/complete` client op, server-initiated elicitation. See ADR-030 revision log (2026-06-07). The bilingual client builds under the 2026-default workspace and the framework alias cutover has landed (see the cutover entry below).
- **ADR-027** — *Targeting MCP DRAFT-2026-v1*. Records the wire-string choice (`"DRAFT-2026-v1"` until the upstream RC ships its `2026-07-28` literal), the schema-pin regeneration trigger, the per-crate versioning policy, and the consequences for downstream consumers. Revision log captures the 2026-05-24 initial cut, the 2026-05-31 per-crate-versioning adoption, Slice A' schema-fidelity corrections, Slice A'' SEP-2577 deprecation annotations, and the Slice C status update (§Consequences replaced; Phase 9.4 moves *into* 0.4.0).
- **ADR-029** — *Spec-version coexistence via mutually-exclusive cargo features (default 2026-07-28)*. The load-bearing 0.4.0 architecture decision. Default = 2026-07-28; opt-in `protocol-2025-11-25` feature on `turul-mcp-protocol`; `compile_error!` mutex; Phase 9.4 flip-all-at-once (landed — see the cutover entry below).
- **ADR-030** — *turul-mcp-client spec coexistence — bilingual default*. Client diverges from server's single-spec strategy because a client has no process-wide state-machine lock — it talks to whatever's on the wire. Per-connection version detection via try-`server/discover`-then-fallback-to-`initialize`; opt-in `client-2025-11-25-only`/`client-2026-07-28-only` narrowing features for binary size.
- **8 existing-ADR amendments** documenting the default-2026 cascade through ADR-027 (consequences replaced + status update + revision log), ADR-006 (stateless variant; GET SSE is 2025-only), ADR-009 (`McpProtocolVersion` becomes feature-exclusive), ADR-023 (per-request fingerprint persistence), ADR-001-lambda (stateless 2026 Lambda variant — ~50 vs ~200 LOC), and revision-log entries on ADR-025, ADR-026, ADR-028.
- **`docs/plans/2026-07-28-architecture-review.md`** — doc-form persistence of the 5-pattern architecture-review workflow that recommended Pattern A (cargo-feature gating). Persists what was previously in `/tmp` so the analysis is permanently in the repo.
- **`docs/plans/2026-07-28-feature-gating-rollout.md`** — phase-by-phase implementation plan for wiring `#[cfg(feature = "protocol-...")]` through the framework crates, examples, and test crates. This plan was the verification artifact for the cutover rollout that has since landed (see the cutover entry under §Changed).
- **`docs/plans/2026-07-28-codex-review-summary.md`** — self-contained codex-review-ready summary covering the decision, files-touched inventory, technical risks, and codex focus areas.
- **SEP-2577 deprecation annotations** on Roots / Sampling / Logging types and traits (`#[deprecated(since = "0.4.0", note = "...")]` with migration-path guidance and 2027-07-28+ earliest-removal date). Annotation-only this revision; types remain fully functional during the 12-month migration window. `LoggingLevel` (the value type for the non-deprecated `RequestMetaObject.log_level` replacement) is intentionally NOT deprecated.
- **ADR-028** — *Extensions strategy* (SEP-2133 / SEP-2663). Documents how the framework will host out-of-tree extensions — originally as schema-version-suffixed crates; superseded by the spec-neutral `turul-mcp-ext-tasks` / `turul-mcp-ext-apps` names (ADR-028 amendments 2026-06-07 and 2026-06-12).

### Added (2026-06-10, release-prep sweep)

- **Client-side `subscriptions/listen`** — `McpClient::subscriptions_listen(filter)` (2026 connections only) opens the long-lived stream via a new additive `Transport::send_request_streaming` (Streamable HTTP implements it; other transports default to unsupported). The client consumes and validates the mandatory acknowledgement first and returns a `SubscriptionStream` exposing the honored filter subset and the subscription id, then yields each notification; dropping the stream closes it (= cancellation per Streamable HTTP). e2e: real client ↔ real server — open stream, trigger server-wide broadcasts from a second request, receive only the opted-in type stamped with the subscription id.
- **ADR-025 framework shim cut landed** — `turul-mcp-server`/`turul-http-mcp-server`/`turul-mcp-builders`/`turul-mcp-aws-lambda` now depend on `turul-rpc` directly (146 path swaps; the shim mirrors `turul-rpc` paths 1:1 and re-exports the same types, so no public API changed). The shim remains in-workspace solely for the frozen 2025 protocol snapshots and 2025-pinned test/example crates, with its manifest restored to the terminal `0.3.47` (the mechanical 0.4.0 sweep value was never publishable). ADR-025/ADR-027 revision logs updated; the 2025 regression lane is recorded as the per-crate matrix (a workspace-wide flag sweep trips the spec mutex by design).

- **MRTR on `resources/read` and `prompts/get`** — completes the SEP-2322 triple (the only methods permitted to return `input_required`). The conversion + client-capability gate is now one shared helper (`handlers::input_required_to_result`, also adopted by `tools/call`). Resources surface the retry's `inputResponses`/`requestState` via the session extensions (same as tools); prompts receive them in the render args under reserved `io.modelcontextprotocol/*` keys, because `McpPrompt::render` has no session parameter and changing it would break the public trait (documented on the trait; reserved-namespace keys cannot collide with wire prompt arguments, which are plain strings). Tests: two-leg round trips for both methods + a `-32003`/400 capability-gate case on `resources/read` (all real-HTTP; the handlers previously leaked the sentinel to `-32603`).
- **`resources.subscribe` capability truthfulness** — with `subscriptions/listen` serving per-URI `resources/updated`, both capability-construction sites now advertise `subscribe: true` on the 2026 lane (still `false` on 2025, which has no `resources/subscribe` handler). Wire test asserts the `server/discover` advertisement.
- **`completion/complete` e2e coverage** — sessionless dispatch + `CompleteResult` wire shape (`completion.values`) + capability advertisement, closing the last zero-e2e core method on the 2026 path.

### Fixed (2026-06-10, release-prep sweep)

- **Stale crate-doc versions**: 41 dependency-snippet strings across 15 non-frozen crate READMEs/lib-docs said `"0.3"`; all now `"0.4"` (the frozen 2025 protocol snapshots and the terminal-0.3.x shim correctly keep `"0.3"`).
- **Legacy prose labeled in default-lane docs**: `turul-http-mcp-server` README's session/SSE-resumability features and curl examples, and `turul-mcp-server` README's strict-lifecycle note, are now explicitly marked *2025-11-25 opt-in lane* (the 2026 default is POST-only, non-resumable, handshake-free).
- **Comment hygiene in touched 2026 paths**: internal-phase tags, fix-history phrasing, and the `subscriptions.rs` module narrative replaced with present-tense, spec-anchored descriptions.

### Fixed (2026-06-10)

- **`turul-mcp-oauth` CIMD/DCR posture dispositioned (docs/tests-only by design).** Audited against the live draft authorization spec: Client ID Metadata Documents are a SHOULD for *authorization servers and MCP clients*; Dynamic Client Registration is deprecated upstream (MAY, AS back-compat; not removed — earliest removal 2027-07-28). This crate implements the resource-server role only — RFC 9728 Protected Resource Metadata and OAuth 2.1 §5.2/RFC 8707 token validation, both unchanged — so no CIMD or DCR surface belongs in it and none was invented. The role posture is now documented in the crate header, and a wire-shape test pins that the published RFC 9728 document carries no client-registration keys (`registration_endpoint`, `client_id*`, `redirect_uris`, …). Client-side CIMD belongs to a future full MCP OAuth client flow.
- **Builders/derive schema pipeline is lossless on the 2026 path.** Two defects destroyed JSON Schema 2020-12 fidelity between a tool author's types and `tools/list`: (1) `ToolSchema::from_schemars` stripped `$defs`/`definitions` from the root while passing properties through verbatim — every `#/$defs/X` pointer dangled; the 2026 root now RETAINS `$defs`/`definitions`/`$schema` (the 2025 typed lane keeps its inline-resolution and stripping). (2) The derive macros funneled schemars-generated parameter and output schemas through the typed-enum converter, silently collapsing data-bearing unions (`oneOf` + `const` tags → bare `{"type":"object"}`) and other 2020-12 compositions. New lane-aware `turul_mcp_builders::schemars_param_schema`: on 2026 it inlines local `$ref`s (cycle-guarded `resolve_local_refs`; `$ref` siblings compose via `allOf`) and carries the result verbatim via the new transparent `JsonSchema::Raw` variant (untagged escape hatch on the 2026 typed enum — also the deserialize fallback for subschemas the structured variants reject); on 2025 it is the status-quo typed conversion. **Documented limitations with rejection tests** (not silent loss): cyclic `$ref`s cannot be inlined into a property subschema (error names the cycle; restructure the type or use a root `from_schemars` document), and non-local/network `$ref`s are rejected per the spec's no-auto-deref rule. Tests: 7 builders fidelity tests (nested `$defs` inlining with enum/required intact, tagged-union `oneOf` survival, composition-keyword verbatim round-trip, cycle/non-local rejection, `$ref`-sibling `allOf` composition, root `$defs` retention) + 2 real-HTTP e2e (`schema_fidelity_2026.rs`: a derived tool's tagged-union param and schemars output reach `tools/list` undamaged with no dangling `$ref`; `tools/call` `structuredContent` satisfies the ADVERTISED `outputSchema` wrapper field discovered from `tools/list`). Revert-and-fail: with the 2026 arm forced through the old converter, the tagged-union test fails showing the exact loss (`"shape":{"type":"object"}`) — recorded. No public macro/builder API shape changed.
- **Protocol-fidelity sweep, part 2 — `ttlMs` as a schema `number` + SEP-2577 marker absorption.** (a) `CacheableResult.ttlMs` (and its embeddings in the tools/resources/prompts/discover results) is now `f64` per the schema's `number` type: fractional values are accepted on deserialize and survive round trips, negative/non-finite values reject (`@minimum 0`), and whole values keep the compact integer wire form (byte-stable for the common `ttlMs: 0` case). (b) The re-pinned schema's SEP-2577 deprecations are now fully absorbed as `#[deprecated]` markers: `LoggingLevel` (+ `LogLevel` alias), the per-request `_meta` `logLevel` key and `RequestMetaObject.log_level`/`with_log_level`, `ServerCapabilities.logging`, `ModelHint`/`ModelPreferences`/`ToolChoice`, the `ContentBlock::ToolUse`/`ToolResult` variants and constructors, and the sampling trait surface (`HasCreateMessageRequestParams`/`CreateMessageRequest`/`CreateMessageResult`/`HasLevelParam`). The earlier rustdoc claim that `LoggingLevel`/`logLevel` were "the non-deprecated replacement" was wrong against the re-pin and is corrected — the whole Logging surface (including the per-request opt-in this branch implements) is deprecated-but-normative through the migration window. Framework-internal use sites carry scoped `#[allow(deprecated)]` (the framework intentionally serves the surface through the window); downstream consumers now get compiler nudges.
- **Protocol-fidelity sweep, part 1 (wire/type drift vs the pinned schema).** (a) `ToolChoice` no longer carries a non-spec `name` field on the wire (the `specific()` constructor is gone) and `mode` is optional per schema (`{}` parses; absent means `"auto"`; `effective_mode()` helper). (b) `PromptReference` is `BaseMetadata`-shaped: gains `title`, drops the non-spec `description`. (c) `Annotations.audience` is the closed `Role[]` union instead of `Vec<String>` — wire-invalid values like `"system"` are now rejected at parse time; the builders' `annotation_audience` takes `Role` (converted to strings on the frozen 2025 lane). (d) The duplicate `Role` binding is gone — `sampling::Role` re-exports the single `prompts::Role`. (e) `LoggingCapabilities`/`CompletionsCapabilities` match the schema's opaque `JSONObject`: the invented `enabled`/`levels` keys are removed from the bindings and from both server builders' capability advertisements (presence of the object is the signal). 5 new wire-shape contract tests in `compliance.rs`; existing tests migrated with the contract (e.g. the empty `ToolChoice` parse fails against the pre-fix required-`mode` binding).

- **2025 opt-in lane build regression (same day, pre-push).** The elicitation enum-union slice used the 2026-only union accessors in `turul-mcp-builders` code that also compiles under `protocol-2025-11-25`, breaking the opt-in lane builds (caught by `scripts/ci-gates.sh all`). The validation is now `#[cfg]`-split per lane.
- **`tools/list` now advertises `outputSchema` on the 2026 path.** The 2026 `ToolDefinition::to_tool()` hardcoded `output_schema: None` (a type-bridge gap: the trait returns the object-rooted `ToolSchema`, the 2026 wire type is the free-form `ToolOutputSchema`), so no tool could ever advertise its output contract and clients had no way to know `structuredContent` conformance applied. Bridged via a lossless serde round-trip. Real-HTTP test asserts the derive-declared `output = String` schema appears in `tools/list` (failing pre-fix).
- **Elicitation enum schemas no longer lose constraints through the untagged unions.** `PrimitiveSchemaDefinition`'s untagged deserialize matched `{type:"string", enum:[...]}` against `StringSchema` first, silently DROPPING the `enum` (and `enumNames`/numeric bounds in the analogous cases). The primitive structs (`StringSchema`/`NumberSchema`/`BooleanSchema`) and untitled select shapes now carry `deny_unknown_fields`, so each payload lands on its precise variant. The schema's enum union is now bound faithfully: new `EnumSchema` = `SingleSelectEnumSchema | MultiSelectEnumSchema | LegacyTitledEnumSchema` (upstream order); the old struct misusing the `EnumSchema` name is renamed `LegacyTitledEnumSchema` and gains its missing `default` field; union helpers (`new` → spec-pure untitled single-select, `allowed_values()`, `is_multi_select()`) keep the builders API working, and the elicitation builder validates multi-select array submissions. 7 new round-trip fidelity tests (incl. through `ElicitationSchema.properties` — the previously untested path); revert-and-fail: 5 of 7 fail with `deny_unknown_fields` removed from `StringSchema` alone.
- **`resources/read` results default to `cacheScope: "private"`.** Read contents routinely depend on the authenticated user; the previous blanket `public` default invited shared caches to serve one user's resource to another (the caching guidance's exact warning). List results keep `public`; user-independent read results opt back in via `with_cache()`. Contract test pins the default.

### Added (2026-06-10)

- **Per-request log gating (2026-07-28).** `notifications/message` is now opt-in per request: a `tools/call` whose `_meta` lacks `io.modelcontextprotocol/logLevel` gets NO message notifications (spec MUST), and the declared level is the severity threshold (replaces the removed `logging/setLevel` session threshold, which remains the filter on the 2025 lane). The tools/call handler surfaces the declared level to the session context; `notify_log` gates emission. **Also fixes a pre-existing POST-SSE ordering race**: the final response frame could beat (and the shutdown path silently DROP) request-scoped notifications already queued on the progress channel — the progress task now flushes queued events on shutdown and the final frame is sent only after the flush handshake, so notifications precede the final response on the wire. Tests: `log_gating_2026.rs` (real-HTTP SSE: suppressed without `logLevel`, delivered with `"info"`, filtered below an `"error"` threshold; the opt-in case failed against the pre-fix ordering — revert-and-fail evidence), wired into the default CI lane.
- **SEP-2243 `Mcp-Param-*` custom-header mirroring (client emission + server validation).** Completes the deferred remainder of the request-metadata headers work. Protocol crate: pure SEP-2243 logic in `headers.rs` — `scan_x_mcp_headers()` (annotation discovery at any nesting depth with the full constraint set: non-empty, `tchar` syntax, case-insensitive uniqueness, string/integer/boolean only), `encode_param_value()` / `decode_param_value()` (string/integer/boolean conversion, JS safe-integer range, `=?base64?…?=` sentinel incl. the self-matching-sentinel re-encode rule; unit tests reproduce all five spec encoding examples). Server: the tools/call handler validates every annotated parameter's mirrored header against the body argument (sentinel-decoded; integers compared numerically) — value-without-header, header-without-value, or decoded mismatch → `-32001 HeaderMismatch` at HTTP 400 (the transport surfaces request headers to handlers via the rpc session metadata; the inline 2026 JSON path now maps `-32001` to 400 alongside `-32003`). Client: `tools/list` rejects tool definitions with invalid `x-mcp-header` values (excluded + warning, per spec) and captures per-tool bindings BEFORE the 2025-vocabulary remap (which cannot carry the annotation); `tools/call` mirrors annotated arguments into `Mcp-Param-{name}` headers via a new `Transport::send_request_with_extra_headers` (default delegates to `send_request` — non-HTTP transports MAY ignore the annotations). Tests: `mcp_param_2026.rs` (4 real-HTTP server cases; revert-and-fail recorded — both negative cases fail with validation disabled) and a closed-loop client e2e (the validating server rejects missing mirrors, so the green client call proves emission; covers plain ASCII and Base64-sentinel values). Not implemented (acceptable per spec): the client's optional schema-stale auto-retry (`tools/list` + retry on rejection is left to the application).
- **Schema pin re-vendored (content sha256 `1bf94a60…`, fixture pin `1304c8fe`).** One substantive upstream change: `ElicitationCompleteNotificationParams` extracted into a named interface extending `NotificationParams` (surface 159 → 160). The Rust binding already modeled the optional `_meta`, so the previously recorded deviation resolved upstream. ADR-027 revision log + COMPLIANCE.md/schema README hash records updated.
- **Docs/ADR reconciliation to implemented behavior**: ADR-029 §CI surface rewritten to the as-built lanes (the prescribed `cargo test --workspace` matrices never compiled — spec-pinned workspace members trip the ADR's own mutex; per-crate matrix is the operative shape); ADR-025 revision entry recording the shim's branch reality (still consumed by four non-frozen crates; manifest carries an unpublishable 0.4.0 from the version sweep; the framework-wide cut remains 0.4.0 release-prep work); `docs/plans/2026-07-28-schema-coverage-matrix.md` marked STALE/superseded (authoritative coverage = COMPLIANCE.md + the compliance harness); COMPLIANCE.md elicitation-union and extension-crate-name notes refreshed.
- **MRTR (SEP-2322): `InputRequiredResult` production (server) and consumption (client).** Server: a tool returning the new `McpError::InputRequired { input_requests, request_state }` sentinel (2026 protocol crate; NOT a wire error — the only return channel available to tool impls) is converted by the `tools/call` handler into a successful `InputRequiredResult` (`resultType: "input_required"`), after enforcing that every input request targets a capability the client declared in that request's `_meta` `clientCapabilities` — undeclared → `-32003 MissingRequiredClientCapability` at HTTP 400 (the 2026 JSON-framed response path now dispatches inline so the HTTP status can reflect the JSON-RPC outcome; SSE-framed responses inherently stay 200). On the retry leg, `CallToolRequestParams.inputResponses`/`requestState` are surfaced to tools via `SessionContext::input_responses()` / `mrtr_request_state()` (requestState documented as attacker-controlled). Client: `call_tool` surfaces `resultType: "input_required"` as the new `McpClientError::InputRequired` carrying `inputRequests`/`requestState`; the application gathers inputs and retries via `call_tool_with_input_responses(name, args, responses, request_state)` (fresh JSON-RPC id; 2026 connections only). New `ClientConfig.declared_capabilities` (elicitation/sampling/roots, all off by default) feeds both the 2026 per-request `_meta` and the 2025 `initialize` capabilities — previously hardcoded empty. Tests: `mrtr_2026.rs` (real-HTTP two-leg round trip + `-32003`/400 capability rejection; the suite does not compile against the pre-slice tree — the sentinel variant did not exist) and a full client-driven MRTR e2e in `e2e_2026_real_server.rs`. Limitations (tracked): MRTR production is wired for `tools/call` only (`resources/read`/`prompts/get` handlers have no input hooks yet); the framework does not generate or verify `requestState` integrity (HMAC is the tool author's concern).
- **Unknown-method mapping on the 2026 path: HTTP 404 + JSON-RPC `-32601`.** A request for a method the server does not implement now returns `404 Not Found` with a `-32601` body (the body distinguishes this from a legacy HTTP+SSE server's 404), checked pre-dispatch against the dispatcher's registered methods (the streaming architecture commits the status before dispatch completes, so the check cannot ride the dispatch result). Methods absent from the pinned 2026-07-28 schema — `ping`, `initialize`, `tasks/*`, `logging/setLevel`, `resources/subscribe` — are never registered on a 2026 build and land here. The 2025-era sessionless-ping bypass is now `protocol-2025-11-25`-only (it previously let `ping` dodge header validation and answer 200/`-32601` on the 2026 path). Tests: `error_mapping_2026.rs` (3 real-HTTP cases incl. a sweep over the absent methods; failing pre-fix — revert-and-fail evidence), wired into the default CI lane. With this, the 2026 error/status contract is: 401/403 auth (middleware) → `-32004` unsupported version (400) → `-32001` header mismatch (400) → `-32602` missing/incomplete `_meta` (400) → 404/`-32601` unknown method → dispatch.
- **SEP-2243 request-metadata headers enforced (server) and emitted (client) on the 2026 path.** Server (`turul-http-mcp-server`, §Server Validation): every POST must carry `MCP-Protocol-Version` (a 2026-only build supports no pre-2025-06-18 clients, so an absent header is rejected) and `Mcp-Method` matching the body method; `tools/call`/`prompts/get` (`params.name`) and `resources/read` (`params.uri`) additionally require a matching `Mcp-Name`. Failures → HTTP 400 + JSON-RPC `-32001 HeaderMismatch` (id-less for notifications). A requested version this build does not implement → HTTP 400 + `-32004 UnsupportedProtocolVersionError` with `data.supported`/`data.requested` (previously never emitted); the header/body `_meta` protocolVersion disagreement moved from `-32602` to `-32001` (it is a header-validation failure). The 2026 build now routes ALL requests to the streamable handler — a legacy version header can no longer detour into the 2025-era session handler around version validation (`server.rs`). Client (`turul-mcp-client`): 2026 connections mirror `method` into `Mcp-Method` and `params.name`/`params.uri` into `Mcp-Name`; the `server/discover` probe now advertises `MCP-Protocol-Version: 2026-07-28` (header must match its 2026 `_meta` — the old legacy-header probe is rejected by a validating 2026 server) and the fallback arm restores the 2025 header before `initialize`; a 400 whose body is a JSON-RPC error surfaces its code to the negotiation classifier, and `-32004` now triggers the 2025 fallback (structured negotiation signal; bare 4xx still aborts). `headers.rs` (protocol crate) rewritten to the live-draft wire shape: `x-mcp-header` is a schema annotation key, the wire header is `Mcp-Param-{name}` (+ `=?base64?…?=` sentinels), and `-32001` gets a named constant — replacing the incorrect `x-mcp-header-<name>` wire-prefix constant. Tests: `mcp_headers_2026.rs` (9 real-HTTP enforcement cases, 7 failing pre-fix), `e2e_2026_real_server.rs` (bilingual client ↔ real in-process 2026 server: negotiation + tools round-trip, failing pre-fix on the probe header), strengthened wiremock matchers (stubs now require the headers), `-32004` classifier unit test. Existing 2026 suites migrated to send the now-mandatory headers. Not yet done (tracked): `Mcp-Param-*` emission/validation (requires `x-mcp-header` inputSchema scanning), client-side `subscriptions/listen` API.
- **`turul-mcp-client` no longer depends on the `turul-mcp-protocol` alias** — closing the ADR-030 drift. The frozen `turul-mcp-protocol-2025-11-25` crate is now an unconditional dependency serving as the public type vocabulary; `turul-mcp-protocol-2026-07-28` stays feature-gated. This is load-bearing beyond hygiene: the alias pin (`protocol-2025-11-25`) made any dependency graph containing both the client and a 2026-default server trip the ADR-029 spec mutex — which is exactly what blocked the new real-server e2e test. Narrowing features now control which wire paths compile (`client-2025-11-25-only = []`). See ADR-030 revision log (2026-06-10).
- **Server-side `subscriptions/listen` (2026-07-28).** The stateless transport now serves the Subscriptions pattern that replaced the GET notification stream and the `resources/subscribe` RPC: a `subscriptions/listen` POST opens a long-lived SSE stream whose first message is `notifications/subscriptions/acknowledged` echoing the honored filter subset (requested types without a corresponding server capability section are omitted); only opted-in types are delivered (`toolsListChanged`/`promptsListChanged`/`resourcesListChanged` plus per-URI `resourceSubscriptions` filtering of `notifications/resources/updated`); every delivered notification is stamped with `io.modelcontextprotocol/subscriptionId` in `_meta`, set to the JSON-RPC id of the listen request. Delivery is gated at the broadcast layer (subscription registry entry created even for an empty filter) with a per-URI + type filter at the stream layer; the client cancels by closing the stream. Real-HTTP acceptance suite `subscriptions_listen_2026.rs` (3 tests: ack-first + cross-request broadcast delivery with filtering, SSE-Accept required, unsupported-type omission in the ack; all failing pre-implementation — revert-and-fail evidence) wired into the default CI lane. Not yet done: capability advertisement reconciliation (`resources.subscribe`) and the client-side listen API.

### Changed

- **2026 HTTP surface gated to POST-only (2026-06-10).** Under the `protocol-2026-07-28` default, the MCP endpoint now answers legacy-era traffic per the Streamable HTTP binding's Backward Compatibility rules: HTTP GET (the removed standalone SSE stream) and DELETE (the removed session termination) return `405 Method Not Allowed` with `Allow: POST, OPTIONS`; an inbound `Mcp-Session-Id` header is ignored at parse time (never honored as a session, never echoed — internal per-request sessions are no longer client-pinnable); `Last-Event-ID` is ignored (streams are not resumable in this revision); notification `202 Accepted` responses no longer carry a session header. The 2025-11-25 opt-in lane keeps the full stateful GET-SSE/session surface unchanged. New real-HTTP acceptance suite `stateless_2026_http_surface.rs` (5 tests, all failing pre-fix — revert-and-fail evidence) wired into the default CI lane (`ci.yml` + `scripts/ci-gates.sh`).
- **Default spec flipped to 2026-07-28 — the 0.4 cutover landed (branch-scoped, 2026-06-07).** `crates/turul-mcp-protocol/Cargo.toml` now declares `default = ["protocol-2026-07-28"]`; the alias re-exports the 2026-07-28 crate by default and `protocol-2025-11-25` is the opt-in escape hatch (`--no-default-features --features protocol-2025-11-25`). The protocol feature topology cascaded through every framework crate (`turul-mcp-session-storage`/`-task-storage`/`-builders`/`-derive`/`turul-http-mcp-server`/`turul-mcp-server`/`turul-mcp-aws-lambda`), each forwarding the spec choice. `ToolBuilder` and dynamic-tools were adapted to the 2026 result types (`resultType` + `CacheableResult` `ttlMs`/`cacheScope`) and work under the 2026 default. The bilingual `turul-mcp-client` builds under the 2026-default workspace while still speaking either spec per connection. Tasks are gated to the 2025-11-25 opt-in (`#[cfg(feature = "protocol-2025-11-25")]`) — under the 2026 default tasks are an extension (ADR-028). Example fleet migrated: **43 examples on the 2026 default**, 8 redundant duplicates removed (builders-showcase, comprehensive-server, sampling-with-tools-showcase, task-types-showcase, client-task-lifecycle, dynamic-tools-test-client, performance-testing, lambda-mcp-server-streaming), and a small 2025-11-25 regression suite pinned (tasks-e2e pair + logging/sampling/elicitation/client/lambda examples held at the 2025 opt-in); the integration-test crates are pinned to the 2025-11-25 opt-in. Default-members build is green at 0 warnings; the `compile_error!` mutex fires correctly under both feature configurations. Not merged to `main`. See ADR-027 / ADR-029 revision logs (2026-06-07).

- **Per-crate independent versioning policy adopted.** Every non-frozen crate's `Cargo.toml` migrated from `version.workspace = true` to a literal `version = "0.4.0"`. After this cut, individual crates may patch and publish independently — bump only the crate that changed, not the whole workspace. `[workspace.package].version` remains for tooling compatibility but is no longer authoritative. `[workspace.dependencies]` pins each internal crate path to its current literal version.
- **Frozen historical protocol crates** `turul-mcp-protocol-2025-06-18` and `turul-mcp-protocol-2025-11-25` received a one-time literal `version = "0.3.47"` pin in their respective `Cargo.toml` files. Without this they would inherit the new `[workspace.package].version = "0.4.0"` and silently bump the published version of crates that are explicitly frozen against historical spec snapshots. No source files were touched in either frozen crate. See ADR-027 §"Revision log" entry **2026-05-31** for the one-time-exception record.
- **`turul-mcp-json-rpc-server` is now a compatibility shim** re-exporting `turul-rpc 0.1`. New code should depend on `turul-rpc` directly (the 2026-07-28 protocol crate already does, isolated to `0.2.2` via a per-crate dep override). The shim continues to satisfy the rest of the framework on 0.1 through the 0.3.x line; framework-wide cutover is deferred to a later slice. See ADR-025.

### Notes for downstream consumers

- `turul-mcp-protocol` (the active-spec re-export alias) is now a **feature-gated re-export defaulting to `protocol-2026-07-28`**, with `protocol-2025-11-25` as the opt-in (`--no-default-features --features protocol-2025-11-25`). Phase 9.4 (the flip + every consumer crate migrating to the forwarding-feature topology) **has landed on this branch** across `turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`, `turul-mcp-builders`, the derive macros, and the migrated example fleet (43 examples on the 2026 default; 8 redundant examples removed; a small 2025-11-25 regression suite pinned). Publication to crates.io is still gated per ADR-027 (upstream final-spec publication + maintainer go-ahead); a full workspace `--no-default-features --features protocol-2025-11-25` CI matrix is the remaining coverage item.
- Branch lock: the `2026-07-28-MCP-Specification` branch remains unmerged from `main`. Pulling `main` against 0.4.0 gives a working tree that still ships MCP 2025-11-25 on the wire.

## [0.3.47] - 2026-05-23

### Fixed

- **`turul-http-mcp-server` returned HTTP 401 for missing `Mcp-Session-Id` instead of the spec-required HTTP 400.** MCP 2025-11-25 § Session Management states: *"Servers that require a session ID SHOULD respond to requests without an MCP-Session-Id header (other than initialization) with HTTP 400 Bad Request."* Two code paths were affected:
  - **Streamable HTTP POST non-initialize, non-allowed-ping**: `crates/turul-http-mcp-server/src/streamable_http.rs:1347-1373` was returning `StatusCode::UNAUTHORIZED`. Now returns `StatusCode::BAD_REQUEST`. The pre-init ping bypass at line 1174 is preserved; with `allow_unauthenticated_ping=false`, sessionless ping rejection also lands in this path and correctly returns 400 (same missing-header contract). Stale comment at line 296 documenting the bug as if it were spec is corrected.
  - **Legacy `session_handler.rs` GET SSE (protocol ≤ 2024-11-05)**: `crates/turul-http-mcp-server/src/session_handler.rs:864-870` was returning HTTP 200 with a JSON-RPC error body via `jsonrpc_error_to_unified_body` (which hardcodes 200). The JSON-RPC error body shape is preserved, but the response is now wrapped in a 400 status instead of 200. Cross-transport consistency with the Streamable HTTP path.
- **Streamable HTTP GET and DELETE** were already returning 400 for missing session (`streamable_http.rs:546` and `:1083`); no code change needed on those paths — only their test assertions were tightened (the GET test was tolerant of either 400/401; now requires 400).
- **Test compliance**: per CLAUDE.md §"Test Compliance" ("Tests validate the MCP spec — never change tests to preserve buggy behavior"), four test files were updated from asserting `401` to asserting `400`:
  - `tests/session_id_compliance.rs` (6 assertions + 2 test renames + header comment)
  - `tests/mcp_behavioral_compliance.rs` (sessionless-non-ping-rejected assertion, sessionless-ping-with-flag-off assertion, plus a new regression test for the legacy GET SSE missing-session path that pins both HTTP status and JSON-RPC envelope body)
  - `tests/streamable_http_e2e.rs` (POST hard assertion + GET tightened-tolerant assertion + stale comments)
  - `tests/phase5_regression_tests.rs` (line 136 assertion)
- **CLAUDE.md §"Session Status Codes" table** updated to reflect the spec-correct mapping, including the ping/`allow_unauthenticated_ping` interaction and an explicit row for the legacy SSE path.

### Versioning rule override

This is an MCP transport contract correction. By the prior versioning rule ("Minor bumps cover A2A/MCP/schema contract changes") it would have been a minor (`0.4.0`) bump. We ship it as a patch (`0.3.47`) because:

1. The change brings the framework into compliance with an existing spec, not adoption of a new spec revision; existing-spec compliance corrections are bug fixes by nature.
2. The user-global versioning rule has been updated to: patch bumps cover bug fixes, contract corrections, and spec-compliance fixes; minor bumps are reserved for new MCP spec adoption or explicit instruction.
3. Observable client impact is minimal: any conforming MCP 2025-11-25 client already handles 400 for missing session per spec; the prior 401 was a server-side defect that clients should already have been tolerant of (treating either 400 or 401 as "session is gone, restart `initialize`").

### Revert-and-fail evidence

After applying both fixes, reverting them via `git stash` and re-running the targeted tests produces:

```
test_sessionless_non_ping_rejected                                  left: 401, right: 400
test_legacy_handler_get_sse_without_session_returns_400             left: 200, right: 400
test_unauthenticated_ping_disabled_rejects_sessionless_ping         left: 401, right: 400
test result: FAILED. 0 passed; 3 failed
```

Restoring the fix returns all 11 targeted tests to GREEN (8 `feature_tests` + 3 `compliance`). The test net catches both bug classes.

## [0.3.46] - 2026-05-17

### Fixed

- **`turul-mcp-session-storage` failed to compile with `--features postgres` alone** (without `sqlite`). The `From<sqlx::Error> for SessionStorageError` impl in `crates/turul-mcp-session-storage/src/traits.rs` was gated `#[cfg(feature = "sqlite")]`, but `postgres.rs` contains a bare `?` on a `sqlx::Result` inside the expiration-cleanup transaction (`crates/turul-mcp-session-storage/src/postgres.rs:772`), which requires that `From` impl to exist. Enabling only the `postgres` feature therefore yielded 18 `E0277: the trait \`From<sqlx::Error>\` is not implemented` errors across the postgres module. Fix is a single feature-gate change to `#[cfg(any(feature = "sqlite", feature = "postgres"))]`, matching the gate already used in `turul-mcp-task-storage/src/error.rs:47`. Revert-and-fail evidence: `cargo check -p turul-mcp-session-storage --no-default-features --features postgres` fails with the 18 errors before the change, succeeds after. Verified clean across the four feature subsets users actually combine: `--features postgres`, `--features sqlite`, `--features dynamodb`, and `--features sqlite,postgres,dynamodb`. **Consumer impact**: Anyone depending on `turul-mcp-session-storage = { version = "0.3.45", features = ["postgres"] }` without also enabling `"sqlite"` could not build at all on 0.3.34–0.3.45; this is unblocked on 0.3.46. **Scope check confirming this is the only instance**: `turul-mcp-task-storage` already had the correct `any(...)` gate; `turul-mcp-server-state-storage` (the tool-fingerprint backend) has no `From<sqlx::Error>` impl and doesn't need one — its postgres backend uses `.map_err(...)` consistently and compiles cleanly under each single-feature combo.

## [0.3.45] - 2026-05-16

### Changed

- **`turul-mcp-client` migrates to `turul-rpc` directly, ahead of the rest of the framework** (scoped 0.3.x exception per [ADR-025](docs/adr/025-extract-turul-rpc.md) §"Revision log" 2026-05-16 entry). The client crate's `Cargo.toml` no longer depends on the `turul-mcp-json-rpc-server` shim — it depends on `turul-rpc` directly. The remaining framework crates (`turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`, etc.) continue to depend on the shim through the rest of 0.3.x; the framework-wide cutover lands at 0.4.0 per the original ADR-025 lifecycle. **Consumer impact**: `turul-mcp-client` users do not need to add `turul-rpc` to their own `Cargo.toml` — the dep is internal. Public API surface of `turul-mcp-client` is unchanged. Anyone explicitly importing types via `turul_mcp_json_rpc_server::*` from inside their own application code is unaffected (the shim crate still ships).

### Refactor

- **`turul-mcp-client` JSON-RPC envelopes now flow through `turul-rpc`'s typed constructors instead of 20+ hand-rolled `json!({"jsonrpc": "2.0", ...})` literals**. Two new private helpers in `crates/turul-mcp-client/src/client.rs` — `build_request(method, params)` and `build_notification(method, params)` — route every outbound MCP method (initialize, tools/list ×2, tools/call ×2, resources/list ×2, resources/read, resources/templates/list ×2, prompts/list ×2, prompts/get, ping, tasks/get, tasks/list ×2, tasks/cancel, tasks/result, notifications/initialized) through `turul_rpc::JsonRpcRequest::new` / `JsonRpcNotification::new`. The JSON-RPC 2.0 envelope shape (`jsonrpc` version, field ordering, `params` present-vs-absent semantics) now lives in one place rather than being copy-pasted across the file. **Wire bytes are semantically equivalent** to the prior hand-rolled form; this is a maintainability slice, not a behaviour change.
  - **Empty-params preservation**: `Value::Object(empty)` is intentionally preserved as `"params":{}` on the wire (not omitted via `skip_serializing_if`), matching the prior hand-rolled form so any MCP server that distinguishes `params: {}` from a missing `params` field continues to see the same envelope. `Value::Null` is correctly omitted (no `params` field).
  - **Defensive scalar handling**: `value_to_request_params(Value)` panics with `unreachable!()` for scalar `Value` inputs — no MCP client call site passes a scalar, and silently wrapping in a positional-array `RequestParams::Array` would be a wire-format change masking misuse rather than surfacing it.

### Test

- **17 new tests guarding the typed-envelope refactor**, totalling 130 client tests on the slice (was 113):
  - **5 unit tests** for `value_to_request_params` (Null → None; empty Object → `Some(Object(empty))`; Object preserves entries; Array preserved; scalar `#[should_panic]`).
  - **5 unit tests** for `build_request` (envelope shape with `jsonrpc/method/id/params`; nested object params; ID monotonic increment per call; `Value::Null` params omits the field on the wire; semantic JSON-envelope equality with the prior hand-rolled `json!({"jsonrpc": "2.0", ...})` form).
  - **3 unit tests** for `build_notification` (envelope shape with no `id` per JSON-RPC 2.0 §4.1; nested object params; `Value::Null` omits both `id` and `params`).
  - **1 unit test** `test_build_request_preserves_nested_array_values_in_arguments` — array values inside `params.arguments` (numeric, string, nested-array) round-trip through `RequestParams::Object(HashMap<String, Value>)` intact, distinct from JSON-RPC envelope-level positional params.
  - **3 wire-layer tests** in `tests/wire_compliance.rs` exercising `JsonRpcRequest` / `JsonRpcNotification` directly through `HttpTransport` against wiremock — typed request envelope on wire; empty-Object params preserves `"params":{}` on wire (not omitted); typed notification omits `id` field on wire.
  - **1 wire-layer test** `test_mcp_client_ping_sends_typed_jsonrpc_envelope_through_full_stack` — end-to-end production-path coverage walking `McpClient::connect()` + `ping()` against a wiremock server, capturing the `ping` POST body via `received_requests()`, asserting the JSON-RPC 2.0 envelope shape, AND asserting `notifications/initialized` POST has no `id` field.
  - **1 wire-layer test** `test_mcp_client_call_tool_preserves_array_argument_values_on_wire` — end-to-end `McpClient::call_tool("compute_stats", json!({"values": [1,2,3,4,5], "tags": [...], "matrix": [[1,2],[3,4]]}))` against wiremock, capturing the `tools/call` POST body, asserting `body["params"]["arguments"].is_object()` (proves MCP uses named args, not JSON-RPC positional) and that all three array values survive intact at `body["params"]["arguments"].{values, tags, matrix}` with no flattening, coercion, or stringification.

### Cleanup

- **`MockTransport` and `StatefulMockTransport` test fixtures now advertise `tools.listChanged: false`** (was `true`). Both fixtures previously claimed the capability during initialize but never emitted `notifications/tools/list_changed` from the mock itself, violating MCP capability truthfulness ("server MUST NOT claim a capability it does not actually deliver"). The three `test_*_list_changed_notification_invalidates_cache` tests inject the notification out-of-band via `MockTransport::event_sender()` and continue to pass — confirmed by the cache-invalidation handler at `client.rs:175-193` which processes the notification unconditionally rather than gating on the capability flag. No production-code change.

## [0.3.44] - 2026-05-15

### Added

- **`McpClient::set_bearer()` / `Transport::update_auth_header()`** — rotate the `Authorization` header on a live transport without rebuilding the underlying `reqwest::Client` (which would invalidate the HTTP/2 connection pool and force a fresh TLS handshake per rotation). Per-request `RequestBuilder::header(...)` overrides any same-named entry in `default_headers`, so existing `ConnectionConfig::headers`-baked bearers remain the initial value and become the fallback after `set_bearer(None)`. `Transport::update_auth_header` has a default no-op impl, so non-HTTP transports (stdio, SSE) are unchanged. Wired through `send_request`, `send_request_with_headers`, `send_notification`, `send_delete`, and the SSE GET listener task, so every outbound surface honours the live override.

### Fixed

- **`McpClient::disconnect()` could send DELETE under a stale bearer after OAuth `client_credentials` rotation** (`turul-mcp-client`). Discovered while investigating downstream consumer logs (sv-common / sw-common) that showed `HTTP 403 Forbidden` returned in ~15 ms from two unrelated upstream MCP servers fronting Lambdas (`st.aussierobots.com.au/mcp`, `sd.aussierobots.com.au/mcp`) on every `disconnect()` DELETE that followed a rotation event. Root cause was **not** server-side principal pinning — code inspection confirms the framework's DELETE handler in `turul-http-mcp-server` does not authenticate at all (both `streamable_http.rs` and `session_handler.rs` route DELETE around `MiddlewareStack::execute_before_session`, and `turul-mcp-oauth` only returns 401, never 403). The 403 originated upstream (API Gateway authorizer / ALB OIDC / equivalent) evaluating the bearer the client actually put on the wire — which was the *old* one. Reason: `HttpTransport` injected the `Authorization` header via `reqwest::ClientBuilder::default_headers()` at construction, with no API to mutate it thereafter. Callers that rotated the M2M token by creating a fresh `McpClient` were left holding old clients with bearers baked into the connection-pool-owning `reqwest::Client`; their cleanup `disconnect()` therefore sent DELETE under a bearer the AS had typically already revoked. Fix: `HttpTransport` now holds an `Arc<RwLock<Option<String>>>` auth override applied per-request via `RequestBuilder::header()`, with `Transport::update_auth_header()` / `McpClient::set_bearer()` as the rotation API. Callers rotate the bearer immediately before `disconnect()`, and the DELETE flies under the fresh token. Regression coverage: three wire-layer tests in `tests/wire_compliance.rs` exercising the actual reqwest pipeline against a wiremock server — `test_send_delete_uses_overridden_bearer_after_rotation` (the headline contract), `test_send_request_uses_overridden_bearer_after_rotation` (parity for POST), and `test_clearing_override_falls_back_to_default_headers` (confirms `None` removes the override). Revert-and-fail check recorded: with `apply_auth_override` removed from `send_delete` and `send_request`, the wire shows `authorization: Bearer OLD` and wiremock's `expect(1).matching(Authorization: Bearer NEW)` fails; the clearing test correctly stays green (it asserts the OLD-bearer fallback, which the unmodified code path still produces). Wire-layer rule per CLAUDE.md §"Test Coverage Discipline" #3 satisfied: tests assert what reqwest actually puts on the wire, not framework-internal state.

## [0.3.43] - 2026-05-15

### Fixed

- **`McpClient::disconnect()` followed by `Drop` no longer fires a second doomed DELETE** (`turul-mcp-client`). `SessionManager::terminate()` previously flipped state to `Terminated` but left `session_id` populated; the `Drop` impl then read `session_id_optional()`, observed `Some(_)`, and spawned a second `transport.send_delete(...)` against a session the server had already torn down — typically arriving after the originating bearer had expired in OAuth deployments, surfacing as a 401/410 noise event in server logs and prompting confused investigations on the server side. Fix: `terminate()` now clears `session_id` after logging it, establishing the invariant "a terminated session has no ID". Both production callers (`disconnect()` and `Drop`) already route through `terminate()`, so the single-point fix makes the whole lifecycle idempotent without any public API change or new method. The `Drop`-without-`disconnect()` path is unchanged — bare drop still fires exactly one DELETE, preserving server-side cleanup for callers that don't disconnect explicitly. Regression coverage: `test_disconnect_clears_session_so_drop_is_noop` (locks in DELETE-count == 1 across disconnect+drop) and `test_drop_without_disconnect_still_fires_delete` (regression guard for the implicit-cleanup path); `test_session_lifecycle` extended to assert `session_id_optional()` is `None` after `terminate()`. Revert-and-fail check recorded: with the one-line `session_id = None` clear removed, the new tests fail with `left: 2, right: 1` (double-DELETE) and the lifecycle assertion fails on the cleared-id check; the regression guard correctly stays green (it asserts an orthogonal invariant). Fix discovered by downstream consumers (sv-common / sw-common) hitting the second DELETE after explicit disconnect with a near-expired bearer — they will additionally adopt proactive disconnect at 95% bearer lifetime as a belt-and-suspenders measure on the consumer side.

### Note on v0.3.43 numbering

A previously-planned v0.3.43 (Lambda empty-body streaming) was investigated and **closed as documented limitation** rather than published — see the v0.3.42 entry below for the full reasoning. The version number v0.3.43 is therefore reused here for an **unrelated** client-side disconnect/Drop fix. There is no Lambda streaming behavior change in v0.3.43; the empty-body limitation continues to require APIGW MOCK on OPTIONS as documented in ADR-026.

## [0.3.42] - 2026-05-11

### Note (post-release): v0.3.43 Lambda empty-body investigation closed as documented limitation

Production verification by the downstream consumer (sd-mcp v0.7.12) confirmed that v0.3.42's `EnsureOneFrame` adapter does not actually fix the empty-body Lambda streaming `IncompleteMessage` / 60s timeout / APIGW 502 case it claimed to solve. A wire-level diagnostic harness on branch `park/wire-level-test-harness` (retained on origin) replicates `lambda_runtime-1.2.0/src/requests.rs` serialization verbatim and confirms `BodyDataStream` yielding a zero-byte data frame does not satisfy the AWS Lambda Runtime API wire contract. Three resolution paths were considered: (a) sentinel-byte fix in `EnsureOneFrame`, (b) reject empty bodies with a clear error, (c) document the limitation; APIGW MOCK on OPTIONS is the permanent pattern. **Decision: (c).** Reasons in ADR-026 §"Resolution 2026-05-11". v0.3.42 stays published; framework code is unchanged; fleet deployments (sd-mcp v0.7.13, plus sv-track/gps-trust-mcp/gps-trust-agent-mcp port wave) use APIGW MOCK on all OPTIONS methods. CLAUDE.md "Test Coverage Discipline" gained rule 3 (wire-layer coverage for transport-protocol boundaries) as a permanent gate improvement — this is the recurrence prevention for the class of failure mode v0.3.42 hit, regardless of how this specific bug resolved. The v0.3.43 version number was subsequently used for an unrelated client-side disconnect/Drop fix — see the v0.3.43 entry above.



### Fixed

- **Lambda streaming response with zero-data-frame body caused `IncompleteMessage` / 60 s timeout / API Gateway 502** (`turul-mcp-aws-lambda`). `into_lambda_stream_response` accepted any `B: http_body::Body + Unpin + Send + 'static`, but when `B` produced zero `Frame::Data` frames (e.g. `http_body_util::Empty::<Bytes>::new()`), the resulting `BodyDataStream` yielded zero items. The Lambda Response Streaming multipart envelope wrote the prelude + metadata JSON + trailer separator and then closed the body stream without ever writing a body chunk. Lambda's Runtime API client (hyper) requires at least one chunk before EOF for the framing to terminate cleanly; without one, the connection closed mid-frame with `hyper::Error(IncompleteMessage)`. The function appeared to hang for its full timeout, AWS reported `Status: timeout` (not `Status: error`, no `Errors` metric increment), and API Gateway emitted 502 to the client after the timeout. Common trigger: `.well-known/oauth-protected-resource` OPTIONS short-circuits in `run_streaming_with` dispatch closures returning `Response<UnsyncBoxBody<Bytes, hyper::Error>>` with `Empty::new()` body. **This is a pre-existing latent bug, not a v0.3.39 → v0.3.40 regression** — `f6438cb` does not touch any code path affected by it; consumer dispatch closures simply began exercising the empty-body path. Fix: internal `EnsureOneFrame<B>` body adapter wraps `B` in `into_lambda_stream_response` and emits a single zero-length `Frame::data` if the underlying body would otherwise yield no data frames. Bodies that natively produce ≥1 data frame are unaffected (first frame forwarded as soon as `B` yields it; no buffering, no pre-polling, streaming semantics preserved). The zero-length frame is invisible at the HTTP layer — no `Content-Length` header added, no response bytes visible to the client. Contract documented in ADR-026. Revert-and-fail recorded in commit message.

## [0.3.41] - 2026-05-11

### Fixed

- **`LambdaMcpServer::handler()` silently dropped builder-configured CORS** (`turul-mcp-aws-lambda`). `LambdaMcpServerBuilder::cors(...)` / `.cors_allow_all_origins()` / `.cors_allow_origins(...)` / `.cors_from_env()` populated `LambdaMcpServer.cors_config`, but `handler()` constructed the `LambdaMcpHandler` via `with_middleware_and_fingerprint(...)` (which initializes `cors_config: None`) and never chained `.with_cors(self.cors_config.clone())` onto the result. Every `if let Some(ref cors_config) = self.cors_config` branch inside `LambdaMcpHandler` — preflight short-circuit, custom-route injections, final-response injection — was therefore unreachable through the documented builder entry point. Pre-existing since `5d4bdd3` (2025-10-05, "feat: add middleware support for Lambda"); silently broken across all releases from then through v0.3.40. The injection logic added in v0.3.40 for streaming custom-route branches was functionally correct but unreachable for builder-constructed handlers, which is what surfaced the bug. Fix: `handler()` now chains `.with_cors(self.cors_config.clone())` after the notifier/registry wiring, before returning. Three new regression tests assert builder-path CORS coverage explicitly: streaming OPTIONS preflight, streaming 401 challenge (non-preflight, with `WWW-Authenticate` preserved + exposed), and negative-path (no CORS config → no CORS headers). The previous `test_cors_configuration` only smoke-checked `stream_manager`, which is why the bug slipped past CI for ~7 months — that smoke check is retained but no longer the sole guard.

## [0.3.40] - 2026-05-11

### Fixed

- **Lambda streaming custom-route CORS asymmetry** (`turul-mcp-aws-lambda`): `LambdaMcpHandler::handle_streaming()` returned route-matched and route-validation-error responses **without** injecting the configured `CorsConfig`, while the buffered `handle()` injected CORS for the equivalent branches. Browser-facing custom routes registered via `LambdaMcpServerBuilder::route(...)` (e.g. `.well-known/oauth-protected-resource` for RFC 9728 OAuth discovery) therefore returned `200 OK` with the correct body but no `Access-Control-Allow-Origin`, even when CORS was configured. Streaming now matches buffered: both branches inject CORS before returning. Regression tests added covering matched-route CORS, validation-error CORS, no-CORS-config-no-injection, and a 401 challenge end-to-end through the streaming transport with `WWW-Authenticate` preserved and exposed.

### Changed

- **Default `Access-Control-Expose-Headers` now includes `WWW-Authenticate`** in both `turul-mcp-aws-lambda::CorsConfig::default()` and `turul-http-mcp-server::cors::CORS_EXPOSE_HEADERS`. Browser OAuth clients cannot read non-safelisted response headers unless they appear in `Access-Control-Expose-Headers`; RFC 9728 discovery requires clients to parse `WWW-Authenticate` on `401` responses, so it must be exposed by default for any browser-fronted MCP server. Behavioural impact: a previously-not-exposed response header is now exposed to browser JS by default. Consumers passing a custom `expose_headers` list retain full control and must add `WWW-Authenticate` explicitly if they want browser OAuth to work. No change to non-browser clients.

### Added

- **Public re-exports for CORS helpers** in `turul-mcp-aws-lambda`: `inject_cors_headers` and `create_preflight_response` are now re-exported from the crate root and `prelude`, alongside the existing `CorsConfig` re-export. This is the supported escape hatch for `run_streaming_with` dispatch closures that short-circuit before calling `handler.handle_streaming()` — call the helper before returning to keep CORS behaviour consistent with the framework's built-in routing path. **Boundary**: the framework guarantees CORS on every response built inside `LambdaMcpHandler::handle_streaming()`; the consumer is responsible for CORS on responses built before `handle_streaming` is called, including custom `run_streaming_with` short-circuits — use the re-exported `inject_cors_headers` / `create_preflight_response`.

## [0.3.39] - 2026-05-10

### Changed

- **`turul-mcp-json-rpc-server` is now a re-export shim over [`turul-rpc`](https://github.com/aussierobots/turul-rpc)** (a new sibling repository / crate family). All implementation moved to `turul-rpc-core`, `turul-rpc-jsonrpc`, and `turul-rpc-server`; the framework crate `turul-mcp-json-rpc-server 0.3.39` is now ~50 lines of `pub use turul_rpc::*` plus module re-exports. Public API surface is preserved at every original path with identical nominal types — existing `turul_mcp_json_rpc_server::*` imports continue to compile and behave identically. Internal framework crates (`turul-mcp-protocol-2025-{06-18,11-25}`, `turul-mcp-builders`, `turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`) continue to depend on `turul-mcp-json-rpc-server` through 0.3.x; framework 0.4.0 will migrate those imports to `turul-rpc` directly and drop the shim crate. **There is no planned 0.4 release of `turul-mcp-json-rpc-server`** — 0.3.39 is the terminal shim. Existing 0.3.x consumers may continue depending on it indefinitely; new code should depend on `turul-rpc` directly. See [ADR-025](docs/adr/025-extract-turul-rpc.md).

### Added

- **JSON-RPC 2.0 batch processing** (via `turul-rpc-jsonrpc`): the original `turul_mcp_json_rpc_server::dispatch::parse_json_rpc_messages` was a stub claiming "JSON-RPC 2.0 removed batch support" (it didn't). `turul-rpc-jsonrpc 0.1.0` ships a spec-conformant batch implementation — `parse_json_rpc_batch` returns a `BatchOrSingle` discriminator; `JsonRpcDispatcher::handle_batch` dispatches per-member with notification-response suppression, all-notifications no-response semantics, and empty-batch → single `Invalid Request` (`-32600`) per [JSON-RPC 2.0 §6](https://www.jsonrpc.org/specification#batch).
  - **Reachable through the shim**: `JsonRpcDispatcher::handle_batch` (the dispatcher type is re-exported and methods come with the type — listed here per ADR-003 §"additive items reviewed").
  - **Not reachable through the shim**: `parse_json_rpc_batch` and `BatchOrSingle` live in `turul_rpc::batch`, a module the shim does **not** re-export. Users who want them depend on `turul-rpc` directly. This preserves the v0.3.38-surface discipline.

### Note on JSON-RPC 2.0 compliance posture

`turul-rpc 0.1` advertises JSON-RPC 2.0 with **one documented departure**: incoming requests with `"id": null` are rejected as `Invalid Request` (`-32600`). The spec permits null id with a discouragement note. The strict posture is **inherited from `turul-mcp-json-rpc-server 0.3.38`** — relaxing it in the shim release would be a behaviour change. A v0.2 candidate is to surface a permissive codec-level type for callers who need null-id requests. See `turul-rpc/docs/adr/002-json-rpc-2-compliance.md`.

### Documentation

- **Lambda eager handler init** (`turul-mcp-aws-lambda`): documented and exemplified building `LambdaMcpHandler` eagerly in `main()` before the Lambda runtime hand-off, avoiding request-path lazy initialization for fan-out-sensitive workloads. Removed `static OnceCell<LambdaMcpHandler>` patterns from `examples/lambda-mcp-server`, `examples/lambda-mcp-server-streaming`, and `examples/middleware-auth-lambda`; each example now builds the handler once in `main()` and `move`-captures it into a small `service_fn(move |req| lambda_handler(handler.clone(), req))` wrapper. Updated `crates/turul-mcp-aws-lambda/README.md` quick-start and "Custom Dispatch with `run_streaming_with()`" sections to match. The existing `LambdaMcpServerBuilder::build().await?` followed by `server.handler().await?` already performs full eager init (DDB session storage, server-state-storage, server build, tool/resource registration, session cleanup spawn, dynamic-tools sync, cold-start task recovery) — no new API is added. Per-request `info!` logging in example handlers dropped to `debug!` to avoid CloudWatch flood at production traffic. Closes #15. See [ADR-024](docs/adr/024-lambda-eager-handler-init.md).

## [0.3.38] - 2026-05-03

### Fixed

- **SSE GET 4xx hot-loop on streamable HTTP transport** (`turul-mcp-client`): `HttpTransport`'s SSE listener task previously treated every non-2xx GET response identically — `warn!` then sleep 5s + `continue` — producing an infinite retry loop against servers that legitimately reject the request (e.g. an MCP server with `strict_lifecycle(true)` returning HTTP 400 for a GET issued after session termination). The listener now distinguishes status classes:
  - **4xx → terminal**: clear the cached `Mcp-Session-Id` (only if it still matches what the failing GET sent — see CAS note below), emit `ServerEvent::Error("SSE GET rejected with HTTP <status> — listener exiting")`, and exit the spawned task. The cache clear ensures the caller's next `initialize` POST goes out without a stale session header (mirrors the canonical POST-404 recovery in `McpClient::send_request_raw`).
  - **5xx and other non-2xx → transient**: existing `warn!` + 5s sleep + retry behavior is preserved.

  Caller contract: on terminal SSE GET 4xx the listener exits cleanly; the caller may then re-run its normal `initialize` / `start_event_listener` flow. No new `ServerEvent` variants, no public API additions, no extension of `is_session_expired()` (which remains 404-only). The legacy `transport/sse.rs` is unchanged in this slice.

### Note

Surfaced by a Lambda MCP client logging repeated `"SSE stream error: error decoding response body"` followed by `"SSE connection lost, attempting to reconnect..."` against an API Gateway-fronted server with `strict_lifecycle(true)`. The 29s API GW idle timeout killed the SSE stream; the listener re-issued GET, which 400'd, and the loop never terminated. Four regression tests in `tests/sse_terminal_4xx.rs` lock in: terminal-on-400 with cache clear, transient-on-503, listener-works-without-session-id (stateless mode guard), and compare-and-swap on cache clear (does not clobber a fresher session).

The CAS detail matters because `McpClient::connect()` spawns the SSE listener **before** running `initialize_session()` (see `client.rs:135-229`). The two race: the listener may build a GET while `session_id` is still `None`, then `initialize` writes a real session ID into the cache, then the in-flight GET 4xx's. An unconditional cache clear would clobber the just-initialized session and break every subsequent POST. The fix snapshots the session header sent at request-build time and only clears the cache if it still matches that snapshot — preserving the strict-lifecycle bug-fix semantics (snapshot==current==Some(stale) → clear) while leaving a fresher value alone (snapshot=None, current=Some(new) → no-op).

## [0.3.37] - 2026-04-24

### Fixed

- **HTTP/2 connection drop detection** (`turul-mcp-client`): `HttpTransport` now configures `reqwest`'s h2 keepalive PINGs (`http2_keep_alive_interval = 30s`, `http2_keep_alive_timeout = 10s`, `http2_keep_alive_while_idle = true`) on both `new()` and `with_config()` construction paths. Without these, a connection silently dropped by the server or an intermediary (API Gateway ~350s idle, NAT, ALB) looks alive to the client pool until the next request — which then pays the full reconnect cost. PING keepalives surface the drop proactively so idle pooled connections either stay alive or fail fast and reconnect before a user-facing request uses them. No-op on h1-only backends (ALPN-negotiated h1 connections don't engage h2 keepalive state).

### Note

Values chosen as conservative defaults: 30s interval detects drops well before typical intermediary idle windows without being wasteful (~10 bytes per PING). 10s timeout halves reqwest's default 20s for faster fail-over on flaky paths. `while_idle = true` is the load-bearing bit — it keeps pooled idle connections being probed, which is precisely where silent-drop bimodality manifests. No new `ConnectionConfig` fields were added; if tuning becomes necessary it will land alongside other pending 0.4 surface changes.

## [0.3.36] - 2026-04-24

### Changed

- **`turul-mcp-client` now compiled with `reqwest/http2` feature**: reqwest auto-negotiates HTTP/2 via ALPN when the backend advertises `h2`. For servers that only speak HTTP/1.1, ALPN falls back to h1 — no behavior change. For h2-capable backends (AWS API Gateway, ALB, CloudFront, most modern HTTPS servers), concurrent `call_tool` invocations on one `Arc<McpClient>` are now multiplexed over a single TLS connection instead of opening N separate h1 connections. Resolves #13.

### Testing

- `tests/http2_feature.rs`: compile-time regression test that fails if a future `Cargo.toml` edit accidentally disables `reqwest/http2`.

### Note on validation

This change enables h2 at the dependency layer; the wire-level negotiation is handled entirely by reqwest + rustls ALPN. End-to-end validation (latency improvement on concurrent fan-out against h2-capable backends) is owned by downstream consumers — no specific latency claim is attached to this release. See #13 for the measurement plan and expected behavior.

## [0.3.35] - 2026-04-24

### Fixed

- **`ConnectionConfig` fields now honored** (`turul-mcp-client`): `HttpTransport::with_config` previously advertised six configuration fields but consumed only three (`user_agent`, `follow_redirects`, `headers`). `max_redirects`, `pool_settings.max_idle_per_host`, and `pool_settings.idle_timeout` were silent no-ops — callers set them and `reqwest` defaults applied instead. These three are now wired through to `reqwest::ClientBuilder` (`Policy::limited`, `pool_max_idle_per_host`, `pool_idle_timeout`).

### Deprecated

- **`ConnectionConfig::keep_alive`** and **`PoolConfig::max_lifetime`** (`turul-mcp-client`): no reqwest equivalent. `reqwest` exposes `tcp_keepalive(Option<Duration>)`, not a boolean, and has no per-connection max-lifetime API. Both fields will be removed in 0.4. Callers who do not reference them are unaffected.

### Changed

- **`PoolConfig::default().max_idle_per_host`** raised from 5 to 32 (`turul-mcp-client`): the previous default was silently ignored (reqwest's internal default `usize::MAX` applied). Now that the field is honored, the previous default would cap callers at 5 idle connections per host — a regression for fan-out workloads. 32 matches typical HTTP client sizing; callers can still set their own value.

### Note

This release fixes `ConnectionConfig` API truthfulness only. It does not change HTTP/2 support, connection protocol negotiation, or any other transport-layer behavior. A separate investigation (#13) is evaluating whether enabling `reqwest/http2` measurably affects cold-path tail latency; no decision has been made on that feature.

## [0.3.34] - 2026-04-21

### Fixed

- **DynamoDB read-your-writes on critical paths** (`turul-mcp-session-storage`, `turul-mcp-server-state-storage`): Added `consistent_read(true)` to the DynamoDB read sites that must observe just-written values across instances. Eventual-consistency reads on these paths could cause cold-start Lambda instances to miss sessions, session state, persisted events, or fingerprints written by other instances — breaking MCP SSE resumability and the `initialize` handshake.
  - `get_session`, `set_session_state` read-before-write, `store_event` session-exists check, and `store_event` max-eventId query (visibility; races still handled by the existing conditional `PutItem` + `MAX_RETRIES` loop).
  - `get_fingerprint` — cold-start instance must observe the latest fingerprint.

### Added

- **Storage contract regression tests** (`#[ignore = "requires DynamoDB"]`): `read_your_writes_contract` (session, state, event-replay) and `read_your_writes_contract_fingerprint`. Classified as storage contract regression tests; documented that DynamoDB-Local / LocalStack does not reliably reproduce AWS eventual reads, so passing locally does not prove AWS consistency correctness.

## [0.3.33] - 2026-04-21

### Changed

- **`Transport` trait — `&self` on hot-path methods** (`turul-mcp-client`): `connect`, `disconnect`, `send_request`, `send_request_with_headers`, `send_notification`, `send_delete`, `set_session_id`, `clear_session_id`, `start_event_listener`, and `health_check` now take `&self`. `McpClient::transport` is now `Arc<BoxedTransport>` — the outer `tokio::sync::Mutex` that serialized every request has been removed.

### Fixed

- **Concurrent client requests no longer serialize** (`turul-mcp-client`): N parallel `call_tool` / `list_tools` / etc. on one `Arc<McpClient>` now run in parallel through `reqwest`'s internal connection pool. Before: total wall time ≈ Σ per-call latency (Mutex-serialized). After: wall time ≈ max per-call latency.

### Breaking

- External implementors of `turul_mcp_client::transport::Transport` must change `&mut self` to `&self` on the listed methods and move any bare-mutable state into interior-mutable wrappers (`Atomic*` / `parking_lot::Mutex`). The stock `HttpTransport` and `SseTransport` already use interior mutability on all hot-path state.

## [0.3.32] - 2026-04-15

### Fixed

- **Client session retry on -32031**: `McpClient::call_tool()` (and all request methods) now detect JSON-RPC error code `-32031` ("Session not initialized") and automatically disconnect, reconnect, and retry once. Fixes cold-start race condition where `notifications/initialized` hasn't been processed before the first request arrives — especially visible on Lambda behind API Gateway.

### Added

- **`McpClientError::is_session_not_initialized()`**: Detects session-not-initialized errors by code (-32031) or message content.

## [0.3.31] - 2026-03-30

### Fixed

- **SSE replay**: No replay without `Last-Event-ID` — reverted bounded replay that caused duplicate notifications on API Gateway timeout reconnections. With `Last-Event-ID`: exact resume. Without: live events only.
- **Dead SSE connections**: Removed immediately on send failure, delivery falls back to next live connection. `has_connections()` now ignores closed senders.
- **DynamoDB event ID monotonic**: Conditional write (`attribute_not_exists`) with retry prevents duplicate event IDs across Lambda cold starts.
- **DynamoDB timestamp read**: Fixed numeric millis read (was parsing as RFC3339 string, always fell back to `Utc::now()`).
- **Distributed session targeting**: `broadcast_event()` enumerates targets from `storage.list_sessions()` for Custom events. `dispatch_custom_event()` for per-session delivery without cache dependency.
- **SessionEventDispatcher**: Guaranteed notification persistence on request path. `broadcast_event()` returns `Result` — dispatcher failures propagate.
- **Initialize live fingerprint**: Dynamic mode uses `ToolRegistry::fingerprint()` for new sessions, not build-time static.
- **DynamoDB `get_active_entities`**: Removed `entityId` from filter expression (DynamoDB rejects sort keys in filters).

### Added

- **`ToolChangeNotifier` trait**: Awaitable callback for restart/redeploy fingerprint mismatch notifications, backed by `SessionManager::dispatch_custom_event()`.
- **`dispatch_custom_event()`**: Storage-backed per-session event dispatch, not cache-gated.
- **`SessionEventDispatcher` trait**: Awaitable dispatcher on `SessionManager` for guaranteed Custom event persistence.
- **ADR-023 updates**: Distributed session targeting, session-backed event sequencing future consideration.

## [0.3.30] - 2026-03-29

### Fixed

- **DynamoDB `get_active_entities` filter** (`turul-mcp-server-state-storage`): Removed `entityId` (sort key) from `filter_expression` — DynamoDB rejects primary key attributes in filter expressions. Now uses application-level filtering.
- **Restart/redeploy notification persistence** (`turul-http-mcp-server`): Fingerprint mismatch in `validate_session_exists()` now emits `notifications/tools/list_changed` through the `ToolChangeNotifier` → `SessionManager` → dispatcher architecture. Failure propagates (500), not warn-and-continue.
- **DynamoDB TTL defaults** (`turul-mcp-session-storage`): Session and event TTL defaults increased from 5 to 30 minutes.

### Added

- **`ToolChangeNotifier` trait** (`turul-http-mcp-server`): Awaitable callback for restart/redeploy fingerprint mismatch notifications. Implemented by the server layer via `SessionManager::send_event_to_session()`.
- **`send_event_to_session()` with dispatcher** (`turul-mcp-server`): Per-session event dispatch with guaranteed persistence for Custom events. Retains NotFound error for missing sessions.

## [0.3.29] - 2026-03-29

### Added

- **SessionEventDispatcher** (`turul-mcp-server`): Awaitable dispatcher trait on `SessionManager` for guaranteed notification persistence on the request path. Custom events are persisted via `StreamManager::broadcast_to_session()` before `broadcast_event()` returns. Installed by the runtime (HTTP server, Lambda).
- **Mandatory persistence enforcement**: `broadcast_event()` returns `Result<(), String>` for Custom events. `broadcast_notification()` returns `Result<(), ToolRegistryError::NotificationFailed>`. `activate_tool()`, `deactivate_tool()`, `check_for_changes()` propagate dispatcher failures — no silent success when mandatory persistence fails.
- **Live registry fingerprint for new sessions**: In Dynamic mode, `SessionAwareInitializeHandler` reads `ToolRegistry::fingerprint()` instead of the build-time static value. New sessions after runtime tool mutations get the correct baseline — no spurious mismatch notification.

### Fixed

- **DynamoDB error observability** (`turul-mcp-server-state-storage`): `dynamo_err_debug()` uses `{:?}` (Debug) format instead of `{}`  (Display) for AWS SDK errors, surfacing error code, message, HTTP status, and request ID instead of generic "service error".
- **SSE bridge narrowed to observer-only**: The detached bridge task no longer persists or delivers `SessionEvent::Custom` events — the awaited dispatcher handles that on the request path. Eliminates duplicate persistence.

### Changed

- **BREAKING: `broadcast_event()` returns `Result`**: Callers that previously ignored the return value of `SessionManager::broadcast_event()` must now handle the `Result<(), String>` return for Custom events. Non-custom events always return `Ok(())`.

## [0.3.28] - 2026-03-29

### Fixed

- **Non-deterministic tool fingerprint** (`turul-mcp-server`): `compute_tool_fingerprint()` now canonicalizes JSON (recursive key sorting) before FNV hashing.

## [0.3.27] - 2026-03-29

### Changed

- **BREAKING: Default features reduced** (`turul-mcp-server`): Default features now `["http", "sse"]` only. SQLite, PostgreSQL, and DynamoDB backends are opt-in via `features = ["sqlite"]`, `features = ["postgres"]`, `features = ["dynamodb"]`. This significantly reduces compile time and binary size for projects that only need in-memory storage.
- **Backend features forward to all storage crates** (`turul-mcp-server`): `sqlite`/`postgres`/`dynamodb` features now forward to both `turul-mcp-session-storage` AND `turul-mcp-task-storage` (previously only session-storage).
- **Unified backend features** (`turul-mcp-server`): `sqlite`/`postgres`/`dynamodb` features use weak dependency forwarding (`?/`) to also enable backends on `turul-mcp-server-state-storage` when `dynamic-tools` is active. No separate compound features needed.
- **Lambda backend features** (`turul-mcp-aws-lambda`): Added `sqlite`, `postgres` forwarding features.

### Migration

If you previously depended on `turul-mcp-server` without specifying features and used SQLite, PostgreSQL, or DynamoDB backends, add the backend feature explicitly:

```toml
# Before (backends included by default)
turul-mcp-server = "0.3.26"

# After (backends opt-in)
turul-mcp-server = { version = "0.3.27", features = ["sqlite"] }
```

## [0.3.26] - 2026-03-29

### Fixed

- **Non-deterministic tool fingerprint** (`turul-mcp-server`): `compute_tool_fingerprint()` now canonicalizes JSON (recursive key sorting) before hashing. HashMap iteration order in `ToolSchema.properties`, `ToolSchema.additional`, and nested `JsonSchema.properties` caused different Lambda instances to compute different fingerprints for the same tool set, triggering spurious mismatch cycles on every cold start.

## [0.3.25] - 2026-03-29

### Added

- **Dynamic tool activation** (`turul-mcp-server`): `ToolChangeMode::Dynamic` enables runtime `activate_tool()`/`deactivate_tool()` with MCP-compliant `notifications/tools/list_changed`. Requires `dynamic-tools` feature.
- **ToolRegistry** (`turul-mcp-server`): Live registry for precompiled tools with `RwLock<ToolState>`, fingerprint tracking, and cross-instance coordination via `ServerStateStorage`.
- **ServerStateStorage** (`turul-mcp-server-state-storage`): New crate with InMemory, SQLite, PostgreSQL, DynamoDB backends for cross-instance tool state coordination.
- **Lambda dynamic tools** (`turul-mcp-aws-lambda`): `tool_change_mode()` and `server_state_storage()` on `LambdaMcpServerBuilder`. Request-time change detection with configurable TTL (`TURUL_TOOL_CHECK_TTL_SECS`, default 10s).
- **Client tool change notifications** (`turul-mcp-client`): `refresh_tools()`, cached tool lists, `notifications/tools/list_changed` auto-invalidation.
- **Dynamic tools example**: `examples/dynamic-tools-server` and `examples/dynamic-tools-test-client`.

### Fixed

- **POST SSE notification replay** (`turul-http-mcp-server`): Removed event replay from POST SSE responses — connection is registered before dispatch, so all events are delivered live. Prevents duplicate notification delivery.
- **Derive macro zero-config output preservation** (`turul-mcp-derive`): `#[tool(output = Type)]` without `name`/`description` now correctly preserves the output type via `extract_tool_meta_partial()`. Previously, the fallback path discarded all attributes.
- **OAuth dev-deps** (`turul-mcp-oauth`): Migrated to workspace dependency references. Updated `rsa` to 0.10, `jsonwebtoken` to 10 with `rust_crypto` feature.
- **Test suite MCP handshake** (tests): Added missing `notifications/initialized` to all E2E test suites (prompts, resources, elicitation, roots, sampling, session validation).

### Changed

- **Workspace dependency rule**: All crate dependencies must use `workspace = true` references (added to CLAUDE.md).
- **reqwest workspace default**: `default-features = false` at workspace level; crates opt-in to features individually.

## [0.3.24] - 2026-03-21

### Fixed

- **MCP client Accept header** (`turul-mcp-client`): POST requests now send `Accept: application/json, text/event-stream` per MCP spec. Notifications also include Accept header.
- **MCP client SSE POST responses** (`turul-mcp-client`): Client can now parse `text/event-stream` responses to POST requests instead of rejecting them.
- **MCP client session ID optional** (`turul-mcp-client`): Client no longer hard-fails when server doesn't return `Mcp-Session-Id` — stateless sessions are spec-valid.
- **MCP client protocol version enforcement** (`turul-mcp-client`): Client rejects servers that negotiate unsupported protocol versions.
- **MCP client 404 re-initialization** (`turul-mcp-client`): HTTP 404 triggers session reset, clears stale session ID from transport, and re-initializes.
- **MCP client JSON-RPC error preservation** (`turul-mcp-client`): Error frames pass through transport preserving code/message/data instead of flattening to opaque strings.
- **MCP client SSE double-routing** (`turul-mcp-client`): SSE path no longer duplicates events to both event channel and queue.
- **MCP client SSE data field parsing** (`turul-mcp-client`): Accepts `data:` with or without space after colon per SSE spec.

### Changed

- **`call_tool()` return type** (`turul-mcp-client`): Returns `CallToolResult` instead of `Vec<ToolResult>` — preserves `is_error`, `structuredContent`, `_meta` fields. **Breaking:** callers need `.content` to get the previous `Vec<ToolResult>`.
- **`get_prompt()` return type** (`turul-mcp-client`): Returns `GetPromptResult` instead of `Vec<PromptMessage>` — preserves `description`, `_meta` fields. **Breaking:** callers need `.messages` to get the previous `Vec<PromptMessage>`.
- **`Transport` trait** (`turul-mcp-client`): Added required `clear_session_id()` method. **Breaking** for custom `Transport` implementations.

### Added

- **GET SSE listener for HttpTransport** (`turul-mcp-client`): `server_events: true` enables server-initiated requests/notifications over GET SSE stream.
- **Server request routing** (`turul-mcp-client`): JSON-RPC frames with `method` + non-null `id` are routed as `ServerEvent::Request` (not `Notification`) in both SSE and JSON stream paths.
- **`HttpTransport::with_config()`** (`turul-mcp-client`): Constructor that applies `ConnectionConfig` (custom headers, user-agent, redirect policy).
- **`TransportError::HttpStatus`** (`turul-mcp-client`): Structured error variant preserving HTTP status code.
- **Builder transport detection** (`turul-mcp-client`): `McpClientBuilder` defers transport construction to `build()` so `with_config()` works regardless of call order.
- **21 behavioral tests** (`turul-mcp-client`): Protocol compliance, regression, and wire-level tests using `StatefulMockTransport` and `wiremock`.

## [0.3.23] - 2026-03-20

### Fixed

- **`after_dispatch` middleware mutations silently discarded** (`turul-http-mcp-server`): `DispatcherResult` was cloned into middleware, mutated, then the original `JsonRpcMessage` returned unchanged — mutations now applied back via `apply_dispatcher_result()`.
- **`after_dispatch` middleware errors silently ignored** (`turul-http-mcp-server`): `let _ = execute_after(...)` swallowed `Err(MiddlewareError)` — errors now propagated through `map_middleware_error_to_jsonrpc()` with correct semantic error codes.

## [0.3.22] - 2026-03-16

### Fixed

- **SSE wire-format test compliance** (`tests`): Replaced `strip_prefix("data: ").unwrap_or(...)` workaround in `session_id_compliance` test with explicit Content-Type assertion — tests now branch on the response's declared Content-Type instead of silently accepting both SSE and JSON formats.
- **DynamoDB events table check** (`turul-mcp-session-storage`): `ensure_events_table_exists()` now skipped when `verify_tables` is false (table assumed to exist via CloudFormation/Terraform).

### Added

- **Content-Type negotiation policy** (`turul-http-mcp-server`): `StreamableHttpContext::should_use_sse()` — conservative method-level heuristic for combined `Accept: application/json, text/event-stream`. Non-streaming methods (`tools/list`, `resources/list`, etc.) return `application/json`; streaming-capable methods (`tools/call`, `sampling/createMessage`, `elicitation/create`) return `text/event-stream`.
- **Content-Type negotiation tests** (`tests`): 4 new tests asserting wire-format consistency for JSON-only, SSE-only, combined+tools/call, and combined+tools/list Accept patterns.
- **Test Compliance rule** (`CLAUDE.md`): Tests must assert wire-format compliance — never silently accept multiple formats.
- **ADR-006 amendment**: Documented Content-Type negotiation policy, its architectural limitations, and the per-tool metadata improvement path.

## [0.3.21] - 2026-03-16

### Fixed

- **Lambda `resources/read` handler missing by default** (`turul-mcp-aws-lambda`): HTTP server registered it unconditionally; Lambda only added it when resources were configured. Now registered in `new()` matching HTTP parity.
- **Lambda `resources/templates/list` registered unconditionally** (`turul-mcp-aws-lambda`): Was registered even with no template resources, unlike HTTP which only adds it conditionally. Removed from `new()`, now only added in `build()` when templates exist.
- **Strict lifecycle tests made explicit** (`turul-mcp-aws-lambda`): `build_strict_streaming_handler()` now explicitly sets `.strict_lifecycle(true)` instead of relying on the default.

### Added

- **Lambda handler parity tests** (`turul-mcp-aws-lambda`): `resources/read` registered-by-default test and `resources/templates/list` absent-without-templates test.

## [0.3.20] - 2026-03-16

### Fixed

- **P0: Lambda missing `notifications/initialized` handler** (`turul-mcp-aws-lambda`): Lambda server never registered `InitializedNotificationHandler`, making `strict_lifecycle: true` (default since v0.3.19) non-functional — clients could never complete the MCP handshake. Now registered identically to the HTTP server path.
- **P1: Lambda `tools/list` not session-aware** (`turul-mcp-aws-lambda`): `ListToolsHandler` in Lambda was constructed without session manager, bypassing strict lifecycle checks. Now uses `new_with_session_manager()` consistent with the HTTP server.
- **P1: Streamable HTTP notification race** (`turul-http-mcp-server`): `notifications/initialized` was processed asynchronously via `tokio::spawn`, returning 202 before `is_initialized` was set. If the client sent `tools/list` immediately after, the session would be rejected. Now processed synchronously for `notifications/initialized` specifically; other notifications remain async.

### Added

- **Lambda strict lifecycle E2E tests** (`turul-mcp-aws-lambda`): 4 new tests over `handle_streaming()` with `MCP-Protocol-Version: 2025-11-25` — full handshake, rejection before initialized (with `-32031` error code assertions), immediate post-initialized race proof, and lenient mode fallback.

## [0.3.19] - 2026-03-15

### Changed

- **Strict MCP lifecycle is now the default** (`turul-mcp-server`, `turul-mcp-aws-lambda`): Both `McpServerBuilder` and `LambdaMcpServerBuilder` now default to `strict_lifecycle: true`, requiring clients to send `notifications/initialized` after `initialize` before any other operations. This matches the MCP 2025-11-25 spec. Use `.strict_lifecycle(false)` for legacy clients that skip the notification.

### Fixed

- **Integration tests now perform full MCP handshake** — `mcp_behavioral_compliance`, `session_id_compliance`, and `sse_progress_delivery` tests updated to send `notifications/initialized` after `initialize`.

## [0.3.18] - 2026-03-15

### Changed

- **`create_tables_if_missing` replaced with `verify_tables` + `create_tables`** (`turul-mcp-session-storage`, `turul-mcp-task-storage`): All 6 storage config structs (SQLite, PostgreSQL, DynamoDB × session + task) now use two granular flags. `verify_tables: false` (default) skips all startup verification — eliminates ~1,884 DynamoDB API calls/hour per Lambda server. `create_tables: true` creates tables when missing (only when `verify_tables: true`). **Breaking:** default changed from auto-create to skip-all. For first-time setup, use `verify_tables: true, create_tables: true`.

### Fixed

- **SQLite/PostgreSQL session storage now respect table verification flag** — previously called `migrate()` unconditionally, ignoring the config flag.

## [0.3.17] - 2026-03-15

### Added

- **Custom struct input parameter schema via schemars** (`turul-mcp-derive`): Unknown types in `#[mcp_tool]` parameters (e.g., `Vec<ObserverPoint>`, `MyStruct`) now use `schemars::schema_for!()` to generate correct JSON Schema at runtime instead of falling back to `"type": "string"`. Requires the parameter type to derive `schemars::JsonSchema`. This fixes `Vec<CustomStruct>` parameters generating `{"type": "array", "items": {"type": "string"}}` — they now correctly produce `{"type": "array", "items": {"type": "object", "properties": {...}}}`.

## [0.3.16] - 2026-03-15

### Added

- **Fixed-size array `[T; N]` support in `#[mcp_tool]` schema generation** (`turul-mcp-derive`): `type_to_schema` now handles `[f64; 3]`, `[String; 2]`, `[i32; 4]`, etc. — generating `{"type": "array", "items": ..., "minItems": N, "maxItems": N}` instead of silently falling back to `"type": "string"`. Also handles `Option<[T; N]>`.
- **`with_min_items()` / `with_max_items()` builder methods** (`turul-mcp-protocol-2025-11-25`): `JsonSchema::Array` now supports min/max item count constraints via builder chain.

### Fixed

- **E2E test expected 401 instead of 404 for nonexistent session** (`streamable_http_e2e.rs`): Updated `test_strict_lifecycle_enforcement_over_streamable_http` to expect 404 per MCP 2025-11-25 spec (regression from v0.3.14 session-404 fix).

## [0.3.15] - 2026-03-14

### Added

- **`.icons()` builder method** (`turul-mcp-server`, `turul-mcp-aws-lambda`): Both `McpServerBuilder` and `LambdaMcpServerBuilder` now support `.icons(vec![...])` for setting server icons displayed by MCP clients (e.g., Claude Desktop). Use `Icon::new("https://...")` for URL icons or `Icon::data_uri("image/svg+xml", "<base64>")` for embedded data URIs.
- **`Icon` in protocol prelude** (`turul-mcp-protocol-2025-11-25`): `Icon` is now re-exported via `turul_mcp_server::prelude::*` for convenience.

## [0.3.14] - 2026-03-14

### Fixed

- **Stale/terminated sessions now return 404 per MCP spec** (`turul-http-mcp-server`): `StreamableHttpHandler` previously returned 401 Unauthorized for nonexistent or terminated session IDs. MCP 2025-11-25 requires 404 Not Found so clients know to create a fresh session (not re-authenticate). Missing `Mcp-Session-Id` header (no session ID at all) still returns 401. Storage backend errors return 500.

## [0.3.13] - 2026-03-13

### Changed

- **CORS headers centralized behind constants** (`turul-http-mcp-server`): All CORS header values (`Allow-Methods`, `Allow-Headers`, `Expose-Headers`, `Max-Age`) are now defined as `pub(crate)` constants in `cors.rs`. Inline CORS headers removed from `options_response()`, `StreamableHttpHandler` OPTIONS handler, and `sse_response_headers()`. `CorsLayer::apply_cors_headers()` in `server.rs` is now the single source of truth.
- **`enable_cors = false` now fully respected** (`turul-http-mcp-server`): Previously, inline OPTIONS handlers leaked partial CORS headers even when CORS was disabled. Now `enable_cors = false` produces zero CORS headers on all responses.

### Removed

- **`CorsLayer::apply_cors_headers_for_origin()`** (`turul-http-mcp-server`): Removed — was never wired into the server request pipeline and would be overwritten by the wildcard `apply_cors_headers()` in `server.rs`. For origin-restricted CORS, configure at the reverse proxy layer.
- **`sse_response_headers()`** (`turul-http-mcp-server`): Removed — was never called by the framework. SSE responses are built inline by `StreamableHttpHandler` and `SessionMcpHandler`.
- **Orphan test files** (`turul-http-mcp-server`): Deleted `http_transport_tests.rs` and `sse_tests.rs` — not compiled (missing from `tests/mod.rs`) with 93 compilation errors against the current API.

## [0.3.12] - 2026-03-12

### Fixed

- **CORS: expose `Mcp-Session-Id` header for browser MCP clients** (`turul-http-mcp-server`): Browser-based MCP clients couldn't read the `Mcp-Session-Id` response header because CORS didn't expose it. Added `Access-Control-Expose-Headers: Mcp-Session-Id`, added `Mcp-Session-Id` to `Access-Control-Allow-Headers`, and added `DELETE` to `Access-Control-Allow-Methods` for session teardown. Applies to both wildcard and origin-specific CORS configurations.

## [0.3.11] - 2026-03-09

### Added

- **`run_streaming_with()` custom dispatch** (`turul-mcp-aws-lambda`): Accepts a custom `Fn(Request) -> Future<Response>` closure for Lambda streaming, with the same completion-invocation handling as `run_streaming()`. Use this when you need pre-dispatch logic (e.g., `.well-known` routing) that runs before the MCP handler. Fixes completion-invocation ERROR logs for custom dispatch patterns; does not claim to resolve all Lambda streaming timeout behavior.
- **Prelude re-exports**: `run_streaming` and `run_streaming_with` are now available via `turul_mcp_aws_lambda::prelude::*`.

### Changed

- **`lambda-mcp-server-streaming` example**: Refactored from raw `lambda_http::run_with_streaming_response(service_fn(...))` to `turul_mcp_aws_lambda::run_streaming()`, demonstrating the framework's recommended streaming entry point.

## [0.3.10] - 2026-03-07

### Changed

- **`JwtValidator::new()` now requires audience** (`turul-mcp-oauth`): `JwtValidator::new(jwks_uri, audience)` — audience is a mandatory parameter per MCP spec requirement that servers MUST validate token audience. The optional `with_audience()` method has been removed.
- **`ProtectedResourceMetadata::new()` is now fallible** (`turul-mcp-oauth`): Returns `Result<Self, OAuthError>`. Validates `resource` and `authorization_servers` URIs using `url::Url` — requires http/https scheme, authority present, no fragment. Empty AS list rejected.
- **`oauth_resource_server()` is now fallible** (`turul-mcp-oauth`): Returns `Result<..., OAuthError>`. Enforces exactly one authorization server in metadata (no silent `[0]` fallback). Auto-wires audience from `metadata.resource` and issuer from single AS.

### Added

- **Scope in `WWW-Authenticate`** (`turul-mcp-oauth`): When `scopes_supported` is configured on metadata, challenge responses include `scope="scope1 scope2"` per RFC 6750 §3.
- **`Cache-Control: no-store`** (`turul-http-mcp-server`): All 401/403 challenge responses include `Cache-Control: no-store` per OAuth 2.1 §5.3. Applied in both Streamable HTTP and legacy transports.
- **Canonical URI validation** (`turul-mcp-oauth`): `ProtectedResourceMetadata::new()` validates resource and AS URIs — absolute URI with http/https scheme, authority required, no fragment allowed. New error variants: `OAuthError::InvalidResourceUri`, `OAuthError::InvalidConfiguration`.
- **Single-AS issuer enforcement** (`turul-mcp-oauth`): `oauth_resource_server()` rejects metadata with multiple authorization servers, preventing misconfigured deployments.

## [0.3.9] - 2026-03-06

### Added

- **Lambda streaming event classification** (`turul-mcp-aws-lambda`): Three-way classification of raw Lambda runtime payloads via `classify_runtime_event()` — distinguishes API Gateway events, streaming completion invocations, and unrecognized payloads. Prevents ERROR logs and CloudWatch Lambda Error metrics from completion invocations.
- **`run_streaming()` public API**: Replaces `lambda_http::run_with_streaming_response()` for MCP Lambda servers. Gracefully acknowledges completion invocations (200 + `debug` log) and unrecognized payloads (200 + `warn` log) instead of failing deserialization.
- **Testable surfacing contract**: `handle_runtime_payload()` returns typed `HandleResult { response, event_type }` for observability; `event_log_level()` maps event types to tracing levels — both independently testable without log capture.
- **OAuth resource server foundation** (`turul-http-mcp-server`): Bearer token middleware, route registry, request-scoped extensions on `SessionContext` for auth claims propagation.
- 25 classification/action-path/contract tests with `include_str!` fixture files for API Gateway v1/v2, streaming completion variants, and precedence edge cases.

### Fixed

- **Benchmark compilation**: `SessionContext` struct initializers in `performance-testing` benchmarks updated for new `extensions` field.

## [0.3.8] - 2026-03-05

### Fixed

- **Client streaming response forwarding** (P1): Server-initiated requests (`sampling/createMessage`, `elicitation/create`) now receive JSON-RPC responses back from the client callback. Previously responses were logged and discarded, causing servers to hang indefinitely. Architecture: `StreamHandler` → response channel → consumer task → `transport.send_notification()`. See [ADR-020](docs/adr/020-client-response-forwarding-architecture.md).
- **HTTP transport event classification**: Server-initiated requests (with both `method` and `id`) were misclassified as notifications. Fixed classification order: `method+id` → Request, `method` only → Notification, `id` only → Response.
- **`json_schema_derive.rs` `Option<T>` type-schema**: `generate_field_schema()` now uses `segments.last()` instead of `get_ident()` to handle generic types. `Option<u32>` correctly generates `integer` schema (was falling through to `string`). `is_option_type()` fixed to use `segments.last()` for qualified path support (`std::option::Option<T>`).

### Added

- **Resource `title` attribute**: All three macro paths (`#[derive(McpResource)]`, `#[mcp_resource]`, `resource!{}`) now support `title = "..."` attribute. `HasResourceMetadata::title()` returns the configured value.
- `ServerEvent::Response` variant for distinguishing id-only SSE frames (responses to client-originated requests) from server-initiated requests. `StreamHandler` ignores these — they are handled by the normal request/response matching path.
- Null/missing `id` guard: Server requests without a valid `id` invoke the callback but do not emit a response (per JSON-RPC 2.0 spec).
- 11 new tests covering client response forwarding pipeline (unit + integration + mock transport).

## [0.3.7] - 2026-03-04

### Added

- **Tool annotations macro support**: `#[derive(McpTool)]`, `#[mcp_tool]`, and `tool!{}` now support `read_only`, `destructive`, `idempotent`, `open_world`, `title`, and `annotation_title` attributes — generates `ToolAnnotations` with camelCase JSON keys (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`) per MCP 2025-11-25
- `title` attribute on all three macro paths sets `Tool.title` (via `HasBaseMetadata`); `annotation_title` sets `ToolAnnotations.title` independently
- Boolean annotation type validation: `#[mcp_tool]` rejects wrong types (e.g., `read_only = "true"`) with a compile error

### Fixed

- Terminated sessions (after `DELETE /mcp`) now correctly reject subsequent POST and GET requests in both Streamable HTTP and legacy JSON transports

## [0.3.6] - 2026-03-03

### Fixed

- `#[mcp_tool]` and `#[derive(McpTool)]`: `Option<bool>`, `Option<u32>`, `Option<f64>`, `Vec<T>`, and `Option<Vec<T>>` parameters now generate correct JSON Schema types in `tools/list` input schemas (was incorrectly advertising `"type": "string"` for all generic-arg types)
- Fully-qualified paths (`std::option::Option<T>`, `std::vec::Vec<T>`) now correctly detected across all `is_option_type` checks

## [0.3.5] - 2026-03-03

### Added

- `McpClient::list_resource_templates()` and `list_resource_templates_paginated()` for `resources/templates/list` discovery

### Fixed

- `HttpTransport`: downgraded spurious session ID warning on `initialize` request from `warn!` to `debug!`

## [0.3.4] - 2026-03-03

### Fixed

- `HttpTransport::connect()` and `SseTransport::connect()` no longer send OPTIONS/HEAD pre-flight requests that fail with 405 (direct servers) or 502 (Lambda streaming servers) — connectivity failures now surface at `initialize` time instead of preflight time, matching MCP Inspector behavior
- `#[mcp_tool]` function-attribute macro: `Option<T>` parameters are now correctly excluded from the `required` array in the generated JSON schema (was incorrectly marking them as required unless `#[param(optional)]` was explicitly set)

### Changed

**DynamoDB Storage: camelCase Attribute Names (One-Way Migration):**
- New DynamoDB tables created by `turul-mcp-session-storage` and `turul-mcp-task-storage` now use camelCase attribute names (`sessionId`, `taskId`, `createdAt`, `lastActivity`, etc.) — aligning with DynamoDB convention
- Existing snake_case tables (`session_id`, `task_id`, `created_at`, etc.) are auto-detected via `describe_table()` key schema inspection and continue to work without any changes
- Per-table detection: session and events tables are detected independently, supporting mixed-convention deployments
- Read tolerance: non-key attributes written with either convention are readable via fallback lookup

**Rollback Contract (Breaking Storage Format):**
- This is a **one-way storage format change**. Once new tables are created with camelCase key schemas, pre-v0.3.4 code cannot read them (it has hardcoded snake_case key names)
- New code reads legacy snake_case tables: **Yes** (auto-detected)
- New code creates fresh tables with camelCase: **Yes**
- Old code reads legacy snake_case tables: **Yes** (unchanged)
- **Old code reads new camelCase tables: No — will fail**
- Rolling back to pre-v0.3.4 code after creating camelCase tables will break. Plan accordingly.

## [0.3.3] - 2026-03-01

### Fixed

- PostgreSQL task storage: `tasks.session_id` column type changed from `TEXT` to `VARCHAR(36)` to match `sessions.session_id` and `events.session_id`

## [0.3.2] - 2026-02-28

### Added

- `HasExecution` trait for per-tool task support declaration (follows `HasIcons` supertrait pattern)
- `task_support` attribute on `#[derive(McpTool)]` and `#[mcp_tool]` (`"optional"` | `"required"` | `"forbidden"`)
- `.execution()` builder method on `ToolBuilder`
- Build-time coherence guard rejects `taskSupport=required` without task runtime configured
- `tools/list` strips `execution` field when server has no tasks capability (truthful capability advertisement)
- `tools/call` with `params.task` returns `InvalidParameters` when server has no task runtime (was silent sync fallback)

### Changed

- **Breaking**: `HasExecution` added to `ToolDefinition` supertrait — manual tool impls must add `impl HasExecution for MyTool {}`

### Fixed

- `ToolDefinition::to_tool()` now populates `execution` field from trait (was hardcoded `None`)
- `tools/call` rejects task-augmented requests to tools that don't declare `task_support` (was silently accepted)

## [0.3.1] - 2026-02-28

### Fixed

- `ToolSchemaExt::from_schemars()` now handles schemars v1 nullable type arrays (`"type": ["string", "null"]`) and `anyOf`/null patterns for `Option<T>` fields
- `from_schemars()` enforces `type: "object"` root schema validation per MCP protocol requirements
- `from_schemars()` resolves `$ref` references through both `$defs` and `definitions` maps (merged, not first-hit)

## [0.3.0] - 2026-02-26

### Added

**MCP 2025-11-25 Protocol Support:**
- `turul-mcp-protocol-2025-11-25` crate with full spec compliance (127+ protocol tests)
- `turul-mcp-protocol` alias now re-exports 2025-11-25 types (ADR-015)
- `Icon` struct (`src`, `mime_type`, `sizes`, `theme`) on tools, resources, prompts, resource templates, and implementations
- `Task` struct with `task_id`, `TaskStatus` (`Working`/`InputRequired`/`Completed`/`Failed`/`Cancelled`), `created_at`/`last_updated_at`, `ttl`, `poll_interval`
- `ToolUse` and `ToolResult` content block variants
- `ToolExecution`, `ToolChoice`, `ToolChoiceMode` (`Auto`/`None`/`Required`)
- `TaskStatusNotification` and `ElicitationCompleteNotification`
- URL elicitation mode (`ElicitRequestURLParams`) alongside existing form mode
- `$schema` field on `ElicitationSchema`
- `tools` field on `CreateMessageParams` for sampling with tools
- `ModelHint { name }` struct (replaces closed enum)
- `Implementation` gains `description` and `website_url` fields
- Structured `TasksCapabilities` with `list`, `cancel`, `requests` sub-fields

**Task Storage (`turul-mcp-task-storage` crate):**
- `TaskStorage` trait with zero-Tokio public API
- `InMemoryTaskStorage` with state machine enforcement
- SQLite backend (`SqliteTaskStorage`) — optimistic locking, `julianday()` TTL, background cleanup
- PostgreSQL backend (`PostgresTaskStorage`) — `version` column optimistic locking, JSONB, partial index for stuck tasks
- DynamoDB backend (`DynamoDbTaskStorage`) — conditional writes, GSIs, native TTL, base64 cursors
- 11-function parity test suite shared across all backends
- Feature flags: `sqlite`, `postgres`, `dynamodb` (each opt-in with Tokio)

**Task Runtime & Executor:**
- `TaskExecutor` trait and `TokioTaskExecutor` in `turul-mcp-server`
- `CancellationHandle` for cooperative task cancellation
- `TaskRuntime` with `::new(storage, executor)`, `::with_default_executor(storage)`, `::in_memory()` constructors
- Server handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result` (blocks until terminal per spec)
- Auto-capability advertisement via `McpServer::builder().with_task_runtime()`

**Task Examples:**
- `tasks-e2e-inmemory-server` — task-enabled MCP server with `slow_add` tool
- `tasks-e2e-inmemory-client` — full task lifecycle client (create, poll, cancel, result)
- `client-task-lifecycle` — task API demonstration
- `task-types-showcase` — print-only demo of Task, TaskStatus, TaskMetadata, CRUD types

**Lambda Examples:**
- `lambda-authorizer` — API Gateway REQUEST authorizer with wildcard methodArn for MCP Streamable HTTP

**README Testing Infrastructure:**
- `skeptic` crate for automated markdown code block testing
- README.md files validated as part of `cargo test` suite

### Changed

**Protocol Types (Breaking):**
- `CreateMessageResult` flattened — `role` and `content` at top level (no `message` wrapper)
- `Role` enum: only `User` and `Assistant` (removed `System` variant; system prompts use `systemPrompt` field)
- `ProgressNotificationParams.progress`: `f64` (was `u64`)
- `icon` fields renamed to `icons: Option<Vec<Icon>>` (singular string → plural object array)
- `HasIcon` trait renamed to `HasIcons`; `HasSamplingTools` trait added
- Notification method strings use underscores (`notifications/tools/list_changed`) per spec; JSON capability keys remain camelCase (`listChanged`)
- Default protocol version is 2025-11-25 everywhere; backward-compat 2025-06-18 paths annotated with `// Intentional`

**Test Infrastructure:**
- 1,560+ workspace tests passing, 98 doctests, zero warnings
- Test binaries reduced from 155 to 43 via consolidation (Phase F)
- Root integration tests: 39 → 8 binaries (5 consolidated in `tests/consolidated/` + 3 standalone)
- Sub-crate integration tests: 24 → 7 binaries (`tests/*/tests/all.rs` with `#[path]` imports)
- Derive crate integration tests moved to workspace root (2 binaries eliminated)

**Examples:**
- 58 active examples (up from 42+ in v0.1.0), 25 archived
- 12 core crates in workspace

**Documentation:**
- README narrative updated to reflect spec-pure protocol crate design
- All 20+ protocol crate README code examples tested and verified
- Documentation accuracy fixes across READMEs, ADRs, and compliance reports (repo URL, config field names, notification method strings, version references, port numbers)
- CHANGELOG duplicate `[0.2.0]` sections merged
- ADR-009 updated with `V2025_03_26` and `V2025_11_25` protocol versions
- ADR-004 status updated from CRITICAL to Accepted (Implemented)
- Stale MIGRATION_0.2.1.md references removed workspace-wide

### Fixed

- Sampling server README: removed `System` role, fixed `ModelHint` to object form, corrected snake_case JSON fields to camelCase
- Session storage README: corrected config field names (`session_timeout_minutes`, `database_url`, `PostgresConfig`)
- Compliance reports marked as historical with accurate resolution status
- Client README compatibility list now includes 2025-11-25
- Protocol alias ADR updated from 2025-06-18 to 2025-11-25
- Notification method strings in ADR-005 and E2E test plan corrected to `list_changed`

## [0.2.1] - 2025-10-08

### Breaking Changes

**Schemars Integration (Detailed Schema Generation):**
- **BREAKING**: Tool output types MUST now derive `schemars::JsonSchema`
- **Impact**: Tools with custom output types generate detailed schemas with full property information
- **Migration**: Add `#[derive(JsonSchema)]` to all tool output types:
  ```rust
  use schemars::JsonSchema;

  #[derive(Serialize, Deserialize, JsonSchema)]  // Added JsonSchema
  struct MyOutput {
      result: f64,
      message: String,
  }
  ```
- **Benefit**: All tools now provide detailed schemas in `tools/list` with property names, types, and descriptions
- **Note**: `schemars` is already a workspace dependency - no Cargo.toml changes needed

**Framework Trait Reorganization (Protocol Crate Purity):**
- **BREAKING**: All framework traits moved from `turul-mcp-protocol` to `turul-mcp-builders::traits`
- **BREAKING**: `HasNotificationPayload::payload()` now returns `Option<Value>` (owned) instead of `Option<&Value>` (reference)
- **Impact**: Protocol crate is now 100% MCP spec-pure (no framework-specific code)
- **Migration**: Update imports to use preludes:
  ```rust
  // Before
  use turul_mcp_protocol::{ToolDefinition, ResourceDefinition};

  // After
  use turul_mcp_builders::prelude::*;  // or turul_mcp_server::prelude::*
  ```
- **Migration Guide**: See the breaking changes listed above for step-by-step migration instructions

### Fixed

**Critical Notification Payload Regression:**
- Fixed all notification types returning `None` for payloads (data loss bug)
- Base Notification now properly serializes `params.other` and `_meta`
- ProgressNotification now preserves progressToken, progress, total, message, _meta
- ResourceUpdatedNotification now preserves uri, _meta
- CancelledNotification now preserves requestId, reason, _meta
- All list-changed notifications now preserve _meta fields
- Added 18 comprehensive tests validating notification payload correctness

### Changed

**Framework Trait Locations:**
- Moved 10 trait hierarchies (~1200 LOC) from protocol to builders crate
- All protocol type implementations now in `turul-mcp-builders/src/protocol_impls.rs`
- Derive macros updated to generate correct trait signatures
- All examples and tests updated to use new import paths

## [0.2.0] - 2025-10-05

### Added

**MCP 2025-06-18 Specification:**
- Full compliance with MCP 2025-06-18 spec
- Session-Aware Resources: All resources now support `session: Option<&SessionContext>` parameter
- Sampling Validation Framework: `ProvidedSamplingHandler` for request validation
- SSE Streaming: Chunked transfer encoding with real-time notifications
- CLI Support: All test servers now support `--port` argument with dynamic binding
- Path Normalization: Traversal attack detection in roots validation
- Strict Lifecycle Mode: Optional strict session initialization enforcement

**Middleware System:**
- Complete middleware architecture for HTTP and Lambda transports
- `.middleware()` builder method on `McpServer` and `LambdaMcpServerBuilder`
- Transport-agnostic middleware execution (FIFO before dispatch, LIFO after)
- Session-aware middleware with `StorageBackedSessionView` and `SessionInjection`
- Error short-circuiting with semantic JSON-RPC error codes

**Middleware Examples:**
- `middleware-auth-server` - API key authentication (HTTP)
- `middleware-auth-lambda` - API key authentication (AWS Lambda)
- `middleware-logging-server` - Request timing and tracing
- `middleware-rate-limit-server` - Per-session rate limiting

**Testing Infrastructure:**
- Shared verification utilities (`tests/shared/bin/wait_for_server.sh`)
- Test server bin targets in all test packages (tools, prompts, resources, sampling, roots, elicitation)
- Comprehensive example verification suite (5 phases, 31 servers)
- Session lifecycle compliance: `notifications/initialized` in all e2e tests

### Changed

- **Resource Trait**: Updated `read()` signature to include session parameter
- **Tool Output**: Tools with `outputSchema` automatically include `structuredContent`
- **Error Handling**: Session lifecycle violations use `SessionError` type
- **Pagination**: Reject `limit=0` to prevent stalls
- **HTTP Transport**: Protocol-based routing (≥2025-03-26 uses streaming, ≤2024-11-05 uses buffered)
- SSE keepalives use comment syntax for better client compatibility
- DynamoDB queries use strongly consistent reads
- Lambda `LambdaMcpHandler` now cached globally (preserves DynamoDB client, StreamManager, middleware instances)
- Test packages updated to Rust edition 2024 and tokio version "1"
- Middleware stack execution order documented (FIFO/LIFO)

### Fixed

**Examples (4 bugs fixed):**
- pagination-server: Database unique constraint error (email generation duplicates)
- comprehensive-server: Missing resources and prompts registration
- audit-trail-server: SQLite connection URL missing protocol and create mode
- All 30/31 examples now verified working (96.8% passing, 1 skipped for PostgreSQL)

**Protocol & Core:**
- SSE resumability: Keepalive events preserve Last-Event-ID for proper reconnection
- MCP Inspector compatibility: Events use standard `event: message` format
- Lambda notifications: DynamoDB consistent reads fix race condition
- Lambda handler caching: Global `OnceCell` preserves handler instance (DynamoDB client, StreamManager, middleware) across invocations
- Tool output: Schema and runtime field names now consistent
- CamelCase: Proper acronym handling (GPS → gps, HTTPServer → httpServer)
- Lambda compilation: Fixed `LambdaError::Config` reference
- **TestServerManager**: Blocking wait for process termination, prevents zombie processes
- **Session Tests**: Correct response structure (`output` vs `value`)
- **Prompt Arguments**: Fix argument name mismatches in test expectations
- **MCP Inspector**: Enable compatibility with MCP Inspector and FastMCP clients
- **Zero-Config**: Correct output field expectations for derived tools
- **Borrow Checker**: Resolve errors in `roots_derive` macro

**Code Quality:**
- Fixed 14 collapsible_if clippy warnings using Rust 2024 let-chain syntax
- Fixed unused variable warnings in test suite
- Fixed useless type conversions in Lambda tests
- All clippy warnings addressed (100% clean workspace builds with `-D warnings`)

**Verification Infrastructure:**
- Scripts use deterministic 15s polling instead of fixed sleeps
- Pre-built binaries eliminate compilation timeouts
- SKIPPED tracked separately from PASSED (no hidden failures)
- Build errors properly diagnosed with detailed logs

### Examples
- Restored `roots-server` with clap CLI (108 lines, down from 512)
- Updated `elicitation-server` with multi-path data loading
- Updated `sampling-server` with dynamic port binding
- Updated `pagination-server` with proper SQLite URI (`?mode=rwc`)
- All 31 core examples verified and working

### Documentation

- README middleware section with examples and testing commands
- AGENTS.md middleware guidance with ADR 012 reference
- Doctests passing: turul-mcp-derive (25/25), turul-mcp-protocol (7/7)
- Complete verification run documented with bug fixes and runbook
- Middleware testing scripts: `test_middleware_live.sh` and Lambda examples
- Updated CLAUDE.md with session-aware patterns
- Updated EXAMPLES.md with validation results
- Added curl and jq to auto-approved commands
- Comprehensive test coverage documentation

### Tests

- 440+ unit tests passing (161 integration tests across 20 test suites)
- 30/31 examples verified (Phases 1-5: 100% passing)
- Middleware parity tests verify HTTP/Lambda consistency
- All critical functionality validated

## [0.1.0] - Initial Release

### Added
- Core MCP server framework
- Tool creation patterns (function, derive, builder, manual)
- Resource management with templates
- Prompt generation system
- Session management with multiple storage backends
- HTTP transport layer
- Client library
- Builder patterns
- AWS Lambda support
- 42+ working examples

[Unreleased]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.47...HEAD
[0.3.47]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.46...v0.3.47
[0.3.46]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.45...v0.3.46
[0.3.45]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.44...v0.3.45
[0.3.44]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.43...v0.3.44
[0.3.43]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.42...v0.3.43
[0.3.42]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.41...v0.3.42
[0.3.41]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.40...v0.3.41
[0.3.40]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.39...v0.3.40
[0.3.39]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.38...v0.3.39
[0.3.38]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.37...v0.3.38
[0.3.37]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.36...v0.3.37
[0.3.36]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.35...v0.3.36
[0.3.35]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.34...v0.3.35
[0.3.34]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.33...v0.3.34
[0.3.22]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.21...v0.3.22
[0.3.21]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.20...v0.3.21
[0.3.20]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.19...v0.3.20
[0.3.19]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.18...v0.3.19
[0.3.18]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.17...v0.3.18
[0.3.17]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.16...v0.3.17
[0.3.16]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.15...v0.3.16
[0.3.15]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.14...v0.3.15
[0.3.14]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.13...v0.3.14
[0.3.13]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.12...v0.3.13
[0.3.12]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.11...v0.3.12
[0.3.11]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.10...v0.3.11
[0.3.10]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.9...v0.3.10
[0.3.9]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.8...v0.3.9
[0.3.8]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aussierobots/turul-mcp-framework/releases/tag/v0.1.0
