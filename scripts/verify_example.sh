#!/bin/bash
# Verify a single MCP example by starting it and testing it

set -e

EXAMPLE_NAME=$1
PORT=$2
TEST_TYPE=${3:-"basic"}  # basic, calculator, resource, etc.

if [ -z "$EXAMPLE_NAME" ] || [ -z "$PORT" ]; then
    echo "Usage: $0 <example-name> <port> [test-type]"
    exit 1
fi

echo "=== Testing: $EXAMPLE_NAME on port $PORT ==="

# Start the example in background
cargo run -p "$EXAMPLE_NAME" > "/tmp/${EXAMPLE_NAME}.log" 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Check if server is running
if ! ps -p $SERVER_PID > /dev/null; then
    echo "❌ FAILED: Server did not start"
    cat "/tmp/${EXAMPLE_NAME}.log"
    exit 1
fi

# Test based on type
case "$TEST_TYPE" in
    basic)
        # Just check if server responds
        if curl -s -f "http://127.0.0.1:${PORT}/mcp" > /dev/null; then
            echo "✅ PASSED: Server responding on port $PORT"
            RESULT=0
        else
            echo "❌ FAILED: Server not responding"
            RESULT=1
        fi
        ;;

    calculator)
        # Test calculator tool
        SESSION_ID=$(curl -s -X POST "http://127.0.0.1:${PORT}/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
            | grep -o '"Mcp-Session-Id":"[^"]*"' | cut -d'"' -f4)

        if [ -n "$SESSION_ID" ]; then
            # Send initialized notification
            curl -s -X POST "http://127.0.0.1:${PORT}/mcp" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                -H "Mcp-Session-Id: $SESSION_ID" \
                -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

            sleep 0.1

            # Test calculator
            RESULT_JSON=$(curl -s -X POST "http://127.0.0.1:${PORT}/mcp" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                -H "Mcp-Session-Id: $SESSION_ID" \
                -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add","arguments":{"a":5,"b":3}}}')

            if echo "$RESULT_JSON" | grep -q '"result":8\|"content":\[{"type":"text","text":"8"}'; then
                echo "✅ PASSED: Calculator works (5+3=8)"
                RESULT=0
            else
                echo "❌ FAILED: Calculator returned unexpected result: $RESULT_JSON"
                RESULT=1
            fi
        else
            echo "❌ FAILED: Could not get session ID"
            RESULT=1
        fi
        ;;

    resource)
        # Test resource listing
        SESSION_ID=$(curl -s -X POST "http://127.0.0.1:${PORT}/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
            | grep -o '"Mcp-Session-Id":"[^"]*"' | cut -d'"' -f4)

        if [ -n "$SESSION_ID" ]; then
            RESULT_JSON=$(curl -s -X POST "http://127.0.0.1:${PORT}/mcp" \
                -H "Content-Type: application/json" \
                -H "Accept: application/json" \
                -H "Mcp-Session-Id: $SESSION_ID" \
                -d '{"jsonrpc":"2.0","id":2,"method":"resources/list"}')

            if echo "$RESULT_JSON" | grep -q '"resources":\['; then
                echo "✅ PASSED: Resources list works"
                RESULT=0
            else
                echo "❌ FAILED: Resources list failed: $RESULT_JSON"
                RESULT=1
            fi
        else
            echo "❌ FAILED: Could not get session ID"
            RESULT=1
        fi
        ;;
esac

# Cleanup
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

exit $RESULT
