#!/usr/bin/env bash
# Local release gates for the 2026-07-28 cutover branch — run by hand, no hosted CI.
#
#   default lane  = MCP 2026-07-28 (stateless core)
#   opt-in lane   = 2025-11-25 (legacy)
#
# Gates mirror docs/plans/2026-07-28-final-readiness-audit.md §7.
# Usage:  scripts/ci-gates.sh [default|opt-in-2025|mutex|docs|all]   (default: all)
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
run() { echo "=== $1 ==="; shift; if "$@"; then echo "  PASS"; else echo "  FAIL ($*)"; fail=1; fi; }

gate_default() {
  run "build (default = protocol-2026-07-28)" cargo build
  run "clippy (deny warnings)" cargo clippy --all-targets -- -D warnings
  run "test" cargo test
  echo "=== warning sweep on the test build ==="
  n=$(cargo test --no-run 2>&1 | grep -c 'warning:' || true)
  echo "  test-build warnings: $n"; [ "$n" = "0" ] || { echo "  FAIL"; fail=1; }
  run "2026 stateless acceptance (real HTTP)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test discover_stateless_2026
  run "2026 HTTP surface (GET/DELETE 405, session-id ignored)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test stateless_2026_http_surface
  run "2026 subscriptions/listen (ack-first, filtered delivery)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test subscriptions_listen_2026
  run "2026 request-metadata headers (Mcp-Method/Mcp-Name, -32001/-32004)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test mcp_headers_2026
  run "2026 unknown-method mapping (404 + -32601)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test error_mapping_2026
  run "2026 MRTR (input_required round trip, -32003 capability gate)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test mrtr_2026
  run "2026 Mcp-Param-* mirroring (x-mcp-header validation)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test mcp_param_2026
  run "2026 per-request log gating (logLevel opt-in)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test log_gating_2026
  run "2026 schema fidelity (derive/builders pipeline to the wire)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test schema_fidelity_2026
  run "protocol-2026 compliance + upstream wire fixtures" \
    cargo test -p turul-mcp-protocol-2026-07-28 --features compliance
  run "bilingual client (not in default-members)" cargo test -p turul-mcp-client
  run "2026 client example (pairs with minimal-server)" cargo build -p streamable-http-client
  run "client-using examples (not in default-members)" cargo build -p mrtr-elicitation-server -p bilingual-fleet-client -p ext-tasks-server --bins
  run "Tasks extension (SEP-2663, opt-in feature)" cargo test -p turul-mcp-server --no-default-features --features http,sse,ext-tasks --test ext_tasks_2026
  run "ext crates standalone" cargo test -p turul-mcp-ext-tasks -p turul-mcp-ext-apps
  run "Tasks extension client e2e" cargo test -p turul-mcp-client --features ext-tasks --test ext_tasks_e2e_2026
}

gate_opt_in_2025() {
  run "server 2025-11-25 build"  cargo build  -p turul-mcp-server      --no-default-features --features http,sse,protocol-2025-11-25
  run "server 2025-11-25 clippy" cargo clippy -p turul-mcp-server      --no-default-features --features http,sse,protocol-2025-11-25 -- -D warnings
  run "builders 2025-11-25"      cargo build  -p turul-mcp-builders    --no-default-features --features protocol-2025-11-25
  run "http-server 2025-11-25"   cargo build  -p turul-http-mcp-server --no-default-features --features sse,protocol-2025-11-25
  run "derive 2025-11-25"        cargo build  -p turul-mcp-derive      --no-default-features --features protocol-2025-11-25
  run "lambda 2025-11-25"        cargo build  -p turul-mcp-aws-lambda  --no-default-features --features cors,sse,protocol-2025-11-25
  run "client 2025-11-25-only"   cargo build  -p turul-mcp-client      --no-default-features --features http,sse,client-2025-11-25-only
  run "client 2026-07-28-only"   cargo build  -p turul-mcp-client      --no-default-features --features http,sse,client-2026-07-28-only
  run "client-initialise-server" cargo build  -p client-initialise-server --no-default-features
  run "tools E2E"       cargo test -p mcp-tools-tests
  run "resources E2E"   cargo test -p mcp-resources-tests
  run "prompts E2E"     cargo test -p mcp-prompts-tests
  run "roots E2E"       cargo test -p mcp-roots-tests
  run "sampling E2E"    cargo test -p mcp-sampling-tests
  run "elicitation E2E" cargo test -p mcp-elicitation-tests
  run "tasks E2E"       cargo test -p turul-mcp-framework-integration-tests --test tasks_e2e_inmemory
  run "ping auth E2E"   cargo test -p turul-mcp-framework-integration-tests --test ping_auth_2025
}

gate_mutex() {
  echo "=== spec mutex (both features must NOT compile) ==="
  if cargo build -p turul-mcp-protocol --features protocol-2025-11-25,protocol-2026-07-28 2>/dev/null; then
    echo "  FAIL: both protocol features compiled together"; fail=1
  else
    echo "  PASS: spec mutex fired as expected"
  fi
}

gate_docs() {
  echo "=== rustdoc (deny warnings) ==="
  if RUSTDOCFLAGS="-D warnings" cargo doc --no-deps; then echo "  PASS"; else echo "  FAIL"; fail=1; fi
}

case "${1:-all}" in
  default)      gate_default ;;
  opt-in-2025)  gate_opt_in_2025 ;;
  mutex)        gate_mutex ;;
  docs)         gate_docs ;;
  all)          gate_default; gate_opt_in_2025; gate_mutex; gate_docs ;;
  *) echo "usage: $0 [default|opt-in-2025|mutex|docs|all]"; exit 2 ;;
esac

echo; [ "$fail" = "0" ] && echo "ALL GATES PASSED" || echo "ONE OR MORE GATES FAILED"
exit "$fail"
