# Lambda Authorizer Integration - Testing Guide

Complete test plan to verify Lambda API Gateway authorizer context extraction works end-to-end.

## Test Hierarchy

```
Phase 1: Unit Tests              ✅ Adapter logic isolation
    ↓
Phase 2: Compile Verification    ✅ Type safety
    ↓
Phase 3: Lambda Local Testing    🧪 Full integration
    ↓
Phase 4: Session State Testing   🧪 Tool access
```

---

## Phase 1: Unit Tests ✅

**Tests**: Core extraction and sanitization logic

```bash
cargo test --package turul-mcp-aws-lambda --lib adapter::tests::authorizer_tests
```

**Coverage**:
- ✅ `test_sanitize_field_name_camelcase` - userId → user_id (snake_case conversion)
- ✅ `test_sanitize_field_name_snake_case` - device_id → device_id (unchanged)
- ✅ `test_sanitize_field_name_acronyms` - APIKey → api_key (acronyms as unit)
- ✅ `test_sanitize_field_name_with_numbers` - userId123 → user_id123
- ✅ `test_sanitize_field_name_special_chars` - user@email → user-email
- ✅ `test_sanitize_field_name_unicode` - 用户 → --
- ✅ `test_extract_authorizer_no_context` - Empty context handling
- ✅ `test_lambda_to_hyper_without_authorizer` - No crash without authorizer

**Expected**: `test result: ok. 6 passed; 0 failed`

---

## Phase 2: Compile Verification ✅

**Tests**: Code compiles, types are correct

```bash
cargo build --package middleware-auth-lambda
cargo build --package turul-mcp-aws-lambda
```

**Expected**: `Finished \`dev\` profile`

---

## Phase 3: Lambda Local Testing 🧪

### Quick Test (Automated)

```bash
./test_authorizer.sh
```

**What it does**:
1. Runs unit tests
2. Builds example
3. Starts Lambda locally (background)
4. Tests API Gateway V2 event
5. Tests API Gateway V1 event
6. Checks logs for authorizer extraction
7. Cleanup

**Expected output**:
```
✅ Unit tests passed (6/6)
✅ Build successful
✅ Lambda started
📋 Authorizer context: user_id = user-123
📋 Authorizer context: tenant_id = tenant-456
📋 Authorizer context: role = admin
📋 Authorizer context: permissions = read,write,delete
📋 Authorizer context: custom_claim = example-value
✅ Extracted 5 authorizer fields
✅ All tests completed!
```

### Manual Test (Step-by-Step)

#### Step 1: Start Lambda

```bash
export RUST_LOG=debug
cargo lambda watch --package middleware-auth-lambda
```

**Expected logs**:
```
🚀 Starting AWS Lambda MCP Server with Authentication Middleware
🔐 Authentication middleware registered
  Valid API keys:
    - secret-key-123 (user-alice)
    - secret-key-456 (user-bob)
✅ Lambda MCP server created successfully with middleware and CORS
🎯 Lambda handler ready with auth middleware
```

#### Step 2: Test API Gateway V2 (HTTP API)

```bash
# Terminal 2
cargo lambda invoke middleware-auth-lambda \
  --data-file test-events/apigw-v2-with-authorizer.json \
  --output-format json | jq .
```

**Expected response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "serverInfo": {
      "name": "middleware-auth-lambda",
      "version": "1.0.0"
    }
  }
}
```

**Expected logs** (Terminal 1):
```
📋 Authorizer context: user_id = user-123
📋 Authorizer context: tenant_id = tenant-456
📋 Authorizer context: role = admin
📋 Authorizer context: permissions = read,write,delete
📋 Authorizer context: custom_claim = example-value
✅ Extracted 5 authorizer fields
```

#### Step 3: Test API Gateway V1 (REST API)

```bash
cargo lambda invoke middleware-auth-lambda \
  --data-file test-events/apigw-v1-with-authorizer.json \
  --output-format json | jq .
```

**Expected**: Same response and logs as V2 (proves format independence!)

---

## Phase 4: Session State Testing 🧪

**Goal**: Verify authorizer context flows to session state and tools can access it

### Test Event Breakdown

Both test events include:

| Field | Value | Stored As |
|-------|-------|-----------|
| `userId` | user-123 | `user_id` |
| `tenantId` | tenant-456 | `tenant_id` |
| `role` | admin | `role` |
| `permissions` | read,write,delete | `permissions` |
| `customClaim` | example-value | `custom_claim` |

**Format Differences**:

- **V2** (HTTP API): `requestContext.authorizer.{field}`
- **V1** (REST API): `requestContext.authorizer.lambda.{field}`

---

## Verification Checklist

### ✅ Success Criteria

- [x] **Unit tests pass**: 6/6 tests passing
- [x] **Compiles cleanly**: No errors or warnings
- [ ] **Lambda starts**: Handler ready message appears
- [ ] **V2 extraction works**: Debug logs show 5 authorizer fields
- [ ] **V1 extraction works**: Same fields extracted as V2
- [ ] **Field sanitization**: camelCase → snake_case (`userId` → `user_id`)
- [ ] **Defensive behavior**: No crashes on missing/invalid data
- [ ] **Initialize succeeds**: Returns valid MCP initialize response

### 🔍 What to Look For

**✅ Working (Expected)**:
```
📋 Authorizer context: user_id = user-123
✅ Extracted 5 authorizer fields
```

**❌ Not Working (Investigate)**:
```
⚠️  No authorizer fields extracted
Error: Missing accountid from authorizer
```

---

## Troubleshooting

### No Authorizer Logs Appear

**Cause**: `RUST_LOG` not set to `debug`

**Fix**:
```bash
export RUST_LOG=debug
cargo lambda watch --package middleware-auth-lambda
```

### "cargo-lambda not found"

**Install**:
```bash
pip install cargo-lambda
```

Or with Homebrew:
```bash
brew tap cargo-lambda/cargo-lambda
brew install cargo-lambda
```

### DynamoDB Connection Error

**Cause**: DynamoDB backend requires AWS credentials

**Quick fix**: Test in CI without actual DynamoDB:
```bash
export CI_SANDBOX=1  # Uses in-memory storage for testing
cargo lambda watch --package middleware-auth-lambda
```

### Field Names Don't Match

**Remember**: Fields are snake_case!
- ✅ `user_id` (not `userId`)
- ✅ `tenant_id` (not `tenantId`)
- ✅ `custom_claim` (not `customClaim`)

---

## Test Artifacts

After running `./test_authorizer.sh`:

- `/tmp/lambda-output.log` - Full Lambda logs with debug info
- `/tmp/v2-response.json` - API Gateway V2 response
- `/tmp/v1-response.json` - API Gateway V1 response

**View authorizer extraction**:
```bash
cat /tmp/lambda-output.log | grep "Authorizer context"
```

---

## Next Steps

1. **Run automated test**: `./test_authorizer.sh`
2. **Review logs**: Check `/tmp/lambda-output.log`
3. **Verify responses**: Check JSON responses are valid MCP
4. **Add custom fields**: Modify test events with your own fields

---

## Example: Custom Authorizer Fields

Edit `test-events/apigw-v2-with-authorizer.json`:

```json
{
  "requestContext": {
    "authorizer": {
      "userId": "user-123",
      "organizationId": "org-456",      // ← Add your field
      "subscriptionTier": "premium",     // ← Add your field
      "features": "analytics,exports"    // ← Add your field
    }
  }
}
```

**Stored in session as**:
- `organizationid` → org-456
- `subscriptiontier` → premium
- `features` → analytics,exports

**Access in tools**:
```rust
let authorizer: Option<HashMap<String, String>> =
    session.get_typed_state("authorizer").await.ok().flatten();

let org_id = authorizer
    .and_then(|ctx| ctx.get("organizationid").cloned())
    .unwrap_or_default();
```

---

## Reference

- **ADR**: `docs/adr/013-lambda-authorizer-integration.md`
- **Implementation**: `crates/turul-mcp-aws-lambda/src/adapter.rs`
- **Example**: `examples/middleware-auth-lambda/src/main.rs`
- **Unit Tests**: Search for `authorizer_tests` in `adapter.rs`
