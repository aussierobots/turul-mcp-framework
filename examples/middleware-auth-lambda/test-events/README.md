# Lambda Test Events

Test events for validating API Gateway authorizer context extraction.

## Files

- `apigw-v1-with-authorizer.json` - API Gateway REST API (V1) format with nested Lambda authorizer context (`authorizer.lambda.{field}`)
- `apigw-v1-flat-authorizer.json` - API Gateway REST API (V1) format with flat authorizer context (`authorizer.{field}`); internal fields (`principalId`, `integrationLatency`) are filtered out automatically
- `apigw-v2-with-authorizer.json` - API Gateway HTTP API (V2) format with authorizer context (`authorizer.{field}`)

## Three Authorizer Shapes

API Gateway produces three distinct authorizer context shapes. The adapter handles all three:

**V1 Nested** (`apigw-v1-with-authorizer.json`):
```json
{ "requestContext": { "authorizer": { "lambda": { "userId": "user-123" } } } }
```

**V1 Flat** (`apigw-v1-flat-authorizer.json`):
```json
{ "requestContext": { "authorizer": { "userId": "user-123", "principalId": "...", "integrationLatency": 42 } } }
```
The flat shape includes API Gateway internal fields that are **not** user context. The adapter filters out `principalId`, `integrationLatency`, and `usageIdentifierKey` automatically.

**V2** (`apigw-v2-with-authorizer.json`):
```json
{ "requestContext": { "authorizer": { "userId": "user-123" } } }
```

## Authorizer Fields

All three events include common authorizer context fields (as set by API Gateway authorizer):

- `userId`: user-123 (stored in session as `user_id`)
- `tenantId`: tenant-456 (stored in session as `tenant_id`)
- `role`: admin (stored in session as `role`)
- `permissions`: read,write,delete (stored in session as `permissions`)
- `customClaim`: example-value (stored in session as `custom_claim`)

**Note**: Field names are converted from camelCase to snake_case for Rust conventions. Your authorizer can return any fields relevant to your application (e.g., organizationId, subscriptionTier, etc.).

## Usage

### With cargo lambda CLI

```bash
# Test V1 nested format
cargo lambda invoke middleware-auth-lambda --data-file test-events/apigw-v1-with-authorizer.json

# Test V1 flat format
cargo lambda invoke middleware-auth-lambda --data-file test-events/apigw-v1-flat-authorizer.json

# Test V2 format
cargo lambda invoke middleware-auth-lambda --data-file test-events/apigw-v2-with-authorizer.json
```

### Expected Behavior

1. **Adapter Extraction**: turul-mcp-aws-lambda extracts authorizer context from request extensions
2. **Header Injection**: Converts fields to `x-authorizer-*` headers (camelCase → snake_case):
   - `userId` → `x-authorizer-user_id: user-123`
   - `tenantId` → `x-authorizer-tenant_id: tenant-456`
   - `role` → `x-authorizer-role: admin`
   - `permissions` → `x-authorizer-permissions: read,write,delete`
   - `customClaim` → `x-authorizer-custom_claim: example-value`
3. **Middleware Processing**: AuthMiddleware reads headers and stores in session state
4. **Tool Access**: Tools can access via `session.get_typed_state("authorizer")` using snake_case keys

## Verification

Check the logs for debug messages showing authorizer context extraction:

```
📋 Authorizer context: user_id = user-123
📋 Authorizer context: tenant_id = tenant-456
📋 Authorizer context: role = admin
📋 Authorizer context: permissions = read,write,delete
📋 Authorizer context: custom_claim = example-value
✅ Extracted 5 authorizer fields
```

## Field Sanitization

Field names are sanitized to valid HTTP header format:

1. Convert camelCase to snake_case
2. Convert to ASCII lowercase
3. Non-alphanumeric characters (except `-` and `_`) replaced with `-`
4. Unicode characters replaced with `-`

Examples:
- `userId` → `user_id`
- `tenantId` → `tenant_id`
- `customClaim` → `custom_claim`
- `APIKey` → `api_key` (acronyms as single unit)
- `HTTPSEnabled` → `https_enabled`
- `user@email` → `user-email`
- `subscription_tier` → `subscription_tier`

## Transport

This example uses MCP 2026-07-28 **Streamable HTTP** transport via REST API (V1). REST API supports standard HTTP POST with full request/response control, making it compatible with Streamable HTTP. The 2026 core is stateless: each request carries its own `_meta` (protocolVersion, clientInfo, clientCapabilities); there is no `initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id`.

**Note**: All three authorizer shapes (V1 nested, V1 flat, V2) are supported for context extraction. However, Streamable HTTP transport requires REST API (V1).
