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
| Does not open a GET SSE stream on the 2026 lane | MUST | **Not implemented** | `transport/http.rs:1085-1140` spawns the listener before negotiation resolves | fails in practice: the peer answers 405 and the listener exits | **fail** | **fail** | — | — |

**Defect found by interop.** On every connection the client issues a GET SSE
request that 2026-07-28 removed, receives HTTP 405, and logs *"SSE connection
failed with status: 405"* and *"SSE request attempted without session ID"* —
both naming concepts the spec deleted. It degrades correctly (covered by
`sse_terminal_4xx.rs`) but costs a wasted round trip per connection and misleads
anyone reading logs. The fix is a connection-lifecycle change: the listener is
spawned before `connect()` resolves the negotiated version, and that ordering is
deliberate for the 2025 lane.

## 5. Result handling: `resultType`, cacheability, pagination, progress, cancellation

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Resolves `resultType` on all 2026 results; absent means `"complete"` | MUST | Implemented | `v2026_07_28.rs:63-78` applied at 11 call sites | `e2e_2026_real_server.rs::mrtr_round_trip_through_the_bilingual_client` | pass | — | — | — |
| Surfaces `ttlMs` / `cacheScope` to callers | SHOULD | **Not implemented** | `v2026_07_28.rs:5-8,94-101` — results are remapped through the 2025-11-25 public types, which have no such fields | — | — | — | — | — |
| Honours `cursor` / `nextCursor` on list operations | MUST | Implemented | `client.rs:823-954`, `:1560`, `:1663`, `:1765` | **NOT FOUND** — no named cursor round-trip test | — | — | — | — |
| Correlates `notifications/progress` by `progressToken` | MUST | Implemented | `client.rs:1217-1255` | construction-only (`build_notification` envelope assertion) | partial | — | — | — |
| Can cancel an in-flight request (`notifications/cancelled`) | MUST | **Not implemented** | `client.rs:647` `build_notification` is private and used only by a test | — | — | — | — | — |
| `completion/complete` | MUST | **Not implemented** | no client method exists | — | n/a | n/a | n/a | n/a |

**The remap is the root cause of two rows.** `turul-mcp-client` returns
2025-11-25 public types and converts 2026 results into them, so every 2026-only
field is dropped at the boundary: `ttlMs`, `cacheScope`, and — because
`JsonSchema` in the 2025 types is a closed enum — any 2020-12 `inputSchema`
using `oneOf`/`$ref`/`$defs` that the server is now allowed to advertise. The
server implements full 2020-12 (`schema_fidelity_2026.rs`); the client cannot
represent it.

---

## Gap register

1. **The client opens a GET SSE stream the spec removed** — one wasted round
   trip and two misleading warnings per connection.
2. **No public cancellation API.** `task_cancel`/`cancel_task` are the Tasks
   extension's `tasks/cancel`, a different mechanism.
3. **`completion/complete` is unreachable from the client** — no method exists,
   so the interop probe records it as UNSUPPORTED rather than passing or failing.
4. **Cacheability hints are silently discarded** by the remap through 2025-11-25
   public types, along with any 2020-12 schema construct those types cannot hold.
5. **Pagination cursors have no named test.**
6. **Progress correlation is construction-only tested.**
7. **Unknown-`resultType` rejection is implemented but unasserted.**
8. **Dual-framing parse has no named test** — only the Accept header is asserted.
