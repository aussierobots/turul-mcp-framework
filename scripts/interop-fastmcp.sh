#!/usr/bin/env bash
# Third-party interoperability probe for the MCP 2026-07-28 lane.
#
# Drives a turul server with FastMCP — an independent implementation, no turul
# code in the client path — through a logging proxy, and asserts on the bytes
# the client actually sent. This is one of the few checks in the repo whose
# client half was not written by this project; most of the suite is our code on
# both ends.
#
# Journeys (see docs/plans/interop-test-matrix.md):
#   J1  server/discover -> tools/list -> tools/call
#   J2  the read surface: resources, templates, prompts, completion
#   J5  negative paths, driven with raw HTTP because a conformant client will
#       not emit the malformed requests they test
#
# Not wired into the blocking gate: it needs network access and pins a
# pre-release FastMCP. Run it by hand before a release, and re-run whenever the
# pinned client moves.
#
#   scripts/interop-fastmcp.sh [PORT]
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
        req = urllib.request.Request(UPSTREAM, data=body, method="POST")
        for k, v in self.headers.items():
            if k.lower() not in ("host", "content-length"):
                req.add_header(k, v)
        try:
            with urllib.request.urlopen(req) as r:
                data, code, ctype = r.read(), r.status, r.headers.get("Content-Type", "application/json")
        except urllib.error.HTTPError as e:
            data, code, ctype = e.read(), e.code, e.headers.get("Content-Type", "application/json")
        # The response is captured too: J2's assertions are about what the
        # server returned, and reading it here avoids trusting the client's
        # rendering of it.
        result, error = None, None
        try:
            parsed = json.loads(data)
        except Exception:
            # The server answers a POST with either a single JSON object or an
            # SSE stream, and picks per request; FastMCP negotiates its way into
            # both. Unwrap the framing so the assertions below see the payload
            # either way.
            parsed, text = None, data.decode("utf-8", "replace")
            for line in text.splitlines():
                if line.startswith("data: "):
                    try:
                        frame = json.loads(line[6:])
                    except Exception:
                        continue
                    if "result" in frame or "error" in frame:
                        parsed = frame
            framing = "sse"
        else:
            framing = "json"
        if parsed is None:
            error = {"unparseable": data[:200].decode("utf-8", "replace")}
        else:
            result, error = parsed.get("result"), parsed.get("error")
        CAPTURED.append({
            "protocol_version": self.headers.get("MCP-Protocol-Version"),
            "mcp_method": self.headers.get("Mcp-Method"),
            "session_id": self.headers.get("Mcp-Session-Id"),
            "rpc": rpc,
            "status": code,
            "result": result,
            "error": error,
            "framing": framing,
        })
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
    print(f"  MCP-Protocol-Version={c['protocol_version']!r} Mcp-Method={c['mcp_method']!r} "
          f"rpc={c['rpc']!r} status={c['status']} framing={c['framing']}"
          + (f" error={c['error']}" if c["error"] else ""))

errors = []
skips = []
rpcs = [c["rpc"] for c in CAPTURED]

# --- J1: the stateless core -------------------------------------------------
for required in ("server/discover", "tools/list", "tools/call"):
    if required not in rpcs:
        errors.append(f"J1: client never sent {required} (sent: {rpcs})")
for c in CAPTURED:
    if c["protocol_version"] != "2026-07-28":
        errors.append(f"J1 {c['rpc']}: MCP-Protocol-Version was {c['protocol_version']!r}, expected '2026-07-28'")
    if c["session_id"] is not None:
        errors.append(f"J1 {c['rpc']}: sent a Mcp-Session-Id ({c['session_id']!r}); 2026-07-28 is stateless")
    if c["rpc"] in ("initialize", "notifications/initialized"):
        errors.append(f"J1: client sent removed lifecycle method {c['rpc']}")

# --- J2: the read surface ---------------------------------------------------
# Cacheable results per the pinned schema's \`extends CacheableResult\`:
# server/discover, tools/list, resources/list, resources/templates/list,
# resources/read, prompts/list. Notably NOT prompts/get or completion/complete.
CACHEABLE = {
    "server/discover", "tools/list", "resources/list",
    "resources/templates/list", "resources/read", "prompts/list",
}
J2 = ("resources/list", "resources/read", "resources/templates/list",
      "prompts/list", "prompts/get", "completion/complete")
for method in J2:
    if method not in rpcs:
        skips.append(f"J2: client never sent {method} — not exercised")

for c in CAPTURED:
    if c["result"] is None:
        if c["error"] is not None:
            errors.append(f"J2 {c['rpc']}: server returned an error: {c['error']}")
        continue
    if "resultType" not in c["result"]:
        errors.append(f"J2 {c['rpc']}: result carries no resultType: {c['result']}")
    if c["rpc"] in CACHEABLE:
        for field in ("ttlMs", "cacheScope"):
            if field not in c["result"]:
                errors.append(f"J2 {c['rpc']}: cacheable result is missing {field}: {c['result']}")

if "resources/templates/list" in rpcs:
    tmpl = next(c["result"] for c in CAPTURED if c["rpc"] == "resources/templates/list")
    if tmpl is None or "resourceTemplates" not in tmpl:
        errors.append(f"J2: resources/templates/list must answer with a list, got {tmpl}")

if "RESULT: ok" not in client.stdout:
    errors.append(f"client did not complete the journey; stderr tail: {client.stderr.strip()[-600:]}")

if skips:
    print("\nSKIPPED (not exercised — not a pass):")
    for s in skips: print(f"  - {s}")
if errors:
    print("\nFAILURES:")
    for e in errors: print(f"  - {e}")
    sys.exit(1)
print(f"\nPASS: FastMCP completed {len([c for c in CAPTURED])} requests over the stateless "
      f"2026-07-28 wire (no initialize, no session header)")
PYEOF

cat > client.py <<'PYEOF'
import asyncio, sys
from fastmcp import Client

async def main(url: str) -> None:
    async with Client(url) as client:
        # J1
        tools = await client.list_tools()
        names = sorted(t.name for t in tools)
        print("TOOLS:", ", ".join(names) or "(none)")
        if "echo" not in names:
            raise SystemExit(f"fixture server must advertise echo, got {names}")
        result = await client.call_tool("echo", {"text": "interop"})
        print("CALL echo ->", str(getattr(result, "content", result))[:120])

        # J2 — each leg is individually guarded: a FastMCP release that lacks
        # one of these methods should surface as an unexercised leg, not as a
        # failure of the server it was pointed at.
        for label, coro in (
            ("resources/list", client.list_resources()),
            ("resources/templates/list", client.list_resource_templates()),
            ("resources/read", client.read_resource("file:///fixture/readme.md")),
            ("prompts/list", client.list_prompts()),
            ("prompts/get", client.get_prompt("greeting", {"name": "Ada"})),
        ):
            try:
                out = await coro
                print(f"{label} -> {str(out)[:100]}")
            except Exception as exc:
                print(f"{label} !! {type(exc).__name__}: {exc}")

        try:
            out = await client.complete(
                {"type": "ref/prompt", "name": "greeting"}, {"name": "name", "value": "a"}
            )
            print(f"completion/complete -> {str(out)[:100]}")
        except AttributeError:
            print("completion/complete !! client has no complete() in this release")
        except Exception as exc:
            print(f"completion/complete !! {type(exc).__name__}: {exc}")

    print("RESULT: ok")

asyncio.run(main(sys.argv[1]))
PYEOF

cd - >/dev/null
echo "=== starting interop-fixture-server on :$PORT (2026-07-28 default build) ==="
RUST_LOG=error cargo run -q -p interop-fixture-server -- --port "$PORT" >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null' EXIT

for _ in $(seq 1 60); do
  curl -sf -o /dev/null "http://127.0.0.1:$PORT/mcp" 2>/dev/null && break
  sleep 1
done
sleep 2

# --- J5: negative paths, driven with raw HTTP -------------------------------
# A conformant client will not send these, so they are issued directly. The
# codes are the 2026-07-28 set: -32020 header mismatch, -32022 unsupported
# version, -32601 unknown method, -32602 invalid params.
#
# The HTTP status is part of the contract and splits by layer: a request the
# transport rejects before dispatch (bad or missing headers, unsupported
# version) answers 4xx, an unknown method answers 404, and a well-formed
# request that fails inside a handler answers 200 with the error in the
# JSON-RPC body. The unknown-resource case below pins that last rule.
echo
echo "=== J5: negative paths (raw HTTP, no client involved) ==="
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"interop","version":"1.0.0"},"io.modelcontextprotocol/clientCapabilities":{}}'
j5_fail=0
j5() {
  local case="$1" want_status="$2" want_code="$3"; shift 3
  local body status code
  body=$(curl -sS -o /tmp/j5.$$ -w '%{http_code}' "$@" "http://127.0.0.1:$PORT/mcp" 2>/dev/null)
  status="$body"; code=$(jq -r '.error.code // "none"' /tmp/j5.$$ 2>/dev/null)
  if [ "$status" = "$want_status" ] && [ "$code" = "$want_code" ]; then
    echo "  PASS  $case -> $status + $code"
  else
    echo "  FAIL  $case -> got $status + $code, wanted $want_status + $want_code"
    cat /tmp/j5.$$; echo
    j5_fail=1
  fi
  rm -f /tmp/j5.$$
}

j5 "unsupported MCP-Protocol-Version" 400 -32022 \
  -H 'Accept: application/json' -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 1999-01-01' -H 'Mcp-Method: tools/list' \
  --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

j5 "missing MCP-Protocol-Version" 400 -32020 \
  -H 'Accept: application/json' -H 'Content-Type: application/json' \
  -H 'Mcp-Method: tools/list' \
  --data "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{$META}}"

j5 "Mcp-Method disagrees with body" 400 -32020 \
  -H 'Accept: application/json' -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: prompts/list' \
  --data "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/list\",\"params\":{$META}}"

j5 "unknown method" 404 -32601 \
  -H 'Accept: application/json' -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: does/not/exist' \
  --data "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"does/not/exist\",\"params\":{$META}}"

j5 "unknown resource uri" 200 -32602 \
  -H 'Accept: application/json' -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: resources/read' \
  -H 'Mcp-Name: file:///nope.txt' \
  --data "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/read\",\"params\":{\"uri\":\"file:///nope.txt\",$META}}"

cd "$WORK"
STATUS=1
for PY_VER in $PYTHON_VERSIONS; do
  echo
  echo "=== J1+J2 environment (uv, Python $PY_VER, fastmcp $FASTMCP_VERSION) ==="
  setup_env "$PY_VER" || { echo "  (Python $PY_VER unavailable — skipping)"; continue; }
  .venv/bin/python probe.py
  STATUS=$?
  if [ "$STATUS" -eq 0 ]; then
    echo "  (verified on Python $PY_VER)"
    break
  fi
  echo "  (probe did not complete on Python $PY_VER — trying next interpreter)"
done

[ "$j5_fail" = "0" ] || STATUS=1
exit "$STATUS"
