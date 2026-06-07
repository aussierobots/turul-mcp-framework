# MCP 2026-07-28: Pinned-snapshot vs LIVE-draft website audit

**Type:** READ-ONLY audit. No code/test/example modified. This file is the only artifact.
**Repo:** `/Users/nick/turul-mcp-framework` · **Branch:** `feat/turul-mcp-protocol-2026-07-28`
**Date:** 2026-06-07
**Scope:** Cross-check the framework's MCP 2026-07-28 implementation (code, pinned schema, ADRs, tests, examples) against the ACTUAL published MCP draft spec on `modelcontextprotocol.io/specification/draft/`.

---

## (a) Spec pages read

| # | URL | Draft version/date the page states |
|---|-----|------------------------------------|
| 1 | https://modelcontextprotocol.io/specification/draft/deprecated | Registry references `2026-07-28` as the deprecation revision; no version label of its own |
| 2 | https://modelcontextprotocol.io/specification/draft/basic | Base protocol overview; stateless core; `_meta` required keys; example versions `"2026-07-28"` |
| 3 | https://modelcontextprotocol.io/specification/draft/basic/versioning | `supported: ["2026-07-28", "2025-11-25"]`; "Modern" = `2026-07-28` and later; no advance past 2026-07-28 |
| 4 | https://modelcontextprotocol.io/specification/draft/client/sampling | Deprecated banner cites `2026-07-28` / SEP-2577 |
| 5 | https://modelcontextprotocol.io/specification/draft/server/tools | `resultType`, `ttlMs`/`cacheScope`, JSON Schema 2020-12, `inputSchema` composition keywords, `x-mcp-header` |
| 6 | https://modelcontextprotocol.io/specification/draft/basic/authorization/client-registration | DCR deprecated; Client ID Metadata Documents (CIMD) is the replacement |

**Has the live draft advanced past 2026-07-28?** **No.** The live draft's negotiation example lists `supported: ["2026-07-28", "2025-11-25"]` and the versioning page defines "Modern" as "revision `2026-07-28` and later". `2026-07-28` is the latest published wire string. Our pin (`COMPLIANCE.md` §Pin, re-vendored 2026-06-07, `LATEST_PROTOCOL_VERSION = "2026-07-28"`) matches the live draft's current wire string.

Our pinned snapshot read in full: `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` (3082 lines).

---

## (b) Findings table (ranked P0–P3)

Severity key (per prompt): **P0** = wire-incompatible with the LIVE draft; **P1** = live draft has moved past our pin / needs re-pin or ADR/code update; **P2** = doc/ADR wording drift; **P3** = nit.

| # | Sev | Area | LIVE-SPEC-SAYS (quote + URL) | OUR-CODE/SCHEMA/ADR/TEST DOES (file:line) | Remediation + gate |
|---|-----|------|------------------------------|-------------------------------------------|--------------------|
| 1 | **P0 — none found** | — | — | — | No wire-incompatible divergence found between our 2026-07-28 path and the live draft. The bilingual acceptance tests assert the exact live shapes (`resultType`, `ttlMs`/`cacheScope`, per-request `_meta`, `supportedVersions`, ping/tasks-rejected). | — |
| 2 | **P1** | Auth / DCR deprecation | DCR is now a **Deprecated** feature as of `2026-07-28`: "Dynamic Client Registration is deprecated. New implementations should use **Client ID Metadata Documents** instead." (https://modelcontextprotocol.io/specification/draft/basic/authorization/client-registration) and the registry row "[Dynamic Client Registration] … Deprecated in `2026-07-28` … Migration path: Client ID Metadata Documents … Earliest removal: first revision on/after 2027-07-28" (https://modelcontextprotocol.io/specification/draft/deprecated) | The DCR deprecation is **not in our pinned `draft-schema.ts`** (DCR is an auth-flow concern, not a schema type) and **not tracked** in `COMPLIANCE.md` §SEP-2577 or in AGENTS.md §Branch Lock (which lists the auth-hardening SEPs but not the DCR→CIMD deprecation). `turul-mcp-oauth` is a **Resource Server** only (RFC 9728 protected-resource metadata + JWT validation — `crates/turul-mcp-oauth/src/lib.rs:90-96`, `well_known.rs`, `metadata.rs`); it does **not** implement DCR or CIMD, so there is no wire defect. | Not a wire bug. Add a one-line note to `COMPLIANCE.md`/Branch Lock that DCR is deprecated→CIMD as of 2026-07-28, and that `turul-mcp-oauth` (RS-only) is unaffected. Gate: doc-only; no code/test change. |
| 3 | **P2** | Deprecations completeness | The live registry also lists, deprecated in `2026-07-28`/SEP-2577: **Roots, Sampling, Logging** — all three carry "Earliest removal: First revision released on or after 2027-07-28" (https://modelcontextprotocol.io/specification/draft/deprecated) | We match Roots/Sampling/Logging exactly. `roots.rs:24-29`, `sampling.rs:167-180`, `notifications.rs:377-396` each carry `#[deprecated(since="0.4.0", note="… Replacement: … Earliest removal: first release on/after 2027-07-28.")]` — replacement strings (tool params/resource URIs/config; integrate with LLM provider APIs; stderr/OpenTelemetry) verbatim match the registry's migration-path column. | None required — this row confirms fidelity. |
| 4 | **P2** | `includeContext` soft-deprecation | Registry: "`includeContext: \"thisServer\"`/`\"allServers\"` … Deprecated in `2025-11-25` (SEP-2596) … will be removed no later than the Sampling feature itself" (https://modelcontextprotocol.io/specification/draft/deprecated) | Matches: `sampling.rs:12-14` module doc + the schema's own `@deprecated` on `CreateMessageRequestParams.includeContext` (pinned `draft-schema.ts:1972-1975`). `SEP-2596` cited in `sampling.rs:12`. | None — fidelity confirmed. |
| 5 | **P2** | HTTP+SSE transport deprecation | Registry lists "HTTP+SSE transport … Deprecated in `2025-03-26` (SEP-2596) … Streamable HTTP" (https://modelcontextprotocol.io/specification/draft/deprecated) | Not represented as a schema type (it's a transport mode). Our default transport is Streamable HTTP (`streamable_http.rs`); the legacy `session_handler.rs` (protocol ≤ 2024-11-05) exists for back-compat. No drift. | None — informational. |
| 6 | **P3** | Stale code comment | Live schema has **no** `ping`/`PingRequest` in the 2026-07-28 method set (absent from `draft-schema.ts` `ClientRequest`/`ServerRequest` unions; bilingual test asserts ping rejected — `bilingual_2026_operations.rs:132`). | `crates/turul-mcp-protocol-2026-07-28/src/ping.rs:81-82` comment references a non-existent `PingRequest` ("Note: PingRequest contains method field…"). The module itself correctly declares only `EmptyParams`/`EmptyResult` (legitimate `ClientResult`/`ServerResult` empty shapes) and **no `ping` method string** — verified no `"ping"` literal in the crate. | Nit: stale comment names a type that doesn't exist. Optional cleanup; no contract impact. |

---

## (c) Pinned-schema vs live-schema diff summary — is a re-pin needed?

**No re-pin needed.** Spot-checked 10 representative types in our pinned `draft-schema.ts` against the live schema-reference prose; all match:

| Type | Pinned (`draft-schema.ts`) | Live draft | Match |
|------|-----------------------------|------------|-------|
| `LATEST_PROTOCOL_VERSION` | `"2026-07-28"` (line 37) | `"2026-07-28"` (versioning page negotiation example) | ✅ |
| `RequestMetaObject` required keys | `protocolVersion`, `clientInfo`, `clientCapabilities` (lines 83-98) | same 3 required + `progressToken?`/`logLevel?` optional (basic §_meta table) | ✅ |
| `_meta` missing-field rule | (doc only) | "A request missing any required field is malformed; the server **MUST** reject it with JSON-RPC error code `-32602` … HTTP `400`" (basic §_meta) | ✅ — our `meta.rs:278-284` documents the same MUST |
| `CacheableResult` | `ttlMs: number`, `cacheScope: "public"|"private"` (lines 991-1018) | tools example shows `ttlMs`/`cacheScope` on `tools/list` result | ✅ |
| `ResultType` | `"complete"|"input_required"|string` (line 169) | "polymorphic result types"; absent ⇒ treat as `"complete"` (basic §ResultType) | ✅ |
| `server/discover` | `DiscoverResult extends CacheableResult` w/ `supportedVersions`/`capabilities`/`serverInfo`/`instructions?` (lines 584-607) | "Servers **MUST** implement `server/discover`" (versioning) | ✅ — handler at `server.rs:1414-1471` |
| `SubscriptionFilter` / `subscriptions/listen` | lines 1178-1225 | versioning + tools pages reference `subscriptions/listen` replacing GET endpoint & `resources/subscribe` | ✅ |
| `Tool.inputSchema` 2020-12 + composition | `oneOf`/`anyOf`/`allOf`/`$ref`/`$defs` allowed (lines 1836-1855) | tools §Data Types + basic §JSON Schema Usage (default 2020-12) | ✅ |
| `InputRequiredResult` (MRTR) | `inputRequests?`/`requestState?` (lines 492-503) | tools §Input Required Tool Results; sampling MRTR flow | ✅ |
| DCR deprecation | **absent** (not a schema type) | registry + client-registration page | n/a (auth flow, not schema) — see Finding #2 |

`COMPLIANCE.md` records schema content sha256 `20df36f9…` captured 2026-06-07; the live wire string is still `2026-07-28`. The only live-spec movement since our pin that is NOT reflected in the schema is the **DCR→CIMD deprecation** (Finding #2), which lives on the authorization prose pages, not in `schema.ts`, so a schema `refresh --write` would not surface it.

---

## (d) Verdict

**Faithful-with-exceptions.**

Our 2026-07-28 implementation is wire-faithful to the live draft on every checked surface: the stateless core (no `initialize`/`notifications/initialized`/`Mcp-Session-Id` on the 2026 path — `server.rs:1407-1471`, `streamable_http.rs:1440-1448`), per-request `_meta` with the 3 required `io.modelcontextprotocol/*` keys and the `-32602`/HTTP-400 missing-field rule (`meta.rs`), `server/discover` MUST (implemented), `CacheableResult` (`ttlMs`/`cacheScope`), `resultType` polymorphism, JSON Schema 2020-12, `subscriptions/listen` replacing `resources/subscribe`, and the Roots/Sampling/Logging SEP-2577 deprecations with replacement + 2027-07-28 timeline strings that match the live registry verbatim. The bilingual acceptance tests (`bilingual_2026_operations.rs`) assert the live shapes and reject removed-from-core contracts (`ping`, `tasks/*`, `tasks/list`). ADR-028's tasks-as-extension model matches the live `io.modelcontextprotocol/tasks` extension. The two example hits on removed contracts (`logging/setLevel`, `tasks/list`) are explicitly pinned to `--features protocol-2025-11-25` (valid on the legacy spec), not the 2026 default.

The exceptions are documentation/tracking, not wire defects:
- **P1** — the live draft added a **DCR→Client ID Metadata Documents deprecation** (2026-07-28/SEP-2577 + PR #2858) that we don't track; harmless because `turul-mcp-oauth` is Resource-Server-only and implements neither DCR nor CIMD, but worth a one-line note in `COMPLIANCE.md`/Branch Lock.
- **P3** — a stale `ping.rs` comment names a non-existent `PingRequest`.

No P0 (wire-incompatible) findings.

---

## Return summary

- **Verdict:** Faithful-with-exceptions.
- **Findings by severity:** P0 = 0 · P1 = 1 (DCR→CIMD deprecation untracked; no wire impact) · P2 = 3 (deprecation-fidelity confirmations / informational) · P3 = 1 (stale `ping.rs` comment).
- **Live draft advanced past 2026-07-28?** No — `2026-07-28` is still the latest published wire string (`supported: ["2026-07-28","2025-11-25"]`).
- **Report path:** `/Users/nick/turul-mcp-framework/docs/plans/2026-07-28-spec-vs-website-audit.md`
