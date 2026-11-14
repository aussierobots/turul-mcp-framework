#!/usr/bin/env bash
# Test-build warning sweep: fails when any audited test build emits a warning.
#
#   scripts/warning-sweep.sh default       # MCP 2026-07-28 lane
#   scripts/warning-sweep.sh opt-in-2025   # MCP 2025-11-25 lane
#
# `cargo test --no-run` at the root builds only `default-members`, so every
# package a gate tests with `-p` outside them is swept here too, with the same
# feature set the gate uses. The one list lives in this file; scripts/ci-gates.sh
# and .github/workflows/ci.yml both call it.
set -uo pipefail
cd "$(dirname "$0")/.."

case "${1:-}" in
  default)
    builds=(
      ""
      "-p turul-mcp-client"
      "-p turul-mcp-client --features ext-apps"
      "-p turul-mcp-server --no-default-features --features http,sse,ext-tasks"
      "-p turul-mcp-server --no-default-features --features http,sse,ext-skills"
      "-p turul-mcp-ext-tasks -p turul-mcp-ext-apps -p turul-mcp-ext-skills"
      "-p turul-ext-tasks-backend-e2e"
    ) ;;
  opt-in-2025)
    builds=(
      "-p turul-mcp-server --no-default-features --features http,sse,protocol-2025-11-25"
      "-p turul-mcp-builders --no-default-features --features protocol-2025-11-25"
      "-p turul-http-mcp-server --no-default-features --features sse,protocol-2025-11-25"
      "-p turul-mcp-aws-lambda --no-default-features --features cors,sse,protocol-2025-11-25"
      "-p turul-mcp-client --no-default-features --features http,sse,client-2025-11-25-only"
      "-p turul-mcp-client --no-default-features --features http,sse,client-2026-07-28-only"
      "-p turul-mcp-framework-integration-tests"
    ) ;;
  *) echo "usage: $0 default|opt-in-2025" >&2; exit 2 ;;
esac

fail=0
for args in "${builds[@]}"; do
  # shellcheck disable=SC2086 # word-splitting the argument string is intended
  n=$(cargo test --no-run $args 2>&1 | grep -c 'warning:' || true)
  echo "  test-build warnings [${args:-default-members}]: $n"
  [ "$n" = "0" ] || fail=1
done
exit $fail
