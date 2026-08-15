#!/usr/bin/env bash
# End-to-end acceptance for the Lambda transport on the MCP 2025-11-25 lane.
#
# `cargo test -p turul-mcp-aws-lambda --no-default-features --features
# cors,sse,protocol-2025-11-25` only drives the handler in-process
# (crates/turul-mcp-aws-lambda/tests/middleware_parity.rs calls
# LambdaMcpHandler::handle() directly), so it never exercises the AWS Lambda
# Runtime API. This script does: `cargo lambda watch` boots the real
# control-plane emulator against a build of the same binary compiled for the
# 2025-11-25 feature set, and every assertion below is on bytes/headers that
# crossed a Function URL request/response cycle.
#
# The 2025-11-25 contract is different from 2026-07-28's stateless core: it
# still has the `initialize` -> `notifications/initialized` handshake and the
# `Mcp-Session-Id` header, so the assertions here are about that session
# state surviving the Runtime API round trip — exactly what the in-process
# test cannot see, because it never serializes a session id through a wire
# boundary at all.
#
#   scripts/e2e-lambda-local-2025-11-25.sh [INVOKE_PORT]
#
# Requires cargo-lambda (https://cargo-lambda.info). The first invocation
# compiles the function, so the readiness wait is deliberately generous.
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-9001}"
FUNCTION="lambda-echo-server"
BASE="http://127.0.0.1:$PORT/lambda-url/$FUNCTION"
URL="$BASE/mcp"
LOG="${TMPDIR:-/tmp}/turul-lambda-watch-2025-11-25-$PORT.log"

command -v cargo-lambda >/dev/null 2>&1 || {
  echo "SKIP: cargo-lambda not installed — https://cargo-lambda.info" >&2
  exit 0
}

pass=0
fail=0
ok()   { echo "  PASS  $1"; pass=$((pass + 1)); }
bad()  { echo "  FAIL  $1"; echo "        $2"; fail=$((fail + 1)); }

# post <case> <session-id|-> <body>
# Writes the response body to $BODY, the status to $STATUS, and leaves
# response headers in $TMPHEAD. `-` skips the Mcp-Session-Id request header.
post() {
  local sid="$2" body="$3"
  local args=(-sS -o "$TMPBODY" -w '%{http_code}' -D "$TMPHEAD"
              -H 'Accept: application/json'
              -H 'Content-Type: application/json'
              -H 'MCP-Protocol-Version: 2025-11-25')
  [ "$sid" != "-" ] && args+=(-H "Mcp-Session-Id: $sid")
  args+=(--data "$body" "$URL")
  STATUS=$(curl "${args[@]}" 2>/dev/null)
  BODY=$(cat "$TMPBODY")
}

TMPBODY=$(mktemp)
TMPHEAD=$(mktemp)
trap 'rm -f "$TMPBODY" "$TMPHEAD"; [ -n "${WATCH_PID:-}" ] && kill "$WATCH_PID" 2>/dev/null; exit' EXIT INT TERM

echo "=== booting cargo lambda watch on :$PORT (2025-11-25 feature set) ==="
cargo lambda watch -p turul-mcp-aws-lambda --bin "$FUNCTION" \
  --no-default-features -F cors,sse,protocol-2025-11-25 \
  --invoke-port "$PORT" --ignore-changes >"$LOG" 2>&1 &
WATCH_PID=$!

init_body='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"lambda-e2e-2025","version":"1.0.0"}}}'

# The emulator answers before the function is built; the first real invoke is
# what triggers compilation, so poll the endpoint itself rather than the port.
# `initialize` doubles as the readiness probe since 2025-11-25 has no
# unauthenticated method simpler than it; each retry mints a throwaway session.
echo "  waiting for the function to build and answer (up to 10 min)..."
ready=0
for _ in $(seq 1 200); do
  kill -0 "$WATCH_PID" 2>/dev/null || { echo "FAIL: watch exited early"; tail -30 "$LOG"; exit 1; }
  code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 60 \
    -H 'Accept: application/json' -H 'Content-Type: application/json' \
    -H 'MCP-Protocol-Version: 2025-11-25' \
    --data "$init_body" "$URL" 2>/dev/null)
  if [ "$code" = "200" ]; then ready=1; break; fi
  sleep 3
done
[ "$ready" = "1" ] || { echo "FAIL: function never answered on $URL"; tail -40 "$LOG"; exit 1; }
echo "  function is live"
echo

echo "=== session lifecycle over the Lambda Runtime API ==="

post init - "$init_body"
SESSION_ID=$(grep -i '^mcp-session-id:' "$TMPHEAD" | tr -d '\r' | cut -d' ' -f2-)
if [ "$STATUS" = "200" ] && [ -n "$SESSION_ID" ] && echo "$BODY" | jq -e '.result.capabilities' >/dev/null 2>&1; then
  ok "initialize returns 200 with a Mcp-Session-Id response header ($SESSION_ID)"
else
  bad "initialize" "status=$STATUS session_header='$SESSION_ID' body=$BODY"
fi

if [ -n "$SESSION_ID" ]; then
  post initialized "$SESSION_ID" \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  if [ "$STATUS" = "202" ]; then
    ok "notifications/initialized with the minted session id -> 202"
  else
    bad "notifications/initialized" "status=$STATUS body=$BODY"
  fi
else
  bad "notifications/initialized" "skipped — no session id captured from initialize"
fi

if [ -n "$SESSION_ID" ]; then
  post list_with_session "$SESSION_ID" \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
  if [ "$STATUS" = "200" ] && echo "$BODY" | jq -e '.result.tools[] | select(.name == "echo")' >/dev/null 2>&1; then
    ok "tools/list with the session id succeeds and advertises echo"
  else
    bad "tools/list with session id" "status=$STATUS body=$BODY"
  fi
else
  bad "tools/list with session id" "skipped — no session id captured from initialize"
fi

post list_without_session - \
  '{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}'
if [ "$STATUS" = "400" ]; then
  ok "tools/list without Mcp-Session-Id -> 400"
else
  bad "tools/list without session id" "status=$STATUS body=$BODY"
fi

post list_nonexistent_session "00000000000000000000000000nonexistent" \
  '{"jsonrpc":"2.0","id":4,"method":"tools/list","params":{}}'
if [ "$STATUS" = "404" ]; then
  ok "tools/list with a nonexistent session id -> 404 (distinct from missing -> 400)"
else
  bad "tools/list with nonexistent session id" "status=$STATUS body=$BODY"
fi

echo
echo "=== $pass passed, $fail failed ==="
[ "$fail" = "0" ] || { echo "watch log tail:"; tail -30 "$LOG"; }
exit $((fail > 0))
