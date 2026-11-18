# ADR-039: OAuth Client Credentials on the client — `turul-mcp-client` feature `oauth-client-credentials`

**Status:** Accepted
**Date:** 2026-11-18
**Crates:** `turul-mcp-client` (feature `oauth-client-credentials`: `ClientCredentials`, `ClientAuth`, `AssertionAudience`, the token source in the HTTP transport)
**Related:** ADR-021 (OAuth 2.1 resource server), ADR-028 (extensions strategy)

## Context

The OAuth Client Credentials extension (`modelcontextprotocol/ext-auth`,
`specification/stable/oauth-client-credentials/2026-07-28`, pinned at commit
`15ca394`) lets a machine client obtain an access token without a user. It
declares no capability and no wire field: it is a client flow against an
authorization server (AS), layered on the core authorization spec.

- The client uses pre-registered credentials. Dynamic Client Registration is
  not used.
- The client authenticates to the token endpoint with a JWT assertion
  (RFC 7523 §2.2, RECOMMENDED) or a client secret (OAuth 2.1 §2.4.1).
- The AS metadata lists `private_key_jwt` or `client_secret_basic` in
  `token_endpoint_auth_methods_supported`, and the signing algorithms when JWT
  authentication is offered.
- The flow: an unauthenticated MCP request gets 401 with `WWW-Authenticate`;
  the client reads `resource_metadata`, fetches the Protected Resource
  Metadata (PRM), picks an AS, fetches its metadata (RFC 8414, then OpenID
  Connect discovery), and posts `grant_type=client_credentials` with
  `resource` (RFC 8707) and an optional `scope`.

The resource-server side is unchanged: `turul-mcp-oauth` already answers 401
with `resource_metadata`, serves the PRM and validates JWTs (ADR-021). What
the framework lacks is the client half. Today a caller fetches a token
elsewhere and passes it with `McpClient::set_bearer`.

## Decision

1. **Client-only, in `turul-mcp-client`, behind feature `oauth-client-credentials`,
   off by default.** Not in `turul-mcp-oauth`: that crate is the resource
   server and depends on `turul-http-mcp-server`, which a client must not pull.
   The module is `oauth_client_credentials`. It owns the extension identifier
   constant `EXTENSION_ID = "io.modelcontextprotocol/oauth-client-credentials"`.

2. **Credentials are bound to one issuer.** `ClientCredentials` takes the AS
   issuer identifier with the client id, because pre-registered credentials
   belong to one AS (core spec, SEP-2352). The client sends them only to that AS:
   - the PRM's `authorization_servers` must list the issuer, else the flow
     stops before any AS request;
   - the AS metadata `issuer` must equal it exactly (RFC 8414 §3.3).

   Without this, any resource server could name an AS it controls and collect
   the secret.

3. **Client authentication.** `ClientAuth` is one of:
   - `Secret`: `client_secret_basic` when the AS lists it or lists nothing
     (the RFC 8414 default), else `client_secret_post` when listed, else an
     error. For basic, id and secret are form-urlencoded before base64
     (RFC 6749 §2.3.1). Basic is preferred on purpose, although the extension's
     prose and only example send the secret in the body. Its metadata rule
     names `client_secret_basic`, RFC 8414 makes basic the default, and
     OAuth 2.1 §2.4.1 requires authorization servers to support it.
   - `PrivateKeyJwt`: requires an asymmetric algorithm (an HMAC key would be
     `client_secret_jwt`, which the extension does not name), `private_key_jwt`
     in the AS list, and the key's
     algorithm in `token_endpoint_auth_signing_alg_values_supported` when that
     list is present. The assertion carries `iss` = `sub` = client id, `aud`,
     `iat`, `exp` = `iat` + 60 s and a random `jti`; the request carries no
     `client_id` (RFC 7523 §3). `aud` is the issuer by default
     (`AssertionAudience::Issuer`); `AssertionAudience::TokenEndpoint` sends the
     token endpoint URL, for authorization servers that require it.

   The secret and key never appear in `Debug` output.

4. **Discovery follows the core spec.**
   - PRM: the `resource_metadata` URL from the 401, else
     `/.well-known/oauth-protected-resource<path>` (keeping the endpoint's
     query, RFC 9728 §3.1), then the root form. The
     PRM `resource` must equal the MCP endpoint URL (RFC 9728 §3.3, compared
     as parsed URLs).
   - AS metadata for issuer `https://as/<p>`: `/.well-known/oauth-authorization-server/<p>`,
     `/.well-known/openid-configuration/<p>`, `/<p>/.well-known/openid-configuration`;
     for an issuer with no path, the two root forms.
   - Every AS URL the client calls (metadata, token endpoint) must be `https`,
     except on a loopback host.

5. **Token request.** `grant_type=client_credentials`, `resource` = the PRM
   `resource`, and `scope` chosen in this order: the scope set on
   `ClientCredentials`; the `scope` of the 401 challenge; the PRM
   `scopes_supported` joined by spaces; none. The response must have
   `token_type` `Bearer` (case-insensitive). An error response becomes
   `McpClientError::Auth` with its `error` and `error_description`.

6. **The HTTP transport owns the token.** `HttpTransport::with_oauth_client_credentials`
   and `McpClientBuilder::with_oauth_client_credentials` attach a token source.
   - Every request carries the cached token. One shared async lock serialises
     fetches, so concurrent requests cause one token request.
   - A token is refreshed before the request once it is within 30 s of its
     `expires_in`, or within half its lifetime when that is shorter, so a
     short-lived token is not fetched for every request. If that early refresh
     fails while the token is still unexpired, the token is sent anyway.
   - `expires_in` may be a number or a numeric string. A value too large to add
     to the clock is treated as unknown: the token is used until a 401.
   - On 401 the source refreshes once and the request is sent once more. A
     second 401 is returned to the caller. If another request already
     refreshed, its token is reused. The challenge's `scope` replaces the
     discovered one unless the credentials set their own. A challenge whose
     `resource_metadata` differs from the PRM already read starts discovery
     again.
   - The SSE GET listener, which runs only on the 2025-11-25 lane, sends the
     current token. On a 401 it refreshes once and reconnects, instead of
     treating the 401 as the end of the session.
   - With credentials attached, the token source owns `Authorization`;
     `set_bearer` does not override it.
   - `McpClientBuilder::build()` panics when credentials are combined with
     `with_transport`, because a caller-built transport cannot receive them;
     attach them to that `HttpTransport` instead.

7. **Not built.**
   - No AS: the framework is a resource server and a client.
   - No refresh tokens: the client-credentials grant issues none (OAuth 2.1 §4.2).
   - No scope step-up on 403 `insufficient_scope`: a client-credentials client
     is granted what its registration allows, so it sets `scope` itself.
   - No server-side change, and no extension declaration on either side: the
     extension defines none.
   - A failed discovery is not cached. Each request that gets a 401 before
     discovery succeeds tries again, costing at most two PRM and three AS
     metadata requests, with no loop.

## Tests

`crates/turul-mcp-client/tests/oauth_client_credentials.rs` (feature
`oauth-client-credentials`), against a wiremock AS, PRM and MCP endpoint:

- Secret, basic: 401 → PRM → AS metadata → token request with the basic
  header, `grant_type`, `resource` and `scope` → the retried request carries
  the bearer.
- Secret, post: an AS listing only `client_secret_post` gets the id and secret
  in the body and no `Authorization` header; the scope falls back to the PRM's
  `scopes_supported`.
- Private key JWT: the assertion verifies with the public key and carries
  `iss`, `sub`, `aud`, `exp`, `jti`; the body has no `client_id`; the
  `TokenEndpoint` audience variant sends the endpoint URL.
- Issuer binding: a PRM that does not list the issuer fails with no request
  to the AS at all, and AS metadata with a different `issuer` fails with no
  request to the token endpoint. An HMAC key, or an algorithm the AS does not
  accept, fails before the token endpoint.
- A PRM whose `resource` differs from the endpoint fails.
- Discovery: the challenge's scope wins over the PRM's; with no
  `resource_metadata`, the path-form then root-form PRM, then for a path
  issuer all three AS-metadata locations in order (asserted from the requests
  made); OpenID Connect discovery as the fallback.
- Caching: a second request makes no token request; a token inside its refresh
  window is replaced before sending (asserted from the `Authorization` headers
  sent); a 401 with a cached token refreshes once and a second 401 is returned;
  concurrent first requests, and concurrent 401s on a stale token, each make one
  token request; an overflowing `expires_in` does not panic.
- Isolation: `ConnectionConfig` headers never reach the AS, and a token
  endpoint redirect is not followed.
- The SSE GET refreshes on 401 and keeps the session.
- `Debug` shows neither the secret nor the key.
- Unit: challenge parsing, the https-or-loopback check, basic-auth encoding,
  well-known URL construction, string `expires_in`, the refresh window.
- End to end: a real `turul-mcp-server` with `turul-mcp-oauth` validating
  RS256 tokens the wiremock AS signs; `list_tools` succeeds through the full
  flow.

All run in `ci-gates.sh default` and `ci.yml`.

## Consequences

- `turul-mcp-client` gains an optional `jsonwebtoken` dependency (signing,
  same major as `turul-jwt-validator`).
- `set_bearer` stays for callers who obtain tokens another way.

## Revision log

- **2026-11-18**: Initial, then accepted after review. Changes from the review:
  - `expires_in` overflow is handled, and a string value is accepted;
  - refresh-window and refresh-failure rules for tokens;
  - re-discovery when the PRM moves;
  - SSE GET 401 recovery;
  - HMAC keys refused;
  - the PRM query is kept;
  - the basic-over-post rationale is recorded;
  - tests that prove refresh-before-send, stale-token reuse, AS isolation and
    redirect refusal.
