#!/bin/bash
RUST_LOG=info cargo run --package middleware-rate-limit-server -- --port 8675 &
PID=$!
sleep 5

INIT_RESP=$(curl -si -X POST http://localhost:8675/mcp -H "Content-Type: application/json" -H "Accept: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')
SID=$(echo "$INIT_RESP" | grep -i "mcp-session-id:" | awk '{print $2}' | tr -d '\r')
echo "Session: $SID"

curl -s -X POST http://localhost:8675/mcp -H "Content-Type: application/json" -H "Mcp-Session-Id: $SID" -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null

for i in {1..6}; do
  echo "Request $i:"
  curl -s -X POST http://localhost:8675/mcp -H "Content-Type: application/json" -H "Mcp-Session-Id: $SID" -d "{\"jsonrpc\":\"2.0\",\"id\":$i,\"method\":\"tools/list\",\"params\":{}}" | jq -c '.error // "OK"'
done

kill $PID 2>/dev/null
