#!/bin/bash
#
# Calculator Learning Progression - Intent-Based Verification
# Tests all 4 tool creation patterns with actual math verification.
#
# All 5 example servers below build against the default (2026-07-28
# stateless) feature set: there is no `initialize` handshake or
# `Mcp-Session-Id` — every request carries `MCP-Protocol-Version`,
# `Mcp-Method`/`Mcp-Name`, and a `_meta` block. See scripts/lib/mcp2026.sh.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

source "$SCRIPT_DIR/lib/mcp2026.sh"

echo "======================================================================"
echo "Calculator Learning Progression - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify all 4 tool creation patterns produce"
echo "                   identical, correct calculator behavior"
echo ""

PASSED=0
FAILED=0
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
    pkill -f "minimal-server" 2>/dev/null || true
    pkill -f "calculator-add-function-server" 2>/dev/null || true
    pkill -f "calculator-add-simple-server-derive" 2>/dev/null || true
    pkill -f "calculator-add-builder-server" 2>/dev/null || true
    pkill -f "calculator-add-manual-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper function to test a server
test_server() {
    local server_name=$1
    local port=$2
    local tool_name=$3
    local test_description=$4
    local url="http://127.0.0.1:${port}/mcp"

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Tool: $tool_name"
    echo "Description: $test_description"
    echo "----------------------------------------"

    # Start server (use pre-built binary to avoid compilation delays)
    echo "Starting server..."
    if [ ! -f "./target/debug/$server_name" ]; then
        echo "Building $server_name..."
        cargo build --bin "$server_name" > /dev/null 2>&1
    fi
    RUST_LOG=error ./target/debug/"$server_name" --port "$port" > "/tmp/${server_name}_${port}.log" 2>&1 &
    SERVER_PID=$!

    if ! mcp2026_wait_for_server "$port"; then
        echo -e "${RED}FAILED${NC}: Server did not answer server/discover within 15s"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Test 1: List tools
    echo "Test 1: Listing tools..."
    TOOLS_RESPONSE=$(mcp2026_request "$url" "tools/list" "" '{}')
    TOOL_COUNT=$(echo "$TOOLS_RESPONSE" | jq -r '.result.tools | length // 0')

    if [ "$TOOL_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: No tools found"
        echo "Response: $TOOLS_RESPONSE"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Found $TOOL_COUNT tool(s)"

    # Test 2: Call calculator tool with actual math
    echo "Test 2: Calling tool with math (5.0 + 3.0 = ?)..."

    # Different servers have different parameter schemas
    if [ "$tool_name" = "echo" ]; then
        # minimal-server just echoes
        CALL_RESPONSE=$(mcp2026_request "$url" "tools/call" "$tool_name" \
            "{\"name\":\"$tool_name\",\"arguments\":{\"text\":\"test\"}}")

        RESULT=$(echo "$CALL_RESPONSE" | jq -r '.result.content[0].text // empty')

        if [ -z "$RESULT" ]; then
            echo -e "${RED}FAILED${NC}: Tool call returned no result"
            echo "Response: $CALL_RESPONSE"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi

        echo "Tool result: $RESULT"
        echo -e "${GREEN}PASSED${NC}: minimal-server echo tool works"

    else
        # Calculator tools
        CALL_RESPONSE=$(mcp2026_request "$url" "tools/call" "$tool_name" \
            "{\"name\":\"$tool_name\",\"arguments\":{\"a\":5.0,\"b\":3.0}}")

        RESULT=$(echo "$CALL_RESPONSE" | jq -r '.result.content[0].text // empty')

        if [ -z "$RESULT" ]; then
            echo -e "${RED}FAILED${NC}: Tool call returned no result"
            echo "Response: $CALL_RESPONSE"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi

        # Check if result is 8.0 (5.0 + 3.0)
        if echo "$RESULT" | grep -q "8"; then
            echo "Tool result: $RESULT"
            echo -e "${GREEN}PASSED${NC}: Math correct (5.0 + 3.0 = 8.0)"
        else
            echo -e "${RED}FAILED${NC}: Math incorrect (expected 8.0, got: $RESULT)"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi

    # Cleanup
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    PASSED=$((PASSED + 1))
    echo -e "${GREEN}SUCCESS${NC}: $server_name verification complete"
    echo ""
    return 0
}

# Test all 5 servers
test_server "minimal-server" 8641 "echo" "Level 0: Absolute minimum (echo tool)"
test_server "calculator-add-function-server" 8648 "calculator_add_function" "Level 1: Function macro"
test_server "calculator-add-simple-server-derive" 8647 "calculator_add_derive" "Level 2: Derive macro"
test_server "calculator-add-builder-server" 8649 "calculator_add_builder" "Level 3: Builder pattern"
test_server "calculator-add-manual-server" 8646 "calculator_add_manual" "Level 4: Manual implementation"

# Final summary
echo "======================================================================"
echo "Calculator Learning Progression Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ CALCULATOR EXAMPLES COMPLETE${NC}: All calculator patterns verified"
    exit 0
else
    echo -e "${RED}❌ CALCULATOR EXAMPLES FAILED${NC}: $FAILED server(s) failed verification"
    exit 1
fi
