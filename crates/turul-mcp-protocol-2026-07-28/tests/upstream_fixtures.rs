//! Build-time wire-format compliance gate.
//!
//! Calls the same [`compliance::roundtrip::run_all`] as the CLI binary, so
//! a green test guarantees a green `mcp-compliance-2026-07-28` on the same
//! host and pin. Network is required ONCE (first run) to populate the cache
//! at `target/upstream-fixtures/`; subsequent runs are offline.

use std::path::PathBuf;
use std::sync::OnceLock;

use turul_mcp_protocol_2026_07_28::compliance::fetch::{ensure_fixtures, PIN};
use turul_mcp_protocol_2026_07_28::compliance::roundtrip;

/// Per-test cache directory rooted in `target/` so it survives across runs
/// but is wiped by `cargo clean`. Distinct from the binary's `$TMPDIR` cache.
fn test_cache_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("upstream-fixtures");
    p
}

/// Serialize the fetch across concurrent test threads — `cargo test` runs
/// tests in parallel by default and `git init` is not concurrency-safe on a
/// shared cache directory. The first caller fetches; subsequent callers see
/// a populated cache and `ensure_fixtures` short-circuits on the .pin marker.
fn ensure_once() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        ensure_fixtures(&test_cache_dir(), &PIN).expect("fetch pinned upstream fixtures");
    });
}

/// Coverage floor — modeled count must not regress. Bumped per migration batch.
/// Current modeled set (8): `Tool`, `CallToolRequestParams`, `CallToolResult`,
/// `ListToolsResult`, `Resource`, `Root`, `ListRootsResult`, `ElicitResult` —
/// all round-trip cleanly against every upstream fixture. Raise this in
/// subsequent slices as more bindings flip from `NotModeled` to a real `Kind`.
const COVERAGE_FLOOR: usize = 8;

#[test]
fn coverage_table_matches_upstream() {
    ensure_once();
    let dest = test_cache_dir();
    if let Err(e) = roundtrip::assert_table_matches_upstream(&dest) {
        panic!("coverage table out of sync with upstream pin: {e}");
    }
}

#[test]
fn coverage_floor_holds() {
    ensure_once();
    let dest = test_cache_dir();
    let report = roundtrip::run_all(&dest).expect("fetch + run_all");
    assert!(
        report.modeled >= COVERAGE_FLOOR,
        "coverage regressed: modeled = {} < floor = {COVERAGE_FLOOR}",
        report.modeled
    );
}

#[test]
fn all_modeled_fixtures_round_trip_cleanly() {
    ensure_once();
    let dest = test_cache_dir();
    let report = roundtrip::run_all(&dest).expect("fetch + run_all");

    if !report.is_clean() {
        let mut msg = format!(
            "{} fixture(s) failed compliance:\n",
            report.failed.len()
        );
        for f in &report.failed {
            msg.push_str(&format!(
                "  - {}/{:?}: {:?}\n",
                f.dir,
                f.file.file_name().unwrap_or_default(),
                f.outcome
            ));
        }
        panic!("{msg}");
    }
}
