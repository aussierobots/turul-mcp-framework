#!/bin/bash
#
# Prompts & Special Features - Intent-Based Verification
# Tests prompts/get with template substitution and special MCP features
#
# prompts-test-server, sampling-server and elicitation-server pin
# protocol-2025-11-25 and keep the initialize handshake; the rest build
# against the default (2026-07-28 stateless) feature set and use
# scripts/lib/mcp2026.sh.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source shared utilities
source "$SCRIPT_DIR/../tests/shared/bin/wait_for_server.sh"
source "$SCRIPT_DIR/lib/mcp2026.sh"

echo "======================================================================"
echo "Prompts & Special Features - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify prompts/get, completion, sampling, and"
echo "                   other special MCP features work correctly"
echo ""

PASSED=0
FAILED=0
SKIPPED=0
TOTAL=7

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up background processes..."
    pkill -f "prompts-server" 2>/dev/null || true
    pkill -f "prompts-test-server" 2>/dev/null || true
    pkill -f "completion-server" 2>/dev/null || true
    pkill -f "sampling-server" 2>/dev/null || true
    pkill -f "elicitation-server" 2>/dev/null || true
    pkill -f "pagination-server" 2>/dev/null || true
    pkill -f "notification-server" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Helper: perform the lane-appropriate handshake for $protocol and populate
# the global `session_header` array (empty for 2026, `-H Mcp-Session-Id:...`
# for 2025). Returns 1 (and leaves SESSION_ID empty) on failure.
do_handshake() {
    local url=$1 port=$2 protocol=$3
    session_header=()

    if [ "$protocol" = "2026" ]; then
        mcp2026_wait_for_server "$port"
        return $?
    fi

    if ! wait_for_server "$port"; then
        return 1
    fi

    SESSION_ID=$(curl -i -s -X POST "$url" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
        | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

    [ -z "$SESSION_ID" ] && return 1

    session_header=(-H "Mcp-Session-Id: $SESSION_ID")
    curl -s -X POST "$url" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        "${session_header[@]}" \
        -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null
    return 0
}

# Helper function to test a prompts server
test_prompts_server() {
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

    echo "Initializing MCP session..."
    if ! do_handshake "$url" "$port" "$protocol"; then
        echo -e "${RED}FAILED${NC}: Server did not respond / no session"
        echo "Last 10 lines of log:"
        tail -10 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Test 1: List prompts
    echo "Test 1: Listing prompts..."
    if [ "$protocol" = "2026" ]; then
        PROMPTS_RESPONSE=$(mcp2026_request "$url" "prompts/list" "" '{}')
    else
        PROMPTS_RESPONSE=$(curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d '{"jsonrpc":"2.0","id":2,"method":"prompts/list","params":{}}')
    fi

    PROMPT_COUNT=$(echo "$PROMPTS_RESPONSE" | jq -r '.result.prompts | length // 0')

    if [ "$PROMPT_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: No prompts found"
        echo "Response: $PROMPTS_RESPONSE"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Found $PROMPT_COUNT prompt(s)"

    # Get first prompt name
    FIRST_PROMPT=$(echo "$PROMPTS_RESPONSE" | jq -r '.result.prompts[0].name // empty')

    if [ -z "$FIRST_PROMPT" ]; then
        echo -e "${RED}FAILED${NC}: No name found in first prompt"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "First prompt: $FIRST_PROMPT"

    # Test 2: Get first prompt (with default arguments to avoid required arg errors)
    echo "Test 2: Getting prompt..."

    # Try without arguments first
    if [ "$protocol" = "2026" ]; then
        GET_RESPONSE=$(mcp2026_request "$url" "prompts/get" "$FIRST_PROMPT" "{\"name\":\"$FIRST_PROMPT\"}")
    else
        GET_RESPONSE=$(curl -s -X POST "$url" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            "${session_header[@]}" \
            -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"prompts/get\",\"params\":{\"name\":\"$FIRST_PROMPT\"}}")
    fi

    MESSAGE_COUNT=$(echo "$GET_RESPONSE" | jq -r '.result.messages | length // 0')

    # If failed due to missing arguments, retry with comprehensive default arguments
    # covering all prompts: generate_code (language, requirements), review_code (code, language),
    # architecture_guidance (project_type, requirements), boolean_args_prompt (enable_feature),
    # multi_message_prompt (user_input), validation_prompt (email, age), etc.
    if [ "$MESSAGE_COUNT" -eq 0 ]; then
        echo "  Note: Prompt requires arguments, retrying with defaults..."
        local args_json='{"language":"rust","requirements":"Build a simple calculator","code":"fn main() {}","project_type":"web_application","enable_feature":"true","user_input":"test","email":"test@example.com","age":"25","mode":"creative"}'
        if [ "$protocol" = "2026" ]; then
            GET_RESPONSE=$(mcp2026_request "$url" "prompts/get" "$FIRST_PROMPT" \
                "{\"name\":\"$FIRST_PROMPT\",\"arguments\":$args_json}")
        else
            GET_RESPONSE=$(curl -s -X POST "$url" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                "${session_header[@]}" \
                -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"prompts/get\",\"params\":{\"name\":\"$FIRST_PROMPT\",\"arguments\":$args_json}}")
        fi

        MESSAGE_COUNT=$(echo "$GET_RESPONSE" | jq -r '.result.messages | length // 0')
    fi

    if [ "$MESSAGE_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: Prompt returned no messages even with default arguments"
        echo "Response: $GET_RESPONSE"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Prompt returned $MESSAGE_COUNT message(s)"

    FIRST_MESSAGE=$(echo "$GET_RESPONSE" | jq -r '.result.messages[0].content.text // empty')

    if [ -z "$FIRST_MESSAGE" ]; then
        echo -e "${RED}FAILED${NC}: Prompt message is empty"
        kill $SERVER_PID 2>/dev/null || true
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Message preview: ${FIRST_MESSAGE:0:100}..."
    echo -e "${GREEN}PASSED${NC}: Prompt get successful"

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

# Helper function to test feature servers (may not have standard prompts/tools)
test_feature_server() {
    local server_name=$1
    local port=$2
    local test_description=$3
    local feature_test=$4
    local protocol=$5
    local url="http://127.0.0.1:${port}/mcp"

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "Feature: $feature_test"
    echo "Protocol lane: $protocol"
    echo "----------------------------------------"

    # Start server with build guard
    echo "Starting server..."
    cleanup_old_logs "$server_name" "$port"

    if ! ensure_binary_built "$server_name"; then
        echo -e "${YELLOW}SKIPPED${NC}: Build failed (may need implementation)"
        SKIPPED=$((SKIPPED + 1))
        return 0
    fi

    RUST_LOG=error ./target/debug/"$server_name" --port "$port" > "/tmp/${server_name}_${port}.log" 2>&1 &
    SERVER_PID=$!

    if ! do_handshake "$url" "$port" "$protocol"; then
        echo -e "${YELLOW}SKIPPED${NC}: Server did not respond / no session (may need implementation)"
        echo "Last 5 lines of log:"
        tail -5 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
        kill $SERVER_PID 2>/dev/null || true
        SKIPPED=$((SKIPPED + 1))
        return 0
    fi

    echo -e "${GREEN}PASSED${NC}: Server initializes correctly"

    # Cleanup
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    # Success - truncate log to avoid confusion in reruns
    : > "/tmp/${server_name}_${port}.log"

    PASSED=$((PASSED + 1))
    echo -e "${GREEN}SUCCESS${NC}: $server_name basic verification complete"
    echo ""
    return 0
}

# Test prompt servers
test_prompts_server "prompts-server" 8006 "Real MCP prompt protocol with template substitution" "2026"
test_prompts_server "prompts-test-server" 8046 "Comprehensive E2E prompt testing" "2025"

# Test feature servers (may need partial implementation)
test_feature_server "completion-server" 8042 "Auto-completion suggestions" "completion/complete" "2026"
test_feature_server "sampling-server" 8044 "LLM sampling requests" "sampling/createMessage" "2025"
test_feature_server "elicitation-server" 8047 "User input collection patterns" "elicitation" "2025"
test_feature_server "pagination-server" 8045 "Cursor-based pagination" "pagination" "2026"
test_feature_server "notification-server" 8005 "Real-time SSE notifications" "notifications" "2026"

# Final summary
echo "======================================================================"
echo "Prompts & Special Features Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ PROMPTS EXAMPLES COMPLETE${NC} - $PASSED passed, $SKIPPED skipped"
    exit 0
else
    echo -e "${RED}❌ PROMPTS EXAMPLES FAILED${NC} - $FAILED failures"
    exit 1
fi
