#!/bin/bash
#
# Phase 5: Advanced/Composite Servers - Intent-Based Verification
# Tests complex servers with real business logic and multiple capabilities
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source shared utilities
source "$SCRIPT_DIR/../tests/shared/bin/wait_for_server.sh"

echo "======================================================================"
echo "Phase 5: Advanced/Composite Servers - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify complex servers with real business logic,"
echo "                   multiple capabilities, and advanced features"
echo ""

PASSED=0
FAILED=0
SKIPPED=0
TOTAL=10

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up background processes..."
    pkill -f "comprehensive-server" 2>/dev/null || true
    pkill -f "alert-system-server" 2>/dev/null || true
    pkill -f "audit-trail-server" 2>/dev/null || true
    pkill -f "simple-logging-server" 2>/dev/null || true
    pkill -f "zero-config-getting-started" 2>/dev/null || true
    pkill -f "function-macro-server" 2>/dev/null || true
    pkill -f "derive-macro-server" 2>/dev/null || true
    pkill -f "manual-tools-server" 2>/dev/null || true
    pkill -f "tools-test-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper function to test an advanced server
test_advanced_server() {
    local server_name=$1
    local port=$2
    local test_description=$3
    local capabilities=$4

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "Capabilities: $capabilities"
    echo "----------------------------------------"

    # Compute actual_port FIRST (before any usage in logs or curl)
    local actual_port
    if [[ "$port" == *":"* ]]; then
        actual_port=$(echo "$port" | cut -d: -f2)
    else
        actual_port="$port"
    fi

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

    # Wait deterministically (replaces sleep 5)
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
    SESSION_ID=$(curl -i -s -X POST "http://127.0.0.1:$actual_port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
        | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

    if [ -z "$SESSION_ID" ]; then
        echo -e "${RED}FAILED${NC}: Could not get session ID from header"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Session ID: $SESSION_ID"

    # Send notifications/initialized to complete strict lifecycle (returns 202, no response body)
    curl -s -X POST "http://127.0.0.1:$actual_port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "Mcp-Session-Id: $SESSION_ID" \
        -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

    # Test capabilities based on server type
    local tests_passed=0
    local tests_total=0

    # Test Tools if applicable
    if echo "$capabilities" | grep -q "tools"; then
        echo "Testing capability: Tools..."
        tests_total=$((tests_total + 1))

        TOOLS_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$actual_port/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}')

        TOOL_COUNT=$(echo "$TOOLS_RESPONSE" | jq -r '.result.tools | length // 0')

        if [ "$TOOL_COUNT" -gt 0 ]; then
            echo "  ✓ Found $TOOL_COUNT tool(s)"
            tests_passed=$((tests_passed + 1))
        else
            echo "  ✗ No tools found"
        fi
    fi

    # Test Resources if applicable
    if echo "$capabilities" | grep -q "resources"; then
        echo "Testing capability: Resources..."
        tests_total=$((tests_total + 1))

        RESOURCES_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$actual_port/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d '{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}')

        RESOURCE_COUNT=$(echo "$RESOURCES_RESPONSE" | jq -r '.result.resources | length // 0')

        if [ "$RESOURCE_COUNT" -gt 0 ]; then
            echo "  ✓ Found $RESOURCE_COUNT resource(s)"
            tests_passed=$((tests_passed + 1))
        else
            echo "  ✗ No resources found"
        fi
    fi

    # Test Prompts if applicable
    if echo "$capabilities" | grep -q "prompts"; then
        echo "Testing capability: Prompts..."
        tests_total=$((tests_total + 1))

        PROMPTS_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$actual_port/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d '{"jsonrpc":"2.0","id":4,"method":"prompts/list","params":{}}')

        PROMPT_COUNT=$(echo "$PROMPTS_RESPONSE" | jq -r '.result.prompts | length // 0')

        if [ "$PROMPT_COUNT" -gt 0 ]; then
            echo "  ✓ Found $PROMPT_COUNT prompt(s)"
            tests_passed=$((tests_passed + 1))
        else
            echo "  ✗ No prompts found"
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
test_advanced_server "function-macro-server" 8003 "Function macro showcase with multiple parameter types" "tools"
test_advanced_server "derive-macro-server" "127.0.0.1:8765" "Real-world code generation and template engine" "tools"
test_advanced_server "manual-tools-server" 8007 "Advanced manual implementation with session state" "tools"
test_advanced_server "tools-test-server" 8050 "Comprehensive E2E tool testing server" "tools"

# Test composite servers
test_advanced_server "comprehensive-server" "127.0.0.1:8002" "Development Team Integration Platform (all MCP features)" "tools,resources,prompts"
test_advanced_server "alert-system-server" 8010 "Enterprise alert management system" "tools"
test_advanced_server "audit-trail-server" 8009 "Comprehensive audit logging system" "tools"
test_advanced_server "simple-logging-server" 8008 "Simplified logging patterns" "tools"
test_advanced_server "dynamic-resource-server" 8048 "Enterprise API Data Gateway (tools server, not resources)" "tools"

# Test tutorial server
test_advanced_server "zero-config-getting-started" 8641 "Absolute beginner tutorial (zero-configuration quickstart)" "tools"

# Final summary
echo "======================================================================"
echo "Phase 5 Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ PHASE 5 COMPLETE${NC} - $PASSED passed, $SKIPPED skipped"
    exit 0
else
    echo -e "${RED}❌ PHASE 5 FAILED${NC} - $FAILED failures"
    exit 1
fi