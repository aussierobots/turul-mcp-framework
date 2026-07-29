#!/bin/bash
#
# Advanced/Composite Servers - Intent-Based Verification
# Tests complex servers with real business logic and multiple capabilities
#
# tools-test-server pins protocol-2025-11-25 and keeps the initialize
# handshake; the rest build against the default (2026-07-28 stateless)
# feature set and use scripts/lib/mcp2026.sh.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source shared utilities
source "$SCRIPT_DIR/../tests/shared/bin/wait_for_server.sh"
source "$SCRIPT_DIR/lib/mcp2026.sh"

echo "======================================================================"
echo "Advanced/Composite Servers - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify complex servers with real business logic,"
echo "                   multiple capabilities, and advanced features"
echo ""

PASSED=0
FAILED=0
SKIPPED=0
TOTAL=5

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up background processes..."
    pkill -f "audit-trail-server" 2>/dev/null || true
    pkill -f "zero-config-getting-started" 2>/dev/null || true
    pkill -f "function-macro-server" 2>/dev/null || true
    pkill -f "derive-macro-server" 2>/dev/null || true
    pkill -f "tools-test-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper function to test an advanced server
# protocol: "2025" (initialize handshake) or "2026" (stateless)
test_advanced_server() {
    local server_name=$1
    local port=$2
    local test_description=$3
    local capabilities=$4
    local protocol=$5

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "Capabilities: $capabilities"
    echo "Protocol lane: $protocol"
    echo "----------------------------------------"

    # Compute actual_port FIRST (before any usage in logs or curl)
    local actual_port
    if [[ "$port" == *":"* ]]; then
        actual_port=$(echo "$port" | cut -d: -f2)
    else
        actual_port="$port"
    fi
    local url="http://127.0.0.1:${actual_port}/mcp"

    # Start server with build guard
    echo "Starting server..."
    cleanup_old_logs "$server_name" "$actual_port"

    if ! ensure_binary_built "$server_name"; then
        echo -e "${RED}FAILED${NC}: Build error"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Launch (handle full-address format)
    if [[ "$port" == *":"* ]]; then
        # Full address format - pass as positional argument
        RUST_LOG=error ./target/debug/"$server_name" "$port" > "/tmp/${server_name}_${actual_port}.log" 2>&1 &
    else
        # Port number only - use --port flag
        RUST_LOG=error ./target/debug/"$server_name" --port "$port" > "/tmp/${server_name}_${actual_port}.log" 2>&1 &
    fi
    SERVER_PID=$!

    local session_header=()
    if [ "$protocol" = "2026" ]; then
        if ! mcp2026_wait_for_server "$actual_port"; then
            echo -e "${RED}FAILED${NC}: Server did not answer server/discover within 15s"
            echo "Last 10 lines of log:"
            tail -10 "/tmp/${server_name}_${actual_port}.log" 2>/dev/null || echo "(no log)"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi
    else
        if ! wait_for_server "$actual_port"; then
            echo -e "${RED}FAILED${NC}: Server did not respond within 15s"
            echo "Last 10 lines of log:"
            tail -10 "/tmp/${server_name}_${actual_port}.log" 2>/dev/null || echo "(no log)"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi

        # Initialize and get session ID
        echo "Initializing MCP session..."
        SESSION_ID=$(curl -i -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
            | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

        if [ -z "$SESSION_ID" ]; then
            echo -e "${RED}FAILED${NC}: Could not get session ID from header"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi

        echo "Session ID: $SESSION_ID"
        session_header=(-H "Mcp-Session-Id: $SESSION_ID")

        # Send notifications/initialized to complete strict lifecycle (returns 202, no response body)
        curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null
    fi

    # Test capabilities based on server type
    local tests_passed=0
    local tests_total=0

    # Test Tools if applicable
    if echo "$capabilities" | grep -q "tools"; then
        echo "Testing capability: Tools..."
        tests_total=$((tests_total + 1))

        if [ "$protocol" = "2026" ]; then
            TOOLS_RESPONSE=$(mcp2026_request "$url" "tools/list" "" '{}')
        else
            TOOLS_RESPONSE=$(curl -s -X POST "$url" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                "${session_header[@]}" \
                -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}')
        fi

        TOOL_COUNT=$(echo "$TOOLS_RESPONSE" | jq -r '.result.tools | length // 0')

        if [ "$TOOL_COUNT" -gt 0 ]; then
            echo "  ✓ Found $TOOL_COUNT tool(s)"
            tests_passed=$((tests_passed + 1))
        else
            echo "  ✗ No tools found"
            echo "  Response: $TOOLS_RESPONSE"
        fi
    fi

    # Test Resources if applicable
    if echo "$capabilities" | grep -q "resources"; then
        echo "Testing capability: Resources..."
        tests_total=$((tests_total + 1))

        if [ "$protocol" = "2026" ]; then
            RESOURCES_RESPONSE=$(mcp2026_request "$url" "resources/list" "" '{}')
        else
            RESOURCES_RESPONSE=$(curl -s -X POST "$url" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                "${session_header[@]}" \
                -d '{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}')
        fi

        RESOURCE_COUNT=$(echo "$RESOURCES_RESPONSE" | jq -r '.result.resources | length // 0')

        if [ "$RESOURCE_COUNT" -gt 0 ]; then
            echo "  ✓ Found $RESOURCE_COUNT resource(s)"
            tests_passed=$((tests_passed + 1))
        else
            echo "  ✗ No resources found"
            echo "  Response: $RESOURCES_RESPONSE"
        fi
    fi

    # Test Prompts if applicable
    if echo "$capabilities" | grep -q "prompts"; then
        echo "Testing capability: Prompts..."
        tests_total=$((tests_total + 1))

        if [ "$protocol" = "2026" ]; then
            PROMPTS_RESPONSE=$(mcp2026_request "$url" "prompts/list" "" '{}')
        else
            PROMPTS_RESPONSE=$(curl -s -X POST "$url" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                "${session_header[@]}" \
                -d '{"jsonrpc":"2.0","id":4,"method":"prompts/list","params":{}}')
        fi

        PROMPT_COUNT=$(echo "$PROMPTS_RESPONSE" | jq -r '.result.prompts | length // 0')

        if [ "$PROMPT_COUNT" -gt 0 ]; then
            echo "  ✓ Found $PROMPT_COUNT prompt(s)"
            tests_passed=$((tests_passed + 1))
        else
            echo "  ✗ No prompts found"
            echo "  Response: $PROMPTS_RESPONSE"
        fi
    fi

    # Evaluate results
    if [ $tests_passed -eq $tests_total ] && [ $tests_total -gt 0 ]; then
        echo -e "${GREEN}PASSED${NC}: All $tests_total capability tests passed"
    elif [ $tests_total -eq 0 ]; then
        echo -e "${GREEN}PASSED${NC}: Server initialized successfully"
    else
        echo -e "${RED}FAILED${NC}: Only $tests_passed/$tests_total capability tests passed"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Cleanup
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    # Success - truncate log to avoid confusion in reruns
    : > "/tmp/${server_name}_${actual_port}.log"

    PASSED=$((PASSED + 1))
    echo -e "${GREEN}SUCCESS${NC}: $server_name verification complete"
    echo ""
    return 0
}

# Test showcase/demonstration servers (advanced tool patterns)
test_advanced_server "function-macro-server" 8003 "Function macro showcase with multiple parameter types" "tools" "2026"
test_advanced_server "derive-macro-server" "127.0.0.1:8765" "Real-world code generation and template engine" "tools" "2026"
test_advanced_server "tools-test-server" 8050 "Comprehensive E2E tool testing server" "tools" "2025"

# Test audit server
test_advanced_server "audit-trail-server" 8009 "Comprehensive audit logging system" "tools" "2026"

# Test tutorial server
test_advanced_server "zero-config-getting-started" 8641 "Absolute beginner tutorial (zero-configuration quickstart)" "tools" "2026"

# Final summary
echo "======================================================================"
echo "Advanced/Composite Servers Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ADVANCED SERVERS COMPLETE${NC} - $PASSED passed, $SKIPPED skipped"
    exit 0
else
    echo -e "${RED}❌ ADVANCED SERVERS FAILED${NC} - $FAILED failures"
    exit 1
fi
