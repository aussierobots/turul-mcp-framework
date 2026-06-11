# ADR-031: Origin validation (DNS-rebinding protection) on the HTTP transport

**Status:** Accepted
**Date:** 2026-06-11
**Crates:** `turul-http-mcp-server` (enforcement), `turul-mcp-server` (builder passthrough)
**Related:** ADR-027 (Targeting DRAFT-2026-v1), ADR-012 (middleware architecture)
**Driver:** `docs/plans/2026-07-28-spec-compliance.md` gap **TX/GAP-1 (P0)**

## Context

The Streamable HTTP transport spec (draft `basic/transports/streamable-http`
§Security, and the same clause in 2025-11-25) requires:

> Servers MUST validate the `Origin` header on all incoming connections to
> prevent DNS rebinding attacks. If the `Origin` header is present and
> invalid, servers MUST respond with HTTP 403 Forbidden.

The framework had no code path reading `Origin`. `cors.rs` only *emits*
`Access-Control-Allow-Origin: *` on responses, which weakens browser-side
protection rather than providing server-side validation. A DNS-rebinding
attacker serves a page from `attacker.example`, rebinds the name to
`127.0.0.1`, and scripts requests at a loopback-bound MCP server; the
browser sends `Origin: http://attacker.example`, which a validating server
must reject.

## Decision

### Policy type

`turul-http-mcp-server` gains `OriginPolicy` on `ServerConfig`:

```rust
pub enum OriginPolicy {
    /// Default. Origin absent → allowed. Origin present → allowed only if
    /// its host is loopback (`localhost`, `127.0.0.0/8`, `[::1]`) or its
    /// authority matches the request's `Host` header (default-port
    /// normalized for http/https). Anything else → HTTP 403.
    SameOriginOrLoopback,
    /// `SameOriginOrLoopback` semantics PLUS an explicit allowlist of
    /// origins (`scheme://host[:port]`, compared case-insensitively,
    /// default-port normalized).
    AllowList(Vec<String>),
    /// No validation. For deployments that enforce origin upstream
    /// (API Gateway, ALB, reverse proxy) or are not browser-reachable.
    Disabled,
}
```

### Enforcement point

At the entry of **both** transport handlers, so every deployment shape
inherits it (hyper server *and* `turul-mcp-aws-lambda`, which constructs the
handlers directly):

- `StreamableHttpHandler::handle_request` — checked for POST/GET/DELETE,
  after the OPTIONS short-circuit.
- `SessionMcpHandler::handle_mcp_request` (legacy ≤ 2024-11-05 path) — same
  check at entry.

Rules:

- **OPTIONS preflight is exempt.** A preflight response carries no data; the
  actual request that follows is gated. Rejecting preflight would only
  change *where* the browser fails, while breaking legitimate CORS flows.
- **Custom routes (`.well-known/*`) are exempt.** Protected-resource
  metadata is public discovery data outside the MCP transport contract, and
  RFC 9728 clients legitimately fetch it cross-origin.
- **`Origin` absent → allowed.** The spec constrains only "present and
  invalid". Non-browser clients (curl, SDKs) send no Origin and are
  unaffected.
- **`Origin: null` → invalid** under `SameOriginOrLoopback` (sandboxed
  iframe / `file://` provenance is indistinguishable from an attacker). It
  can be explicitly allowed via `AllowList(vec!["null".into()])`.
- The rejection is **HTTP 403 with a short plain body**, emitted before
  protocol-version routing, body parsing, and auth middleware. 403-before-401
  is deliberate: origin validation is a connection-level security gate (the
  WAF position), not a per-identity decision, and rejecting early avoids
  spending token validation on rebound traffic.

### Builder surface

- `HttpMcpServerBuilder::origin_policy(OriginPolicy)`
- `McpServer::builder().origin_policy(OriginPolicy)` passthrough (same
  pattern as `allow_unauthenticated_ping`).
- `LambdaMcpServerBuilder::origin_policy(OriginPolicy)`.
- Default everywhere: `SameOriginOrLoopback`.

### CORS-derived policy (Lambda builder)

`LambdaMcpServerBuilder` has an *explicit* CORS configuration surface
(`cors_allow_all_origins()` / `cors_allow_origins(...)` / `cors_from_env()`)
— there is no CORS unless the operator configures it. That configuration IS
the operator's declaration of allowed origins, so at `build()` the origin
policy is derived from it unless `.origin_policy()` was set explicitly:

- allowed origins contain `"*"` → `Disabled`
- explicit origin list → `AllowList(list)`
- no CORS config → `SameOriginOrLoopback` default

This derivation deliberately does **NOT** apply to `turul-http-mcp-server`'s
`enable_cors: bool` (default `true`): that flag is a blanket allow-all
response-header default, not an explicit origin declaration, and deriving
`Disabled` from it would silently void this ADR's protection on every
default-configured server.

### Default-choice rationale

The TS SDK ships DNS-rebinding protection **off** by default; we ship it
**on** because (a) the spec clause is a MUST, (b) the framework's primary
deployment targets (Lambda behind API Gateway, EC2 behind ALB, loopback dev
servers) all keep working — no Origin header, loopback origins, or
same-host origins all pass — and (c) the only broken-by-default scenario is
a browser app on a *different* origin calling the server directly, which is
exactly the case that needs a conscious `AllowList`/`Disabled` decision.

## Consequences

- Browser clients on a different origin must be allowlisted
  (`origin_policy(OriginPolicy::AllowList(...))`) or the check disabled —
  this is a behavior change for cross-origin browser deployments that
  previously relied on `Access-Control-Allow-Origin: *` alone.
- `cors.rs` response-header emission is unchanged; CORS (browser-side
  consent) and origin validation (server-side rejection) are independent
  layers.
- Wire tests: `crates/turul-mcp-server/tests/origin_validation_2026.rs`
  (production path: Builder → `server.run()` → real HTTP). Unit tests for
  the matcher live in `turul-http-mcp-server/src/origin.rs`.

## Revision log

- **2026-06-11** — initial. Accepted with the slice that closes spec-compliance
  gap TX/GAP-1.
- **2026-06-11 (same slice, CORS derivation)** — Lambda CORS/OAuth streaming
  tests exposed the contradiction between explicit `cors_allow_all_origins()`
  and the `SameOriginOrLoopback` default (browser told "allowed", server
  403s). Added the §"CORS-derived policy" rule for the Lambda builder and the
  explicit non-derivation rule for `enable_cors`.
