#!/bin/bash
# Shared MCP server health check utilities
# Mirrors TestServerManager::start() polling logic from mcp_e2e_shared

wait_for_server() {
    local port=$1
    local max_attempts=50  # 15 seconds total (50 * 300ms)
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        # Try initialize request (same as TestServerManager health check)
        if curl -s -X POST "http://127.0.0.1:$port/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"health-check","version":"1.0.0"}}}' \
            > /dev/null 2>&1; then
            return 0
        fi
        sleep 0.3
        attempt=$((attempt + 1))
    done
    return 1
}

ensure_binary_built() {
    local server_name=$1

    if [ ! -f "./target/debug/$server_name" ]; then
        echo "Building $server_name..."
        local build_output
        build_output=$(cargo build --bin "$server_name" 2>&1)
        local build_status=$?

        if [ $build_status -ne 0 ]; then
            echo "Build failed - check for missing dependencies (SQLite, PostgreSQL, etc.)"
            echo "Build output (last 10 lines):"
            echo "$build_output" | tail -10
            return 1
        fi
        echo "$build_output" | grep -E "(Finished|Compiling)" | tail -1
    fi
    return 0
}

cleanup_old_logs() {
    local server_name=$1
    local port=$2
    rm -f "/tmp/${server_name}_${port}.log" 2>/dev/null || true
}
