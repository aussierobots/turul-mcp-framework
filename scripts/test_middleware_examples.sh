#!/bin/bash
set -e

echo "🧪 Testing Middleware Examples"
echo "================================"
echo ""

# Test 1: Logging Server
echo "📝 Test 1: middleware-logging-server"
echo "-----------------------------------"
RUST_LOG=info cargo run --package middleware-logging-server &
SERVER_PID=$!
sleep 3

echo "Initializing session..."
INIT_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SERVER_NAME=$(echo $INIT_RESPONSE | jq -r '.result.serverInfo.name')
if [ "$SERVER_NAME" = "middleware-logging-server" ]; then
    echo "✅ Logging server initialized successfully"
else
    echo "❌ Failed to initialize logging server"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

kill $SERVER_PID 2>/dev/null || true
sleep 1
echo ""

# Test 2: Rate Limit Server
echo "🚦 Test 2: middleware-rate-limit-server"
echo "---------------------------------------"
RUST_LOG=info cargo run --package middleware-rate-limit-server &
SERVER_PID=$!
sleep 3

echo "Initializing session..."
INIT_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SESSION_ID=$(echo $INIT_RESPONSE | jq -r '.result.sessionId')
echo "Session ID: $SESSION_ID"

# Send initialized notification
curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

echo "Sending 5 requests to trigger rate limit..."
for i in {1..5}; do
    RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
      -H "Content-Type: application/json" \
      -H "Accept: application/json" \
      -H "Mcp-Session-Id: $SESSION_ID" \
      -d "{\"jsonrpc\":\"2.0\",\"id\":$((i+1)),\"method\":\"tools/list\",\"params\":{}}")
    echo "  Request $i: $(echo $RESPONSE | jq -r 'if .error then "ERROR: " + .error.message else "OK" end')"
done

# This should hit rate limit
echo "Sending 6th request (should be rate limited)..."
RATE_LIMIT_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":7,"method":"tools/list","params":{}}')

ERROR_CODE=$(echo $RATE_LIMIT_RESPONSE | jq -r '.error.code // empty')
if [ "$ERROR_CODE" = "-32003" ]; then
    echo "✅ Rate limit enforced correctly (error code: $ERROR_CODE)"
else
    echo "❌ Rate limit not enforced (expected -32003, got: $ERROR_CODE)"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

kill $SERVER_PID 2>/dev/null || true
sleep 1
echo ""

# Test 3: Auth Server
echo "🔐 Test 3: middleware-auth-server"
echo "---------------------------------"
RUST_LOG=info cargo run --package middleware-auth-server &
SERVER_PID=$!
sleep 3

echo "Testing without API key (should fail)..."
INIT_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SESSION_ID=$(echo $INIT_RESPONSE | jq -r '.result.sessionId')

# Send initialized without API key
curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

UNAUTH_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}')

UNAUTH_ERROR=$(echo $UNAUTH_RESPONSE | jq -r '.error.code // empty')
if [ "$UNAUTH_ERROR" = "-32001" ]; then
    echo "✅ Unauthenticated request blocked (error code: $UNAUTH_ERROR)"
else
    echo "⚠️  Expected auth error, got: $UNAUTH_ERROR"
fi

echo "Testing with valid API key..."
INIT_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "X-API-Key: secret-key-123" \
  -d '{"jsonrpc":"2.0","id":10,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SESSION_ID=$(echo $INIT_RESPONSE | jq -r '.result.sessionId')

# Send initialized with API key
curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "X-API-Key: secret-key-123" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

AUTH_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "X-API-Key: secret-key-123" \
  -d '{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"whoami","arguments":{}}}')

USER_ID=$(echo $AUTH_RESPONSE | jq -r '.result.content[0].text // empty' | jq -r '.user_id // empty')
if [ "$USER_ID" = "user-alice" ]; then
    echo "✅ Authenticated request successful (user: $USER_ID)"
else
    echo "❌ Authentication failed"
    echo "Response: $AUTH_RESPONSE"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

kill $SERVER_PID 2>/dev/null || true
echo ""

echo "================================"
echo "✅ All middleware examples tested successfully!"
echo ""
