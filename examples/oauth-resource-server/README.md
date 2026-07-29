# OAuth 2.1 Resource Server

An MCP server that validates incoming Bearer tokens against an **external**
Authorization Server via JWKS. It is an OAuth 2.1 Resource Server per RFC 9728
— it never issues tokens.

"Resource" here is the OAuth sense (a protected resource), not MCP
`resources/*`. This server exposes one tool and no MCP resources.

## Spec lane

**2026-07-28** (workspace default). Stateless — no handshake, no
`Mcp-Session-Id`; every request carries its own `_meta`. `turul-mcp-oauth` is
not spec-version-gated, so the RS mechanics are the same on either lane.

## What it wires up

- `JwtValidator` — fetches the AS's JWKS and validates the token's signature,
  audience (`--resource`) and issuer (`--auth-server`)
- `OAuthResourceMiddleware` — runs before session creation; a token missing any
  `--required-scope` gets HTTP 403 with a `WWW-Authenticate` challenge carrying
  `error="insufficient_scope"`
- `WellKnownOAuthHandler` — RFC 9728 §3 metadata at both the root and path
  forms of `/.well-known/oauth-protected-resource`
- Tool `whoami` — reads the verified claims the middleware left in
  `SessionContext.extensions` under `__turul_internal.auth_claims`

It is built the long way (validator → middleware → routes) rather than with the
one-call `oauth_resource_server` factory, because the factory does not take
required scopes.

## Run

Needs a reachable Authorization Server with a JWKS endpoint — the defaults
point at `auth.example.com` and will fail to validate anything.

```bash
cargo run -p oauth-resource-server -- \
  --port 8080 \
  --jwks-uri https://auth.example.com/.well-known/jwks.json \
  --resource https://example.com/mcp \
  --auth-server https://auth.example.com \
  --required-scope mcp:read
```

## Try it

```bash
# Discovery needs no token
curl -s http://127.0.0.1:8080/.well-known/oauth-protected-resource | jq

# Everything on /mcp needs a valid Bearer token
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: whoami' \
  -H "Authorization: Bearer $JWT" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"whoami\",\"arguments\":{},$META}}"

# No token → 401; valid token without mcp:read → 403 insufficient_scope
curl -s -o /dev/null -w '%{http_code}\n' -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{$META}}"
```

Never put a real token in a script or a committed file — pass it through an
environment variable as above.

## Not yet implemented

`turul-mcp-oauth` does not yet cover several 2026-07-28 auth-hardening
requirements (RFC 9207 `iss` validation, OIDC `application_type` on DCR,
authorization-server binding of persisted client credentials, scope
accumulation, the `.well-known` discovery suffix change). Check
`plugins/turul-mcp-skills/skills/auth-patterns/SKILL.md` before relying on this
crate for a fully 2026-07-28-compliant deployment.
