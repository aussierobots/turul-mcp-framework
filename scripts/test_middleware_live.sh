#!/bin/bash
# Live test for middleware examples using different ports
#
# All three examples build against the default (2026-07-28 stateless)
# feature set: no initialize handshake, no Mcp-Session-Id. See
# scripts/lib/mcp2026.sh.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

source "$SCRIPT_DIR/lib/mcp2026.sh"

echo "🧪 Testing Middleware Examples (Live)"
echo "======================================"
echo ""

# Test 1: Logging Server
echo "📝 Test 1: middleware-logging-server (port 8670)"
echo "------------------------------------------------"
cargo build -p middleware-logging-server > /dev/null 2>&1
RUST_LOG=error ./target/debug/middleware-logging-server --port 8670 > /tmp/middleware_logging.log 2>&1 &
SERVER_PID=$!

if ! mcp2026_wait_for_server 8670; then
    echo "❌ Logging server did not answer server/discover"
    tail -20 /tmp/middleware_logging.log
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

RESPONSE=$(mcp2026_request "http://127.0.0.1:8670/mcp" "server/discover" "" '{}')
SERVER_NAME=$(echo "$RESPONSE" | jq -r '.result._meta."io.modelcontextprotocol/serverInfo".name // "ERROR"')
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
cargo build -p middleware-rate-limit-server > /dev/null 2>&1
RUST_LOG=error ./target/debug/middleware-rate-limit-server --port 8671 > /tmp/middleware_ratelimit.log 2>&1 &
SERVER_PID=$!

# Plain TCP-open readiness check, not server/discover: the rate limiter runs
# pre-session on every dispatch, so probing with an MCP request would
# consume the quota this test counts.
ready=0
for _ in $(seq 1 50); do
    if (exec 3<>"/dev/tcp/127.0.0.1/8671") 2>/dev/null; then
        exec 3<&- 3>&-
        ready=1
        break
    fi
    sleep 0.3
done
if [ "$ready" -ne 1 ]; then
    echo "❌ Rate limit server did not open its port"
    tail -20 /tmp/middleware_ratelimit.log
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

echo "Sending 5 requests..."
for i in 1 2 3 4 5; do
    mcp2026_request "http://127.0.0.1:8671/mcp" "tools/list" "" '{}' > /dev/null
done

# 6th request should hit rate limit
RATE_LIMIT_RESPONSE=$(mcp2026_request "http://127.0.0.1:8671/mcp" "tools/list" "" '{}')
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
cargo build -p middleware-auth-server > /dev/null 2>&1
RUST_LOG=error ./target/debug/middleware-auth-server --port 8672 > /tmp/middleware_auth.log 2>&1 &
SERVER_PID=$!

if ! mcp2026_wait_for_server 8672; then
    echo "❌ Auth server did not answer server/discover"
    tail -20 /tmp/middleware_auth.log
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

# Test without API key (should be rejected before it ever reaches the tool)
UNAUTH_RESPONSE=$(mcp2026_request "http://127.0.0.1:8672/mcp" "tools/call" "whoami" '{"name":"whoami","arguments":{}}')
UNAUTH_ERROR=$(echo "$UNAUTH_RESPONSE" | jq -r '.error.code // empty')
if [ "$UNAUTH_ERROR" = "-32001" ]; then
    echo "✅ Unauthenticated request blocked (error code: $UNAUTH_ERROR)"
else
    echo "❌ Expected auth error -32001, got: $UNAUTH_ERROR"
    echo "Response: $UNAUTH_RESPONSE"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

# Test with valid API key
echo "Testing with valid API key..."
body=$(jq -n --argjson meta "$(mcp2026_meta)" \
    '{jsonrpc:"2.0", id:1, method:"tools/call", params:{name:"whoami", arguments:{}, _meta:$meta}}')
AUTH_RESPONSE=$(curl -s -X POST "http://127.0.0.1:8672/mcp" \
    -H "Content-Type: application/json" -H "Accept: application/json" \
    -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: tools/call" -H "Mcp-Name: whoami" \
    -H "X-API-Key: secret-key-123" \
    -d "$body")

USER_ID=$(echo "$AUTH_RESPONSE" | jq -r '.result.content[0].text // empty' | jq -r '.output.user_id // .user_id // empty')
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
