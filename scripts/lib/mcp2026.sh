#!/bin/bash
# Shared helpers for probing MCP 2026-07-28 stateless servers from shell scripts.
#
# 2026-07-28 removed `initialize` / `notifications/initialized` / `Mcp-Session-Id`.
# Every POST must carry `MCP-Protocol-Version`, `Mcp-Method` (and `Mcp-Name` for
# tools/call), plus a spec-complete `params._meta` object. There is no session
# to establish first — readiness is checked with `server/discover`.

mcp2026_meta() {
    echo '{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}'
}

# mcp2026_request URL METHOD MCP_NAME PARAMS_JSON
# PARAMS_JSON is a JSON object (e.g. '{}' or '{"name":"echo","arguments":{}}');
# `_meta` is merged in automatically. Prints the raw response body.
# MCP_NAME is REQUIRED (mirroring the body) for tools/call (params.name),
# resources/read (params.uri) and prompts/get (params.name); pass "" for
# methods that carry no such header (e.g. tools/list, resources/list).
mcp2026_request() {
    local url="$1" method="$2" name="$3" params="${4:-{\}}"
    local body
    body=$(jq -n --arg method "$method" --argjson params "$params" --argjson meta "$(mcp2026_meta)" \
        '{jsonrpc:"2.0", id:1, method:$method, params: ($params + {_meta:$meta})}')

    local args=(-s -X POST "$url"
        -H "Content-Type: application/json" -H "Accept: application/json"
        -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: $method")
    [ -n "$name" ] && args+=(-H "Mcp-Name: $name")
    curl "${args[@]}" -d "$body"
}

# mcp2026_wait_for_server PORT — polls server/discover until it answers 200.
mcp2026_wait_for_server() {
    local port=$1
    local max_attempts=50
    local attempt=0
    local url="http://127.0.0.1:${port}/mcp"

    while [ $attempt -lt $max_attempts ]; do
        local code
        code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$url" \
            -H "Content-Type: application/json" -H "Accept: application/json" \
            -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: server/discover" \
            -d "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"server/discover\",\"params\":{\"_meta\":$(mcp2026_meta)}}" \
            2>/dev/null)
        [ "$code" = "200" ] && return 0
        sleep 0.3
        attempt=$((attempt + 1))
    done
    return 1
}
