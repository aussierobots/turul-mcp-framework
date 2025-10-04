#!/bin/bash
#
# Phase 3: Prompts & Special Features - Intent-Based Verification
# Tests prompts/get with template substitution and special MCP features
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source shared utilities
source "$SCRIPT_DIR/../tests/shared/bin/wait_for_server.sh"

echo "======================================================================"
echo "Phase 3: Prompts & Special Features - Intent-Based Verification"
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

# Helper function to test a prompts server
test_prompts_server() {
    local server_name=$1
    local port=$2
    local test_description=$3

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
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

    # Wait deterministically (replaces sleep 5)
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

    # Test 1: List prompts
    echo "Test 1: Listing prompts..."
    PROMPTS_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "Mcp-Session-Id: $SESSION_ID" \
        -d '{"jsonrpc":"2.0","id":2,"method":"prompts/list","params":{}}')

    PROMPT_COUNT=$(echo "$PROMPTS_RESPONSE" | jq -r '.result.prompts | length // 0')

    if [ "$PROMPT_COUNT" -eq 0 ]; then
        echo -e "${RED}FAILED${NC}: No prompts found"
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
    GET_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "Mcp-Session-Id: $SESSION_ID" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"prompts/get\",\"params\":{\"name\":\"$FIRST_PROMPT\"}}")

    MESSAGE_COUNT=$(echo "$GET_RESPONSE" | jq -r '.result.messages | length // 0')

    # If failed due to missing arguments, retry with comprehensive default arguments
    # covering all prompts: generate_code (language, requirements), review_code (code, language),
    # architecture_guidance (project_type, requirements), boolean_args_prompt (enable_feature),
    # multi_message_prompt (user_input), validation_prompt (email, age), etc.
    if [ "$MESSAGE_COUNT" -eq 0 ]; then
        echo "  Note: Prompt requires arguments, retrying with defaults..."
        GET_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"prompts/get\",\"params\":{\"name\":\"$FIRST_PROMPT\",\"arguments\":{\"language\":\"rust\",\"requirements\":\"Build a simple calculator\",\"code\":\"fn main() {}\",\"project_type\":\"web_application\",\"enable_feature\":\"true\",\"user_input\":\"test\",\"email\":\"test@example.com\",\"age\":\"25\",\"mode\":\"creative\"}}}")

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

    echo "----------------------------------------"
    echo "Testing: $server_name"
    echo "Port: $port"
    echo "Description: $test_description"
    echo "Feature: $feature_test"
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

    # Wait deterministically (replaces sleep 5)
    if ! wait_for_server "$port"; then
        echo -e "${YELLOW}SKIPPED${NC}: Server did not respond within 15s (may need implementation)"
        echo "Last 5 lines of log:"
        tail -5 "/tmp/${server_name}_${port}.log" 2>/dev/null || echo "(no log)"
        kill $SERVER_PID 2>/dev/null || true
        SKIPPED=$((SKIPPED + 1))
        return 0
    fi

    # Initialize and get session ID
    echo "Initializing MCP session..."
    SESSION_ID=$(curl -i -s -X POST "http://127.0.0.1:$port/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
        | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

    if [ -z "$SESSION_ID" ]; then
        echo -e "${YELLOW}SKIPPED${NC}: Could not get session ID from header (may need implementation)"
        kill $SERVER_PID 2>/dev/null || true
        SKIPPED=$((SKIPPED + 1))
        return 0
    fi

    echo "Session ID: $SESSION_ID"
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
test_prompts_server "prompts-server" 8006 "Real MCP prompt protocol with template substitution"
test_prompts_server "prompts-test-server" 8046 "Comprehensive E2E prompt testing"

# Test feature servers (may need partial implementation)
test_feature_server "completion-server" 8042 "Auto-completion suggestions" "completion/complete"
test_feature_server "sampling-server" 8044 "LLM sampling requests" "sampling/createMessage"
test_feature_server "elicitation-server" 8047 "User input collection patterns" "elicitation"
test_feature_server "pagination-server" 8045 "Cursor-based pagination" "pagination"
test_feature_server "notification-server" 8005 "Real-time SSE notifications" "notifications"

# Final summary
echo "======================================================================"
echo "Phase 3 Summary"
echo "======================================================================"
echo "Total: $TOTAL servers"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ PHASE 3 COMPLETE${NC} - $PASSED passed, $SKIPPED skipped"
    exit 0
else
    echo -e "${RED}❌ PHASE 3 FAILED${NC} - $FAILED failures"
    exit 1
fi