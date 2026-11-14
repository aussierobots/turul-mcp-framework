#!/usr/bin/env bash
# Skills over MCP on Lambda, end to end (ADR-035 §Tests 6): boots
# `lambda-skills-server` under `cargo lambda watch` (the real Lambda Runtime
# API) and runs the same live gate as the HTTP example against it — every
# served file re-hashed against its manifest and the bytes on disk, then the
# upstream sep-2640 server and host scenarios.
#
#   scripts/e2e-lambda-skills.sh [INVOKE_PORT]
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-9002}"
FUNCTION="lambda-skills-server"
URL="http://127.0.0.1:$PORT/lambda-url/$FUNCTION/mcp"
LOG="${TMPDIR:-/tmp}/turul-lambda-skills-watch-$PORT.log"
meta='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"lambda-skills-e2e","version":"1.0.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

command -v cargo-lambda >/dev/null 2>&1 || {
  echo "SKIP: cargo-lambda not installed — https://cargo-lambda.info" >&2
  exit 0
}

echo "=== booting cargo lambda watch: $FUNCTION on :$PORT ==="
cargo lambda watch -p turul-mcp-aws-lambda --bin "$FUNCTION" --features ext-skills \
  --invoke-port "$PORT" --ignore-changes >"$LOG" 2>&1 &
WATCH_PID=$!
trap '[ -n "${WATCH_PID:-}" ] && kill "$WATCH_PID" 2>/dev/null; pkill -P "$WATCH_PID" 2>/dev/null' EXIT

# The first invoke triggers the build, so poll the endpoint, not the port.
ready=0
for _ in $(seq 1 200); do
  kill -0 "$WATCH_PID" 2>/dev/null || { echo "FAIL: watch exited early"; tail -30 "$LOG"; exit 1; }
  code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 60 \
    -H 'Accept: application/json' -H 'Content-Type: application/json' \
    -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"server/discover\",\"params\":{$meta}}" \
    "$URL" 2>/dev/null)
  if [ "$code" = "200" ]; then ready=1; break; fi
  sleep 3
done
[ "$ready" = "1" ] || { echo "FAIL: function never answered on $URL"; tail -40 "$LOG"; exit 1; }
echo "  function is live"

SKILLS_URL="$URL" ./scripts/ext-skills-conformance.sh
rc=$?
[ "$rc" = "0" ] || { echo "watch log tail:"; tail -30 "$LOG"; }
exit "$rc"
