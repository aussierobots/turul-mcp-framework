# MCP 2026-07-28 — Four-Source Spec Reconciliation

Branch: `feat/turul-mcp-protocol-2026-07-28`. Read-only research artifact, written 2026-06-08.
Purpose: replace the prior live-spec audit's over-confident "the live draft has not moved past our pin" claim with a precise reconciliation across the four sources a reviewer (codex) correctly demanded, because earlier statements conflated them.

Our declared TARGET (AGENTS.md / CLAUDE.md §Branch Lock): the **MCP 2026-07-28 release candidate** — stateless core, `server/discover` replaces `initialize`, per-request `_meta`, no `Mcp-Session-Id`, tasks → extension, `-32002`→`-32602`, JSON Schema 2020-12, Roots/Sampling/Logging deprecated.

## 1. The four sources (verified 2026-06-08)

| # | Source | URL | Version label |
|---|--------|-----|---------------|
| 1 | Website PROSE — Deprecated registry | https://modelcontextprotocol.io/specification/draft/deprecated | deprecations dated `2026-07-28` |
| 2 | Website Schema Reference (rendered) | https://modelcontextprotocol.io/specification/draft/schema | generated from source 3 |
| 3 | **RAW `schema.ts` (GitHub `main`)** — authoritative | https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts | `LATEST_PROTOCOL_VERSION = "2026-07-28"` |
| 4 | Branch-pinned local schema | `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` | `"2026-07-28"` |

The raw `schema.ts` (source 3) is the normative machine-readable contract; the rendered Schema Reference (source 2) is generated from it. Source 1 (deprecated registry) is "a derived view kept consistent with the per-feature deprecation notices and changelog entries, which are the normative records."

## 2. Reconciliation table

| Contested item | Source 3 (raw `schema.ts`, authoritative) | Source 1 (deprecated registry) | Source 4 (our pin) | Verdict |
|---|---|---|---|---|
| Protocol version string | `LATEST_PROTOCOL_VERSION = "2026-07-28"` (verbatim) | deprecations "Deprecated in `2026-07-28`" | `"2026-07-28"` | **Match.** The earlier "DRAFT-2026-v1" reading is a stale snapshot; the live schema has finalized to `"2026-07-28"`. No re-pin needed. |
| `RequestParams._meta` | **required**: `interface RequestParams { _meta: RequestMetaObject; }` (verbatim) | — | required (`json_rpc.rs:36`, not `Option`) | **Match — required.** Earlier "`_meta?:` optional" reading is stale. ⇒ the server MUST reject missing/incomplete `_meta`. |
| `RequestMetaObject` required keys | `io.modelcontextprotocol/protocolVersion`, `/clientInfo`, `/clientCapabilities` | — | same (`meta.rs`) | **Match.** |
| `initialize`, `notifications/initialized`, `ping`, `tasks/*`, `Mcp-Session-Id`, `resources/subscribe`/`unsubscribe`, `logging/setLevel`, `notifications/roots/list_changed` | **ABSENT (removed)** | **NOT listed** (removed by the stateless-core redesign, not the deprecation lifecycle — the registry says "No features removed under this policy yet") | gated `#[cfg(feature = "protocol-2025-11-25")]` / absent in 2026 | **Match.** These were *removed* by the 2026-07-28 breaking change, distinct from the deprecation policy. Our gating-out-of-2026-default is faithful. |
| Roots / Sampling / Logging | **present, `#[deprecated]`** | **Deprecated** (SEP-2577, removal ≥ 2027-07-28) | present, `#[deprecated]` (17 annotations) | **Match — deprecated-but-present, NOT removed.** Docs must say "deprecated/opt-in/discouraged", never "removed". |
| HTTP+SSE transport / `includeContext` / DCR→CIMD | deprecation notices present | **Deprecated** | DCR/CIMD not implemented (oauth = Resource Server only); HTTP+SSE legacy path | **Doc-only gap:** DCR→CIMD deprecation not noted in COMPLIANCE.md (we don't implement DCR). |
| Transport: body `_meta` source-of-truth + header must match | per Base Protocol prose | — | server reads `MCP-Protocol-Version` header only; never cross-checks `_meta.protocolVersion` | **Gap:** no header/body mismatch rejection. |

## 3. Verdict

**Defensible — faithful to the 2026-07-28 release candidate, with one documented narrowing and two enforcement gaps.**

- The earlier "faithful, no re-pin" conclusion is **correct**: the authoritative raw `schema.ts` on `main` is `"2026-07-28"` with `_meta` required and the removed-set absent — identical to our pin. The reviewer's contrary schema citation (`"DRAFT-2026-v1"`, optional `_meta`, methods present) was a **stale snapshot** and does not reflect the current published schema.
- The "deprecated ≠ removed" caution is **valid but already correctly handled**: we treat Roots/Sampling/Logging as deprecated-but-present and `initialize`/`ping`/`tasks` as removed — matching the two separate mechanisms. The only residue is **doc misstatements** (e.g. ADR-023 saying `listChanged` "removed") to correct.
- **Documented narrowing (not a divergence):** the framework gates the removed-from-core methods out of the *default 2026 build* and keeps deprecated methods available; this matches the schema. It should be stated as such in COMPLIANCE.md.

## 4. `_meta` enforcement recommendation

**Enforce.** The authoritative schema makes `RequestParams._meta` **required** with three required `RequestMetaObject` keys, and the Base Protocol prose requires `-32602`/HTTP-400 on missing required fields and on header/body version mismatch. Our typed `CallToolRequestParams` already requires `_meta` (tools/call rejects without it), but `server/discover` and the loosely-parsed transport path do not. ⇒ validate `params._meta` as a `RequestMetaObject` on the 2026 request path, reject `-32602` on missing/incomplete, and cross-check `_meta.protocolVersion` against the `MCP-Protocol-Version` header. This is a **conformance fix**, not optional — unless explicitly shipped as a documented preview gap in COMPLIANCE.md.

## 5. Net correction to the readiness verdict

The schema-faithfulness is **confirmed**, so the prior live-spec audit's conclusion stands on that axis. But the readiness verdict remains **"ready for continued branch work, NOT release/merge"** for reasons unrelated to schema drift: the `_meta` enforcement gap (above), the 14 default examples advertising removed contracts, the missing CI matrix, and the ADR/doc misstatements. Those P1s must be closed (or the `_meta` gap explicitly documented as preview) before release-readiness.
