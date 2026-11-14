#!/bin/bash
#
# Clients & Test Utilities - Intent-Based Verification
# Tests CLIENT applications and integration tests (NOT servers)
#
# Each binary is built with `-p` into its lane's target dir (see
# scripts/lib/examples.sh): the 2025-11-25-pinned examples
# (client-initialise-server, logging-test-server/-client) cannot share a build
# with the 2026-07-28 default, so a bare `cargo build --workspace` cannot make
# them. The raw-wire / client utilities build in the default lane.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

source "$SCRIPT_DIR/lib/examples.sh"

echo "======================================================================"
echo "Clients & Test Utilities - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify CLIENT applications and test utilities"
echo "                   work correctly with MCP servers"
echo ""

# lane per binary
declare -A LANE=(
    [client-initialise-server]=2025
    [logging-test-server]=2025
    [logging-test-client]=2025
    [client-initialise-report]=2026
    [streamable-http-client]=2026
    [streamable-http-client-2025-11-25]=2026
    [session-management-compliance-test]=2026
    [minimal-server]=2026
)
BUILD_FAILED=0
for b in "${!LANE[@]}"; do
    if ! example_build "$b" "${LANE[$b]}"; then BUILD_FAILED=1; fi
done
[ "$BUILD_FAILED" -eq 0 ] || { echo "FAILED: build error"; exit 1; }
bin() { example_bin "$1" "${LANE[$1]}"; }

PASSED=0
FAILED=0
TOTAL=5

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track PIDs for cleanup
PIDS=()

# Run a command, recording its status in CAPTURED_EXIT instead of ending the run.
#
# `set -e` aborts on a failing command in plain position, so the `cmd` /
# `STATUS=$?` shape reads naturally but exits before the assignment — making
# every FAILED branch below unreachable and discarding the reason along with the
# log tail it was about to print. The status is only observable if the command
# runs somewhere the shell already tolerates failure, such as an `if`.
#
# Redirections belong on the `capture` call; they apply to the command through it.
capture() {
    if "$@"; then CAPTURED_EXIT=0; else CAPTURED_EXIT=$?; fi
}

# Cleanup function (PID-based, not pkill)
cleanup() {
    echo ""
    echo "Cleaning up background processes..."
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    sleep 1
}

trap cleanup EXIT

# Test 1: client-initialise-server and client-initialise-report
test_client_initialization() {
    echo "----------------------------------------"
    echo "Testing: client-initialise-server + client-initialise-report"
    echo "Description: MCP client session initialization testing"
    echo "----------------------------------------"

    # Start the test server
    echo "Starting client-initialise-server..."
    RUST_LOG=error timeout 30s "$(bin client-initialise-server)" --port 52935 &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    # Check if server is running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: client-initialise-server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Run the client report
    echo "Running client-initialise-report..."
    RUST_LOG=error timeout 10s "$(bin client-initialise-report)" --url http://127.0.0.1:52935/mcp > /tmp/client_report.log 2>&1 &
    CLIENT_PID=$!

    # Wait for client to complete
    if wait $CLIENT_PID 2>/dev/null; then CLIENT_EXIT=0; else CLIENT_EXIT=$?; fi

    # Cleanup server
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    if [ $CLIENT_EXIT -eq 0 ]; then
        echo -e "${GREEN}PASSED${NC}: Client initialization test successful"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}: Client initialization test failed (exit code: $CLIENT_EXIT)"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test 2: streamable-http-client (2026-07-28 default lane, pairs with minimal-server)
test_streamable_client() {
    echo "----------------------------------------"
    echo "Testing: streamable-http-client"
    echo "Description: Streamable HTTP client library testing"
    echo "----------------------------------------"

    # Start minimal-server as test target
    echo "Starting minimal-server as test target..."
    RUST_LOG=error timeout 30s "$(bin minimal-server)" --port 8641 &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    # Check if server is running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: minimal-server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Run streamable client. It takes the server URL as a bare positional
    # argument, not a --url flag.
    echo "Running streamable-http-client..."
    capture env RUST_LOG=error timeout 10s "$(bin streamable-http-client)" \
        http://127.0.0.1:8641/mcp > /tmp/streamable_client.log 2>&1
    CLIENT_EXIT=$CAPTURED_EXIT

    # Cleanup server
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    if [ $CLIENT_EXIT -eq 0 ] && grep -q "Negotiated protocol: Some(V2026_07_28)" /tmp/streamable_client.log; then
        echo -e "${GREEN}PASSED${NC}: Streamable HTTP client test successful"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}: streamable-http-client did not complete a 2026-07-28 round trip"
        echo "Output: $(cat /tmp/streamable_client.log)"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test 3: logging-test-client + logging-test-server (both protocol-2025-11-25)
test_logging_client() {
    echo "----------------------------------------"
    echo "Testing: logging-test-client + logging-test-server"
    echo "Description: Client-server logging integration"
    echo "----------------------------------------"

    # Start logging test server
    echo "Starting logging-test-server..."
    RUST_LOG=error timeout 30s "$(bin logging-test-server)" --port 8052 &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    # Check if server is running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: logging-test-server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Run logging test client. It takes --port (matching the server),
    # --quick-test keeps this bounded for a CI gate.
    echo "Running logging-test-client..."
    capture env RUST_LOG=error timeout 20s "$(bin logging-test-client)" \
        --port 8052 --quick-test > /tmp/logging_client.log 2>&1
    CLIENT_EXIT=$CAPTURED_EXIT

    # Cleanup server
    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    if [ $CLIENT_EXIT -eq 0 ]; then
        echo -e "${GREEN}PASSED${NC}: Logging client test successful"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}: logging-test-client exited $CLIENT_EXIT"
        echo "Output: $(tail -20 /tmp/logging_client.log)"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test 4: session-management-compliance-test (2025-11-25 ONLY by design; needs
# a protocol-2025-11-25-pinned target — client-initialise-server, per its own
# doc comment).
test_session_compliance() {
    echo "----------------------------------------"
    echo "Testing: session-management-compliance-test"
    echo "Description: Session management compliance verification"
    echo "----------------------------------------"

    echo "Starting client-initialise-server as test target..."
    RUST_LOG=error timeout 30s "$(bin client-initialise-server)" --port 52951 &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: client-initialise-server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Running session management compliance test..."
    capture env RUST_LOG=error timeout 20s "$(bin session-management-compliance-test)" \
        http://127.0.0.1:52951/mcp > /tmp/session_compliance.log 2>&1
    TEST_EXIT=$CAPTURED_EXIT

    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    if [ $TEST_EXIT -eq 0 ]; then
        echo -e "${GREEN}PASSED${NC}: Session compliance test successful"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}: session-management-compliance-test exited $TEST_EXIT"
        echo "Output: $(tail -30 /tmp/session_compliance.log)"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test 4b: the 2025-11-25 raw-wire client against client-initialise-server.
# This is what covers the EXAMPLE's progress behaviour. The framework contract
# is pinned by crates/turul-mcp-server/tests/progress_token_match_2025_11_25.rs,
# but that drives a purpose-built tool — nothing else exercises echo_sse, which
# previously answered with a token of its own choosing. The client exits
# non-zero when the token it sent is not the token that comes back.
test_progress_token_echo() {
    echo "----------------------------------------"
    echo "Testing: streamable-http-client-2025-11-25 → client-initialise-server"
    echo "Description: progress notifications echo the request's progressToken"
    echo "----------------------------------------"

    RUST_LOG=error timeout 30s "$(bin client-initialise-server)" --port 52952 &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: client-initialise-server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    capture env RUST_LOG=error timeout 20s "$(bin streamable-http-client-2025-11-25)" \
        --url http://127.0.0.1:52952/mcp > /tmp/progress_token_echo.log 2>&1
    TEST_EXIT=$CAPTURED_EXIT

    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    if [ $TEST_EXIT -eq 0 ]; then
        echo -e "${GREEN}PASSED${NC}: progress notifications carried the request's token"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}: progress-token echo check exited $TEST_EXIT"
        echo "Output: $(tail -20 /tmp/progress_token_echo.log)"
        FAILED=$((FAILED + 1))
        return 1
    fi
}


# Run all client tests
test_client_initialization
test_streamable_client
test_logging_client
test_session_compliance
test_progress_token_echo

# Final summary
echo "======================================================================"
echo "Clients & Test Utilities Summary"
echo "======================================================================"
echo "Total: $TOTAL client/test utilities"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""
echo "Note: Some clients may be skipped if they require specific"
echo "      implementations or external dependencies."
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ CLIENT EXAMPLES COMPLETE${NC}: All client/test utilities verified"
    exit 0
else
    echo -e "${RED}❌ CLIENT EXAMPLES FAILED${NC}: $FAILED client(s) failed verification"
    exit 1
fi
