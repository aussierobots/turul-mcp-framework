# ADR 013: Lambda Authorizer Integration

**Status:** Accepted
**Date:** 2025-01-06
**Authors:** Architecture Team
**Context:** API Gateway Lambda authorizers need seamless integration with MCP middleware

---

## Context

AWS API Gateway authorizers (both REST API/V1 and HTTP API/V2) add authentication/authorization context to Lambda requests. This context typically contains information like:

- User identifiers (`userId`)
- Tenant/organization IDs (`tenantId`)
- User roles (`role`)
- Permissions (`permissions`)
- Custom claims from your authorization logic

The framework needs to:

1. Extract authorizer context from both API Gateway V1 (REST API) and V2 (HTTP API)
2. Make this context available to middleware and tools
3. Maintain transport independence (HTTP ≠ Lambda)
4. Follow existing session state patterns
5. Handle invalid/missing context gracefully

## Decision

We implement **Lambda adapter-based authorizer extraction** with middleware consumption:

### Architecture Flow

```
API Gateway Authorizer
    ↓ (adds context to request)
Lambda Extensions (RequestContext)
    ↓ (preserved across conversion)
turul-mcp-aws-lambda/adapter.rs
    ↓ (extracts + sanitizes + injects)
x-authorizer-* HTTP Headers
    ↓ (standard HTTP metadata)
Middleware (RequestContext.metadata)
    ↓ (reads headers + stores)
Session State (authorizer key)
    ↓ (tools access)
Tool Implementation
```

### Core Components

1. **Field Sanitization** (`adapter.rs::sanitize_authorizer_field_name`)
   ```rust
   pub fn sanitize_authorizer_field_name(field: &str) -> String {
       field
           .to_ascii_lowercase()
           .chars()
           .map(|c| {
               if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                   c
               } else {
                   '-'
               }
           })
           .collect()
   }
   ```

   **Rules:**
   - Convert to ASCII lowercase
   - Replace non-alphanumeric (except `_` and `-`) with dash
   - Unicode characters → `-`

   **Examples:**
   - `userId` → `userid`
   - `tenantId` → `tenantid`
   - `user@email` → `user-email`
   - `subscription_tier` → `subscription_tier`
   - `用户` → `--`

2. **Context Extraction** (`adapter.rs::extract_authorizer_context`)
   ```rust
   pub fn extract_authorizer_context(req: &LambdaRequest) -> HashMap<String, String> {
       let mut fields = HashMap::new();

       let Some(request_context) = req.extensions().get::<RequestContext>() else {
           return fields; // No context, return empty
       };

       match request_context {
           RequestContext::ApiGatewayV2(ctx) => {
               // V2: HashMap format
               if let Some(ref authorizer) = ctx.authorizer {
                   for (key, value) in &authorizer.fields {
                       let sanitized_key = sanitize_authorizer_field_name(key);
                       let value_str = match value {
                           serde_json::Value::String(s) => s.clone(),
                           other => other.to_string(),
                       };
                       fields.insert(sanitized_key, value_str);
                   }
               }
           }
           RequestContext::ApiGatewayV1(ctx) => {
               // V1: Map in fields["lambda"]
               if let Some(serde_json::Value::Object(auth_map)) = ctx.authorizer.fields.get("lambda") {
                   for (key, value) in auth_map {
                       let sanitized_key = sanitize_authorizer_field_name(key);
                       let value_str = match value {
                           serde_json::Value::String(s) => s.clone(),
                           other => other.to_string(),
                       };
                       fields.insert(sanitized_key, value_str);
                   }
               }
           }
           _ => {} // Other contexts (ALB, etc.) - no authorizer
       }

       fields
   }
   ```

3. **Header Injection** (`adapter.rs::lambda_to_hyper_request`)
   ```rust
   pub fn lambda_to_hyper_request(lambda_req: LambdaRequest) -> Result<hyper::Request<MappedFullBody>> {
       // Extract authorizer context BEFORE consuming request
       let authorizer_fields = extract_authorizer_context(&lambda_req);

       // Convert to parts
       let (mut parts, lambda_body) = lambda_req.into_parts();

       // Inject authorizer fields as x-authorizer-* headers (defensive)
       for (field_name, field_value) in authorizer_fields {
           let header_name = format!("x-authorizer-{}", field_name);

           // Skip entry if either fails (defensive - don't break request)
           let Ok(name) = http::HeaderName::from_str(&header_name) else {
               debug!("Skipping authorizer field '{}' - invalid header name", field_name);
               continue;
           };

           let Ok(value) = http::HeaderValue::from_str(&field_value) else {
               debug!("Skipping authorizer field '{}' - invalid header value", field_name);
               continue;
           };

           parts.headers.insert(name, value);
           trace!("Injected authorizer header: {} = {}", header_name, field_value);
       }

       // ... rest of conversion
   }
   ```

4. **Middleware Consumption** (user code in `middleware-auth-lambda` example)
   ```rust
   async fn before_dispatch(
       &self,
       ctx: &mut RequestContext<'_>,
       _session: Option<&dyn SessionView>,
       injection: &mut SessionInjection,
   ) -> Result<(), MiddlewareError> {
       // Extract Lambda authorizer context from x-authorizer-* headers
       let metadata: &serde_json::Map<String, serde_json::Value> = ctx.metadata();
       let mut authorizer_context = HashMap::new();

       for (key, value) in metadata.iter() {
           if let Some(field_name) = key.strip_prefix("x-authorizer-") {
               if let Some(value_str) = value.as_str() {
                   authorizer_context.insert(field_name.to_string(), value_str.to_string());
               }
           }
       }

       if !authorizer_context.is_empty() {
           injection.set_state("authorizer", json!(authorizer_context));
       }

       Ok(())
   }
   ```

5. **Tool Access** (user code)
   ```rust
   #[mcp_tool(name = "get_account")]
   async fn get_account(
       #[param(session)] session: SessionContext,
   ) -> McpResult<serde_json::Value> {
       let authorizer: Option<HashMap<String, String>> =
           session.get_typed_state("authorizer").await.ok().flatten();

       let account_id = authorizer
           .and_then(|ctx| ctx.get("accountid").cloned())  // lowercase!
           .ok_or_else(|| McpError::validation("Missing accountid"))?;

       Ok(json!({ "accountId": account_id }))
   }
   ```

### Defensive Programming

**Never Fail Requests:**
- Invalid header names → skip field (log at debug level)
- Invalid header values → skip field (log at debug level)
- Missing RequestContext → return empty HashMap
- Missing authorizer → return empty HashMap

**Graceful Degradation:**
- Middleware continues even if authorizer context is empty
- Tools check for `None` authorizer state
- No crashes from malformed data

### API Gateway Format Differences

| Aspect | V1 (REST API) | V2 (HTTP API) |
|--------|---------------|----------------|
| Context Type | `RequestContext::ApiGatewayV1` | `RequestContext::ApiGatewayV2` |
| Authorizer Location | `ctx.authorizer.fields.get("lambda")` | `ctx.authorizer.fields` |
| Data Structure | `serde_json::Map` | `HashMap<String, Value>` |
| Example | `{"lambda": {"userId": "..."}}` | `{"userId": "..."}` |

### Testing

**Unit Tests** (`adapter.rs::tests::authorizer_tests`):
- Field sanitization (camelCase, snake_case, special chars, unicode)
- Empty context handling
- Header injection without breaking existing headers

**Integration Tests** (test events):
- `apigw-v1-with-authorizer.json` - REST API format
- `apigw-v2-with-authorizer.json` - HTTP API format

**Verification Command:**
```bash
cargo lambda invoke middleware-auth-lambda \
  --data-file test-events/apigw-v2-with-authorizer.json
```

**Expected Debug Output:**
```
📋 Authorizer context: userid = user-123
📋 Authorizer context: tenantid = tenant-456
📋 Authorizer context: role = admin
📋 Authorizer context: permissions = read,write,delete
📋 Authorizer context: customclaim = example-value
✅ Extracted 5 authorizer fields
```

## Consequences

### Positive

1. **Transport Independence**: Authorizer context becomes standard HTTP headers
2. **Middleware Reusable**: Same middleware pattern as HTTP transport
3. **Type Safe**: Field names sanitized for HTTP header compatibility
4. **Defensive**: Never fails requests due to authorizer data
5. **API Gateway Parity**: Works with both V1 (REST) and V2 (HTTP) formats
6. **Session Integration**: Uses existing session state patterns
7. **Tool Access**: Clean `session.get_typed_state("authorizer")` API

### Negative

1. **Field Name Case**: Original casing lost (userId → userid, tenantId → tenantid)
   - **Mitigation**: Documented clearly in examples and test events
   - **Rationale**: HTTP headers are case-insensitive anyway
2. **Lambda-Specific**: Only works in Lambda environment
   - **Mitigation**: Gracefully degrades (empty context) in HTTP-only
3. **Prefix Requirement**: Tools must know to use `x-authorizer-` metadata
   - **Mitigation**: Middleware example shows extraction pattern

### Trade-offs Considered

**Alternative 1: Pass RequestContext directly to middleware**
- ❌ Breaks transport independence
- ❌ Middleware becomes Lambda-aware
- ❌ Violates abstraction boundary

**Alternative 2: Use custom session metadata field**
- ❌ Duplicates data (already in request extensions)
- ❌ Requires Lambda-specific middleware code
- ✅ Could preserve original casing

**Alternative 3: Preserve original field casing**
- ❌ Complex case-insensitive header matching
- ❌ More error-prone code
- ❌ HTTP headers are case-insensitive anyway

## References

- **ADR 012**: Middleware Architecture
- **Implementation**: `crates/turul-mcp-aws-lambda/src/adapter.rs`
- **Example**: `examples/middleware-auth-lambda/`
- **Tests**: `examples/middleware-auth-lambda/test-events/`
- **AWS Docs**: [API Gateway Lambda Authorizers](https://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-lambda-authorizer-output.html)

## Related Work

- Session state patterns (SessionContext)
- Middleware architecture (McpMiddleware trait)
- HTTP header metadata extraction
- Lambda adapter request conversion
