#!/bin/bash
# Shared helpers for the verify_*.sh scripts: lane-aware example builds.
#
# Examples that pin `protocol-2025-11-25` in their manifest cannot join the
# default 2026-07-28 build (the spec mutex), so each lane builds with `-p` into
# its own target dir. `scripts/ci-gates.sh` ("2025-11-25 lane examples") is the
# authority for which examples are 2025.

EX_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EX_TARGET_2026="${EX_TARGET_2026:-$EX_ROOT/target}"
EX_TARGET_2025="${EX_TARGET_2025:-$EX_ROOT/target-2025}"

# example_target_dir LANE  (LANE: 2025 | 2026)
example_target_dir() {
    if [ "$1" = "2025" ]; then echo "$EX_TARGET_2025"; else echo "$EX_TARGET_2026"; fi
}

# example_bin NAME LANE -> path of the built binary
example_bin() {
    echo "$(example_target_dir "$2")/debug/$1"
}

# example_build NAME LANE -> builds `-p NAME` in the lane's target dir; on
# failure prints the tail of the build output and returns 1.
example_build() {
    local name=$1 lane=$2 out
    if ! out=$(cd "$EX_ROOT" && CARGO_TARGET_DIR="$(example_target_dir "$lane")" \
        cargo build -p "$name" --bin "$name" 2>&1); then
        echo "Build failed for $name (lane $lane):"
        echo "$out" | tail -10
        return 1
    fi
}

# port_in_use PORT -> 0 when something already listens on PORT
port_in_use() {
    ss -ltn "sport = :$1" 2>/dev/null | grep -q LISTEN
}
