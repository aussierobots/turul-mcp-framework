#!/usr/bin/env bash
# Third-party interoperability probe for the MCP 2026-07-28 lane: cell "Go -> R"
# in docs/plans/interop-test-matrix.md.
#
# Drives a turul server with the official MCP Go SDK client — an independent
# implementation, no turul code in the client path — through a logging proxy,
# and asserts on the bytes the proxy actually captured. Architecture mirrors
# scripts/interop-fastmcp.sh: peer client -> local logging proxy -> turul
# server, assertions on captured bytes only, never on the client's self-report.
#
# Peer: github.com/modelcontextprotocol/go-sdk v1.7.0 (first release with full
# 2026-07-28 support: stateless server/discover, per-request `_meta`, MRTR,
# unified subscriptions/listen). Pinned in scripts/interop-go-sdk-probe/go.mod.
#
# Not wired into CI: it needs network access to fetch the Go module and pins a
# peer that is itself only days old. Run it by hand before a release.
#
#   scripts/interop-go-sdk.sh [PORT]
#
# Exit 0 only if:
#   - J1 (modern core): the Go SDK client completed server/discover ->
#     tools/list -> tools/call(echo) -> tools/call(add) with
#     MCP-Protocol-Version: 2026-07-28 on every request, and no initialize,
#     notifications/initialized, or Mcp-Session-Id anywhere in the exchange.
#   - J2 (read surface): resources/list -> resources/read ->
#     resources/templates/list -> prompts/list -> prompts/get ->
#     completion/complete all returned resultType, and the six methods whose
#     result type extends CacheableResult in schema/draft-schema.ts
#     (server/discover, tools/list, resources/list, resources/read,
#     resources/templates/list, prompts/list) all carried ttlMs + cacheScope.
#   - J5 (negative paths, driven with raw HTTP — the Go SDK will not emit a
#     malformed request on purpose): each of the five documented error
#     contracts returned the exact status + JSON-RPC code.
set -uo pipefail
cd "$(dirname "$0")/.."
REPO_ROOT="$(pwd)"

PORT="${1:-8710}"
PROXY_PORT=$((PORT + 1))
WORK="${TMPDIR:-/tmp}/turul-interop-go-sdk"
GO_SDK_VERSION="v1.7.0"  # github.com/modelcontextprotocol/go-sdk — pinned in scripts/interop-go-sdk-probe/go.mod

fail() { echo "FAIL: $*" >&2; exit 1; }

# --- Go toolchain: degrade gracefully when unavailable ------------------
GO_CANDIDATES=(
  "${GOBIN_OVERRIDE:-}"
  "$WORK/goroot/bin/go"
  "/tmp/claude-1000/-home-nick-turul-mcp-framework/e801074a-d05c-4858-9487-e9cceb152994/scratchpad/go/bin/go"
)
GO_BIN=""
for c in "${GO_CANDIDATES[@]}"; do
  [ -n "$c" ] && [ -x "$c" ] && GO_BIN="$c" && break
done
if [ -z "$GO_BIN" ] && command -v go >/dev/null 2>&1; then
  GO_BIN="$(command -v go)"
fi
if [ -z "$GO_BIN" ]; then
  echo "SKIP: no Go toolchain found (checked \$GOBIN_OVERRIDE, the scratchpad path, and PATH)."
  echo "      Install Go 1.25+ or set GOBIN_OVERRIDE=/path/to/go to run this probe."
  exit 0
fi
GOROOT_DIR="$(dirname "$(dirname "$GO_BIN")")"
"$GO_BIN" version || fail "found $GO_BIN but it would not run"

mkdir -p "$WORK"
export GOROOT="$GOROOT_DIR"
export PATH="$GOROOT/bin:$PATH"
export GOPATH="$WORK/gopath"
export GOMODCACHE="$WORK/gomodcache"
export GOFLAGS="${GOFLAGS:-}"

echo "=== Go toolchain: $("$GO_BIN" version) ==="
echo "=== peer: github.com/modelcontextprotocol/go-sdk $GO_SDK_VERSION ==="

PROBE_SRC="$REPO_ROOT/scripts/interop-go-sdk-probe"
[ -f "$PROBE_SRC/go.mod" ] || fail "missing $PROBE_SRC/go.mod"

echo "=== fetching Go module (network required) ==="
( cd "$PROBE_SRC" && "$GO_BIN" mod download ) || fail "go mod download failed — no network, or the pinned peer moved"

PINNED=$( (cd "$PROBE_SRC" && "$GO_BIN" list -m github.com/modelcontextprotocol/go-sdk) 2>/dev/null | awk '{print $2}')
[ "$PINNED" = "$GO_SDK_VERSION" ] || fail "go.mod pins go-sdk $PINNED, script expected $GO_SDK_VERSION — update one to match the other"

PROBE_BIN="$WORK/interop-go-sdk-probe"
echo "=== building probe ==="
( cd "$PROBE_SRC" && "$GO_BIN" build -o "$PROBE_BIN" . ) || fail "go build failed"

echo "=== building turul interop-fixture-server ==="
cd "$REPO_ROOT"
cargo build -q -p interop-fixture-server || fail "cargo build -p interop-fixture-server failed"
SERVER_BIN="$REPO_ROOT/target/debug/interop-fixture-server"
[ -x "$SERVER_BIN" ] || fail "built but $SERVER_BIN is missing"

SERVER_PID=""
trap 'kill "$SERVER_PID" 2>/dev/null' EXIT

start_server() {
  RUST_LOG=error "$SERVER_BIN" --port "$PORT" >/dev/null 2>&1 &
  SERVER_PID=$!
}

server_ready=1
for attempt in 1 2; do
  echo "=== starting turul interop-fixture-server on :$PORT (attempt $attempt) ==="
  start_server
  for _ in $(seq 1 30); do
    kill -0 "$SERVER_PID" 2>/dev/null || break
    # A bare GET /mcp legitimately 405s on this server (no session, no
    # SSE headers) — any HTTP response at all means the listener is up.
    # curl -f would treat that 405 as a connection failure and spin forever.
    code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/mcp" 2>/dev/null)
    [ -n "$code" ] && [ "$code" != "000" ] && { server_ready=0; break; }
    sleep 1
  done
  [ "$server_ready" -eq 0 ] && break
  echo "  server did not come up (process died or never answered) — retrying"
  kill "$SERVER_PID" 2>/dev/null
done
[ "$server_ready" -eq 0 ] || fail "interop-fixture-server never became ready on :$PORT after 2 attempts"
sleep 1

OVERALL=0

# --- J1 + J2: Go SDK client -> proxy -> turul server ---------------------
echo
echo "=== J1 (modern core) + J2 (read surface): Go SDK client via logging proxy ==="
"$PROBE_BIN" -proxy-port "$PROXY_PORT" -upstream-port "$PORT"
J1J2_STATUS=$?
if [ "$J1J2_STATUS" -ne 0 ]; then
  echo "FAIL: cell Go->R, J1/J2 — see wire capture and FAILURES above"
  OVERALL=1
fi

# --- J5: negative paths, driven with raw HTTP -----------------------------
# The Go SDK client will not construct a malformed request on purpose (no API
# to send a mismatched Mcp-Method, an absent MCP-Protocol-Version, or an
# unsupported version), so these five contracts are driven directly with curl
# against the turul server. This is not a client self-report: the response
# bytes captured here are the ground truth being asserted on.
echo
echo "=== J5 (negative paths, raw HTTP — the Go SDK will not emit these on purpose) ==="

J5_FAILURES=()

# Every 2026-07-28 request requires params._meta with at minimum
# io.modelcontextprotocol/protocolVersion and .../clientCapabilities; the
# meta triple is the per-request SEP-2575 mechanism this build enforces.
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}'

check_j5() {
  local name="$1" want_status="$2" want_code="$3"
  shift 3
  local resp status body code
  resp=$(curl -s -D- -o /tmp/turul-interop-go-sdk-j5-body.json "$@" "http://127.0.0.1:$PORT/mcp")
  status=$(printf '%s' "$resp" | head -1 | awk '{print $2}')
  body=$(cat /tmp/turul-interop-go-sdk-j5-body.json)
  code=$(printf '%s' "$body" | grep -o '"code":-\{0,1\}[0-9]*' | head -1 | cut -d: -f2)
  echo "  --- $name ---"
  echo "  status=$status code=$code body=$body"
  if [ "$status" != "$want_status" ] || [ "$code" != "$want_code" ]; then
    J5_FAILURES+=("$name: got status=$status code=$code, want status=$want_status code=$want_code")
    echo "  FAIL"
  else
    echo "  OK"
  fi
}

check_j5 "unsupported MCP-Protocol-Version -> -32022" 400 -32022 \
  -X POST \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 1999-01-01" -H "Mcp-Method: tools/list" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

check_j5 "missing MCP-Protocol-Version -> -32020" 400 -32020 \
  -X POST \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "Mcp-Method: tools/list" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

check_j5 "Mcp-Method disagrees with body method -> -32020" 400 -32020 \
  -X POST \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: tools/call" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

check_j5 "unknown method -> 404 + -32601" 404 -32601 \
  -X POST \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: totally/unknown" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"totally/unknown\",\"params\":{$META}}"

check_j5 "unknown resource URI -> -32602" 200 -32602 \
  -X POST \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: resources/read" -H "Mcp-Name: file:///fixture/missing.md" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/read\",\"params\":{\"uri\":\"file:///fixture/missing.md\",$META}}"

rm -f /tmp/turul-interop-go-sdk-j5-body.json

if [ "${#J5_FAILURES[@]}" -ne 0 ]; then
  echo
  echo "FAIL: cell Go->R, J5:"
  for f in "${J5_FAILURES[@]}"; do echo "  - $f"; done
  OVERALL=1
fi

echo
if [ "$OVERALL" -eq 0 ]; then
  echo "PASS: cell Go->R (github.com/modelcontextprotocol/go-sdk $GO_SDK_VERSION) — J1 + J2 + J5 all green"
else
  echo "FAIL: cell Go->R — see failures above"
fi
exit "$OVERALL"
