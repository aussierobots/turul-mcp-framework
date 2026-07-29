#!/bin/bash
#
# Clients & Test Utilities - Intent-Based Verification
# Tests CLIENT applications and integration tests (NOT servers)
#
# Uses pre-built binaries from target/debug/ for fast execution.
# Run `cargo build --workspace` first, or this script will build for you.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

BIN_DIR="$PROJECT_ROOT/target/debug"

echo "======================================================================"
echo "Clients & Test Utilities - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify CLIENT applications and test utilities"
echo "                   work correctly with MCP servers"
echo ""

# Build all binaries if not already built
if [ ! -f "$BIN_DIR/minimal-server" ]; then
    echo "Pre-built binaries not found. Building workspace..."
    cargo build --workspace
    echo ""
fi

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
    RUST_LOG=error timeout 30s "$BIN_DIR/client-initialise-server" --port 52935 &
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
    RUST_LOG=error timeout 10s "$BIN_DIR/client-initialise-report" --url http://127.0.0.1:52935/mcp > /tmp/client_report.log 2>&1 &
    CLIENT_PID=$!

    # Wait for client to complete
    wait $CLIENT_PID 2>/dev/null
    CLIENT_EXIT=$?

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
    RUST_LOG=error timeout 30s "$BIN_DIR/minimal-server" --port 8641 &
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
    RUST_LOG=error timeout 10s "$BIN_DIR/streamable-http-client" http://127.0.0.1:8641/mcp > /tmp/streamable_client.log 2>&1
    CLIENT_EXIT=$?

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
    RUST_LOG=error timeout 30s "$BIN_DIR/logging-test-server" --port 8052 &
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
    RUST_LOG=error timeout 20s "$BIN_DIR/logging-test-client" --port 8052 --quick-test > /tmp/logging_client.log 2>&1
    CLIENT_EXIT=$?

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
    RUST_LOG=error timeout 30s "$BIN_DIR/client-initialise-server" --port 52951 &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: client-initialise-server failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo "Running session management compliance test..."
    RUST_LOG=error timeout 20s "$BIN_DIR/session-management-compliance-test" \
        http://127.0.0.1:52951/mcp > /tmp/session_compliance.log 2>&1
    TEST_EXIT=$?

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

# Test 5: session-logging-proof-test (protocol-2025-11-25; hardcodes port 8001
# in main.rs — it has no clap/CLI arg parsing at all, so a --port flag would
# be silently ignored).
test_session_logging() {
    echo "----------------------------------------"
    echo "Testing: session-logging-proof-test"
    echo "Description: Session-aware logging verification"
    echo "----------------------------------------"

    echo "Running session logging proof test..."
    RUST_LOG=error timeout 10s "$BIN_DIR/session-logging-proof-test" &
    SERVER_PID=$!
    PIDS+=($SERVER_PID)
    sleep 3

    # Check if server started
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}FAILED${NC}: session-logging-proof-test failed to start"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Test basic initialization
    SESSION_ID=$(curl -i -s -X POST "http://127.0.0.1:8001/mcp" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
        | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

    kill $SERVER_PID 2>/dev/null || true
    sleep 1

    if [ -n "$SESSION_ID" ]; then
        echo -e "${GREEN}PASSED${NC}: Session logging test successful"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}: Could not get session ID from header"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Run all client tests
test_client_initialization
test_streamable_client
test_logging_client
test_session_compliance
test_session_logging

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
