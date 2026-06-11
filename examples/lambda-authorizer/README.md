# Lambda Authorizer Example

API Gateway **REQUEST** authorizer Lambda for MCP Streamable HTTP transport.

## Why This Exists

An MCP API serves more than one method/path even when the MCP endpoint itself
is POST-only (the 2026-07-28 stateless default):

- **POST** `/mcp` — JSON-RPC requests (the only MCP method on 2026-07-28)
- **OPTIONS** `*` — CORS preflight for browser clients
- **GET** `/.well-known/*` — OAuth protected-resource metadata routes
- **GET / DELETE** `/mcp` — SSE streaming and session termination on the
  2025-11-25 lane (the paired `lambda-mcp-server`)

API Gateway REST API (v1) caches authorizer responses keyed by the full `methodArn`
(which includes the HTTP method). Without wildcarding, the first cached policy
(e.g., `Allow POST/`) blocks subsequent `GET` and `DELETE` requests with 403.

This example demonstrates the **methodArn wildcarding** pattern that solves this:
the authorizer replaces the method and resource path in the ARN with `*/*` before
building the IAM policy, so a single cached policy covers all methods and paths.

## How It Pairs With Other Examples

| Example | Role |
|---------|------|
| **`lambda-authorizer`** (this) | Separate Lambda that runs *before* the MCP server — validates API keys, returns IAM policy |
| **`lambda-mcp-server`** | The MCP server Lambda itself — receives requests after authorization |
| **`middleware-auth-lambda`** | Alternative: *in-Lambda* middleware auth (no separate authorizer) |

## Build & Deploy

```bash
# Build
cargo lambda build --release --package lambda-authorizer

# Deploy
cargo lambda deploy lambda-authorizer

# Test locally
cargo lambda watch --package lambda-authorizer
```

## Event Format Support

- **REST API v1**: Returns IAM policy response (`principalId`, `policyDocument`, `context`)
- **HTTP API v2**: Returns simple response (`isAuthorized`, `context`)
