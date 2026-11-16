#!/bin/bash
# cargo_target_dir -> the directory cargo builds into, as cargo itself resolves
# it (CARGO_TARGET_DIR, any [build] target-dir, else <workspace>/target).
# Scripts locate built binaries through this, never through ./target.
cargo_target_dir() {
    cargo metadata --format-version 1 --no-deps \
        | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
}
