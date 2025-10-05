#!/bin/bash
# Live test for Lambda middleware authentication example

set -e

echo "🧪 Testing Lambda Middleware Authentication Example (Live)"
echo "==========================================================="
echo ""

# Start Lambda in background
echo "🚀 Starting Lambda server..."
RUST_LOG=error cargo lambda watch --package middleware-auth-lambda > /tmp/lambda-middleware.log 2>&1 &
LAMBDA_PID=$!

# Wait for server to start
echo "⏳ Waiting for Lambda to initialize..."
sleep 6

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    kill $LAMBDA_PID 2>/dev/null || true
    wait $LAMBDA_PID 2>/dev/null || true
}
trap cleanup EXIT

# Test 1: Initialize without API key (should succeed - initialize skips auth)
echo "📋 Test 1: Initialize without API key (should succeed)"
INIT_RESPONSE=$(curl -s -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

echo "$INIT_RESPONSE" | jq .

SERVER_NAME=$(echo "$INIT_RESPONSE" | jq -r '.result.serverInfo.name // "ERROR"')
SESSION_ID=$(echo "$INIT_RESPONSE" | jq -r '.result.meta.sessionId // "ERROR"')

if [ "$SERVER_NAME" = "middleware-auth-lambda" ]; then
    echo "✅ Initialize succeeded (server: $SERVER_NAME)"
    echo "   Session ID: $SESSION_ID"
else
    echo "❌ Initialize failed"
    echo "Response: $INIT_RESPONSE"
    exit 1
fi

echo ""

# Test 2: tools/list without API key (should fail with -32001)
echo "📋 Test 2: tools/list without API key (should fail with -32001)"
TOOLS_RESPONSE=$(curl -s -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}')

echo "$TOOLS_RESPONSE" | jq .

ERROR_CODE=$(echo "$TOOLS_RESPONSE" | jq -r '.error.code // "NONE"')

if [ "$ERROR_CODE" = "-32001" ]; then
    echo "✅ Authentication correctly rejected request without API key"
else
    echo "❌ Expected error code -32001, got: $ERROR_CODE"
    exit 1
fi

echo ""

# Test 3: tools/list with valid API key (should succeed)
echo "📋 Test 3: tools/list with valid API key (should succeed)"
TOOLS_AUTH_RESPONSE=$(curl -s -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H 'X-API-Key: secret-key-123' \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}')

echo "$TOOLS_AUTH_RESPONSE" | jq .

HAS_TOOLS=$(echo "$TOOLS_AUTH_RESPONSE" | jq -r '.result.tools // "ERROR"')

if [ "$HAS_TOOLS" != "ERROR" ] && [ "$HAS_TOOLS" != "null" ]; then
    echo "✅ tools/list succeeded with valid API key"
else
    ERROR_MSG=$(echo "$TOOLS_AUTH_RESPONSE" | jq -r '.error.message // "Unknown error"')
    echo "❌ tools/list failed: $ERROR_MSG"
    exit 1
fi

echo ""
echo "==========================================================="
echo "✅ All Lambda middleware tests passed!"
echo ""
echo "Valid API keys for testing:"
echo "  - secret-key-123 (user-alice)"
echo "  - secret-key-456 (user-bob)"
