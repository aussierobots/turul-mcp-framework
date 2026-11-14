#!/usr/bin/env bash
# MCP Apps interop: upstream ext-apps host code against examples/ext-apps-server.
#
# Runs the server-facing half of modelcontextprotocol/ext-apps
# `examples/basic-host` (src/implementation.ts: connect, listTools,
# listResources, getToolUiResourceUri, callTool, readResource) in Node, with
# the official TypeScript SDK and ext-apps packages: no turul code on the
# client side. It asserts what a host needs: era 2026-07-28, the ui extension
# in server/discover, tool -> ui:// view links, the view's MIME and content-item
# _meta.ui, and the in-call degradation when the host declares the capability.
#
# basic-host itself uses the SDK default (legacy initialize) and cannot reach a
# 2026-07-28-only server; this probe sets versionNegotiation 'auto'
# (upstream modelcontextprotocol/ext-apps#857).
#
# Not wired into the blocking gate: it needs network access to npm. Run it by
# hand before a release, and re-run whenever a pinned version moves.
#
#   scripts/interop-ext-apps.sh
set -uo pipefail
cd "$(dirname "$0")/.."

PORT=8660 # fixed in examples/ext-apps-server
URL="http://127.0.0.1:$PORT/mcp"
CLIENT_PKG="@modelcontextprotocol/client"
CLIENT_VERSION="2.0.0"
APPS_PKG="@modelcontextprotocol/ext-apps"
APPS_VERSION="2.1.0"
WORK="${TMPDIR:-/tmp}/turul-interop-ext-apps"

fail() { echo "FAIL: $*" >&2; exit 1; }
command -v node >/dev/null || fail "node not found"
command -v npm >/dev/null || fail "npm not found"
curl -s -o /dev/null "$URL" 2>/dev/null && fail "port $PORT is already in use; stop whatever holds it"

# Pin-currency check against what upstream basic-host itself depends on (not
# npm's latest tag): the probe stands in for that host, so it tracks its pins.
HOST_PKG_URL="https://raw.githubusercontent.com/modelcontextprotocol/ext-apps/main/examples/basic-host/package.json"
host_pkg=$(curl -sfL "$HOST_PKG_URL") || fail "could not fetch $HOST_PKG_URL"
for pin in "$CLIENT_PKG $CLIENT_VERSION" "$APPS_PKG $APPS_VERSION"; do
  set -- $pin
  want=$(printf '%s' "$host_pkg" | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>console.log((JSON.parse(s).dependencies||{})[process.argv[1]]||""))' "$1")
  [ "${want#^}" = "$2" ] || echo "WARN: upstream basic-host depends on $1 $want, pinned $2 — re-pin and re-run before trusting this result" >&2
done

mkdir -p "$WORK" && cd "$WORK" || fail "cannot enter $WORK"
rm -rf node_modules package.json package-lock.json
npm install --no-save --no-audit --no-fund "$CLIENT_PKG@$CLIENT_VERSION" "$APPS_PKG@$APPS_VERSION" >/dev/null 2>&1 \
  || fail "npm install failed"
cd - >/dev/null || exit 1

mkdir -p "$WORK/probe"
cat > "$WORK/probe/host.mjs" <<'NODEEOF'
import { Client, StreamableHTTPClientTransport } from '@modelcontextprotocol/client';
import { getToolUiResourceUri, RESOURCE_MIME_TYPE } from '@modelcontextprotocol/ext-apps/app-bridge';

const url = new URL(process.env.URL);
const UI = 'io.modelcontextprotocol/ui';
const errors = [];
const check = (name, ok, got) => {
  console.log(`  ${ok ? 'OK  ' : 'FAIL'}  ${name}${ok ? '' : ` — got ${JSON.stringify(got)}`}`);
  if (!ok) errors.push(name);
};

async function host(declareUi) {
  const client = new Client(
    { name: 'turul-interop-ext-apps', version: '1.0.0' },
    {
      versionNegotiation: { mode: 'auto' },
      ...(declareUi ? { capabilities: { extensions: { [UI]: { mimeTypes: [RESOURCE_MIME_TYPE] } } } } : {}),
    },
  );
  await client.connect(new StreamableHTTPClientTransport(url));
  return client;
}

const text = (r) => r.content?.[0]?.text;

const plain = await host(false);
check('era negotiated is 2026-07-28', plain.getNegotiatedProtocolVersion?.() === '2026-07-28', plain.getNegotiatedProtocolVersion?.());
check('server/discover advertises the ui extension', JSON.stringify(plain.getServerCapabilities()?.extensions?.[UI]) === '{}', plain.getServerCapabilities()?.extensions);

const { tools } = await plain.listTools();
const uri = (n) => getToolUiResourceUri(tools.find((t) => t.name === n) ?? {});
check('get_weather links ui://weather/view.html', uri('get_weather') === 'ui://weather/view.html', uri('get_weather'));
check('app-only refresh_weather is still listed with its link', uri('refresh_weather') === 'ui://weather/view.html', uri('refresh_weather'));

const res = await plain.readResource({ uri: uri('get_weather') });
const c = res.contents?.[0] ?? {};
check('view reads as one content item', res.contents?.length === 1, res.contents?.length);
check('view mimeType is RESOURCE_MIME_TYPE', c.mimeType === RESOURCE_MIME_TYPE, c.mimeType);
check('view carries content-item _meta.ui', typeof c._meta?.ui === 'object', c._meta);
check('view is an HTML document', /^<!DOCTYPE html>/i.test(c.text ?? ''), (c.text ?? '').slice(0, 20));

check('tool returns text content without the ui capability', text(await plain.callTool({ name: 'get_weather', arguments: {} })) === 'Sunny, 22°C', null);
await plain.close();

const ui = await host(true);
check('tool notes the view when the host declares the ui capability', text(await ui.callTool({ name: 'get_weather', arguments: {} })) === 'Sunny, 22°C (view attached)', null);
await ui.close();

if (errors.length) {
  console.log(`APPS failed (${errors.length})`);
  process.exit(1);
}
console.log('APPS ok');
NODEEOF

echo "=== starting ext-apps-server on :$PORT ==="
RUST_LOG=error cargo run -q -p ext-apps-server >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null' EXIT
for _ in $(seq 1 300); do
  curl -s -o /dev/null "$URL" 2>/dev/null && break
  sleep 0.2
done
curl -s -o /dev/null "$URL" 2>/dev/null || fail "ext-apps-server did not start on :$PORT"

echo "=== upstream host path: $CLIENT_PKG@$CLIENT_VERSION + $APPS_PKG@$APPS_VERSION ==="
URL="$URL" node "$WORK/probe/host.mjs"
