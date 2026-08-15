#!/usr/bin/env bash
# Run the Tasks-extension suites against EVERY storage backend.
#
# This gate exists because the superseded 2025 task store shipped SQLite,
# Postgres and DynamoDB backends that no test ever executed and no gate ever
# built — ~4,600 lines whose only evidence of correctness was that nobody had
# complained. The 2026 extension keeps its four backends honest by running the
# same parity contract against all of them, and this script is what makes that
# happen on every push rather than when someone remembers.
#
# Backends are NOT optional here. An unreachable service fails the gate rather
# than skipping, because a skip that reports green is the exact failure mode
# being prevented. Set TURUL_SKIP_PG_TESTS / TURUL_SKIP_DDB_TESTS to opt out
# deliberately on a machine that genuinely cannot run them.
set -uo pipefail
cd "$(dirname "$0")/.."

DDB_PORT="${TURUL_TEST_DDB_PORT:-8123}"
DDB_HOME="${TURUL_DDB_HOME:-$HOME/.cache/turul/dynamodb-local}"
DDB_PID=""
fail=0

note() { printf '%-52s %s\n' "$1" "$2"; }
bad()  { note "$1" "FAIL — $2"; fail=1; }

cleanup() {
    [[ -n "$DDB_PID" ]] && kill "$DDB_PID" 2>/dev/null
    return 0
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Postgres — expected to be reachable; the tests provision their own scratch
# database per run and drop it afterwards.
# ---------------------------------------------------------------------------
if [[ -n "${TURUL_SKIP_PG_TESTS:-}" ]]; then
    note "postgres" "SKIPPED (TURUL_SKIP_PG_TESTS set)"
else
    PG_BASE="${TURUL_TEST_PG_URL:-postgres://$(id -un)@%2Fvar%2Frun%2Fpostgresql}"
    export TURUL_TEST_PG_URL="$PG_BASE"
    # `pg_isready` rather than parsing the sqlx URL: the default form encodes
    # a unix socket path as %2F, which psql does not accept back.
    if pg_isready -q 2>/dev/null; then
        note "postgres reachable" "OK"
    else
        bad "postgres reachable" "no server — start it, set TURUL_TEST_PG_URL, or TURUL_SKIP_PG_TESTS=1"
    fi
fi

# ---------------------------------------------------------------------------
# DynamoDB Local — a plain jar, deliberately NOT docker. Fetched once into a
# cache directory and started for the duration of this script.
# ---------------------------------------------------------------------------
if [[ -n "${TURUL_SKIP_DDB_TESTS:-}" ]]; then
    note "dynamodb" "SKIPPED (TURUL_SKIP_DDB_TESTS set)"
elif ! command -v java >/dev/null; then
    bad "dynamodb java" "java not found — needed for DynamoDB Local (no docker), or set TURUL_SKIP_DDB_TESTS=1"
else
    if [[ ! -f "$DDB_HOME/DynamoDBLocal.jar" ]]; then
        note "dynamodb local" "fetching into $DDB_HOME"
        mkdir -p "$DDB_HOME"
        if ! curl -sSL --max-time 180 \
            https://s3.us-west-2.amazonaws.com/dynamodb-local/dynamodb_local_latest.tar.gz \
            | tar xz -C "$DDB_HOME"; then
            bad "dynamodb local" "download failed"
        fi
    fi
    if [[ -f "$DDB_HOME/DynamoDBLocal.jar" ]]; then
        (cd "$DDB_HOME" && java -Djava.library.path=./DynamoDBLocal_lib \
            -jar DynamoDBLocal.jar -inMemory -port "$DDB_PORT" >/dev/null 2>&1) &
        DDB_PID=$!
        export TURUL_TEST_DDB_URL="http://127.0.0.1:${DDB_PORT}"
        for _ in $(seq 1 40); do
            curl -sS -o /dev/null "http://127.0.0.1:${DDB_PORT}/" 2>/dev/null && break
            sleep 0.25
        done
        if curl -sS -o /dev/null "http://127.0.0.1:${DDB_PORT}/" 2>/dev/null; then
            note "dynamodb local on :$DDB_PORT" "OK"
        else
            bad "dynamodb local on :$DDB_PORT" "did not come up"
        fi
    fi
fi

[[ "$fail" -ne 0 ]] && { echo; echo "BACKEND PREREQUISITES FAILED"; exit 1; }

# ---------------------------------------------------------------------------
# The suites. The parity contract runs inside the first; the client e2e drives
# all four backends over real HTTP.
# ---------------------------------------------------------------------------
run() {
    local label="$1"; shift
    if "$@" >/tmp/turul-backends-$$.log 2>&1; then
        note "$label" "PASS"
    else
        bad "$label" "see output below"
        tail -40 /tmp/turul-backends-$$.log
    fi
    rm -f /tmp/turul-backends-$$.log
}

run "parity contract × 4 backends" \
    cargo test -p turul-mcp-ext-tasks --features sqlite,postgres,dynamodb
# Lives in the publish=false crate `tests-ext-tasks/`, not in turul-mcp-client:
# it links sqlx and aws-sdk-dynamodb, which have no place in a client library's
# dependency graph.
run "client e2e × 4 backends" \
    cargo test -p turul-ext-tasks-backend-e2e
run "server wire suite (ext-tasks)" \
    cargo test -p turul-mcp-server --features ext-tasks --test ext_tasks_2026

echo
if [[ "$fail" -eq 0 ]]; then
    echo "ALL TASK BACKENDS PASSED"
else
    echo "TASK BACKEND GATE FAILED"
fi
exit "$fail"
