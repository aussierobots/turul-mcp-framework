#!/bin/bash
#
# Phase 2: Resource Servers - Intent-Based Verification
# Tests resources/list and resources/read with actual content verification
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "======================================================================"
echo "Phase 2: Resource Servers - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify resources/list and resources/read work"
echo "                   with actual content and template substitution"
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
    pkill -f "resource-server" 2>/dev/null || true
    pkill -f "resources-server" 2>/dev/null || true
    pkill -f "resource-test-server" 2>/dev/null || true
    pkill -f "function-resource-server" 2>/dev/null || true
    pkill -f "dynamic-resource-server" 2>/dev/null || true
    pkill -f "session-aware-resource-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper function to test a resource server
test_resource_server() {
    local server_name=$1
    local port=$2
    local test_description=$3

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "----------------------------------------"

    # Start server
    echo "Starting server..."
    RUST_LOG=error timeout 10s cargo run --bin "$server_name" -- --port "$port" &
    SERVER_PID=$!
    sleep 5

    # Check if server is running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: Server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Initialize and get session ID
    echo "Initializing MCP session..."
    SESSION_ID=$(curl -i -s -X POST "http://127.0.0.1:$port/mcp" \
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

    # Test 1: List resources
    echo "Test 1: Listing resources..."
    RESOURCES_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "MCP-Session-ID: $SESSION_ID" \
        -d '{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}')

    RESOURCE_COUNT=$(echo "$RESOURCES_RESPONSE" | jq -r '.result.resources | length // 0')

    if [ "$RESOURCE_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: No resources found"
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
    READ_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "MCP-Session-ID: $SESSION_ID" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"resources/read\",\"params\":{\"uri\":\"$FIRST_URI\"}}")

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
    TEMPLATES_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "MCP-Session-ID: $SESSION_ID" \
        -d '{"jsonrpc":"2.0","id":4,"method":"resources/templates/list","params":{}}')

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

    PASSED=$((PASSED + 1))
    echo -e "${GREEN}SUCCESS${NC}: $server_name verification complete"
    echo ""
    return 0
}

# Test all 5 resource servers (using their hardcoded ports from main.rs)
# Note: dynamic-resource-server moved to Phase 5 - it's a tools server
# Note: Both function-resource-server and session-aware-resource-server use port 8008,
#       but this works because test_resource_server kills each server after testing
test_resource_server "resource-server" 8007 "Basic resource server with McpResource derive"
test_resource_server "resources-server" 8041 "Development team resource server with external files"
test_resource_server "resource-test-server" 8043 "Comprehensive E2E test server with all resource patterns"
test_resource_server "function-resource-server" 8008 "Function-based resources with templates"
test_resource_server "session-aware-resource-server" 8008 "Session-aware resources with personalization"

# Final summary
echo "======================================================================"
echo "Phase 2 Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ PHASE 2 COMPLETE${NC}: All resource servers verified"
    exit 0
else
    echo -e "${RED}❌ PHASE 2 FAILED${NC}: $FAILED server(s) failed verification"
    exit 1
fi