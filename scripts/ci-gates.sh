#!/usr/bin/env bash
# Local release gates for the 2026-07-28 cutover branch — run by hand, no hosted CI.
#
#   default lane  = MCP 2026-07-28 (stateless core)
#   opt-in lane   = 2025-11-25 (legacy)
#
# Gates mirror docs/plans/2026-07-28-final-readiness-audit.md §7.
# Usage:  scripts/ci-gates.sh [default|opt-in-2025|lambda|mutex|docs|examples|all]  (default: all)
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
run() { echo "=== $1 ==="; shift; if "$@"; then echo "  PASS"; else echo "  FAIL ($*)"; fail=1; fi; }

gate_default() {
  run "schema pin integrity" ./scripts/check-schema-pin.sh
  run "protocol crate purity" ./scripts/check-protocol-purity.sh
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
  run "2026 request-metadata headers (Mcp-Method/Mcp-Name, -32020/-32022)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test mcp_headers_2026
  run "2026 unknown-method mapping (404 + -32601)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test error_mapping_2026
  run "2026 MRTR (input_required round trip, -32021 capability gate)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test mrtr_2026
  run "2026 Mcp-Param-* mirroring (x-mcp-header validation)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test mcp_param_2026
  run "2026 per-request log gating (logLevel opt-in)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test log_gating_2026
  run "2026 streaming wire grammar (SSE framing, ordering, JSON counterpart)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test streaming_e2e_2026
  run "2026 schema fidelity (derive/builders pipeline to the wire)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test schema_fidelity_2026
  run "2026 list pagination (cursor walks, invalid-cursor rejection)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test list_pagination_2026
  run "2026 macro-authored tool icons reach tools/list" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test tool_icons_2026
  run "2026 resource mimeType agreement (list vs read)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test resource_mime_type_2026
  # Spec-neutral test infrastructure: closes the reserve->bind window that let
  # two suites be handed the same ephemeral port under whole-workspace runs.
  run "test port handoff (no reservation overlap)" \
    cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test port_handoff
  run "protocol-2026 compliance + upstream wire fixtures" \
    cargo test -p turul-mcp-protocol-2026-07-28 --features compliance
  run "shipped crate docs agree with the artifacts they describe" \
    cargo test -p turul-mcp-protocol-2026-07-28 --features compliance --test docs_consistency
  run "bilingual client (not in default-members)" cargo test -p turul-mcp-client
  run "2026 client 404 is method-not-found, not session recovery" \
    cargo test -p turul-mcp-client --test unknown_method_404_is_not_session_recovery
  run "2026 client example (pairs with minimal-server)" cargo build -p streamable-http-client
  run "client-using examples (not in default-members)" cargo build -p mrtr-elicitation-server -p bilingual-fleet-client -p ext-tasks-server --bins
  run "Tasks extension (SEP-2663, opt-in feature)" cargo test -p turul-mcp-server --no-default-features --features http,sse,ext-tasks --test ext_tasks_2026
  run "ext crates standalone" cargo test -p turul-mcp-ext-tasks -p turul-mcp-ext-apps
  run "Tasks extension client e2e" cargo test -p turul-mcp-client --features ext-tasks --test ext_tasks_e2e_2026
}

gate_opt_in_2025() {
  run "server 2025-11-25 test"   cargo test   -p turul-mcp-server      --no-default-features --features http,sse,protocol-2025-11-25
  run "server 2025-11-25 clippy" cargo clippy -p turul-mcp-server      --no-default-features --features http,sse,protocol-2025-11-25 -- -D warnings
  run "builders 2025-11-25"      cargo test   -p turul-mcp-builders    --no-default-features --features protocol-2025-11-25
  run "http-server 2025-11-25"   cargo test   -p turul-http-mcp-server --no-default-features --features sse,protocol-2025-11-25
  # build-only on purpose: turul-mcp-derive dev-depends on turul-mcp-server,
  # whose 2026 default unifies both protocol features and trips the alias mutex.
  run "derive 2025-11-25"        cargo build  -p turul-mcp-derive      --no-default-features --features protocol-2025-11-25
  run "lambda 2025-11-25"        cargo test   -p turul-mcp-aws-lambda  --no-default-features --features cors,sse,protocol-2025-11-25
  # Without `cors` too — the cors-enabled run masks a non-cfg-gated `cors_config` use.
  run "lambda 2025-11-25 no-cors" cargo clippy -p turul-mcp-aws-lambda  --no-default-features --features protocol-2025-11-25 --all-targets -- -D warnings
  # Tests, not builds. These two feature sets select different #[cfg] dispatch
  # arms from the bilingual default, so compiling them proved nothing about the
  # code a single-spec consumer actually runs.
  run "client 2025-11-25-only"   cargo test   -p turul-mcp-client      --no-default-features --features http,sse,client-2025-11-25-only
  run "client 2026-07-28-only"   cargo test   -p turul-mcp-client      --no-default-features --features http,sse,client-2026-07-28-only
  run "client-initialise-server" cargo build  -p client-initialise-server --no-default-features
  # Every example whose manifest pins protocol-2025-11-25. They cannot join
  # [default-members] — unifying their features with the 2026 default trips the
  # spec mutex — so this is the only thing that compiles them. tests/examples_guard.rs
  # fails if an example is in neither place.
  run "2025-11-25 lane examples" cargo build --bins \
    -p dynamic-tools-server -p elicitation-server -p lambda-turul-mcp-server \
    -p logging-test-client -p logging-test-server -p prompts-test-server \
    -p resource-test-server -p roots-server -p sampling-server \
    -p session-aware-resource-server -p session-logging-proof-test \
    -p stateful-server -p tasks-e2e-inmemory-client -p tasks-e2e-inmemory-server \
    -p tools-test-server
  run "tools E2E"       cargo test -p mcp-tools-tests
  run "resources E2E"   cargo test -p mcp-resources-tests
  run "prompts E2E"     cargo test -p mcp-prompts-tests
  run "roots E2E"       cargo test -p mcp-roots-tests
  run "sampling E2E"    cargo test -p mcp-sampling-tests
  run "elicitation E2E" cargo test -p mcp-elicitation-tests
  run "tasks E2E"       cargo test -p turul-mcp-framework-integration-tests --test tasks_e2e_inmemory
  run "ping auth E2E"   cargo test -p turul-mcp-framework-integration-tests --test ping_auth_2025
  # Every [[test]] target in tests/Cargo.toml must appear here; a target with no
  # line below is invisible to CI.
  for t in compliance schema_tests example_validation e2e_tests feature_tests \
           session_context_macro_tests derive_comprehensive_tool_tests \
           derive_schemars_integration_test derive_zero_config_output_schema_test \
           dynamic_tools_e2e event_dispatcher_persistence client_integration_test examples_guard \
           mcp_runtime_capabilities_validation streamable_http_behavior_regression \
           reachability_guard; do
    run "integration:$t" cargo test -p turul-mcp-framework-integration-tests --test "$t"
  done
}

# Lambda, end to end through the real Runtime API rather than an in-process
# handler call. Separate from the default gate because it needs cargo-lambda and
# builds the function before it can answer.
gate_lambda() {
  run "2026 Lambda E2E (cargo lambda watch, real Runtime API)" ./scripts/e2e-lambda-local.sh
  # The 2025-11-25 lane is a separate script, not a flag: its contract is the
  # session handshake 2026 removed, so the assertions share no shape with the
  # 2026 ones beyond the boot loop. It builds the same binary with a different
  # feature set.
  run "2025-11-25 Lambda E2E (cargo lambda watch, real Runtime API)" ./scripts/e2e-lambda-local-2025-11-25.sh
  # The other direction: our own client driven against a Lambda-hosted server
  # on both specs. The Lambda transport reassembles responses differently from
  # the plain HTTP path, so a client-side parser assumption shows up here.
  run "client-over-Lambda E2E (turul-mcp-client via cargo lambda watch, both specs)" ./scripts/e2e-lambda-client-local.sh
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
  # The derive doctests gated to the opt-in lane are `rust,ignore` under the
  # default build, so the run above never type-checks them.
  echo "=== rustdoc: 2025-11-25 lane derive examples ==="
  if RUSTDOCFLAGS="-D warnings" cargo doc -p turul-mcp-derive --no-deps \
       --no-default-features --features protocol-2025-11-25; then
    echo "  PASS"
  else
    echo "  FAIL"; fail=1
  fi
}

# Runnable example servers, verified end to end over the real HTTP wire
# (not just compiled) — the shell-level counterpart to gate_default's
# in-process Rust suites. Split from gate_default because it shells out to
# `cargo build -p <example>` + a live server per script rather than running
# under `cargo test`.
gate_examples() {
  # Strict-superset orchestrator: calculator progression, resource servers,
  # prompts/special-features, session storage backends, advanced/composite
  # servers, and client/test utilities (6 phases, each mixing the 2026-07-28
  # default lane with protocol-2025-11-25-pinned examples where those still
  # exist). Lambda examples are gated separately below and in gate_lambda;
  # they were dropped from this orchestrator's phase list as superseded.
  run "example servers (calculator/resource/prompts/storage/advanced/client)" \
    ./scripts/verify_all_examples_unattended.sh
  # middleware-rate-limit-server: pre-session stateless rate limiting,
  # asserts the 6th request gets -32003 (RateLimitExceeded).
  run "example: rate-limit middleware (-32003 on the 6th request)" \
    ./scripts/test_rate_limit.sh
  # middleware-logging-server / middleware-rate-limit-server /
  # middleware-auth-server, each on its own port: init, rate limit,
  # X-API-Key auth gate (-32001 unauthenticated, whoami with a valid key).
  run "example: logging/rate-limit/auth middleware (live)" \
    ./scripts/test_middleware_live.sh
  # middleware-auth-lambda: cargo-lambda cross-target build (debug + release).
  run "example: Lambda auth middleware (cargo-lambda build)" \
    ./scripts/test_lambda_middleware.sh
  # middleware-auth-lambda: cargo-lambda watch, real Lambda Runtime API
  # emulator. Auth-gate assertions (-32001 without a key, whoami with one)
  # run when DynamoDB is reachable; otherwise this reports its own SKIPPED
  # and still exits 0 — there is no AWS/DynamoDB in this sandbox to fake.
  run "example: Lambda auth middleware (cargo-lambda watch, live)" \
    ./scripts/test_lambda_middleware_live.sh
}

case "${1:-all}" in
  default)      gate_default ;;
  opt-in-2025)  gate_opt_in_2025 ;;
  lambda)       gate_lambda ;;
  mutex)        gate_mutex ;;
  docs)         gate_docs ;;
  examples)     gate_examples ;;
  all)          gate_default; gate_opt_in_2025; gate_lambda; gate_mutex; gate_docs; gate_examples ;;
  *) echo "usage: $0 [default|opt-in-2025|lambda|mutex|docs|examples|all]"; exit 2 ;;
esac

echo; [ "$fail" = "0" ] && echo "ALL GATES PASSED" || echo "ONE OR MORE GATES FAILED"
exit "$fail"
