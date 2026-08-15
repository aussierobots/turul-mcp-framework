#!/bin/bash
#
# middleware-rate-limit-server - Intent-Based Verification
#
# The server builds against the default (2026-07-28 stateless) feature set
# and keys its limiter on a pre-session stateless identity (X-API-Key, or
# "anonymous" when absent) — there is no initialize handshake or
# Mcp-Session-Id. See scripts/lib/mcp2026.sh.
#
# Verifies: the first N=5 requests succeed, and the 6th is rejected with
# JSON-RPC error -32003 (RateLimitExceeded).

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

source "$SCRIPT_DIR/lib/mcp2026.sh"

PORT=8675
URL="http://127.0.0.1:${PORT}/mcp"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

cleanup() {
    [ -n "$PID" ] && kill "$PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== middleware-rate-limit-server verification ==="
cargo build -p middleware-rate-limit-server > /dev/null 2>&1

RUST_LOG=error ./target/debug/middleware-rate-limit-server --port "$PORT" > /tmp/middleware_rate_limit.log 2>&1 &
PID=$!

# Plain TCP-open readiness check (NOT server/discover): the rate limiter
# runs pre-session on every dispatch including server/discover, so probing
# with an actual MCP request would consume the very quota this test counts.
ready=0
for _ in $(seq 1 50); do
    if (exec 3<>"/dev/tcp/127.0.0.1/${PORT}") 2>/dev/null; then
        exec 3<&- 3>&-
        ready=1
        break
    fi
    sleep 0.3
done
if [ "$ready" -ne 1 ]; then
    echo -e "${RED}FAILED${NC}: server did not open its port within 15s"
    tail -20 /tmp/middleware_rate_limit.log
    exit 1
fi

FAILED=0
for i in 1 2 3 4 5; do
    RESPONSE=$(mcp2026_request "$URL" "tools/list" "" '{}')
    ERROR_CODE=$(echo "$RESPONSE" | jq -r '.error.code // empty')
    if [ -n "$ERROR_CODE" ]; then
        echo -e "${RED}FAILED${NC}: request $i unexpectedly rejected (code $ERROR_CODE)"
        echo "Response: $RESPONSE"
        FAILED=1
    else
        echo "Request $i: OK"
    fi
done

RATE_LIMIT_RESPONSE=$(mcp2026_request "$URL" "tools/list" "" '{}')
ERROR_CODE=$(echo "$RATE_LIMIT_RESPONSE" | jq -r '.error.code // empty')
if [ "$ERROR_CODE" = "-32003" ]; then
    echo -e "${GREEN}PASSED${NC}: 6th request rate limited correctly (code -32003)"
else
    echo -e "${RED}FAILED${NC}: expected -32003 on the 6th request, got: $ERROR_CODE"
    echo "Response: $RATE_LIMIT_RESPONSE"
    FAILED=1
fi

if [ "$FAILED" = "0" ]; then
    echo -e "${GREEN}✅ RATE LIMIT MIDDLEWARE VERIFIED${NC}"
    exit 0
else
    echo -e "${RED}❌ RATE LIMIT MIDDLEWARE FAILED${NC}"
    exit 1
fi
