#!/usr/bin/env bash
# End-to-end acceptance for the Lambda transport on the MCP 2026-07-28 lane.
#
# The in-process Lambda tests (crates/turul-mcp-aws-lambda/tests/) construct a
# handler and call it directly. That skips the layer this script exercises: the
# AWS Lambda Runtime API. `cargo lambda watch` boots the real control-plane
# emulator, so every assertion below is on bytes that crossed a Function URL
# request/response cycle — the same encode/decode path a deployed function uses.
#
#   scripts/e2e-lambda-local.sh [INVOKE_PORT]
#
# Requires cargo-lambda (https://cargo-lambda.info). The first invocation
# compiles the function, so the readiness wait is deliberately generous.
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-9000}"
FUNCTION="lambda-echo-server"
BASE="http://127.0.0.1:$PORT/lambda-url/$FUNCTION"
URL="$BASE/mcp"
LOG="${TMPDIR:-/tmp}/turul-lambda-watch-$PORT.log"

command -v cargo-lambda >/dev/null 2>&1 || {
  echo "SKIP: cargo-lambda not installed — https://cargo-lambda.info" >&2
  exit 0
}

pass=0
fail=0
ok()   { echo "  PASS  $1"; pass=$((pass + 1)); }
bad()  { echo "  FAIL  $1"; echo "        $2"; fail=$((fail + 1)); }

# Every 2026-07-28 request carries the negotiated version in `_meta`; the
# transport also requires it as a header, and `Mcp-Method` must agree with the
# body's method.
meta='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"lambda-e2e","version":"1.0.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# post <case> <rpc-method> <mcp-name|-> <body> [extra curl args...]
# Writes the response body to $BODY and the status to $STATUS.
post() {
  local rpc="$2" name="$3" body="$4"
  shift 4
  local args=(-sS -o "$TMPBODY" -w '%{http_code}' -D "$TMPHEAD"
              -H 'Accept: application/json'
              -H 'Content-Type: application/json')
  [ "$rpc" != "-" ] && args+=(-H "Mcp-Method: $rpc")
  [ "$name" != "-" ] && args+=(-H "Mcp-Name: $name")
  args+=("$@" --data "$body" "$URL")
  STATUS=$(curl "${args[@]}" 2>/dev/null)
  BODY=$(cat "$TMPBODY")
}

TMPBODY=$(mktemp)
TMPHEAD=$(mktemp)
trap 'rm -f "$TMPBODY" "$TMPHEAD"; [ -n "${WATCH_PID:-}" ] && kill "$WATCH_PID" 2>/dev/null; exit' EXIT INT TERM

echo "=== booting cargo lambda watch on :$PORT (2026-07-28 default features) ==="
cargo lambda watch -p turul-mcp-aws-lambda --bin "$FUNCTION" \
  --invoke-port "$PORT" --ignore-changes >"$LOG" 2>&1 &
WATCH_PID=$!

# The emulator answers before the function is built; the first real invoke is
# what triggers compilation, so poll the endpoint itself rather than the port.
echo "  waiting for the function to build and answer (up to 10 min)..."
ready=0
for _ in $(seq 1 200); do
  kill -0 "$WATCH_PID" 2>/dev/null || { echo "FAIL: watch exited early"; tail -30 "$LOG"; exit 1; }
  # `server/discover` is the readiness probe because 2026-07-28 has no `ping`:
  # it would answer 404 forever and the wait would never resolve.
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
echo

echo "=== J1: stateless core over the Lambda Runtime API ==="

post discover server/discover - \
  "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{$meta}}" \
  -H 'MCP-Protocol-Version: 2026-07-28'
if [ "$STATUS" = "200" ] && echo "$BODY" | jq -e '.result.capabilities' >/dev/null 2>&1; then
  ok "server/discover returns capabilities (HTTP 200)"
else
  bad "server/discover" "status=$STATUS body=$BODY"
fi

# The stateless core removed the session header. A Lambda response that minted
# one would break every client that treats its presence as "resume this session".
if grep -qi '^mcp-session-id:' "$TMPHEAD"; then
  bad "no session header" "response carried Mcp-Session-Id: $(grep -i '^mcp-session-id:' "$TMPHEAD")"
else
  ok "response mints no Mcp-Session-Id"
fi

post list tools/list - \
  "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{$meta}}" \
  -H 'MCP-Protocol-Version: 2026-07-28'
if [ "$STATUS" = "200" ] && echo "$BODY" | jq -e '.result.tools[] | select(.name == "echo")' >/dev/null 2>&1; then
  ok "tools/list advertises echo"
else
  bad "tools/list" "status=$STATUS body=$BODY"
fi

post call tools/call echo \
  "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"echo\",\"arguments\":{\"message\":\"lambda\"},$meta}}" \
  -H 'MCP-Protocol-Version: 2026-07-28'
if [ "$STATUS" = "200" ] && echo "$BODY" | grep -q 'Echo: lambda'; then
  ok "tools/call echo round-trips through the runtime API"
else
  bad "tools/call" "status=$STATUS body=$BODY"
fi

echo
echo "=== J5: negative paths ==="

# Header validation runs before dispatch; a missing version header is a -32020
# HeaderMismatch, not a transport-level failure.
post noversion tools/call echo \
  "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"echo\",\"arguments\":{\"message\":\"x\"},$meta}}"
if [ "$STATUS" = "400" ] && [ "$(echo "$BODY" | jq -r '.error.code' 2>/dev/null)" = "-32020" ]; then
  ok "missing MCP-Protocol-Version -> 400 + -32020"
else
  bad "missing MCP-Protocol-Version" "status=$STATUS body=$BODY"
fi

post mismatch tools/list echo \
  "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"echo\",\"arguments\":{\"message\":\"x\"},$meta}}" \
  -H 'MCP-Protocol-Version: 2026-07-28'
if [ "$STATUS" = "400" ] && [ "$(echo "$BODY" | jq -r '.error.code' 2>/dev/null)" = "-32020" ]; then
  ok "Mcp-Method disagreeing with the body -> 400 + -32020"
else
  bad "Mcp-Method mismatch" "status=$STATUS body=$BODY"
fi

post unknown does/not/exist - \
  "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"does/not/exist\",\"params\":{$meta}}" \
  -H 'MCP-Protocol-Version: 2026-07-28'
if [ "$STATUS" = "404" ] && [ "$(echo "$BODY" | jq -r '.error.code' 2>/dev/null)" = "-32601" ]; then
  ok "unknown method -> 404 + -32601"
else
  bad "unknown method" "status=$STATUS body=$BODY"
fi

# `initialize` was removed in 2026-07-28. Serving it would silently readmit the
# stateful handshake through the Lambda path only.
post init initialize - \
  "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"initialize\",\"params\":{$meta}}" \
  -H 'MCP-Protocol-Version: 2026-07-28'
if [ "$STATUS" = "404" ] || [ "$(echo "$BODY" | jq -r '.error.code' 2>/dev/null)" = "-32601" ]; then
  ok "removed initialize method is not served"
else
  bad "initialize must not be served" "status=$STATUS body=$BODY"
fi

echo
echo "=== HTTP surface ==="
for verb in GET DELETE; do
  code=$(curl -sS -o /dev/null -w '%{http_code}' -X "$verb" \
    -H 'Accept: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
    "$URL" 2>/dev/null)
  if [ "$code" = "405" ]; then
    ok "$verb /mcp -> 405"
  else
    bad "$verb /mcp" "expected 405, got $code"
  fi
done

echo
echo "=== $pass passed, $fail failed ==="
[ "$fail" = "0" ] || { echo "watch log tail:"; tail -30 "$LOG"; }
exit $((fail > 0))
