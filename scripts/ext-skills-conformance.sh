#!/usr/bin/env bash
# Skills extension (SEP-2640) live gate — ADR-033 §Tests "Peer".
#
# Starts examples/ext-skills-server (the repo's own plugin skills), then:
#   1. readback probe: every manifest file is read through resources/read and
#      re-hashed with sha256sum, independent of the framework's digest code,
#      and compared with the bytes on disk;
#   2. the upstream conformance harness's Skills server scenarios;
#   3. its host scenarios (no prefetch; digest, size and frontmatter
#      verification) against turul-mcp-client, via the example's
#      ext-skills-client driver.
# The harness version is read from conformance-suite.sh, its one definition.
#
# Exit 0 = probe passed and every Skills scenario reported 0 failed; 77 = no npx.
set -uo pipefail
cd "$(dirname "$0")/.."
. scripts/lib/mcp2026.sh

PORT="${SKILLS_PORT:-8650}"
URL="http://127.0.0.1:$PORT/mcp"
SKILLS_DIR=plugins/turul-mcp-skills/skills
VERSION=$(grep -oE '^CONFORMANCE_VERSION="[^"]+"' scripts/conformance-suite.sh | cut -d'"' -f2)
[ -n "$VERSION" ] || { echo "FAIL: no CONFORMANCE_VERSION in conformance-suite.sh"; exit 1; }

command -v npx >/dev/null || { echo "  SKIP: npx unavailable"; exit 77; }
cargo build -q -p ext-skills-server --bins || exit 1
CLIENT="$(cargo metadata -q --format-version 1 --no-deps | python3 -I -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')/debug/ext-skills-client"
cargo run -q -p ext-skills-server -- "$PORT" >/dev/null 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null' EXIT
mcp2026_wait_for_server "$PORT" || { echo "FAIL: ext-skills-server did not answer on :$PORT"; exit 1; }

fail=0
bad() { echo "  FAIL $*"; fail=1; }

# --- 1. readback probe -------------------------------------------------------
entries='[]'; cursor=""
while :; do
  params='{}'; [ -n "$cursor" ] && params=$(jq -n --arg c "$cursor" '{cursor:$c}')
  page=$(mcp2026_request "$URL" skills/list "" "$params")
  [ "$(jq -r .result.resultType <<<"$page")" = complete ] || bad "skills/list resultType: $page"
  entries=$(jq -c -s '.[0] + .[1].result.skills' <(echo "$entries") <(echo "$page"))
  cursor=$(jq -r '.result.nextCursor // empty' <<<"$page"); [ -z "$cursor" ] && break
done
expected=$(find "$SKILLS_DIR" -mindepth 2 -maxdepth 2 -name SKILL.md | wc -l)
[ "$(jq length <<<"$entries")" = "$expected" ] || bad "listed $(jq length <<<"$entries") skills, $expected on disk"

files=0
while read -r uri digest size; do
  r=$(mcp2026_request "$URL" resources/read "$uri" "$(jq -n --arg u "$uri" '{uri:$u}')")
  if [ "$(jq -r '.result.contents[0] | has("text")' <<<"$r")" = true ]; then
    bytes() { jq -j '.result.contents[0].text' <<<"$r"; }
  else
    bytes() { jq -r '.result.contents[0].blob' <<<"$r" | base64 -d; }
  fi
  got="sha256:$(bytes | sha256sum | cut -d' ' -f1)"; gsize=$(bytes | wc -c)
  [ "$got" = "$digest" ] && [ "$gsize" = "$size" ] || bad "$uri served $got/$gsize, manifest $digest/$size"
  rel=$(python3 -I -c 'import sys,urllib.parse;print(urllib.parse.unquote(sys.argv[1]))' "${uri#skill://}")
  [ "sha256:$(sha256sum "$SKILLS_DIR/$rel" | cut -d' ' -f1)" = "$digest" ] || bad "$uri digest differs from disk"
  files=$((files + 1))
done < <(jq -r '.[].resources[] | "\(.uri) \(.digest) \(.size)"' <<<"$entries")
echo "  readback: $files files re-hashed across $(jq length <<<"$entries") skills"

code=$(mcp2026_request "$URL" skills/get "" '{"uri":"skill://nope/SKILL.md"}' | jq .error.code)
[ "$code" = -32602 ] || bad "unknown skill answered $code, expected -32602"

# --- 2. upstream conformance scenarios ----------------------------------------
for scenario in sep-2640-skills-enumeration sep-2640-skills-manifest sep-2640-skills-directory; do
  out=$(npx -y "@modelcontextprotocol/conformance@$VERSION" server --url "$URL" --scenario "$scenario" 2>&1 \
        | sed 's/\x1b\[[0-9;]*m//g')
  line=$(grep -E '^Passed: ' <<<"$out" | tail -1)
  echo "  $scenario: ${line:-no result line}"
  [[ "$line" =~ ,\ 0\ failed ]] || { bad "$scenario"; grep -E 'FAILURE|ERROR' <<<"$out" | head -20; }
done

# --- 3. upstream host scenarios against turul-mcp-client ----------------------
for scenario in sep-2640-client-no-prefetch sep-2640-client-verify-digest \
                sep-2640-client-verify-size sep-2640-client-verify-frontmatter; do
  out=$(npx -y "@modelcontextprotocol/conformance@$VERSION" client --command "$CLIENT" --scenario "$scenario" 2>&1 \
        | sed 's/\x1b\[[0-9;]*m//g')
  line=$(grep -E '^Passed: ' <<<"$out" | tail -1)
  echo "  $scenario: ${line:-no result line}"
  [[ "$line" =~ ,\ 0\ failed ]] || { bad "$scenario"; grep -E 'FAILURE|ERROR' <<<"$out" | head -20; }
done

exit "$fail"
