//! Reachability guard for the `tests/` tree.
//!
//! Two silent-gap conditions are caught here.
//!
//! `autotests = false` means a file under `tests/` only runs if it is an
//! explicit `[[test]]` target or pulled in via `#[path]` by one of the
//! consolidated binaries in `tests/consolidated/`. Neither the Slice
//! Completion Gate's content/name greps nor `cargo test` itself notice a file
//! that satisfies neither — it silently stops compiling and running while
//! still looking like coverage.
//!
//! A `[[bin]]` declared in a nested crate under `tests/` is worse: it compiles
//! on every build and looks like a test server, but nothing launches it unless
//! `TestServerManager` names it. The harness spawns servers by binary name from
//! `tests/shared/src/e2e_utils.rs`, so a name absent from that file is a
//! duplicate of an `examples/` server that only drifts from the copy actually
//! under test.

use std::collections::BTreeSet;
use std::path::Path;

/// The only mechanism that launches a binary declared by a nested test crate.
const HARNESS_SOURCE: &str = "shared/src/e2e_utils.rs";

const CARGO_TOML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// Extract every `path = "..."` value from `[[test]]` blocks in `tests/Cargo.toml`.
fn declared_test_paths() -> Vec<String> {
    let mut in_test_block = false;
    let mut paths = Vec::new();

    for line in CARGO_TOML.lines() {
        let trimmed = line.trim();
        if trimmed == "[[test]]" {
            in_test_block = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_test_block = false;
            continue;
        }
        if in_test_block && let Some(rest) = trimmed.strip_prefix("path") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim().trim_matches('"');
                paths.push(value.to_string());
            }
        }
    }

    paths
}

/// Extract every `#[path = "../X.rs"]` target from a consolidated binary's source.
fn path_mod_targets(source: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#[path") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=')
                && let Some(quote_start) = rest.find('"')
            {
                let after = &rest[quote_start + 1..];
                if let Some(quote_end) = after.find('"') {
                    targets.push(after[..quote_end].to_string());
                }
            }
        }
    }
    targets
}

/// The set of `tests/*.rs` basenames that `cargo test` will actually compile
/// and run: direct `[[test]]` targets plus everything consolidated binaries
/// pull in via `#[path]`.
fn reachable_basenames() -> BTreeSet<String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut reachable = BTreeSet::new();

    for declared in declared_test_paths() {
        if let Some(consolidated_name) = declared.strip_prefix("consolidated/") {
            let consolidated_path = manifest_dir.join("consolidated").join(consolidated_name);
            let source = std::fs::read_to_string(&consolidated_path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", consolidated_path.display()));
            for target in path_mod_targets(&source) {
                // Targets are relative to tests/consolidated/, e.g. "../foo.rs".
                let basename = target
                    .strip_prefix("../")
                    .expect("consolidated #[path] targets must be relative to consolidated/");
                reachable.insert(basename.to_string());
            }
        } else {
            // A direct top-level [[test]] path, e.g. "ping_auth_2025.rs".
            reachable.insert(declared);
        }
    }

    reachable
}

/// Every `.rs` file directly under `tests/` (not in a subdirectory — those
/// with their own `Cargo.toml`, like `tests/shared/`, are separate workspace
/// members with normal `autotests` behavior and are out of scope here).
fn top_level_test_files() -> Vec<String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();

    for entry in std::fs::read_dir(manifest_dir).expect("failed to read tests/ directory") {
        let entry = entry.expect("failed to read tests/ directory entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }

    files
}

#[test]
fn every_top_level_test_file_is_reachable() {
    let reachable = reachable_basenames();
    let all_files = top_level_test_files();
    assert!(
        !all_files.is_empty(),
        "sanity check: no .rs files found directly under tests/ — did the layout move?"
    );

    let mut orphaned: Vec<&String> = all_files
        .iter()
        .filter(|f| !reachable.contains(*f))
        .collect();
    orphaned.sort();

    assert!(
        orphaned.is_empty(),
        "{} file(s) under tests/ are neither an explicit [[test]] target in \
         tests/Cargo.toml nor reachable via #[path] from tests/consolidated/, \
         so `cargo test` silently never compiles or runs them: {orphaned:?}. \
         Wire each one in as a [[test]] target or a consolidated #[path] mod, \
         or delete it if it is genuinely superseded.",
        orphaned.len()
    );
}

/// Extract every `name = "..."` value from `[[bin]]` blocks in a manifest.
fn declared_bin_names(manifest: &str) -> Vec<String> {
    let mut in_bin_block = false;
    let mut names = Vec::new();

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed == "[[bin]]" {
            in_bin_block = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_bin_block = false;
            continue;
        }
        if in_bin_block && let Some(rest) = trimmed.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                names.push(rest.trim().trim_matches('"').to_string());
            }
        }
    }

    names
}

/// Every `(crate directory, binary name)` declared by a nested crate under
/// `tests/` — i.e. a subdirectory carrying its own `Cargo.toml`.
fn nested_crate_bins() -> Vec<(String, String)> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut bins = Vec::new();

    for entry in std::fs::read_dir(manifest_dir).expect("failed to read tests/ directory") {
        let entry = entry.expect("failed to read tests/ directory entry");
        let manifest_path = entry.path().join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", manifest_path.display()));
        let dir = entry.file_name().to_string_lossy().into_owned();
        for name in declared_bin_names(&manifest) {
            bins.push((dir.clone(), name));
        }
    }

    bins.sort();
    bins
}

#[test]
fn every_nested_test_crate_bin_is_launched_by_the_harness() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let harness_path = manifest_dir.join(HARNESS_SOURCE);
    let harness = std::fs::read_to_string(&harness_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", harness_path.display()));

    let orphaned: Vec<String> = nested_crate_bins()
        .into_iter()
        .filter(|(_, name)| !harness.contains(name.as_str()))
        .map(|(dir, name)| format!("tests/{dir} → [[bin]] {name}"))
        .collect();

    assert!(
        orphaned.is_empty(),
        "{} binary target(s) declared by nested crates under tests/ are never \
         launched: {orphaned:?}. `TestServerManager` spawns servers by binary \
         name from tests/{HARNESS_SOURCE}, and none of these appear there, so \
         they compile on every build while nothing runs them. Wire each one \
         into the harness, or delete the [[bin]] and its source if an \
         examples/ server already supersedes it.",
        orphaned.len()
    );
}

#[test]
fn declared_test_paths_point_at_real_files() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for declared in declared_test_paths() {
        let full_path = manifest_dir.join(&declared);
        assert!(
            full_path.is_file(),
            "tests/Cargo.toml declares [[test]] path = \"{declared}\" but {} does not exist",
            full_path.display()
        );
    }
}
