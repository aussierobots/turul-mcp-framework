#!/bin/bash
#
# Resource Servers - Intent-Based Verification
# Tests resources/list and resources/read with actual content verification
#
# The five example servers below straddle both specs:
#   - resource-server, resources-server, function-resource-server build
#     against the default (2026-07-28 stateless) feature set.
#   - resource-test-server and session-aware-resource-server pin
#     protocol-2025-11-25 explicitly and keep the initialize handshake.
# Each call below is tagged with its lane; see scripts/lib/mcp2026.sh for
# the stateless helper.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source shared utilities
source "$SCRIPT_DIR/../tests/shared/bin/wait_for_server.sh"
source "$SCRIPT_DIR/lib/mcp2026.sh"

echo "======================================================================"
echo "Resource Servers - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify resources/list and resources/read work"
echo "                   with actual content and template substitution"
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
    pkill -f "resource-server" 2>/dev/null || true
    pkill -f "resources-server" 2>/dev/null || true
    pkill -f "resource-test-server" 2>/dev/null || true
    pkill -f "function-resource-server" 2>/dev/null || true
    pkill -f "session-aware-resource-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper function to test a resource server
# protocol: "2025" (initialize handshake) or "2026" (stateless)
test_resource_server() {
    local server_name=$1
    local port=$2
    local test_description=$3
    local protocol=$4
    local url="http://127.0.0.1:${port}/mcp"

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "Protocol lane: $protocol"
    echo "----------------------------------------"

    # Start server with build guard
    echo "Starting server..."
    cleanup_old_logs "$server_name" "$port"

    if ! ensure_binary_built "$server_name"; then
        echo -e "${RED}FAILED${NC}: Build error"
        FAILED=$((FAILED + 1))
        return 1
    fi

    RUST_LOG=error ./target/debug/"$server_name" --port "$port" > "/tmp/${server_name}_${port}.log" 2>&1 &
    SERVER_PID=$!

    local session_header=()
    if [ "$protocol" = "2026" ]; then
        if ! mcp2026_wait_for_server "$port"; then
            echo -e "${RED}FAILED${NC}: Server did not answer server/discover within 15s"
            echo "Last 10 lines of log:"
            tail -10 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
            kill $SERVER_PID 2>/dev/null || true
            FAILED=$((FAILED + 1))
            return 1
        fi
    else
        if ! wait_for_server "$port"; then
            echo -e "${RED}FAILED${NC}: Server did not respond within 15s"
            echo "Last 10 lines of log:"
            tail -10 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
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

        # Strict lifecycle mode requires notifications/initialized before any
        # other request will be served.
        curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null
    fi

    # Test 1: List resources
    echo "Test 1: Listing resources..."
    if [ "$protocol" = "2026" ]; then
        RESOURCES_RESPONSE=$(mcp2026_request "$url" "resources/list" "" '{}')
    else
        RESOURCES_RESPONSE=$(curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d '{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}')
    fi

    RESOURCE_COUNT=$(echo "$RESOURCES_RESPONSE" | jq -r '.result.resources | length // 0')

    if [ "$RESOURCE_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: No resources found"
        echo "Response: $RESOURCES_RESPONSE"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Found $RESOURCE_COUNT resource(s)"

    # Get first resource URI
    FIRST_URI=$(echo "$RESOURCES_RESPONSE" | jq -r '.result.resources[0].uri // empty')

    if [ -z "$FIRST_URI" ]; then
        echo -e "${RED}FAILED${NC}: No URI found in first resource"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "First resource URI: $FIRST_URI"

    # Test 2: Read first resource
    echo "Test 2: Reading resource..."
    if [ "$protocol" = "2026" ]; then
        READ_RESPONSE=$(mcp2026_request "$url" "resources/read" "$FIRST_URI" "{\"uri\":\"$FIRST_URI\"}")
    else
        READ_RESPONSE=$(curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"resources/read\",\"params\":{\"uri\":\"$FIRST_URI\"}}")
    fi

    CONTENT_COUNT=$(echo "$READ_RESPONSE" | jq -r '.result.contents | length // 0')

    if [ "$CONTENT_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: Resource read returned no content"
        echo "Response: $READ_RESPONSE"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    CONTENT=$(echo "$READ_RESPONSE" | jq -r '.result.contents[0].text // .result.contents[0].blob // empty')

    if [ -z "$CONTENT" ]; then
        echo -e "${RED}FAILED${NC}: Resource content is empty"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Content preview: ${CONTENT:0:100}..."
    echo -e "${GREEN}PASSED${NC}: Resource read successful"

    # Test 3: Check for templates (if applicable)
    echo "Test 3: Checking for resource templates..."
    if [ "$protocol" = "2026" ]; then
        TEMPLATES_RESPONSE=$(mcp2026_request "$url" "resources/templates/list" "" '{}')
    else
        TEMPLATES_RESPONSE=$(curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d '{"jsonrpc":"2.0","id":4,"method":"resources/templates/list","params":{}}')
    fi

    TEMPLATE_COUNT=$(echo "$TEMPLATES_RESPONSE" | jq -r '.result.resourceTemplates | length // 0')

    if [ "$TEMPLATE_COUNT" -gt 0 ]; then
        echo "Found $TEMPLATE_COUNT template(s)"
        FIRST_TEMPLATE=$(echo "$TEMPLATES_RESPONSE" | jq -r '.result.resourceTemplates[0].uriTemplate // empty')
        echo "First template: $FIRST_TEMPLATE"
    else
        echo "No templates (this is OK for some servers)"
    fi

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

# Test all 5 resource servers. Ports below match each server's hardcoded
# bind_address in its own main.rs, NOT the --port flag: neither
# function-resource-server nor session-aware-resource-server actually parses
# a --port argument (no clap/arg handling in main.rs), so passing --port has
# no effect and the server always binds its hardcoded address.
test_resource_server "resource-server" 8007 "Basic resource server with McpResource derive" "2026"
test_resource_server "resources-server" 8041 "Development team resource server with external files" "2026"
test_resource_server "resource-test-server" 8043 "Comprehensive E2E test server with all resource patterns" "2025"
test_resource_server "function-resource-server" 8008 "Function-based resources with templates" "2026"
test_resource_server "session-aware-resource-server" 8010 "Session-aware resources with personalization" "2025"

# Final summary
echo "======================================================================"
echo "Resource Servers Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ RESOURCE SERVERS COMPLETE${NC} - $PASSED passed, $SKIPPED skipped"
    exit 0
else
    echo -e "${RED}❌ RESOURCE SERVERS FAILED${NC} - $FAILED failures"
    exit 1
fi
