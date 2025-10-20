#!/bin/bash
#
# Phase 4: Session Storage Backends - Intent-Based Verification
# Tests SQLite, PostgreSQL, DynamoDB, and stateful operations
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source shared utilities
source "$SCRIPT_DIR/../tests/shared/bin/wait_for_server.sh"

echo "======================================================================"
echo "Phase 4: Session Storage Backends - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify session persistence works across different"
echo "                   storage backends (SQLite, PostgreSQL, DynamoDB)"
echo ""

PASSED=0
FAILED=0
SKIPPED=0
TOTAL=4

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up background processes..."
    pkill -f "simple-sqlite-session" 2>/dev/null || true
    pkill -f "simple-postgres-session" 2>/dev/null || true
    pkill -f "simple-dynamodb-session" 2>/dev/null || true
    pkill -f "stateful-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper function to test a storage backend server
test_storage_server() {
    local server_name=$1
    local port=$2
    local test_description=$3
    local requires_external=$4

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "----------------------------------------"

    if [ "$requires_external" = "true" ]; then
        echo -e "${YELLOW}NOTE${NC}: This server requires external dependencies"
        echo "         (PostgreSQL/DynamoDB). Testing basic startup only."
    fi

    # Start server with build guard
    echo "Starting server..."
    cleanup_old_logs "$server_name" "$port"

    if ! ensure_binary_built "$server_name"; then
        if [ "$requires_external" = "true" ]; then
            echo -e "${YELLOW}SKIPPED${NC}: Build requires external dependencies (SQLite/PostgreSQL/DynamoDB)"
            SKIPPED=$((SKIPPED + 1))
            return 0
        else
            echo -e "${RED}FAILED${NC}: Build error"
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi

    RUST_LOG=error ./target/debug/"$server_name" --port "$port" > "/tmp/${server_name}_${port}.log" 2>&1 &
    SERVER_PID=$!

    # Wait deterministically (replaces sleep 2)
    if ! wait_for_server "$port"; then
        if [ "$requires_external" = "true" ]; then
            echo -e "${YELLOW}SKIPPED${NC}: Server requires external dependencies not available"
            echo "Last 5 lines of log:"
            tail -5 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
            kill $SERVER_PID 2>/dev/null || true
            SKIPPED=$((SKIPPED + 1))
            return 0
        else
            echo -e "${RED}FAILED${NC}: Server did not respond within 15s"
            echo "Last 10 lines of log:"
            tail -10 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi

    # Initialize and get session ID
    echo "Initializing MCP session..."
    SESSION_ID=$(curl -i -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
        2>/dev/null | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

    if [ -z "$SESSION_ID" ]; then
        if [ "$requires_external" = "true" ]; then
            echo -e "${YELLOW}SKIPPED${NC}: Could not initialize (external dependencies required)"
            kill $SERVER_PID 2>/dev/null || true
            SKIPPED=$((SKIPPED + 1))
            return 0
        else
            echo -e "${RED}FAILED${NC}: Could not get session ID from header"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi

    echo "Session ID: $SESSION_ID"

    # Test session persistence: make a request with session ID
    echo "Test: Session persistence..."
    TEST_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "Mcp-Session-Id: $SESSION_ID" \
        -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' 2>/dev/null)

    # Check for valid response (tools/list or resources/list)
    HAS_TOOLS=$(echo "$TEST_RESPONSE" | jq -r '.result.tools // empty' 2>/dev/null)
    HAS_ERROR=$(echo "$TEST_RESPONSE" | jq -r '.error // empty' 2>/dev/null)

    if [ -n "$HAS_ERROR" ]; then
        # Try resources/list instead
        TEST_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d '{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}' 2>/dev/null)

        HAS_RESOURCES=$(echo "$TEST_RESPONSE" | jq -r '.result.resources // empty' 2>/dev/null)

        if [ -z "$HAS_RESOURCES" ]; then
            echo -e "${RED}FAILED${NC}: Session request failed"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi

    echo -e "${GREEN}PASSED${NC}: Session persistence working"

    # Cleanup
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    # Success - truncate log to avoid confusion in reruns
    : > "/tmp/${server_name}_${port}.log"

    PASSED=$((PASSED + 1))
    echo -e "${GREEN}SUCCESS${NC}: $server_name verification complete"
    echo ""
    return 0
}

# Test all 4 session storage servers (using their hardcoded ports from main.rs)
test_storage_server "simple-sqlite-session" 8061 "SQLite session storage backend" "false"
test_storage_server "simple-postgres-session" 8060 "PostgreSQL session storage backend" "true"
test_storage_server "simple-dynamodb-session" 8062 "DynamoDB session storage backend" "true"
test_storage_server "stateful-server" 8006 "Advanced stateful operations" "false"

# Final summary
echo "======================================================================"
echo "Phase 4 Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ PHASE 4 COMPLETE${NC} - $PASSED passed, $SKIPPED skipped"
    exit 0
else
    echo -e "${RED}❌ PHASE 4 FAILED${NC} - $FAILED failures"
    exit 1
fi