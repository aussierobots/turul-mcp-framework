#!/usr/bin/env bash
# Third-party interoperability probe for the MCP 2026-07-28 lane: cell "P2 -> R"
# in docs/plans/interop-test-matrix.md.
#
# Drives a turul server with the OFFICIAL MCP Python SDK (`mcp` on PyPI) — an
# independent implementation, no turul code in the client path — through a
# logging proxy, and asserts on the bytes the proxy captured. Architecture
# mirrors scripts/interop-fastmcp.sh: peer client -> local logging proxy ->
# turul server, assertions on captured bytes only, never on the client's
# self-report.
#
# This is a DIFFERENT peer from scripts/interop-fastmcp.sh. FastMCP is a
# third-party framework that happens to speak MCP; `mcp` is the reference
# implementation published by the protocol authors. A wire disagreement with
# the reference client is a stronger signal than one with any other peer, and
# until this script existed the reference Python client had never been pointed
# at this framework at all.
#
# Journeys (see docs/plans/interop-test-matrix.md):
#   J1  server/discover -> tools/list -> tools/call
#   J2  the read surface: resources, templates, prompts, completion
#   J3  MRTR (SEP-2322): the capability gate (-32021 for a client that did not
#       declare elicitation) and the two-leg round trip. The SDK drives both
#       legs itself, so a pass here is a foreign client completing MRTR unaided.
#   J4  request-scoped notifications/progress: SSE framing, and every frame
#       echoing the progressToken the CLIENT declared.
#   J5  negative paths, driven with raw HTTP because a conformant client will
#       not emit the malformed requests they test
#
# J3 and J4 are the two headline 2026-07-28 features, and until this probe
# covered them no peer had exercised either against this framework.
#
# Not wired into the blocking gate: it needs network access to install from
# PyPI. Run it by hand before a release, and re-run whenever the pin moves.
#
#   scripts/interop-python-sdk.sh [PORT]
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-8740}"
PROXY_PORT=$((PORT + 1))
WORK="${TMPDIR:-/tmp}/turul-interop-python-sdk"
MCP_VERSION="2.0.0"   # first `mcp` release with 2026-07-28 support (PyPI, 2026-07-28)

# The SDK supports 3.10+; 3.12 is the version this cell was measured on. Unlike
# the FastMCP probe there is no CPython 3.14 asyncio segfault to dodge here,
# so a single version is enough.
PYTHON_VERSION="${PYTHON_VERSION:-3.12}"

# Pin-currency check — same rationale as the sibling probes. Warns, never fails:
# the probe's job is to test the version it pinned, not to refuse to run when
# the peer ships a new one.
LATEST=$(curl -sS --max-time 15 https://pypi.org/pypi/mcp/json 2>/dev/null \
  | jq -r '.info.version' 2>/dev/null)
if [ -n "$LATEST" ] && [ "$LATEST" != "null" ] && [ "$LATEST" != "$MCP_VERSION" ]; then
  echo "WARN: PyPI's newest mcp is $LATEST, this probe pins $MCP_VERSION —" >&2
  echo "      re-pin and re-run before treating the result as current" >&2
fi

fail() { echo "FAIL: $*" >&2; exit 1; }
# Exit 77 (not 0) on skip: an unrunnable probe must not read as a green cell.
command -v uv >/dev/null || { echo "SKIP: uv not found — https://docs.astral.sh/uv/" >&2; exit 77; }

mkdir -p "$WORK"
cd "$WORK"

rm -rf .venv
uv venv --python "$PYTHON_VERSION" --quiet 2>/dev/null || fail "could not create a Python $PYTHON_VERSION venv"
uv pip install --quiet "mcp==$MCP_VERSION" 2>/dev/null || fail "could not install mcp==$MCP_VERSION"
.venv/bin/python -c "import sys, mcp; print(f'  mcp $MCP_VERSION on Python {sys.version.split()[0]}')"

cat > probe.py <<PYEOF
"""MCP Python SDK client -> logging proxy -> turul server. Asserts on captured bytes."""
import http.server, socketserver, json, sys, threading, urllib.request, urllib.error, subprocess

UPSTREAM = "http://127.0.0.1:$PORT/mcp"
CAPTURED = []

class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *a): pass
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        rpc, progress_token = None, None
        try:
            parsed_req = json.loads(body)
            rpc = parsed_req.get("method")
            # J4: the token the CLIENT declared. Progress notifications must
            # echo this exact value back, or the client cannot correlate them.
            progress_token = (
                parsed_req.get("params", {}).get("_meta", {}).get("progressToken")
            )
        except Exception:
            pass
        req = urllib.request.Request(UPSTREAM, data=body, method="POST")
        for k, v in self.headers.items():
            if k.lower() not in ("host", "content-length"):
                req.add_header(k, v)
        try:
            with urllib.request.urlopen(req) as r:
                data, code, ctype = r.read(), r.status, r.headers.get("Content-Type", "application/json")
        except urllib.error.HTTPError as e:
            data, code, ctype = e.read(), e.code, e.headers.get("Content-Type", "application/json")
        # Capture the response too: J2's assertions are about what the server
        # returned, and reading it here avoids trusting the client's rendering.
        result, error, progress_frames = None, None, []
        try:
            parsed = json.loads(data)
        except Exception:
            # The server answers a POST with either a single JSON object or an
            # SSE stream, choosing per request. Unwrap the framing so the
            # assertions below see the payload either way.
            parsed, text = None, data.decode("utf-8", "replace")
            for line in text.splitlines():
                if line.startswith("data: "):
                    try:
                        frame = json.loads(line[6:])
                    except Exception:
                        continue
                    if frame.get("method") == "notifications/progress":
                        progress_frames.append(frame)
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
            "mcp_name": self.headers.get("Mcp-Name"),
            "session_id": self.headers.get("Mcp-Session-Id"),
            "rpc": rpc,
            "status": code,
            "result": result,
            "error": error,
            "framing": framing,
            "progress_token": progress_token,
            "progress_frames": progress_frames,
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
        # The J3a capability-gate call is a DELIBERATE negative: it is supposed
        # to come back -32021. Asserting on it here would report the probe's own
        # intended behaviour as a server fault.
        if c["error"] is not None and c["mcp_name"] != "confirm":
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

# --- J3: MRTR round trip (SEP-2322) ---------------------------------------
# The property that matters is NEGATIVE as much as positive: on 2026-07-28 the
# server never pushes a request to the client, so an elicitation/create or an
# elicitation-complete notification anywhere in the capture is a violation even
# if the round trip otherwise succeeded.
for c in CAPTURED:
    if c["rpc"] in ("elicitation/create", "notifications/elicitation/complete"):
        errors.append(
            f"J3: {c['rpc']} appeared on the wire; 2026-07-28 carries inputs "
            "inside InputRequiredResult, the server never initiates"
        )

# J3a — the capability gate. A client that did not declare `elicitation` must
# be refused with -32021, NOT handed an input request it cannot answer.
gate = [c for c in CAPTURED
        if c["mcp_name"] == "confirm" and c["error"]
        and c["error"].get("code") == -32021]
if not gate:
    errors.append("J3a: no -32021 capability-gate response observed. The fixture\n        server always registers `confirm`, so this is a regression, not an\n        environmental skip.")
else:
    required = gate[0]["error"].get("data", {}).get("requiredCapabilities", {})
    if "elicitation" not in required:
        errors.append(
            f"J3a: -32021 did not name the capability it needs; data was {required}"
        )

# J3b — the round trip, from the client that DID declare elicitation.
leg1 = [c for c in CAPTURED
        if c["result"] and c["result"].get("resultType") == "input_required"]
if not leg1:
    errors.append("J3b: no input_required leg observed — the MRTR round trip did\n        not happen. `confirm` is part of the fixture contract.")
else:
    first = leg1[0]["result"]
    state = first.get("requestState")
    if not state:
        errors.append("J3b leg 1: input_required carried no requestState to echo back")
    if not first.get("inputRequests"):
        errors.append("J3b leg 1: input_required carried no inputRequests")
    # Leg 2 must be a SECOND tools/call for the same tool that completes.
    completed = [c for c in CAPTURED
                 if c["result"] and c["result"].get("resultType") == "complete"
                 and c["mcp_name"] == "confirm"]
    if not completed:
        errors.append(
            "J3b leg 2: the retry never completed — no resultType=complete for confirm"
        )
    else:
        # The whole point of MRTR: the ORIGINAL call is what resumes, and the
        # opaque state must have travelled back verbatim for the server to
        # accept it. A mismatch would have surfaced as -32602 instead.
        text = json.dumps(completed[0]["result"])
        if "confirmed" not in text:
            errors.append(
                f"J3b leg 2: completed, but not with the elicited answer: {text[:200]}"
            )

# --- J4: progress (request-scoped notifications) ---------------------------
count_calls = [c for c in CAPTURED if c["mcp_name"] == "count"]
if not count_calls:
    errors.append("J4: no call to `count` reached the server — progress was never\n        exercised. `count` is part of the fixture contract.")
else:
    tok = count_calls[0]["progress_token"]
    if tok is None:
        skips.append("J4: client sent no _meta.progressToken; server correctly streams nothing")
    else:
        frames = count_calls[0]["progress_frames"]
        if not frames:
            errors.append(
                f"J4: request declared progressToken {tok!r} but no "
                "notifications/progress were framed in the response"
            )
        for f in frames:
            got = f.get("params", {}).get("progressToken")
            if got != tok:
                errors.append(
                    f"J4: progress notification carried progressToken {got!r}, "
                    f"but the request declared {tok!r} — a token a client "
                    "cannot match to its own request is noise, not correlation"
                )
        if count_calls[0]["framing"] != "sse":
            errors.append(
                f"J4: a progressToken request was framed as "
                f"{count_calls[0]['framing']}, expected sse"
            )

if "RESULT: ok" not in client.stdout:
    errors.append(f"client did not complete the journey; stderr tail: {client.stderr.strip()[-600:]}")

if skips:
    print("\nSKIPPED (not exercised — not a pass):")
    for s in skips: print(f"  - {s}")
if errors:
    print("\nFAILURES:")
    for e in errors: print(f"  - {e}")
    sys.exit(1)
print(f"\nPASS: MCP Python SDK completed {len(CAPTURED)} requests over the stateless "
      f"2026-07-28 wire (no initialize, no session header)")
PYEOF

cat > client.py <<'PYEOF'
import asyncio, sys
from mcp.client import Client
from mcp.types import ElicitResult

async def main(url: str) -> None:
    # mode='auto' is the SDK's era negotiation: it probes server/discover and
    # only falls back to the initialize handshake for a pre-2026 server. Left
    # at the default deliberately — that this reaches the modern era against a
    # turul server IS the assertion, and the proxy capture proves it.
    async with Client(url) as client:
        # J1
        tools = await client.list_tools()
        names = sorted(t.name for t in tools.tools)
        print("TOOLS:", ", ".join(names) or "(none)")
        if "echo" not in names:
            raise SystemExit(f"fixture server must advertise echo, got {names}")
        result = await client.call_tool("echo", {"text": "interop"})
        print("CALL echo ->", str(getattr(result, "content", result))[:120])

        # J2 — each leg individually guarded: an SDK release lacking one of
        # these should surface as an unexercised leg, not as a failure of the
        # server it was pointed at.
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

        # J4 — progress. Supplying progress_callback makes the SDK put a
        # progressToken in the request's _meta; the server then answers SSE and
        # streams notifications/progress. Assertions are on the proxy capture,
        # but we record what the client's own callback saw too, because the two
        # disagreeing would itself be the finding.
        seen = []

        async def on_progress(progress, total, message):
            seen.append((progress, total, message))

        try:
            out = await client.call_tool(
                "count", {"steps": 3}, progress_callback=on_progress
            )
            print(f"J4 count -> {str(getattr(out, 'content', out))[:100]}")
            print(f"J4 progress events seen by client: {len(seen)} {seen}")
        except Exception as exc:
            print(f"J4 !! {type(exc).__name__}: {exc}")

        # J3a — the capability gate. THIS client declared no elicitation
        # capability (no elicitation_callback), so the server must refuse to
        # demand an input it cannot answer, with -32021. A server that instead
        # returned input_required here would be violating SEP-2322.
        try:
            await client.call_tool("confirm", {"subject": "gate"})
            print("J3a !! expected -32021, but the call succeeded")
        except Exception as exc:
            print(f"J3a capability gate -> {type(exc).__name__}: {exc}")

    print("RESULT: ok")


async def mrtr(url: str) -> None:
    """J3b — the MRTR round trip, with a client that DOES declare elicitation.

    Passing elicitation_callback makes the SDK advertise the capability in the
    per-request _meta clientCapabilities. With input_required_max_rounds > 0 the
    SDK is expected to answer the input request and retry the ORIGINAL call
    itself — so a pass here is a foreign client driving the whole two-leg
    journey unaided, which is the strongest form this test can take.
    """
    async def on_elicit(context, params):
        return ElicitResult(action="accept", content={"proceed": True})

    async with Client(url, elicitation_callback=on_elicit) as client:
        try:
            out = await client.call_tool("confirm", {"subject": "launch"})
            print(f"J3b MRTR -> {str(getattr(out, 'content', out))[:140]}")
        except Exception as exc:
            print(f"J3b !! {type(exc).__name__}: {exc}")

    print("MRTR: done")


asyncio.run(main(sys.argv[1]))
asyncio.run(mrtr(sys.argv[1]))
PYEOF

cd - >/dev/null
echo "=== starting interop-fixture-server on :$PORT (2026-07-28 default build) ==="
RUST_LOG=error cargo run -q -p interop-fixture-server -- --port "$PORT" >/dev/null 2>&1 &
SERVER_PID=$!
cleanup() { kill "$SERVER_PID" 2>/dev/null; wait "$SERVER_PID" 2>/dev/null; }
trap cleanup EXIT

for _ in $(seq 1 60); do
  curl -sf -o /dev/null "http://127.0.0.1:$PORT/mcp" 2>/dev/null && break
  kill -0 "$SERVER_PID" 2>/dev/null || fail "fixture server exited during startup"
  sleep 0.5
done

echo "=== J1 + J2 + J3 + J4: MCP Python SDK $MCP_VERSION -> proxy -> turul ==="
( cd "$WORK" && .venv/bin/python -u probe.py )
J1J2=$?

# --- J5: negative paths, driven with raw HTTP -------------------------------
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"interop","version":"1.0.0"},"io.modelcontextprotocol/clientCapabilities":{}}'
J5=0
echo
echo "=== J5: negative paths (raw HTTP, no client involved) ==="

check() {
  local label="$1" want_status="$2" want_code="$3"; shift 3
  local out status code
  out=$(curl -s -w '\n%{http_code}' "$@" 2>/dev/null)
  status=$(echo "$out" | tail -1)
  code=$(echo "$out" | sed '$d' | jq -r '.error.code // empty' 2>/dev/null)
  if [ "$status" = "$want_status" ] && [ "$code" = "$want_code" ]; then
    echo "  OK    $label (status=$status code=$code)"
  else
    echo "  FAIL  $label: got status=$status code=$code, want status=$want_status code=$want_code"
    J5=1
  fi
}

U="http://127.0.0.1:$PORT/mcp"
check "unsupported MCP-Protocol-Version -> -32022" 400 -32022 \
  -X POST "$U" -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 1999-01-01' -H 'Mcp-Method: tools/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

check "missing MCP-Protocol-Version -> -32020" 400 -32020 \
  -X POST "$U" -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'Mcp-Method: tools/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

check "Mcp-Method disagrees with body -> -32020" 400 -32020 \
  -X POST "$U" -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

check "unknown method -> 404 + -32601" 404 -32601 \
  -X POST "$U" -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: totally/unknown' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"totally/unknown\",\"params\":{$META}}"

check "unknown resource URI -> -32602" 200 -32602 \
  -X POST "$U" -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: resources/read' \
  -H 'Mcp-Name: file:///fixture/missing.md' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/read\",\"params\":{\"uri\":\"file:///fixture/missing.md\",$META}}"

echo
if [ "$J1J2" -eq 0 ] && [ "$J5" -eq 0 ]; then
  echo "PASS: cell Python SDK -> R (mcp==$MCP_VERSION) — J1 + J2 + J3 + J4 + J5 all green"
  exit 0
fi
echo "FAIL: cell Python SDK -> R — J1/J2/J3/J4 exit=$J1J2, J5 exit=$J5"
exit 1
