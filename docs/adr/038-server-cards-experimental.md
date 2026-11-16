# ADR-038: Server Cards (experimental) — `turul-mcp-ext-server-card`

**Status:** Accepted
**Date:** 2026-11-17
**Crates:** `turul-mcp-ext-server-card` (new, 0.1.0: wire types, validation, the card document), `turul-mcp-server` (feature `ext-server-card`: `with_server_card`, the HTTP route), `turul-http-mcp-server` and `turul-mcp-aws-lambda` (a route keeps the CORS headers it sets)
**Related:** ADR-028 (extensions strategy), ADR-035 (Lambda consumes `McpServerBuilder`)

## Context

A Server Card is a static JSON document describing a remote MCP server for
discovery before connecting: identity, remote endpoints, protocol versions. It is
defined by `modelcontextprotocol/ext-server-card` (tracking SEP-2127), whose README
says: **"Experimental. This work is for prototyping and feedback only, and is not
an accepted or official MCP extension."** The pin is commit `526201b` (2026-08-12,
the latest at 2026-11-17).

The extension has no capability negotiation. `docs/discovery.md` reserves
`GET <streamable-http-url>/server-card` as the recommended location and sets these
rules for a hosted card:

- media type `application/mcp-server-card+json`;
- CORS MUST: `Access-Control-Allow-Origin: *`, `Allow-Methods: GET`,
  `Allow-Headers: Content-Type, If-None-Match`, `Expose-Headers: ETag`;
- caching SHOULD: `Cache-Control: public, max-age=3600`, an `ETag`, and
  `304 Not Modified` on a matching `If-None-Match`;
- identity, version, title, description, icons and protocol versions SHOULD NOT
  contradict `server/discover`;
- no credentials or private endpoints; HTTPS in production; rate limiting SHOULD.

Two transport behaviours stand in the way today. The HTTP server applies its
CORS headers after a custom route answers, overwriting the route's own. Lambda
does the same, and answers every `OPTIONS` before it looks at routes. Either one
drops the `ETag` exposure and the `If-None-Match` allowance the CORS MUST requires.

## Decision

1. **New crate `turul-mcp-ext-server-card` 0.1.0, wire-only (ADR-028).** Module
   `v1`, named for the card schema's `vN` family. It owns:
   - `ServerCard`, `Repository`, `Remote`, `RemoteType`, `Input`, `InputFormat`
     and `KeyValueInput`, mirroring `schema.ts`. `icons` reuses the core
     protocol's `Icon`, and `_meta` is a map, so neither type is defined twice.
   - `SCHEMA_URL`, `MEDIA_TYPE` and `PATH_SUFFIX` (`"/server-card"`).
   - `ServerCard::new(name, description)`, which sets `$schema`.
   - `ServerCard::validate()`, which enforces the schema's constraints: the
     `$schema` pattern, name pattern and length, version length and "not a
     range", description and title length, and the remote URL pattern.
   - `ServerCardDocument::new(card)`, which produces the served bytes and their
     `ETag`. It is the one producer of both.

   The crate vendors `schema.ts` and `schema.json` at `526201b`, with sha256
   values in `schema/README.md`. `scripts/check-schema-pin.sh` checks them.
   The README opens with the experimental status.

2. **Server feature `ext-server-card`, off by default.** It adds
   `McpServerBuilder::with_server_card(card)`. At `build()`, the fields
   `server/discover` also serves are reconciled against the server's
   `Implementation`. The first three rules cover `version`, `title`,
   `description`, `websiteUrl` and `icons`:
   - A field the card leaves unset is copied from the server.
   - A field set on both sides with different values is a build error.
   - `version` must equal the server's.

   Each remote's `supportedProtocolVersions` is filled from this build's spec
   lane when unset, and must equal it when set. Then `validate()` runs. Any
   failure is `McpError::ConfigurationError` from `build()`. The spec only says
   these fields SHOULD NOT contradict, but a framework that serves both
   documents from one build can make that true by construction.

   Remote URLs come from the caller. A server cannot know the public URL it is
   reached at behind a proxy, API Gateway or a load balancer.

   `name` is not reconciled. The card's name is a reverse-DNS identifier
   (`com.example/weather`), while `serverInfo.name` is free text, so equal
   values cannot be required without constraining `server/discover`.

3. **The route is `<mcp_path>` + `PATH_SUFFIX`**, registered through the existing
   `RouteRegistry`, so it serves on HTTP and, via `TransportParts`, on Lambda.
   - `GET` and `HEAD` return 200 with `Content-Type: application/mcp-server-card+json`,
     the four CORS headers above, `Cache-Control: public, max-age=3600` and `ETag`.
     `HEAD` has no body.
   - A matching `If-None-Match` (a listed tag, or `*`) returns 304 with `ETag`,
     `Cache-Control` and CORS.
   - `OPTIONS` returns 204 with CORS.
   - Any other method returns 405 with `Allow: GET, HEAD, OPTIONS`.
   - The body and `ETag` are computed once, at build. The `ETag` is the strong
     tag `"<first 32 hex of sha256(body)>"`.

4. **A route keeps the CORS headers it sets.** The HTTP server and Lambda add
   their CORS headers only when a route response carries no
   `Access-Control-Allow-Origin`. Lambda checks routes before its blanket
   `OPTIONS` preflight, so a route answers its own preflight. MCP-endpoint
   responses set no CORS of their own, so they are unchanged.

5. **Not built.**
   - No client fetch helper: nothing consumes one.
   - No AI Catalog document: it is site-wide, which belongs to the deployment.
   - No rate limiter: middleware covers it.
   - No HTTPS enforcement: TLS terminates outside the process.

   The README states all of these.

## Tests

- **Crate.** Every upstream `examples/ServerCard/valid/*.json` deserialises,
  passes `validate()`, re-serialises to the same JSON, and validates against the
  vendored `schema.json`. Every `invalid/*.json` fails both `validate()` and the
  vendored schema. A `ServerCard` the crate builds validates against
  `schema.json`. `ServerCardDocument` is deterministic, and different cards
  produce different `ETag`s.
- **Server (`tests/server_card_2026.rs`, feature `ext-server-card`).**
  - `GET` serves the exact headers and body.
  - `If-None-Match` returns 304.
  - `HEAD` has no body. `OPTIONS` returns 204 with the CORS headers.
  - `POST` returns 405.
  - The card's version, title and icons equal `server/discover`'s.
  - A contradicting field, or an invalid card, fails `build()`.
  - A default build serves 404 at `/mcp/server-card`.
  - With the server's CORS enabled, the route's CORS headers survive.
- **Lambda.** A handler test with the route: the response keeps the card's CORS
  headers, and `OPTIONS` reaches the route.
- **Live.** `examples/server-card-server`, read back with curl
  (`GET`, `If-None-Match`, `OPTIONS`).

Every test above runs in `ci-gates.sh default` and `ci.yml`. The pin runs in
`scripts/check-schema-pin.sh`, which a tampered or unlisted file fails. Test
names:
- crate: `v1::tests::*`;
- server: `crates/turul-mcp-server/tests/server_card_2026.rs`;
- Lambda: `handler::tests::cors_streaming_routes::a_route_that_sets_cors_keeps_it_and_answers_its_preflight`.

## Consequences

- One new published crate (`turul-mcp-ext-server-card`). It publishes after the
  protocol crate and before `turul-mcp-server`.
- Upstream may still change the schema. The crate versions on its own 0.1.x line.
  A breaking upstream revision would publish a new `vN`, which lands as a new
  module.

## Revision log

- **2026-11-17**: Initial; accepted with every test gated. `name` is left unreconciled (§2).
