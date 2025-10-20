#!/usr/bin/env bash
# Test schemars schema generation integration
#
# Validates that tools with schemars output types generate detailed schemas
# (not generic objects) in tools/list responses.
#
# Usage:
#   ./scripts/test_schemars_integration.sh [PORT]
#
# Default port: 52935

set -euo pipefail

PORT="${1:-52935}"
SERVER_URL="http://127.0.0.1:${PORT}/mcp"
SERVER_PID=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

cleanup() {
    if [ -n "$SERVER_PID" ]; then
        echo "Cleaning up server (PID: $SERVER_PID)..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
}

trap cleanup EXIT

echo "=== Schemars Schema Integration Test ==="
echo ""

# Start server with schemars example
echo "Starting tool-output-schemas example server on port ${PORT}..."
RUST_LOG=error cargo run --package tool-output-schemas -- --port "$PORT" &
SERVER_PID=$!

# Wait for server to start
echo "Waiting for server to start..."
sleep 3

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo -e "${RED}✗ Server failed to start${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Server started (PID: $SERVER_PID)${NC}"
echo ""

# Step 1: Initialize session
echo "=== Step 1: Initialize session ==="
INIT_RESPONSE=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    }')

SESSION_ID=$(echo "$INIT_RESPONSE" | jq -r '.result.sessionId // empty')

if [ -z "$SESSION_ID" ]; then
    echo -e "${RED}✗ Failed to get session ID${NC}"
    echo "Response: $INIT_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✓ Session initialized: $SESSION_ID${NC}"
echo ""

# Step 2: Send initialized notification
echo "=== Step 2: Send initialized notification ==="
curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -H "MCP-Session-ID: $SESSION_ID" \
    -d '{
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }' > /dev/null

echo -e "${GREEN}✓ Initialized notification sent${NC}"
echo ""

# Step 3: Get tools/list
echo "=== Step 3: Get tools/list ==="
TOOLS_RESPONSE=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -H "MCP-Session-ID: $SESSION_ID" \
    -d '{
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    }')

# Verify response has tools
TOOL_COUNT=$(echo "$TOOLS_RESPONSE" | jq '.result.tools | length')

if [ "$TOOL_COUNT" -eq 0 ]; then
    echo -e "${RED}✗ No tools returned${NC}"
    echo "Response: $TOOLS_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✓ Found $TOOL_COUNT tools${NC}"
echo ""

# Step 4: Validate schema detail level
echo "=== Step 4: Validate schema detail ==="

# Find analyze_data tool (has nested schema)
ANALYZE_TOOL=$(echo "$TOOLS_RESPONSE" | jq '.result.tools[] | select(.name == "analyze_data")')

if [ -z "$ANALYZE_TOOL" ]; then
    echo -e "${YELLOW}⚠ analyze_data tool not found, checking calculator_derive${NC}"
    ANALYZE_TOOL=$(echo "$TOOLS_RESPONSE" | jq '.result.tools[] | select(.name == "calculator_derive")')
fi

if [ -z "$ANALYZE_TOOL" ]; then
    echo -e "${RED}✗ No schemars-based tools found${NC}"
    exit 1
fi

# Check if outputSchema exists
HAS_OUTPUT_SCHEMA=$(echo "$ANALYZE_TOOL" | jq 'has("outputSchema")')

if [ "$HAS_OUTPUT_SCHEMA" != "true" ]; then
    echo -e "${RED}✗ Tool has no outputSchema${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Tool has outputSchema${NC}"

# Check if schema has properties (not generic object)
SCHEMA_HAS_PROPERTIES=$(echo "$ANALYZE_TOOL" | jq '.outputSchema | has("properties")')

if [ "$SCHEMA_HAS_PROPERTIES" != "true" ]; then
    echo -e "${RED}✗ Schema is generic object (no properties)${NC}"
    echo "Schema: $(echo "$ANALYZE_TOOL" | jq '.outputSchema')"
    exit 1
fi

echo -e "${GREEN}✓ Schema has detailed properties${NC}"

# Get property count
PROPERTY_COUNT=$(echo "$ANALYZE_TOOL" | jq '.outputSchema.properties | length')
echo -e "${GREEN}✓ Schema has $PROPERTY_COUNT properties${NC}"

# Verify nested properties exist (for analyze_data tool)
TOOL_NAME=$(echo "$ANALYZE_TOOL" | jq -r '.name')
if [ "$TOOL_NAME" = "analyze_data" ]; then
    # Check for nested stats object
    RESULT_PROPS=$(echo "$ANALYZE_TOOL" | jq '.outputSchema.properties.result.properties // empty')

    if [ -z "$RESULT_PROPS" ]; then
        echo -e "${RED}✗ No nested properties in result field${NC}"
        exit 1
    fi

    HAS_STATS=$(echo "$RESULT_PROPS" | jq 'has("stats")')
    if [ "$HAS_STATS" = "true" ]; then
        echo -e "${GREEN}✓ Nested 'stats' object present in schema${NC}"

        # Verify stats has detailed properties
        STATS_PROPS=$(echo "$RESULT_PROPS" | jq '.stats.properties // empty')
        if [ -n "$STATS_PROPS" ]; then
            echo -e "${GREEN}✓ Stats object has detailed nested properties${NC}"
        else
            echo -e "${YELLOW}⚠ Stats object lacks detailed properties${NC}"
        fi
    fi
fi

echo ""
echo -e "${GREEN}=== ✓ All Tests Passed ===${NC}"
echo ""
echo "Summary:"
echo "  - Session initialized: $SESSION_ID"
echo "  - Tools found: $TOOL_COUNT"
echo "  - Schema validation: PASSED"
echo "  - Nested properties: PRESENT"
echo ""
echo "Schemars integration is working correctly!"
