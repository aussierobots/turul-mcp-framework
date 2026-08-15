# Client Features — MCP 2026-07-28

Covers `turul-mcp-client`'s obligations as an MCP client, and the client-facing
features a server must support. Column meanings and interop values: see
[README.md](README.md). Interop columns are `turul | python | typescript | go`.

For client rows the interop column means something specific and stronger than
elsewhere: **our client actually drove that method against a foreign server**.
Evidence comes from `scripts/interop-turul-client.sh`.

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

---

## 1. Elicitation and MRTR (SEP-2322)

2026-07-28 replaces server-initiated elicitation with a multi-round-trip
pattern: the server returns `resultType: "input_required"` with `inputRequests`,
and the client re-sends the **original** request with `inputResponses` plus the
verbatim `requestState`. `ElicitRequest` survives in the schema only as a member
of the `InputRequest` union embedded in that result — not as a standalone
held-open RPC.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `tools/call` may return `InputRequiredResult` instead of holding a stream open | MUST | Implemented | `c/turul-mcp-server/src/handlers/mod.rs` (`McpError::InputRequired`) | `mrtr_2026.rs::mrtr_round_trip_completes_the_original_call` | pass | — | — | — |
| Client re-sends the original request with `inputResponses` and verbatim `requestState` | MUST | Implemented | `c/turul-mcp-client/src/client.rs:1285` | `e2e_2026_real_server.rs::mrtr_round_trip_through_the_bilingual_client` | pass | — | — | — |
| MRTR applies to `resources/read` | MUST | Implemented | `client.rs:1322` | `e2e_2026_real_server.rs::mrtr_round_trip_on_resources_read_through_the_client` | pass | — | — | — |
| MRTR applies to `prompts/get` | MUST | Implemented | `client.rs:1358` | `e2e_2026_real_server.rs::mrtr_round_trip_on_prompts_get_through_the_client` | pass | — | — | — |
| Undeclared elicitation/sampling capability → `-32021` | MUST | Implemented | `c/turul-mcp-protocol-2026-07-28/src/lib.rs:477-482` | `mrtr_2026.rs::undeclared_capability_is_rejected_with_32021` | pass | — | — | — |
| Elicitation sub-capabilities (`form`/`url`) gate correctly | SHOULD | Implemented | `c/turul-mcp-client/src/protocol/v2026_07_28.rs:24-33` | `mrtr_2026.rs::url_mode_elicitation_requires_the_url_subcapability`, `::form_mode_elicitation_passes_with_empty_capability_object` | pass | — | — | — |
| `elicitation/create` is not served as a standalone inbound method on the 2026 lane | MUST | Implemented | `c/turul-mcp-server/src/builder.rs:178` gates it to the 2025 lane | `mrtr_2026.rs::roots_and_sampling_capability_arms_are_gated` | pass | — | — | — |
| Client rejects an unrecognised `resultType` | MUST | **Implemented, untested** | `v2026_07_28.rs:63` (`check_result_type`) | **NOT FOUND** for the unknown-discriminator arm | — | — | — | — |

## 2. Roots — deprecated (SEP-2577)

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `roots/list` is not served on the 2026 lane | MUST | Implemented | `builder.rs:169` gated to the 2025 lane; the `with_roots()` builder surface and the dead `root_provider` API were removed | `mrtr_2026.rs::roots_and_sampling_capability_arms_are_gated` | pass | — | — | — |
| The client may still declare a `roots` capability for MRTR embedding | MAY | Implemented | `c/turul-mcp-client/src/config.rs:61` | — | — | — | — | — |

Roots survives in 2026 only inside `InputRequests`, which is why the client-side
capability declaration is retained while the standalone server method is not.

## 3. Sampling — deprecated (SEP-2577)

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `sampling/createMessage` is not served on the 2026 lane | MUST | Implemented | `builder.rs:173` gated to the 2025 lane | `mrtr_2026.rs::roots_and_sampling_capability_arms_are_gated` | pass | — | — | — |
| Declared sampling capability rides every request; absent when not declared | MUST | Implemented | `c/turul-mcp-client/src/protocol/v2026_07_28.rs` | `sampling_capability_2026.rs::sampling_capability_rides_every_request_when_declared`, `::sampling_capability_is_absent_when_not_declared` | pass | — | — | — |
| `tools`-enabled sampling sub-capability gate | SHOULD | Implemented | `v2026_07_28.rs:34-39` | `mrtr_2026.rs::tool_enabled_sampling_requires_the_tools_subcapability` | pass | — | — | — |

## 4. Client obligations on the stateless core

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `MCP-Protocol-Version` on every request | MUST | Implemented | `c/turul-mcp-client/src/client.rs:319` | `wire_compliance.rs::test_mcp_protocol_version_header_on_requests`; **and asserted on foreign-peer captured bytes** by `scripts/interop-turul-client.sh` | pass | **pass** | — | — |
| Never sends `initialize` or `notifications/initialized` | MUST | Implemented | `client.rs` negotiation | `scripts/interop-turul-client.sh` client-obligation block | pass | **pass** | — | — |
| Never sends `Mcp-Session-Id` on the 2026 lane | MUST | Implemented | `client.rs:320,583,681` | `scripts/interop-turul-client.sh` client-obligation block | pass | **pass** | — | — |
| `Mcp-Method` header agrees with the body method on every request | MUST | Implemented | `c/turul-mcp-client/src/transport/http.rs` | `scripts/interop-turul-client.sh` client-obligation block | pass | **pass** | — | — |
| Per-request `_meta` carries protocolVersion + clientCapabilities | MUST | Implemented | `v2026_07_28.rs:17-55` | `sampling_capability_2026.rs::sampling_capability_rides_every_request_when_declared` | pass | — | — | — |
| Accepts both `application/json` and `text/event-stream` for a POST reply | MUST | Implemented | `transport/http.rs:20-21,431-433` | Accept header asserted by `scripts/interop-turul-client.sh`; the dual-mode **parse** has **NOT FOUND** as a named test | partial | partial | — | — |
| `server/discover` returning `-32601` falls back to the 2025 handshake; other codes do not downgrade | MUST | Implemented | `client.rs:334-439` (`classify_probe`) | `bilingual_negotiation.rs::bilingual_client_falls_back_on_400_with_32022_body`, `::pre_renumbering_32004_is_unrecognized_and_aborts` | pass | — | — | — |
| Does not open a GET SSE stream on the 2026 lane | MUST | Implemented | `connect()` runs `negotiate_protocol()` before `start_server_event_listener()`, which returns early on 2026-07-28; `HttpTransport::start_event_listener` refuses independently | `get_sse_listener_lifecycle.rs::a_2026_connection_never_issues_the_removed_get_sse_stream` (0 GETs), `::a_2025_connection_still_opens_the_get_sse_stream_with_its_session_id` | pass | — | — | — |

**Fixed this slice, and the fix exposed a second defect.** The listener used to
spawn inside `connect()` *before* negotiation resolved, so every 2026 connection
issued a GET the revision deleted, took HTTP 405, and logged *"SSE connection
failed with status: 405"* and *"SSE request attempted without session ID"* —
both naming concepts the spec removed. Deferring the spawn until after
`negotiate_protocol()` also fixed a pre-existing **2025-lane** race the old code
only compensated for with a compare-and-swap: the revert-and-fail run failed in
both directions, the 2026 connection issuing 1 GET and the 2025 GET going out
without the negotiated session id. The three "…without session ID" warnings are
now gated on `uses_session_header()` so they fire only where the header exists.

The `py` interop cell moved from **fail** to `—`, not to `pass`. FastMCP
measured the old behaviour; nothing has re-run a probe against the fixed client,
and a fix is not evidence. `scripts/interop-turul-client.sh` is the probe that
would close it.

## 5. Result handling: `resultType`, cacheability, pagination, progress, cancellation

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Resolves `resultType` on all 2026 results; absent means `"complete"` | MUST | Implemented | `v2026_07_28.rs:63-78` applied at 11 call sites | `e2e_2026_real_server.rs::mrtr_round_trip_through_the_bilingual_client` | pass | — | — | — |
| Surfaces `ttlMs` / `cacheScope` to callers | SHOULD | **Not implemented** | `v2026_07_28.rs:5-8,94-101` — results are remapped through the 2025-11-25 public types, which have no such fields | — | — | — | — | — |
| A 2020-12 composition in an `inputSchema` does not fail the listing | MUST | Implemented | `remap_tool` (`v2026_07_28.rs:115-145`) is infallible by construction: direct remap first, then a field-by-field rebuild dropping only the individual parts the public vocabulary cannot hold | `schema_2020_12_tools_list.rs::list_tools_survives_a_property_level_one_of_and_keeps_the_tool_callable`, `::list_tools_paginated_survives_a_property_level_one_of` | pass | — | — | — |
| The untruncated advertised schema stays reachable (no spec requirement — the mitigation for the row above) | n/a | Implemented | `McpClient::tool_input_schema(name)` (`client.rs:895`) returns the raw 2026 schema, cached from the same result that fed the SEP-2243 bindings | `schema_2020_12_tools_list.rs::list_tools_survives_a_property_level_one_of_and_keeps_the_tool_callable` (asserts the raw schema still carries the `oneOf`) | pass | — | — | — |
| Honours `cursor` / `nextCursor` on list operations | MUST | Implemented | `client.rs:823-954`, `:1560`, `:1663`, `:1765` | `bilingual_2026_operations.rs::paginated_list_routes_through_2026_with_meta_and_cursor` — the matcher requires both the 2026 `_meta` and the outbound cursor, and `nextCursor` is read back off the result | pass | — | — | — |
| Correlates `notifications/progress` by `progressToken` | MUST | Implemented | `client.rs:1217-1255` | `e2e_2026_real_server.rs::progress_feed_and_discovered_server_accessors` — against a real 2026 server, the sink receives exactly one event carrying the token the call declared, before the result | pass | — | — | — |
| Can cancel an in-flight request (`notifications/cancelled`) | MUST | Implemented | `McpClient::cancel_request(request_id, reason)` (`client.rs:1972`), spec-neutral — the method exists in both revisions; `SubscriptionStream::request_id()` (`client.rs:2414`) gives a caller a request id it can legitimately name | `completion_and_cancellation_2026.rs::cancel_request_is_accepted_by_the_server` | pass | — | — | — |
| `completion/complete` | MUST | Implemented | `McpClient::complete(reference, argument, context)` (`client.rs:1920`), bilingual dispatch; the result is built field by field because `total` is `f64` on the 2026 wire and `u32` in the public vocabulary | `completion_and_cancellation_2026.rs::complete_reaches_the_server_and_returns_its_suggestions`, `::complete_passes_the_context_arguments_through` — both driven by a registered `McpCompletion` provider on a real server | pass | — | — | — |

**The remap still costs fidelity — it no longer costs tools.** `turul-mcp-client`
returns 2025-11-25 public types and converts 2026 results into them, so every
2026-only field is dropped at the boundary. Until this slice that boundary was
*fatal* for schemas: `JsonSchema` in the 2025 types is an internally-tagged enum
on `"type"` with no fallback arm, `parse_list_tools` collected into a `Result`,
and one property written `{"oneOf": […]}` — legal on 2026 `inputSchema` per
SEP-2106, and what this framework's own server emits for a tagged union — errored
the **entire** listing. Turul-client could not list tools from turul-server on the
revision's headline schema change.

The frozen crate cannot be widened, so the conversion is now infallible and
lossy instead of faithful and fatal. **The cost, stated plainly:** a caller
reading `Tool.input_schema` on a 2026 connection may see a schema missing
properties the server advertised, while `required` still names them. That is
deliberate — silently dropping a valid, callable tool is the worse failure,
because the caller cannot tell it happened. Every dropped path is named in a
`tracing::warn!`, the loss is recoverable through `tool_input_schema`, and the
caveat is documented on `list_tools`/`refresh_tools`/`list_tools_paginated`.
`ttlMs` and `cacheScope` are still dropped with no recovery path.

Exclusion remains reserved for definitions a client MUST NOT act on — a SEP-2243
`x-mcp-header` placement violation or a dialect-invalid schema (`tool_is_admissible`,
`v2026_07_28.rs:200`). A 2020-12 composition is *valid* and merely inexpressible,
which is a different thing.

---

## Gap register

1. **Cacheability hints are silently discarded.** `ttlMs`/`cacheScope` are
   dropped by the remap through 2025-11-25 public types and, unlike schema
   detail, have no recovery accessor. A caller cannot honour a server's caching
   directive because it never sees it.
2. **Schema detail is dropped from `Tool.input_schema` on 2026 connections**
   whenever the advertised schema uses a 2020-12 construct the public
   vocabulary cannot hold, while `required` still names the dropped properties.
   Recoverable via `tool_input_schema`, warned about in logs, documented on the
   three listing methods — but a caller that reads only `input_schema` sees an
   inconsistent object. This is the accepted cost of item 1's alternative, not
   an oversight; it closes only by widening the public vocabulary.
3. **Unknown-`resultType` rejection is implemented but unasserted.**
4. **Dual-framing parse has no named test** — only the Accept header is asserted.
5. **The client's own cancellation is not interop-verified.** `cancel_request`
   is driven against a turul server only; no foreign peer has received one.

**Closed this slice:** the removed GET SSE stream (and a 2025-lane session-id
race it was masking); the missing public cancellation API; `completion/complete`
being unreachable — which also makes the `completion/complete` interop cell
fillable from our side for the first time; the untested pagination cursors; and
progress correlation, which was recorded as construction-only and is in fact
end-to-end against a real server. The single largest change is that a 2020-12
composition no longer fails the whole listing — that was a functional break, not
a coverage gap, and no register entry had named it.
