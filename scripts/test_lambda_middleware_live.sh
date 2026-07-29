#!/bin/bash
# Live test for Lambda middleware authentication example
#
# middleware-auth-lambda builds against the default (2026-07-28 stateless)
# feature set: no initialize handshake, no Mcp-Session-Id. See
# scripts/lib/mcp2026.sh. Requires cargo-lambda.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

source "$SCRIPT_DIR/lib/mcp2026.sh"

URL="http://localhost:9000/lambda-url/middleware-auth-lambda"

echo "🧪 Testing Lambda Middleware Authentication Example (Live)"
echo "==========================================================="
echo ""

# Start Lambda in background
echo "🚀 Starting Lambda server..."
RUST_LOG=error cargo lambda watch --package middleware-auth-lambda > /tmp/lambda-middleware.log 2>&1 &
LAMBDA_PID=$!

cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    kill $LAMBDA_PID 2>/dev/null || true
    wait $LAMBDA_PID 2>/dev/null || true
}
trap cleanup EXIT

# Wait for the Lambda Runtime API emulator to actually accept requests.
# A 200 or the known DynamoDB-unavailable 500 both mean the handler booted;
# any other outcome (000, cold-start still compiling) keeps polling.
echo "⏳ Waiting for Lambda to initialize..."
ready=0
LAST_BODY=""
for _ in $(seq 1 45); do
    LAST_BODY=$(curl -s -X POST "$URL" \
        -H "Content-Type: application/json" -H "Accept: application/json" \
        -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: server/discover" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"server/discover\",\"params\":{\"_meta\":$(mcp2026_meta)}}" \
        2>/dev/null || true)
    if echo "$LAST_BODY" | jq -e '.result or .error' > /dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 2
done
if [ "$ready" -ne 1 ]; then
    echo "❌ Lambda emulator never answered server/discover"
    tail -40 /tmp/lambda-middleware.log
    exit 1
fi

DYNAMODB_UNAVAILABLE=0
if echo "$LAST_BODY" | jq -e '.error.message == "Failed to create session"' > /dev/null 2>&1; then
    DYNAMODB_UNAVAILABLE=1
fi

if [ "$DYNAMODB_UNAVAILABLE" = "1" ]; then
    echo "⚠️  SKIPPED: DynamoDB unavailable in this environment (real AWS/local DynamoDB required)"
    echo "This example needs a live 'mcp-sessions' DynamoDB table even for server/discover;"
    echo "only the build (see test_lambda_middleware.sh) and cargo-lambda boot are verified here."
    exit 0
fi

# Test 1: server/discover without API key (should succeed - discover skips auth)
echo "📋 Test 1: server/discover without API key (should succeed)"
DISCOVER_RESPONSE="$LAST_BODY"
echo "$DISCOVER_RESPONSE" | jq .

SERVER_NAME=$(echo "$DISCOVER_RESPONSE" | jq -r '.result._meta."io.modelcontextprotocol/serverInfo".name // "ERROR"')

if [ "$SERVER_NAME" = "middleware-auth-lambda" ]; then
    echo "✅ server/discover succeeded (server: $SERVER_NAME)"
else
    echo "❌ server/discover failed"
    echo "Response: $DISCOVER_RESPONSE"
    exit 1
fi

echo ""

# Test 2: tools/list without API key (should fail with -32001). The auth
# middleware runs before session/DynamoDB access, so this is reachable even
# without a real DynamoDB table.
echo "📋 Test 2: tools/list without API key (should fail with -32001)"
TOOLS_RESPONSE=$(mcp2026_request "$URL" "tools/list" "" '{}')

echo "$TOOLS_RESPONSE" | jq .

ERROR_CODE=$(echo "$TOOLS_RESPONSE" | jq -r '.error.code // "NONE"')

if [ "$ERROR_CODE" = "-32001" ]; then
    echo "✅ Authentication correctly rejected request without API key"
else
    echo "❌ Expected error code -32001, got: $ERROR_CODE"
    exit 1
fi

echo ""

# Test 3: tools/list with valid API key. Past the auth gate this touches
# DynamoDB for its per-request context — without a real table/local
# DynamoDB this fails with "Failed to create session" and we skip rather
# than fail, matching the external-dependency skip used for the other
# storage-backend examples.
echo "📋 Test 3: tools/list with valid API key (should succeed)"
body=$(jq -n --argjson meta "$(mcp2026_meta)" '{jsonrpc:"2.0", id:1, method:"tools/list", params:{_meta:$meta}}')
TOOLS_AUTH_RESPONSE=$(curl -s -X POST "$URL" \
    -H "Content-Type: application/json" -H "Accept: application/json" \
    -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: tools/list" \
    -H "X-API-Key: secret-key-123" \
    -d "$body")

echo "$TOOLS_AUTH_RESPONSE" | jq .

HAS_TOOLS=$(echo "$TOOLS_AUTH_RESPONSE" | jq -r '.result.tools // "ERROR"')
ERROR_MSG=$(echo "$TOOLS_AUTH_RESPONSE" | jq -r '.error.message // empty')

if [ "$HAS_TOOLS" != "ERROR" ] && [ "$HAS_TOOLS" != "null" ]; then
    echo "✅ tools/list succeeded with valid API key"
elif [ "$ERROR_MSG" = "Failed to create session" ]; then
    echo "⚠️  SKIPPED: DynamoDB unavailable in this environment (real AWS/local DynamoDB required)"
else
    echo "❌ tools/list failed: $ERROR_MSG"
    exit 1
fi

echo ""
echo "==========================================================="
echo "✅ Lambda middleware auth-gate tests passed!"
echo ""
echo "Valid API keys for testing:"
echo "  - secret-key-123 (user-alice)"
echo "  - secret-key-456 (user-bob)"
