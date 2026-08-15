# Middleware Auth Lambda

`McpMiddleware` authentication inside an AWS Lambda MCP server, integrating
with **API Gateway authorizer context**. It validates an `X-API-Key` header
and folds the authorizer's claims into request state that tools can read.

## Spec lane

**MCP 2026-07-28** (the workspace default — no `protocol-2025-11-25` pin in
`Cargo.toml`). The stateless core suits Lambda well: no `initialize`
handshake and no `Mcp-Session-Id` to keep alive across invocations, so every
request carries its own `_meta` (`protocolVersion`, `clientInfo`,
`clientCapabilities`) and each Lambda invocation is self-contained.

## Transport

Streamable HTTP over **REST API (V1)**. The adapter converts the API Gateway
event into a `hyper::Request` that the framework's `StreamableHttpHandler`
handles normally. HTTP API (V2) authorizer context extraction is fully
supported, but the Streamable HTTP transport itself requires REST API (V1).

## Three authorizer context shapes

API Gateway emits the authorizer payload in three different places depending
on API type and authorizer style. This example handles all three:

| Shape | Where the claims land |
|---|---|
| V1 nested (REST, standard Lambda proxy) | `requestContext.authorizer.lambda.{field}` |
| V1 flat (REST, simple Lambda authorizer) | `requestContext.authorizer.{field}` |
| V2 (HTTP API authorizer) | `requestContext.authorizer.{field}` |

## Run and deploy

```bash
cargo lambda watch --package middleware-auth-lambda      # local
cargo lambda build --release --package middleware-auth-lambda
cargo lambda deploy middleware-auth-lambda
```

**[TESTING.md](TESTING.md)** has the full local test procedure, the sample
events under `test-events/`, and the verification checklist.
`test_authorizer.sh` drives the automated pass.

## How this compares

| Example | Auth approach |
|---|---|
| **`middleware-auth-lambda`** (this) | Auth *inside* the Lambda, as MCP middleware |
| [`lambda-authorizer`](../lambda-authorizer/) | A separate authorizer Lambda that runs *before* the MCP server |
| [`middleware-auth-server`](../middleware-auth-server/) | The same middleware pattern on a plain HTTP server |
| [`oauth-resource-server`](../oauth-resource-server/) | OAuth 2.1 resource-server token validation |
