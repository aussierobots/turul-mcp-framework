#!/usr/bin/env bash
# End-to-end acceptance for turul-mcp-client driven against a Lambda-hosted
# server, on both spec lanes.
#
# `examples/lambda-mcp-client` and `examples/interop-client-probe` both use
# the real `turul-mcp-client` crate, but neither is wired into any gate
# against a Lambda target — the client has never been driven over the real
# AWS Lambda Runtime API, only against `turul-http-mcp-server` directly. The
# Lambda transport reassembles the JSON-RPC response differently from the
# plain HTTP path (it goes through the Lambda Runtime API's request/response
# envelope, not a raw hyper connection), which is exactly where a client-side
# parser assumption calibrated against the plain-HTTP path would surface.
#
# `cargo lambda watch` boots the real control-plane emulator (same pattern as
# scripts/e2e-lambda-local.sh and scripts/e2e-lambda-local-2025-11-25.sh), so
# every assertion below is on what the client actually observed after a
# round trip through that emulator — not on server-side logs.
#
# Two lanes, because the two client-side entry points already split along
# spec lines:
#   - 2026-07-28: `interop-client-probe` (unmodified) — it hardcodes the
#     stateless `server/discover` handshake and asserts on
#     `McpVersion::V2026_07_28`, so it is only meaningful against a
#     2026-07-28 build of the target.
#   - 2025-11-25: `examples/lambda-mcp-client`'s `connect` subcommand, which
#     drives the real `turul_mcp_client::McpClient` through the
#     initialize -> notifications/initialized handshake this lane still has.
#     Its `test` subcommand is not used here: most of its TestCase
#     descriptors (test_suite.rs) don't correspond to any arm in
#     test_runner.rs's match and silently fall through to a no-op default
#     that always "passes" — asserting against that would be asserting on
#     log output, not on bytes the client observed.
#
# Both lanes target `lambda-echo-server` (the same minimal binary the other
# two Lambda E2E scripts use), built once per lane with the matching feature
# set — not `lambda-turul-mcp-server`, which requires a live DynamoDB table
# and would make this script's readiness depend on AWS credentials rather
# than only on cargo-lambda.
#
#   scripts/e2e-lambda-client-local.sh [PORT_2026] [PORT_2025]
#
# Requires cargo-lambda (https://cargo-lambda.info). The first invocation on
# each lane compiles the function, so the readiness wait is deliberately
# generous.
set -uo pipefail
cd "$(dirname "$0")/.."

PORT_2026="${1:-9100}"
PORT_2025="${2:-9101}"
FUNCTION="lambda-echo-server"
URL_2026="http://127.0.0.1:$PORT_2026/lambda-url/$FUNCTION/mcp"
URL_2025="http://127.0.0.1:$PORT_2025/lambda-url/$FUNCTION"
LOG_2026="${TMPDIR:-/tmp}/turul-lambda-client-watch-2026-$PORT_2026.log"
LOG_2025="${TMPDIR:-/tmp}/turul-lambda-client-watch-2025-11-25-$PORT_2025.log"

command -v cargo-lambda >/dev/null 2>&1 || {
  echo "SKIP: cargo-lambda not installed — https://cargo-lambda.info" >&2
  exit 0
}

pass=0
fail=0
ok()  { echo "  PASS  $1"; pass=$((pass + 1)); }
bad() { echo "  FAIL  $1"; echo "        $2"; fail=$((fail + 1)); }

WATCH_PID_2026=""
WATCH_PID_2025=""
cleanup() {
  [ -n "$WATCH_PID_2026" ] && kill "$WATCH_PID_2026" 2>/dev/null
  [ -n "$WATCH_PID_2025" ] && kill "$WATCH_PID_2025" 2>/dev/null
}
trap cleanup EXIT INT TERM

wait_ready() {
  # wait_ready <watch-pid-var> <probe-cmd...>
  local pid_var="$1"; shift
  local streak=0
  for _ in $(seq 1 200); do
    kill -0 "${!pid_var}" 2>/dev/null || return 1
    if "$@" >/dev/null 2>&1; then
      streak=$((streak + 1))
      # Three consecutive successes: one probe can land in the window between a
      # lazy build finishing and the watcher swapping the function process, and
      # the connection then dies mid-request as hyper IncompleteMessage.
      [ "$streak" -ge 3 ] && return 0
    else
      streak=0
    fi
    sleep 3
  done
  return 1
}

probe_2026() {
  local meta='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"lambda-client-e2e","version":"1.0.0"},"io.modelcontextprotocol/clientCapabilities":{}}'
  local code
  code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 60 \
    -H 'Accept: application/json' -H 'Content-Type: application/json' \
    -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"server/discover\",\"params\":{$meta}}" \
    "$URL_2026" 2>/dev/null)
  [ "$code" = "200" ]
}

probe_2025() {
  local code
  code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 60 \
    -H 'Accept: application/json' -H 'Content-Type: application/json' \
    -H 'MCP-Protocol-Version: 2025-11-25' \
    --data '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"lambda-client-e2e","version":"1.0.0"}}}' \
    "$URL_2025/mcp" 2>/dev/null)
  [ "$code" = "200" ]
}

# Both watches build the SAME binary name with mutually exclusive feature sets,
# so they must not share a target directory: whichever rebuilt last would
# overwrite target/debug/$FUNCTION and the other lane would serve, or be killed
# mid-request while serving, the wrong build. That surfaces as
# hyper::Error(IncompleteMessage) rather than anything naming the real cause.
# A populated shared target dir hides it — the rebuilds are no-ops — so it
# reproduces only after a cargo clean.
TARGET_2026="${CARGO_TARGET_DIR:-$PWD/target}-lambda-e2e-2026-07-28"
TARGET_2025="${CARGO_TARGET_DIR:-$PWD/target}-lambda-e2e-2025-11-25"

# Build both lanes up front. `cargo lambda watch` otherwise builds on first
# invoke, so the readiness probe races the build and the function is replaced
# underneath whatever request is in flight.
echo "=== pre-building $FUNCTION for both lanes (cold target dirs build ~200 crates) ==="
CARGO_TARGET_DIR="$TARGET_2026" cargo build -p turul-mcp-aws-lambda --bin "$FUNCTION" \
  || { echo "FAIL: 2026-07-28 pre-build failed"; exit 1; }
CARGO_TARGET_DIR="$TARGET_2025" cargo build -p turul-mcp-aws-lambda --bin "$FUNCTION" \
  --no-default-features -F cors,sse,protocol-2025-11-25 \
  || { echo "FAIL: 2025-11-25 pre-build failed"; exit 1; }

echo "=== booting cargo lambda watch: $FUNCTION (2026-07-28 default features) on :$PORT_2026 ==="
CARGO_TARGET_DIR="$TARGET_2026" \
cargo lambda watch -p turul-mcp-aws-lambda --bin "$FUNCTION" \
  --invoke-port "$PORT_2026" --ignore-changes >"$LOG_2026" 2>&1 &
WATCH_PID_2026=$!

echo "=== booting cargo lambda watch: $FUNCTION (2025-11-25 feature set) on :$PORT_2025 ==="
CARGO_TARGET_DIR="$TARGET_2025" \
cargo lambda watch -p turul-mcp-aws-lambda --bin "$FUNCTION" \
  --no-default-features -F cors,sse,protocol-2025-11-25 \
  --invoke-port "$PORT_2025" --ignore-changes >"$LOG_2025" 2>&1 &
WATCH_PID_2025=$!

echo "  waiting for both functions to build and answer (up to 10 min each)..."
if ! wait_ready WATCH_PID_2026 probe_2026; then
  echo "FAIL: 2026-07-28 function never answered on $URL_2026 (or watch exited early)"
  tail -40 "$LOG_2026"
  exit 1
fi
echo "  2026-07-28 function is live"
if ! wait_ready WATCH_PID_2025 probe_2025; then
  echo "FAIL: 2025-11-25 function never answered on $URL_2025/mcp (or watch exited early)"
  tail -40 "$LOG_2025"
  exit 1
fi
echo "  2025-11-25 function is live"
echo

echo "=== lane: turul-mcp-client over Lambda, 2026-07-28 (interop-client-probe) ==="
PROBE_OUT=$(cargo run -q -p interop-client-probe -- "$URL_2026" 2>&1)
PROBE_STATUS=$?
echo "$PROBE_OUT" | sed 's/^/    /'

if [ "$PROBE_STATUS" = "0" ] && echo "$PROBE_OUT" | grep -q '^CORE ok$'; then
  ok "interop-client-probe core legs pass over the real Lambda Runtime API"
else
  bad "interop-client-probe core legs" "exit=$PROBE_STATUS (see probe output above)"
fi
if echo "$PROBE_OUT" | grep -q '^LEG tools/list OK.*"echo"'; then
  ok "tools/list observed by the client includes echo"
else
  bad "tools/list via client" "expected a LEG tools/list OK line naming \"echo\""
fi
if echo "$PROBE_OUT" | grep -q '^LEG tools/call OK.*Echo:'; then
  ok "tools/call round-trip observed by the client carries the tool's real output"
else
  bad "tools/call via client" "expected a LEG tools/call OK line containing 'Echo:'"
fi

echo
# 2025-11-25 client-over-Lambda is NOT exercised here, and this is a structural
# limit of the fixture rather than a skipped assertion.
#
# `cargo lambda watch` runs one function instance and serves invocations
# serially. On 2025-11-25 the client opens a long-lived GET SSE listener
# (`start_server_event_listener`); on 2026-07-28 it does not, because that
# revision removed the GET stream. So on the 2025 lane the listener and the
# following POST race for the single instance: whichever arrives first wins, and
# when the listener wins the POST cannot be served — surfacing as
# "Failed to send request". Idle it usually passes, under load it usually fails,
# which makes it a coin flip rather than a gate. Real Lambda scales to multiple
# instances, so this does not describe production.
#
# Verified not to be connection reuse: two requests on one connection against
# the same emulator report "Reusing existing http: connection" and
# "left intact".
#
# The 2026-07-28 client-over-Lambda legs above DO run against the real Runtime
# API. The 2025-11-25 client is covered off-Lambda by the streaming and
# progress E2Es in crates/turul-mcp-server/tests/.
echo "=== lane: turul-mcp-client over Lambda, 2025-11-25 ==="
echo "  SKIP  not exercisable: cargo lambda watch serves invocations serially and"
echo "        the 2025-11-25 client holds a GET SSE stream open, so the stream and"
echo "        the next POST race for the single function instance"
echo "=== $pass passed, $fail failed ==="
if [ "$fail" != "0" ]; then
  echo "2026-07-28 watch log tail:"; tail -30 "$LOG_2026"
  echo "2025-11-25 watch log tail:"; tail -30 "$LOG_2025"
fi
exit $((fail > 0))
