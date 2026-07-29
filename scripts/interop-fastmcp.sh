#!/usr/bin/env bash
# Third-party interoperability probe for the MCP 2026-07-28 lane.
#
# Drives a turul server with FastMCP — an independent implementation, no turul
# code in the client path — through a logging proxy, and asserts on the bytes
# the client actually sent. This is the only check in the repo whose client half
# was not written by this project; everything else is our code on both ends.
#
# Not wired into CI: it needs network access and pins a pre-release FastMCP.
# Run it by hand before a release, and re-run whenever the pinned client moves.
#
#   scripts/interop-fastmcp.sh [PORT]
#
# Exit 0 only if the client completed server/discover -> tools/list -> tools/call
# with MCP-Protocol-Version: 2026-07-28 on every request and no initialize or
# session header anywhere in the exchange.
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-8690}"
PROXY_PORT=$((PORT + 1))
WORK="${TMPDIR:-/tmp}/turul-interop-fastmcp"
FASTMCP_VERSION="4.0.0b1"   # first FastMCP release supporting 2026-07-28
# Preferred first. FastMCP 4.0.0b1 segfaults inside CPython 3.14's asyncio C
# module after completing the exchange — reproduced with FastMCP's OWN server as
# the peer, so it is a client/interpreter fault, not a wire-format issue. We try
# 3.14 first and fall back so the probe still asserts a completed round trip.
PYTHON_VERSIONS="${PYTHON_VERSIONS:-3.14 3.12}"

fail() { echo "FAIL: $*" >&2; exit 1; }
command -v uv >/dev/null || fail "uv not found — https://docs.astral.sh/uv/"

mkdir -p "$WORK"
cd "$WORK"

setup_env() {
  rm -rf .venv
  uv venv --python "$1" --quiet 2>/dev/null || return 1
  uv pip install --quiet --prerelease=allow "fastmcp==$FASTMCP_VERSION" 2>/dev/null || return 1
  .venv/bin/python -c "import sys,fastmcp;print(f'  fastmcp {fastmcp.__version__} on Python {sys.version.split()[0]}')"
}

cat > probe.py <<PYEOF
"""FastMCP client -> logging proxy -> turul server. Asserts on captured bytes."""
import http.server, socketserver, json, sys, threading, urllib.request, urllib.error, subprocess

UPSTREAM = "http://127.0.0.1:$PORT/mcp"
CAPTURED = []

class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *a): pass
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        try: rpc = json.loads(body).get("method")
        except Exception: rpc = None
        CAPTURED.append({
            "protocol_version": self.headers.get("MCP-Protocol-Version"),
            "mcp_method": self.headers.get("Mcp-Method"),
            "session_id": self.headers.get("Mcp-Session-Id"),
            "rpc": rpc,
        })
        req = urllib.request.Request(UPSTREAM, data=body, method="POST")
        for k, v in self.headers.items():
            if k.lower() not in ("host", "content-length"):
                req.add_header(k, v)
        try:
            with urllib.request.urlopen(req) as r:
                data, code, ctype = r.read(), r.status, r.headers.get("Content-Type", "application/json")
        except urllib.error.HTTPError as e:
            data, code, ctype = e.read(), e.code, e.headers.get("Content-Type", "application/json")
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        self.send_response(405); self.send_header("Content-Length", "0"); self.end_headers()

class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True

srv = Server(("127.0.0.1", $PROXY_PORT), Handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()

client = subprocess.run(
    [".venv/bin/python", "-u", "client.py", "http://127.0.0.1:$PROXY_PORT/mcp"],
    capture_output=True, text=True,
)
srv.shutdown()
print(client.stdout.strip())

print("\n=== wire capture ===")
for c in CAPTURED:
    print(f"  MCP-Protocol-Version={c['protocol_version']!r} Mcp-Method={c['mcp_method']!r} rpc={c['rpc']!r}")

errors = []
rpcs = [c["rpc"] for c in CAPTURED]
for required in ("server/discover", "tools/list", "tools/call"):
    if required not in rpcs:
        errors.append(f"client never sent {required} (sent: {rpcs})")
for c in CAPTURED:
    if c["protocol_version"] != "2026-07-28":
        errors.append(f"{c['rpc']}: MCP-Protocol-Version was {c['protocol_version']!r}, expected '2026-07-28'")
    if c["session_id"] is not None:
        errors.append(f"{c['rpc']}: sent a Mcp-Session-Id ({c['session_id']!r}); 2026-07-28 is stateless")
    if c["rpc"] in ("initialize", "notifications/initialized"):
        errors.append(f"client sent removed lifecycle method {c['rpc']}")
if "RESULT: ok" not in client.stdout:
    errors.append(f"client did not complete the round trip; stderr tail: {client.stderr.strip()[-400:]}")

if errors:
    print("\nFAILURES:")
    for e in errors: print(f"  - {e}")
    sys.exit(1)
print("\nPASS: FastMCP completed the stateless 2026-07-28 journey (no initialize, no session header)")
PYEOF

cat > client.py <<'PYEOF'
import asyncio, sys
from fastmcp import Client

async def main(url: str) -> None:
    async with Client(url) as client:
        tools = await client.list_tools()
        print("TOOLS:", ", ".join(t.name for t in tools) or "(none)")
        if not tools:
            raise SystemExit("server advertised no tools")
        tool = tools[0]
        schema = getattr(tool, "input_schema", None) or getattr(tool, "inputSchema", None) or {}
        props = schema.get("properties", {}) or {}
        args = {k: (5 if v.get("type") == "number" else "hello") for k, v in props.items()}
        result = await client.call_tool(tool.name, args)
        print(f"CALL {tool.name} ->", str(getattr(result, "content", result))[:160])
    print("RESULT: ok")

asyncio.run(main(sys.argv[1]))
PYEOF

cd - >/dev/null
echo "=== starting turul minimal-server on :$PORT (2026-07-28 default build) ==="
RUST_LOG=error cargo run -q -p minimal-server -- --port "$PORT" >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null' EXIT

for _ in $(seq 1 60); do
  curl -sf -o /dev/null "http://127.0.0.1:$PORT/mcp" 2>/dev/null && break
  sleep 1
done
sleep 2

cd "$WORK"
STATUS=1
for PY_VER in $PYTHON_VERSIONS; do
  echo "=== environment (uv, Python $PY_VER, fastmcp $FASTMCP_VERSION) ==="
  setup_env "$PY_VER" || { echo "  (Python $PY_VER unavailable — skipping)"; continue; }
  .venv/bin/python probe.py
  STATUS=$?
  if [ "$STATUS" -eq 0 ]; then
    echo "  (verified on Python $PY_VER)"
    break
  fi
  echo "  (probe did not complete on Python $PY_VER — trying next interpreter)"
done
exit "$STATUS"
