#!/usr/bin/env bash
# Consumer smoke test — builds a crate OUTSIDE this workspace against the
# dependency list the README tells users to write, using the README's own
# Quick Start code.
#
# WHY THIS EXISTS
# ---------------
# Every example in this repo is a workspace member. A workspace member inherits
# `[workspace.dependencies]`, shares one lockfile, and is written by someone who
# already knows the required dependency set. None of that is true for someone
# doing `cargo new` + `cargo add`.
#
# On 2026-08-15 the published README's Quick Start did not compile: it listed
# three dependencies where six are needed (`#[mcp_tool]` emits code naming
# `serde_json`, `async_trait`, `turul_mcp_builders` and `turul_mcp_protocol`,
# and generated code is compiled in the CONSUMER's crate, so those names must be
# the consumer's direct dependencies), and its `main()` body had two type errors.
# 3617 passing tests could not see any of it, because nothing ever built a crate
# from outside the workspace. This gate is that missing check.
#
# It extracts the Quick Start from README.md rather than keeping a copy, so the
# README is the thing under test. A drift between docs and reality fails here.
#
# Local crates are wired in with [patch.crates-io] path overrides, so this runs
# pre-publish against working-tree code. Pass --published to resolve from
# crates.io instead (post-publish verification of what users actually get).
set -uo pipefail
cd "$(dirname "$0")/.."
REPO="$PWD"

PUBLISHED=0
[ "${1:-}" = "--published" ] && PUBLISHED=1

WORK="${TMPDIR:-/tmp}/turul-consumer-smoke.$$"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/src"

# --- extract the README Quick Start (first rust fence under "1. Function Macros")
python3 - "$REPO/README.md" "$WORK/src/main.rs" <<'PY'
import re, sys, pathlib
md = pathlib.Path(sys.argv[1]).read_text()
i = md.index('### 1. Function Macros')
m = re.search(r'```rust\n(.*?)```', md[i:], re.S)
if not m:
    sys.exit("could not find the Quick Start rust block in README.md")
pathlib.Path(sys.argv[2]).write_text(m.group(1))
PY
[ -s "$WORK/src/main.rs" ] || { echo "FAIL: extracted an empty Quick Start"; exit 1; }

# --- the dependency set the README tells a user to write, and nothing more
cat > "$WORK/Cargo.toml" <<'TOML'
[package]
name = "turul-consumer-smoke"
version = "0.0.0"
edition = "2024"

# Detach from any enclosing workspace — the whole point is to NOT be a member.
[workspace]

[dependencies]
turul-mcp-server = "0.4"
turul-mcp-derive = "0.4"
turul-mcp-builders = "0.4"
turul-mcp-protocol = "0.4"
serde_json = "1"
async-trait = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
TOML

if [ "$PUBLISHED" = "0" ]; then
  cat >> "$WORK/Cargo.toml" <<TOML

# Pre-publish: resolve the turul crates from this working tree.
[patch.crates-io]
turul-mcp-server = { path = "$REPO/crates/turul-mcp-server" }
turul-mcp-derive = { path = "$REPO/crates/turul-mcp-derive" }
turul-mcp-builders = { path = "$REPO/crates/turul-mcp-builders" }
turul-mcp-protocol = { path = "$REPO/crates/turul-mcp-protocol" }
turul-mcp-protocol-2026-07-28 = { path = "$REPO/crates/turul-mcp-protocol-2026-07-28" }
turul-mcp-schema-validation = { path = "$REPO/crates/turul-mcp-schema-validation" }
turul-mcp-session-storage = { path = "$REPO/crates/turul-mcp-session-storage" }
turul-mcp-task-storage = { path = "$REPO/crates/turul-mcp-task-storage" }
turul-mcp-server-state-storage = { path = "$REPO/crates/turul-mcp-server-state-storage" }
turul-http-mcp-server = { path = "$REPO/crates/turul-http-mcp-server" }
turul-mcp-oauth = { path = "$REPO/crates/turul-mcp-oauth" }
turul-mcp-ext-tasks = { path = "$REPO/crates/turul-mcp-ext-tasks" }
TOML
  echo "=== consumer smoke: README Quick Start, deps as documented, local code ==="
else
  echo "=== consumer smoke: README Quick Start, deps as documented, crates.io ==="
fi

if (cd "$WORK" && cargo build 2>&1 | tail -25); then
  echo "  PASS — a crate outside the workspace compiles the documented example"
  exit 0
else
  echo "  FAIL — the README's Quick Start does not compile for a new user."
  echo "  Fix the README's dependency list or its code, not this gate."
  exit 1
fi
