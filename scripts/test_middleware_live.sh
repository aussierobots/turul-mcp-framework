#!/bin/bash
# Live test for middleware examples using different ports

set -e

echo "🧪 Testing Middleware Examples (Live)"
echo "======================================"
echo ""

# Test 1: Logging Server
echo "📝 Test 1: middleware-logging-server (port 8670)"
echo "------------------------------------------------"
RUST_LOG=error cargo run --package middleware-logging-server -- --port 8670 &
SERVER_PID=$!
sleep 4

RESPONSE=$(curl -s -X POST http://localhost:8670/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SERVER_NAME=$(echo "$RESPONSE" | jq -r '.result.serverInfo.name // "ERROR"')
if [ "$SERVER_NAME" = "middleware-logging-server" ]; then
    echo "✅ Logging server initialized successfully"
else
    echo "❌ Failed to initialize logging server"
    echo "Response: $RESPONSE"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null || true
echo ""

# Test 2: Rate Limit Server
echo "🚦 Test 2: middleware-rate-limit-server (port 8671)"
echo "---------------------------------------------------"
RUST_LOG=error cargo run --package middleware-rate-limit-server -- --port 8671 &
SERVER_PID=$!
sleep 4

INIT_RESPONSE=$(curl -si -X POST http://localhost:8671/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SESSION_ID=$(echo "$INIT_RESPONSE" | grep -i "mcp-session-id:" | awk '{print $2}' | tr -d '\r')
echo "Session ID: $SESSION_ID"

# Send initialized notification
curl -s -X POST http://localhost:8671/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

# Send 5 requests
echo "Sending 5 requests..."
for i in {1..5}; do
    curl -s -X POST http://localhost:8671/mcp \
      -H "Content-Type: application/json" \
      -H "Accept: application/json" \
      -H "Mcp-Session-Id: $SESSION_ID" \
      -d "{\"jsonrpc\":\"2.0\",\"id\":$((i+1)),\"method\":\"tools/list\",\"params\":{}}" > /dev/null
done

# 6th request should hit rate limit
RATE_LIMIT_RESPONSE=$(curl -s -X POST http://localhost:8671/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":7,"method":"tools/list","params":{}}')

ERROR_CODE=$(echo "$RATE_LIMIT_RESPONSE" | jq -r '.error.code // empty')
if [ "$ERROR_CODE" = "-32003" ]; then
    echo "✅ Rate limit enforced correctly (error code: $ERROR_CODE)"
else
    echo "❌ Rate limit not enforced (expected -32003, got: $ERROR_CODE)"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null || true
echo ""

# Test 3: Auth Server
echo "🔐 Test 3: middleware-auth-server (port 8672)"
echo "---------------------------------------------"
RUST_LOG=error cargo run --package middleware-auth-server -- --port 8672 &
SERVER_PID=$!
sleep 4

# Test with valid API key
INIT_RESPONSE=$(curl -si -X POST http://localhost:8672/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "X-API-Key: secret-key-123" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SESSION_ID=$(echo "$INIT_RESPONSE" | grep -i "mcp-session-id:" | awk '{print $2}' | tr -d '\r')

# Send initialized with API key
curl -s -X POST http://localhost:8672/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "X-API-Key: secret-key-123" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

# Call whoami tool
AUTH_RESPONSE=$(curl -s -X POST http://localhost:8672/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "X-API-Key: secret-key-123" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"whoami","arguments":{}}}')

USER_ID=$(echo "$AUTH_RESPONSE" | jq -r '.result.content[0].text // empty' | jq -r '.user_id // empty')
if [ "$USER_ID" = "user-alice" ]; then
    echo "✅ Authentication successful (user: $USER_ID)"
else
    echo "❌ Authentication failed"
    echo "Response: $AUTH_RESPONSE"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null || true
echo ""

echo "======================================"
echo "✅ All middleware examples tested successfully!"
echo ""
echo "Examples can be run with:"
echo "  cargo run --package middleware-logging-server -- --port 8670"
echo "  cargo run --package middleware-rate-limit-server -- --port 8671"
echo "  cargo run --package middleware-auth-server -- --port 8672"
