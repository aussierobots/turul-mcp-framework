#!/usr/bin/env bash
# Score this framework against upstream's conformance suite.
#
# This is the only check in the repo where the assertions were authored by the
# spec maintainers rather than by us. Everything in tests/ has turul code on
# both ends of the wire and therefore cannot detect a shared misreading of the
# spec; this can, and has — see docs/compliance/README.md for the three defects
# it found, one of them a live DNS-rebinding vulnerability.
#
#   scripts/conformance-suite.sh              # scored scenarios must all pass
#   scripts/conformance-suite.sh --verbose    # keep the harness output on stdout
#
# Exit 0 = every SCORED scenario passed. Unscored scenarios (the tasks
# extension and `pending` ones) are reported but never fail the gate — the
# harness itself excludes them from conformance.
set -euo pipefail

# A PRE-RELEASE pin. 0.2.0-alpha.11 is the newest alpha and the only line that
# scores 2026-07-28 (npm `latest` is 0.1.16, which predates the revision).
# Treat it as a claim with a short shelf life: this repo has already published
# two releases citing a superseded FastMCP prerelease because a currency
# warning went unread. The check below is therefore fatal, not advisory.
CONFORMANCE_VERSION="0.2.0-alpha.11"
PORT="${CONFORMANCE_PORT:-8010}"
VERBOSE=0
[[ "${1:-}" == "--verbose" ]] && VERBOSE=1

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
SERVER_PID=""

cleanup() {
    [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

command -v npx >/dev/null || { echo "SKIP: npx not available"; exit 77; }

echo "==> pin currency"
LATEST_ALPHA="$(npm view "@modelcontextprotocol/conformance" dist-tags.alpha 2>/dev/null || echo "")"
if [[ -n "$LATEST_ALPHA" && "$LATEST_ALPHA" != "$CONFORMANCE_VERSION" ]]; then
    echo "FAIL: pinned $CONFORMANCE_VERSION but dist-tag alpha is now $LATEST_ALPHA."
    echo "      Re-run against the new harness and update CONFORMANCE_VERSION here"
    echo "      AND the score in docs/compliance/README.md. A stale pin makes the"
    echo "      recorded score a measurement of something that no longer exists."
    exit 1
fi
echo "    pinned $CONFORMANCE_VERSION == dist-tag alpha"

echo "==> build fixture server"
cargo build -q -p conformance-fixture-server --manifest-path "$ROOT/Cargo.toml"

"$ROOT/target/debug/conformance-fixture-server" --port "$PORT" >"$WORK/server.log" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 40); do
    if curl -sf -o /dev/null "http://127.0.0.1:$PORT/mcp" \
        -X POST -H 'Content-Type: application/json' \
        -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' \
        -d '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}'
    then break; fi
    sleep 0.25
done

echo "==> conformance server --requirements 2026-07-28"
# --requirements REPLACES --suite/--spec-version; passing both is rejected.
set +e
npx -y "@modelcontextprotocol/conformance@$CONFORMANCE_VERSION" server \
    --requirements 2026-07-28 --url "http://127.0.0.1:$PORT/mcp" \
    -o "$WORK/out" >"$WORK/run.txt" 2>&1
set -e
[[ $VERBOSE -eq 1 ]] && cat "$WORK/run.txt"

python3 - "$WORK/run.txt" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
rows = re.findall(r'^([✓✗]) (\S+):', text, re.M)
unscored = set(re.findall(r'^  [✓✗] (\S+) \((?:extension|pending)\)', text, re.M))
if not rows:
    print("FAIL: no scenario rows parsed — harness output changed shape?")
    print(text[-2000:])
    sys.exit(1)
scored = [(m, n) for m, n in rows if n not in unscored]
failed = [n for m, n in scored if m == '✗']
print(f"    scored     : {len(scored) - len(failed)} pass / {len(failed)} fail")
print(f"    not scored : {len(unscored)} (tasks extension + pending; excluded by the harness)")
if failed:
    print("FAIL: scored scenarios failing:")
    for n in failed:
        print(f"      - {n}")
    sys.exit(1)
print("PASS: every scored 2026-07-28 scenario passed")
PY
