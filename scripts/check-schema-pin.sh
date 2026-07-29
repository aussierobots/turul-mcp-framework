#!/usr/bin/env bash
# Offline, deterministic guard on the vendored MCP schema pin.
#
# Two independent artifacts pin the upstream tree and MUST agree:
#   1. `PIN` in crates/turul-mcp-protocol-2026-07-28/src/compliance/fetch.rs
#   2. the provenance block in crates/turul-mcp-protocol-2026-07-28/schema/README.md
#
# Nothing else recomputes the vendored file's checksum, so a hand-edited schema
# or a half-applied re-pin stays invisible: the compliance suite validates the
# Rust types against whatever bytes are on disk and passes regardless.
#
# No network access — safe to run on every push.
set -uo pipefail
cd "$(dirname "$0")/.."

CRATE=crates/turul-mcp-protocol-2026-07-28
SCHEMA=$CRATE/schema/schema.ts
README=$CRATE/schema/README.md
FETCH=$CRATE/src/compliance/fetch.rs
PINDOC=$CRATE/schema/EXAMPLES_PIN.md

fail=0
note() { printf '%-52s %s\n' "$1" "$2"; }
bad()  { note "$1" "FAIL — $2"; fail=1; }

# 1. Vendored file matches the checksum the provenance block claims.
actual_sha=$(shasum -a 256 "$SCHEMA" | cut -d' ' -f1)
claimed_sha=$(grep -oE '^- \*\*Content sha256\*\*: `[0-9a-f]{64}`' "$README" | grep -oE '[0-9a-f]{64}' | head -1)
if [ -z "$claimed_sha" ]; then
  bad "schema checksum" "no 'Content sha256' line found in $README"
elif [ "$actual_sha" != "$claimed_sha" ]; then
  bad "schema checksum" "on disk $actual_sha != README $claimed_sha"
else
  note "schema checksum" "OK ($actual_sha)"
fi

# 2. fetch.rs PIN and the provenance block name the same upstream commit.
pin_sha=$(grep -oE 'sha: "[0-9a-f]{40}"' "$FETCH" | grep -oE '[0-9a-f]{40}' | head -1)
readme_sha=$(grep -oE '^- \*\*Upstream commit pin\*\*: `[0-9a-f]{40}`' "$README" | grep -oE '[0-9a-f]{40}' | head -1)
if [ -z "$pin_sha" ] || [ -z "$readme_sha" ]; then
  bad "pin parity (fetch.rs vs README)" "could not read one of the commit pins"
elif [ "$pin_sha" != "$readme_sha" ]; then
  bad "pin parity (fetch.rs vs README)" "fetch.rs $pin_sha != README $readme_sha"
else
  note "pin parity (fetch.rs vs README)" "OK ($pin_sha)"
fi

# 3. EXAMPLES_PIN.md names that same commit.
ex_sha=$(grep -oE '`[0-9a-f]{40}`' "$PINDOC" | grep -oE '[0-9a-f]{40}' | head -1)
if [ "$ex_sha" != "$pin_sha" ]; then
  bad "pin parity (EXAMPLES_PIN.md)" "EXAMPLES_PIN $ex_sha != fetch.rs $pin_sha"
else
  note "pin parity (EXAMPLES_PIN.md)" "OK"
fi

# 4. The pinned subpath must be the immutable dated directory. Upstream
#    `schema/draft/` is the NEXT spec cycle's floating pointer — resolving it
#    walks onto next-cycle content while still claiming to implement 2026-07-28.
# head -1 selects the `PIN` const; later matches are test fixtures.
subpath=$(grep -oE 'subpath: "[^"]+"' "$FETCH" | head -1 | sed 's/subpath: "//;s/"//')
case "$subpath" in
  schema/2026-07-28/*) note "pinned subpath" "OK ($subpath)" ;;
  *)                   bad  "pinned subpath" "'$subpath' is not under schema/2026-07-28/" ;;
esac

exit "$fail"
