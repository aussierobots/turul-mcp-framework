#!/usr/bin/env bash
# Third-party interoperability probe for the MCP 2026-07-28 lane.
#
# Drives a turul server with the official MCP TypeScript SDK — an independent
# implementation, no turul code in the client path — through a logging proxy,
# and asserts on the bytes the proxy captured. Same architecture as
# scripts/interop-fastmcp.sh: peer client -> local logging proxy -> turul
# server, never trusting the client's self-report.
#
# Journeys (see docs/plans/interop-test-matrix.md), cell T->R:
#   J1  server/discover -> tools/list -> tools/call
#   J2  the read surface: resources, templates, prompts, completion
#   J5  negative paths, driven with raw HTTP because a conformant client will
#       not emit the malformed requests they test
#   J6  era negotiation — the reason this peer matters most: the SDK's
#       auto-negotiating client must select the modern 2026-07-28 era via
#       server/discover, with no initialize handshake anywhere on the wire.
#       Only the modern leg runs here; the legacy-fallback leg needs a
#       2025-11-25 lane server, which this script does not stand up — see the
#       explicit SKIP at the end.
#
# The peer is the v2 line of the TypeScript SDK, published to npm under new
# scoped package names (`@modelcontextprotocol/core`, `@modelcontextprotocol/
# client`, `@modelcontextprotocol/server`) — NOT under `@modelcontextprotocol/
# sdk`, which still carries only the 1.x line.
#
# Not wired into the blocking gate: it needs network access to npm. Run it by
# hand before a release, and re-run whenever the pinned version moves.
#
#   scripts/interop-typescript-sdk.sh [PORT]
set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${1:-8710}"
PROXY_PORT=$((PORT + 1))
WORK="${TMPDIR:-/tmp}/turul-interop-ts"
CLIENT_PKG="@modelcontextprotocol/client"
PINNED_VERSION="2.0.0"   # first TS SDK npm release with 2026-07-28 era negotiation

fail() { echo "FAIL: $*" >&2; exit 1; }
command -v node >/dev/null || fail "node not found"
command -v npm >/dev/null || fail "npm not found"
command -v jq >/dev/null || fail "jq not found"

# Pin-currency check: this replaces a stale watch that pointed at
# `@modelcontextprotocol/sdk` (1.x only) and could never see the v2 line move.
LATEST_VERSION=$(npm view "$CLIENT_PKG" dist-tags.latest 2>/dev/null) || fail "npm view $CLIENT_PKG failed — network or registry issue"
if [ "$LATEST_VERSION" != "$PINNED_VERSION" ]; then
  echo "WARN: $CLIENT_PKG dist-tags.latest is $LATEST_VERSION, script is pinned to $PINNED_VERSION — re-pin and re-run before trusting this result" >&2
fi

mkdir -p "$WORK"
cd "$WORK"

echo "=== installing $CLIENT_PKG@$PINNED_VERSION from npm ==="
rm -rf node_modules package.json package-lock.json
npm install --no-save --no-audit --no-fund "$CLIENT_PKG@$PINNED_VERSION" >/dev/null 2>&1 \
  || fail "npm install failed for $CLIENT_PKG@$PINNED_VERSION"

CLIENT_PKG_VERSION=$(node -p "require('./node_modules/@modelcontextprotocol/client/package.json').version")
CORE_PKG_VERSION=$(node -p "require('./node_modules/@modelcontextprotocol/core/package.json').version")
echo "  installed: @modelcontextprotocol/client $CLIENT_PKG_VERSION, @modelcontextprotocol/core $CORE_PKG_VERSION"

mkdir -p "$WORK/interop-probe"
cat > "$WORK/interop-probe/probe.mjs" <<'NODEEOF'
// TS SDK v2 client -> logging proxy -> turul server. Asserts on captured bytes.
import { createServer } from 'node:http';
import { Client, StreamableHTTPClientTransport } from '@modelcontextprotocol/client';

const FIXTURE_PORT = process.env.FIXTURE_PORT;
const PROXY_PORT = process.env.PROXY_PORT;
const UPSTREAM = `http://127.0.0.1:${FIXTURE_PORT}/mcp`;
const PROXY_URL = `http://127.0.0.1:${PROXY_PORT}/mcp`;

const CAPTURED = [];

const proxy = createServer(async (req, res) => {
  if (req.method !== 'POST' && req.method !== 'GET') {
    res.writeHead(405, { 'content-length': '0' });
    res.end();
    return;
  }
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const bodyBuf = Buffer.concat(chunks);

  let rpc = null;
  try {
    rpc = JSON.parse(bodyBuf.toString('utf8')).method ?? null;
  } catch {
    /* GET requests and malformed bodies carry no rpc method */
  }

  const forwardHeaders = {};
  for (const [k, v] of Object.entries(req.headers)) {
    if (['host', 'content-length', 'connection'].includes(k.toLowerCase())) continue;
    if (v !== undefined) forwardHeaders[k] = v;
  }

  let upstreamRes;
  try {
    upstreamRes = await fetch(UPSTREAM, {
      method: req.method,
      headers: forwardHeaders,
      body: req.method === 'POST' ? bodyBuf : undefined,
    });
  } catch (err) {
    res.writeHead(502, { 'content-type': 'text/plain' });
    res.end(`proxy upstream fetch failed: ${err}`);
    return;
  }
  const resBuf = Buffer.from(await upstreamRes.arrayBuffer());
  const ctype = upstreamRes.headers.get('content-type') || 'application/json';

  let result = null;
  let error = null;
  let framing = 'json';
  const text = resBuf.toString('utf8');
  try {
    const parsed = JSON.parse(text);
    result = parsed.result ?? null;
    error = parsed.error ?? null;
  } catch {
    // The 2026-07-28 transport answers a POST with either a single JSON
    // object or SSE framing, chosen per request; unwrap SSE so the
    // assertions below see the payload either way.
    framing = 'sse';
    for (const line of text.split('\n')) {
      if (line.startsWith('data: ')) {
        try {
          const frame = JSON.parse(line.slice(6));
          if ('result' in frame || 'error' in frame) {
            result = frame.result ?? null;
            error = frame.error ?? null;
          }
        } catch {
          /* non-JSON SSE comment/keepalive line */
        }
      }
    }
  }

  CAPTURED.push({
    httpMethod: req.method,
    protocolVersion: req.headers['mcp-protocol-version'] ?? null,
    mcpMethod: req.headers['mcp-method'] ?? null,
    mcpName: req.headers['mcp-name'] ?? null,
    sessionId: req.headers['mcp-session-id'] ?? null,
    rpc,
    status: upstreamRes.status,
    result,
    error,
    framing,
    requestBody: bodyBuf.toString('utf8'),
    responseBody: text,
  });

  res.writeHead(upstreamRes.status, { 'content-type': ctype, 'content-length': resBuf.length });
  res.end(resBuf);
});

await new Promise((resolve, reject) => {
  proxy.once('error', reject);
  proxy.listen(Number(PROXY_PORT), '127.0.0.1', resolve);
});

const errors = [];
const skips = [];
let connectError = null;
let era;
let toolNames = [];

const client = new Client(
  { name: 'turul-interop-ts-probe', version: '1.0.0' },
  { versionNegotiation: { mode: 'auto' } }
);

try {
  await client.connect(new StreamableHTTPClientTransport(new URL(PROXY_URL)));
  era = client.getProtocolEra();
  console.log(`CONNECT: era=${era} negotiatedVersion=${client.getNegotiatedProtocolVersion()}`);
} catch (err) {
  connectError = err;
  console.log(`CONNECT !! ${err?.name ?? typeof err}: ${err?.message ?? err}`);
}

if (connectError === null) {
  // --- J1: server/discover (via connect) -> tools/list -> tools/call ------
  try {
    const tools = await client.listTools();
    toolNames = tools.tools.map(t => t.name).sort();
    console.log('TOOLS:', toolNames.join(', ') || '(none)');
    if (!toolNames.includes('echo')) {
      errors.push(`J1: fixture server must advertise echo, got [${toolNames.join(', ')}]`);
    }
    const called = await client.callTool({ name: 'echo', arguments: { text: 'interop' } });
    console.log('CALL echo ->', JSON.stringify(called.content ?? called).slice(0, 160));
  } catch (err) {
    errors.push(`J1: tools/list or tools/call threw: ${err?.name ?? typeof err}: ${err?.message ?? err}`);
  }

  // --- J2: the read surface, each leg individually guarded ------------------
  const j2 = [
    ['resources/list', () => client.listResources()],
    ['resources/templates/list', () => client.listResourceTemplates()],
    ['resources/read', () => client.readResource({ uri: 'file:///fixture/readme.md' })],
    ['prompts/list', () => client.listPrompts()],
    ['prompts/get', () => client.getPrompt({ name: 'greeting', arguments: { name: 'Ada' } })],
    ['completion/complete', () => client.complete({
      ref: { type: 'ref/prompt', name: 'greeting' },
      argument: { name: 'name', value: 'a' },
    })],
  ];
  for (const [label, run] of j2) {
    try {
      const out = await run();
      console.log(`${label} ->`, JSON.stringify(out).slice(0, 140));
    } catch (err) {
      console.log(`${label} !! ${err?.name ?? typeof err}: ${err?.message ?? err}`);
      skips.push(`J2: ${label} threw and was not exercised: ${err?.message ?? err}`);
    }
  }

  await client.close();
}

console.log('\n=== wire capture ===');
for (const c of CAPTURED) {
  const errSuffix = c.error ? ` error=${JSON.stringify(c.error)}` : '';
  console.log(
    `  ${c.httpMethod} rpc=${c.rpc} MCP-Protocol-Version=${c.protocolVersion} ` +
    `Mcp-Method=${c.mcpMethod} Mcp-Name=${c.mcpName} Mcp-Session-Id=${c.sessionId} ` +
    `status=${c.status} framing=${c.framing}${errSuffix}`
  );
  console.log(`    request:  ${c.requestBody}`);
  console.log(`    response: ${c.responseBody}`);
}

const rpcs = CAPTURED.map(c => c.rpc);

// --- J1: the stateless core -------------------------------------------------
if (connectError !== null) {
  errors.push(`J1: client failed to connect: ${connectError?.message ?? connectError}`);
}
for (const required of ['server/discover', 'tools/list', 'tools/call']) {
  if (!rpcs.includes(required)) errors.push(`J1: client never sent ${required} (sent: ${JSON.stringify(rpcs)})`);
}
for (const c of CAPTURED) {
  if (c.protocolVersion !== '2026-07-28') {
    errors.push(`J1 ${c.rpc}: MCP-Protocol-Version was ${JSON.stringify(c.protocolVersion)}, expected '2026-07-28'`);
  }
  if (c.sessionId !== null) {
    errors.push(`J1 ${c.rpc}: sent a Mcp-Session-Id (${JSON.stringify(c.sessionId)}); 2026-07-28 is stateless`);
  }
  if (c.rpc === 'initialize' || c.rpc === 'notifications/initialized') {
    errors.push(`J1: client sent removed lifecycle method ${c.rpc}`);
  }
}

// --- J2: the read surface ---------------------------------------------------
// Cacheable results per the pinned schema's `extends CacheableResult`:
// server/discover, tools/list, resources/list, resources/templates/list,
// resources/read, prompts/list. Notably NOT prompts/get or completion/complete.
const CACHEABLE = new Set([
  'server/discover', 'tools/list', 'resources/list',
  'resources/templates/list', 'resources/read', 'prompts/list',
]);
for (const c of CAPTURED) {
  if (c.result === null) {
    if (c.error !== null) errors.push(`J2 ${c.rpc}: server returned an error: ${JSON.stringify(c.error)}`);
    continue;
  }
  if (!('resultType' in c.result)) errors.push(`J2 ${c.rpc}: result carries no resultType: ${JSON.stringify(c.result)}`);
  if (CACHEABLE.has(c.rpc)) {
    for (const field of ['ttlMs', 'cacheScope']) {
      if (!(field in c.result)) errors.push(`J2 ${c.rpc}: cacheable result is missing ${field}: ${JSON.stringify(c.result)}`);
    }
  }
}
const templatesEntry = CAPTURED.find(c => c.rpc === 'resources/templates/list');
if (templatesEntry) {
  if (templatesEntry.result === null || !('resourceTemplates' in templatesEntry.result)) {
    errors.push(`J2: resources/templates/list must answer with a list, got ${JSON.stringify(templatesEntry.result)}`);
  } else if (!Array.isArray(templatesEntry.result.resourceTemplates) || templatesEntry.result.resourceTemplates.length !== 0) {
    errors.push(`J2: resources/templates/list must be an empty array for this fixture, got ${JSON.stringify(templatesEntry.result.resourceTemplates)}`);
  }
} else {
  skips.push('J2: client never sent resources/templates/list — not exercised');
}

// --- J6: era negotiation, modern leg ----------------------------------------
// The reason this peer matters most: versionNegotiation mode 'auto' must pick
// the modern era via server/discover with zero initialize/notifications on
// the wire — already covered by the J1 lifecycle-method assertions above.
// This block adds the client's own self-report as corroborating evidence,
// never as the assertion itself.
if (era !== 'modern') {
  errors.push(`J6: client negotiated era ${JSON.stringify(era)}, expected 'modern'`);
}

if (skips.length) {
  console.log('\nSKIPPED (not exercised — not a pass):');
  for (const s of skips) console.log(`  - ${s}`);
}
if (errors.length) {
  console.log('\nFAILURES:');
  for (const e of errors) console.log(`  - ${e}`);
  proxy.close();
  process.exit(1);
}
console.log(
  `\nPASS: MCP TypeScript SDK completed ${CAPTURED.length} requests over the stateless ` +
  `2026-07-28 wire (era=modern, no initialize, no session header)`
);
proxy.close();
process.exit(0);
NODEEOF

cd - >/dev/null

echo
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
# A conformant client will not send these malformed requests, so they are
# issued directly against the fixture server (bypassing the proxy and the
# SDK entirely). Codes are the 2026-07-28 set: -32020 header mismatch, -32022
# unsupported version, -32601 unknown method, -32602 invalid params.
echo
echo "=== J5: negative paths (raw HTTP, no client involved) ==="
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}'
j5_fail=0
j5() {
  local case="$1" want_status="$2" want_code="$3"; shift 3
  local status code
  status=$(curl -sS -o /tmp/interop-ts-j5.$$ -w '%{http_code}' "$@" "http://127.0.0.1:$PORT/mcp" 2>/dev/null)
  code=$(jq -r '.error.code // "none"' /tmp/interop-ts-j5.$$ 2>/dev/null)
  if [ "$status" = "$want_status" ] && [ "$code" = "$want_code" ]; then
    echo "  PASS  $case -> $status + $code"
  else
    echo "  FAIL  $case -> got $status + $code, wanted $want_status + $want_code"
    cat /tmp/interop-ts-j5.$$; echo
    j5_fail=1
  fi
  rm -f /tmp/interop-ts-j5.$$
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

# --- J1 + J2 + J6 (modern leg): the SDK client, through the logging proxy --
echo
echo "=== J1+J2+J6(modern): MCP TypeScript SDK npm @modelcontextprotocol/client $CLIENT_PKG_VERSION ==="
FIXTURE_PORT="$PORT" PROXY_PORT="$PROXY_PORT" node "$WORK/interop-probe/probe.mjs"
NODE_STATUS=$?

echo
echo "=== J6: legacy-fallback leg ==="
echo "  SKIP  J6 legacy leg (versionNegotiation 'auto' falling back to the 2025-11-25"
echo "        initialize handshake) — requires a 2025-11-25 lane fixture server;"
echo "        interop-fixture-server only builds the 2026-07-28 lane and standing up a"
echo "        second lane is out of scope for this script. Untested gap, not a pass."

STATUS=0
[ "$j5_fail" = "0" ] || STATUS=1
[ "$NODE_STATUS" = "0" ] || STATUS=1
exit "$STATUS"
