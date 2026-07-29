#!/usr/bin/env bash
# Interop probe for the direction nothing else covers: OUR client against a
# server we did not write.
#
# Every other check in this repo, including scripts/interop-fastmcp.sh, puts a
# turul server on one end. That leaves turul-mcp-client validated only against
# turul-mcp-server — a contract both halves get wrong the same way is
# indistinguishable from one they get right.
#
# Runs two cells:
#   R->R  the control: our client against interop-fixture-server. If a leg fails
#         here too, the fault is ours, not the boundary's.
#   R->P  our client against a FastMCP server.
#
# Assertions are on bytes captured by a proxy sitting between the client and the
# peer, not on what the client reports about itself.
#
#   scripts/interop-turul-client.sh [PORT]
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-8710}"
FIXTURE_PORT="$PORT"
PEER_PORT=$((PORT + 1))
PROXY_PORT=$((PORT + 2))
WORK="${TMPDIR:-/tmp}/turul-interop-client"
FASTMCP_VERSION="4.0.0b1"   # first FastMCP release supporting 2026-07-28
PYTHON_VERSIONS="${PYTHON_VERSIONS:-3.14 3.12}"

command -v uv >/dev/null || { echo "SKIP: uv not found — https://docs.astral.sh/uv/" >&2; exit 0; }

fail=0
mkdir -p "$WORK"

cleanup() { kill ${FIXTURE_PID:-} ${PEER_PID:-} ${PROXY_PID:-} 2>/dev/null; }
trap cleanup EXIT INT TERM

echo "=== building ==="
cargo build -q -p interop-client-probe -p interop-fixture-server || { echo "FAIL: build"; exit 1; }

# ---------------------------------------------------------------- R->R control
echo
echo "=== cell R->R (control: our client, our server) ==="
RUST_LOG=error cargo run -q -p interop-fixture-server -- --port "$FIXTURE_PORT" >/dev/null 2>&1 &
FIXTURE_PID=$!
for _ in $(seq 1 60); do curl -sf -o /dev/null "http://127.0.0.1:$FIXTURE_PORT/mcp" 2>/dev/null && break; sleep 1; done
sleep 1
CONTROL=$(cargo run -q -p interop-client-probe -- "http://127.0.0.1:$FIXTURE_PORT/mcp" 2>&1)
echo "$CONTROL" | sed 's/^/  /'
echo "$CONTROL" | grep -q "^CORE ok" || { echo "  FAIL: control cell failed — fix this before reading R->P"; fail=1; }

# ------------------------------------------------------------------ FastMCP peer
echo
echo "=== cell R->P (our client, FastMCP server) ==="
cat > "$WORK/peer_server.py" <<'PYEOF'
"""A FastMCP server exposing the same surface as interop-fixture-server."""
import sys
from fastmcp import FastMCP

mcp = FastMCP("fastmcp-interop-peer")

@mcp.tool
def echo(text: str) -> str:
    """Echo back the input text"""
    return f"Echo: {text}"

@mcp.resource("file:///fixture/readme.md", mime_type="text/markdown")
def readme() -> str:
    """A small text resource with stable contents"""
    return "# Interop fixture\n\nStable text for cross-implementation probes.\n"

@mcp.prompt
def greeting(name: str) -> str:
    """Greet someone by name"""
    return f"Hello, {name}!"

mcp.run(transport="http", host="127.0.0.1", port=int(sys.argv[1]), show_banner=False)
PYEOF

cat > "$WORK/proxy.py" <<'PYEOF'
"""Logging proxy: records what our client sent, forwards to the peer."""
import http.server, socketserver, json, sys, threading, urllib.request, urllib.error

UPSTREAM, LISTEN, OUT = sys.argv[1], int(sys.argv[2]), sys.argv[3]
CAPTURED = []

class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *a): pass
    def _forward(self, body, method):
        req = urllib.request.Request(UPSTREAM, data=body, method=method)
        for k, v in self.headers.items():
            if k.lower() not in ("host", "content-length"):
                req.add_header(k, v)
        try:
            with urllib.request.urlopen(req) as r:
                return r.read(), r.status, r.headers.get("Content-Type", "application/json")
        except urllib.error.HTTPError as e:
            return e.read(), e.code, e.headers.get("Content-Type", "application/json")
        except Exception as e:
            return json.dumps({"proxy_error": str(e)}).encode(), 502, "application/json"
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        try: rpc = json.loads(body).get("method")
        except Exception: rpc = None
        data, code, ctype = self._forward(body, "POST")
        CAPTURED.append({
            "protocol_version": self.headers.get("MCP-Protocol-Version"),
            "mcp_method": self.headers.get("Mcp-Method"),
            "mcp_name": self.headers.get("Mcp-Name"),
            "session_id": self.headers.get("Mcp-Session-Id"),
            "accept": self.headers.get("Accept"),
            "rpc": rpc,
            "status": code,
        })
        with open(OUT, "w") as f: json.dump(CAPTURED, f)
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        self.send_response(405); self.send_header("Content-Length", "0"); self.end_headers()

class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True

Server(("127.0.0.1", LISTEN), Handler).serve_forever()
PYEOF

setup_env() {
  rm -rf "$WORK/.venv"
  (cd "$WORK" && uv venv --python "$1" --quiet 2>/dev/null) || return 1
  (cd "$WORK" && uv pip install --quiet --prerelease=allow "fastmcp==$FASTMCP_VERSION" 2>/dev/null) || return 1
  "$WORK/.venv/bin/python" -c "import sys,fastmcp;print(f'  fastmcp {fastmcp.__version__} on Python {sys.version.split()[0]}')"
}

PEER_UP=0
for PY_VER in $PYTHON_VERSIONS; do
  setup_env "$PY_VER" || { echo "  (Python $PY_VER unavailable — skipping)"; continue; }
  "$WORK/.venv/bin/python" "$WORK/peer_server.py" "$PEER_PORT" >"$WORK/peer.log" 2>&1 &
  PEER_PID=$!
  for _ in $(seq 1 40); do
    curl -s -o /dev/null "http://127.0.0.1:$PEER_PORT/mcp" 2>/dev/null && { PEER_UP=1; break; }
    kill -0 "$PEER_PID" 2>/dev/null || break
    sleep 0.5
  done
  [ "$PEER_UP" = "1" ] && break
  kill "$PEER_PID" 2>/dev/null
  echo "  (FastMCP server did not come up on Python $PY_VER)"
done

if [ "$PEER_UP" != "1" ]; then
  echo "  SKIP: no FastMCP server could be started — cell R->P NOT EXERCISED"
  tail -15 "$WORK/peer.log" 2>/dev/null | sed 's/^/    /'
  echo
  echo "=== R->R control: $([ "$fail" = 0 ] && echo pass || echo FAIL); R->P: not exercised ==="
  exit "$fail"
fi

CAPTURE="$WORK/capture.json"
rm -f "$CAPTURE"
"$WORK/.venv/bin/python" "$WORK/proxy.py" "http://127.0.0.1:$PEER_PORT/mcp" "$PROXY_PORT" "$CAPTURE" &
PROXY_PID=$!
sleep 1

PEER_OUT=$(cargo run -q -p interop-client-probe -- "http://127.0.0.1:$PROXY_PORT/mcp" 2>&1)
echo "$PEER_OUT" | sed 's/^/  /'

echo
echo "  --- wire capture (what OUR client sent) ---"
"$WORK/.venv/bin/python" - "$CAPTURE" <<'PYEOF'
import json, sys
try:
    captured = json.load(open(sys.argv[1]))
except Exception:
    print("    (no requests captured)"); sys.exit(0)
for c in captured:
    print(f"    rpc={c['rpc']!r} MCP-Protocol-Version={c['protocol_version']!r} "
          f"Mcp-Method={c['mcp_method']!r} Mcp-Name={c['mcp_name']!r} status={c['status']}")
PYEOF

# The client-side obligations are asserted regardless of whether the peer could
# answer: sending `initialize`, omitting the version header, or minting a
# session id would be our defect even against a server that tolerated it.
echo
echo "  --- client-side obligations ---"
"$WORK/.venv/bin/python" - "$CAPTURE" <<'PYEOF'
import json, sys
try:
    captured = json.load(open(sys.argv[1]))
except Exception:
    print("    FAIL: our client sent nothing at all"); sys.exit(1)
errors = []
if not captured:
    errors.append("our client sent no requests")
for c in captured:
    if c["rpc"] in ("initialize", "notifications/initialized"):
        errors.append(f"client sent removed lifecycle method {c['rpc']}")
    if c["session_id"] is not None:
        errors.append(f"{c['rpc']}: client sent Mcp-Session-Id {c['session_id']!r}")
    if c["protocol_version"] != "2026-07-28":
        errors.append(f"{c['rpc']}: MCP-Protocol-Version was {c['protocol_version']!r}")
    if c["mcp_method"] != c["rpc"]:
        errors.append(f"{c['rpc']}: Mcp-Method header was {c['mcp_method']!r}")
    accept = (c["accept"] or "")
    if "application/json" not in accept and "text/event-stream" not in accept:
        errors.append(f"{c['rpc']}: Accept was {accept!r}; a 2026 client must handle both framings")
for e in errors:
    print(f"    FAIL: {e}")
if not errors:
    print(f"    PASS: {len(captured)} request(s), all stateless 2026-07-28, headers agree with bodies")
sys.exit(1 if errors else 0)
PYEOF
[ "$?" = "0" ] || fail=1

# The peer's own coverage is reported, not asserted: FastMCP's server may
# legitimately not implement every method, and that is a fact about the peer.
echo
echo "  --- legs against the FastMCP peer ---"
echo "$PEER_OUT" | grep -E '^LEG ' | sed 's/^/    /'

echo
echo "=== R->R control: $(echo "$CONTROL" | grep -q '^CORE ok' && echo pass || echo FAIL) ==="
exit "$fail"
