#!/usr/bin/env bash
# Offline, deterministic guard on every vendored MCP schema pin in the repo:
# the core protocol crate plus the two extension crates.
#
# Nothing else recomputes a vendored file's checksum, so a hand-edited schema,
# a half-applied re-pin, or a copy taken from the wrong upstream directory
# stays invisible: each crate's compliance suite validates its Rust types
# against whatever bytes are on disk and passes regardless.
#
# No network access — safe to run on every push.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
note() { printf '%-52s %s\n' "$1" "$2"; }
bad()  { note "$1" "FAIL — $2"; fail=1; }

# ---------------------------------------------------------------------------
# Core protocol crate — turul-mcp-protocol-2026-07-28
# ---------------------------------------------------------------------------
# Two independent artifacts pin the upstream tree and MUST agree:
#   1. `PIN` in src/compliance/fetch.rs   (the example fixtures)
#   2. the provenance block in schema/README.md (the vendored schema.ts)

CRATE=crates/turul-mcp-protocol-2026-07-28
SCHEMA=$CRATE/schema/schema.ts
README=$CRATE/schema/README.md
FETCH=$CRATE/src/compliance/fetch.rs
PINDOC=$CRATE/schema/EXAMPLES_PIN.md

# 1. Vendored file matches the checksum the provenance block claims.
actual_sha=$(shasum -a 256 "$SCHEMA" | cut -d' ' -f1)
claimed_sha=$(grep -oE '^- \*\*Content sha256\*\*: `[0-9a-f]{64}`' "$README" | grep -oE '[0-9a-f]{64}' | head -1)
if [ -z "$claimed_sha" ]; then
  bad "core schema checksum" "no 'Content sha256' line found in $README"
elif [ "$actual_sha" != "$claimed_sha" ]; then
  bad "core schema checksum" "on disk $actual_sha != README $claimed_sha"
else
  note "core schema checksum" "OK ($actual_sha)"
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
  schema/2026-07-28/*) note "core pinned subpath" "OK ($subpath)" ;;
  *)                   bad  "core pinned subpath" "'$subpath' is not under schema/2026-07-28/" ;;
esac

# ---------------------------------------------------------------------------
# Apps extension — turul-mcp-ext-apps
# ---------------------------------------------------------------------------
# Upstream modelcontextprotocol/ext-apps publishes BOTH a dated release
# (`specification/2026-01-26/apps.mdx`, immutable) and a floating
# `specification/draft/apps.mdx` (the next Apps cycle). The two differ by
# hundreds of lines, and a copy taken from `draft/` cannot be reproduced.
# Rows come from the provenance table in the crate's schema/README.md:
#   | `file` | `upstream/path` | `<40-hex>` (tag ...) | `<64-hex>` | date |

APPS_DIR=crates/turul-mcp-ext-apps/schema
APPS_README=$APPS_DIR/README.md

apps_rows=$(awk -F'|' '
  /^\|[^|]*`[^`]+`[^|]*\|/ {
    n = split($0, c, "|")
    if (n < 5) next
    file = c[2]; src = c[3]; commit = c[4]; sum = c[5]
    gsub(/[^a-zA-Z0-9._\/-]/, "", file)
    gsub(/[^a-zA-Z0-9._\/-]/, "", src)
    if (match(commit, /[0-9a-f]{40}/)) commit = substr(commit, RSTART, RLENGTH); else commit = ""
    if (match(sum, /[0-9a-f]{64}/))    sum    = substr(sum,    RSTART, RLENGTH); else sum    = ""
    if (file != "" && src != "" && commit != "" && sum != "") print file, src, commit, sum
  }' "$APPS_README")

if [ -z "$apps_rows" ]; then
  bad "ext-apps provenance table" "no complete rows parsed from $APPS_README"
else
  while read -r file src commit sum; do
    path=$APPS_DIR/$file
    if [ ! -f "$path" ]; then
      bad "ext-apps $file" "listed in README but missing on disk"
      continue
    fi
    disk=$(shasum -a 256 "$path" | cut -d' ' -f1)
    if [ "$disk" != "$sum" ]; then
      bad "ext-apps $file checksum" "on disk $disk != README $sum"
    else
      note "ext-apps $file checksum" "OK (${commit:0:12})"
    fi

    # The spec document MUST come from a dated release directory. `src/*.ts`
    # rows are exempt: upstream ships no dated copy of the SDK types, which
    # schema/README.md records explicitly.
    case "$file" in
      *.mdx)
        case "$src" in
          */specification/[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]/*|specification/[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]/*)
            note "ext-apps $file source path" "OK ($src)" ;;
          *)
            bad "ext-apps $file source path" "'$src' is not a dated specification/<YYYY-MM-DD>/ path" ;;
        esac ;;
    esac
  done <<EOF
$apps_rows
EOF
fi

# Every vendored file must be accounted for by a table row, so a stray copy
# cannot sit alongside the pinned ones unchecked.
for f in "$APPS_DIR"/*; do
  base=$(basename "$f")
  [ "$base" = "README.md" ] && continue
  if ! printf '%s\n' "$apps_rows" | cut -d' ' -f1 | grep -qx "$base"; then
    bad "ext-apps $base" "on disk but absent from the $APPS_README table"
  fi
done

# ---------------------------------------------------------------------------
# Tasks extension — turul-mcp-ext-tasks
# ---------------------------------------------------------------------------
# Upstream modelcontextprotocol/ext-tasks has no dated release directory and no
# tags at all — `schema/draft/` is the only thing that exists, so the dated-path
# rule above cannot apply here. The check is that the vendored bytes still match
# what was fetched, and that a commit pin is recorded at all.
#
# TEMPORARY: the expected checksums live here rather than in that crate's
# schema/README.md, which records no checksums. Owner: turul-mcp-ext-tasks.
# Removal trigger: once that README grows `Content sha256` cells like
# turul-mcp-ext-apps has, read them from there and delete this table.

TASKS_DIR=crates/turul-mcp-ext-tasks/schema
TASKS_README=$TASKS_DIR/README.md

tasks_expected="\
draft-schema.ts 2203cc75469e32a92a60f4b7b4de949577e25f18fafff69aa92ec06773ab70f6
draft-schema.json b17cb4a2534379c214b17770bd5d3d54f69fde16a953bfb542c58235a61274bb"

while read -r file sum; do
  path=$TASKS_DIR/$file
  if [ ! -f "$path" ]; then
    bad "ext-tasks $file" "expected vendored file is missing"
    continue
  fi
  disk=$(shasum -a 256 "$path" | cut -d' ' -f1)
  if [ "$disk" != "$sum" ]; then
    bad "ext-tasks $file checksum" "on disk $disk != expected $sum"
  else
    note "ext-tasks $file checksum" "OK"
  fi
done <<EOF
$tasks_expected
EOF

tasks_pin=$(grep -oE '[0-9a-f]{40}' "$TASKS_README" | head -1)
if [ -z "$tasks_pin" ]; then
  bad "ext-tasks commit pin" "no 40-hex upstream commit recorded in $TASKS_README"
else
  note "ext-tasks commit pin" "OK ($tasks_pin)"
fi

exit "$fail"
