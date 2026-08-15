//! Reachability guard for `examples/`.
//!
//! An example directory that nothing builds and nothing documents still looks
//! like a working example to anyone browsing the tree — it rots silently until
//! someone tries to run it. Three independent registries have to agree, and
//! none of them notices when one drifts:
//!
//!   * `[workspace.members]` in the root `Cargo.toml` — is it part of the
//!     workspace at all?
//!   * `[default-members]`, or a `-p <package>` on a gate line in
//!     `scripts/ci-gates.sh` — does anything ever compile it?
//!   * `EXAMPLES.md` — can a reader find it?
//!
//! These tests make each disagreement a hard failure. Note the unit of
//! identity differs per registry: Cargo lists *directories*, `ci-gates.sh`
//! names *packages* (`examples/lambda-mcp-client` builds as
//! `lambda-turul-mcp-client`), and `EXAMPLES.md` is keyed on the directory
//! name, since that is what a reader has in front of them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests/ must have a parent")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Directory name -> Cargo package name, for every `examples/*/` that carries a
/// manifest.
fn example_packages() -> BTreeMap<String, String> {
    let examples = repo_root().join("examples");
    let mut found = BTreeMap::new();

    for entry in std::fs::read_dir(&examples).expect("failed to read examples/") {
        let entry = entry.expect("failed to read an examples/ entry");
        let path = entry.path();
        let manifest = path.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let dir = path
            .file_name()
            .expect("directory entry must have a name")
            .to_string_lossy()
            .into_owned();
        let manifest_text = std::fs::read_to_string(&manifest)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", manifest.display()));
        let name = package_name(&manifest_text)
            .unwrap_or_else(|| panic!("{} has no [package] name", manifest.display()));
        found.insert(dir, name);
    }

    found
}

fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package
            && let Some(rest) = trimmed.strip_prefix("name")
            && let Some(rest) = rest.trim_start().strip_prefix('=')
        {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// The `"examples/..."` entries of a top-level array in the root manifest.
/// `key` is matched at the start of a line, so `members` does not also capture
/// `default-members`.
fn root_manifest_example_list(key: &str) -> BTreeSet<String> {
    let manifest = read("Cargo.toml");
    let mut inside = false;
    let mut found = BTreeSet::new();

    for line in manifest.lines() {
        if !inside {
            let head = line.trim_end();
            if head.starts_with(key) && head[key.len()..].trim_start().starts_with('=') {
                inside = true;
            }
            continue;
        }
        if line.starts_with(']') {
            break;
        }
        if let Some(entry) = quoted(line)
            && let Some(dir) = entry.strip_prefix("examples/")
        {
            found.insert(dir.to_string());
        }
    }

    assert!(
        !found.is_empty(),
        "no `examples/*` entries parsed out of `{key}` in the root Cargo.toml — \
         did the manifest layout change?"
    );
    found
}

fn quoted(line: &str) -> Option<&str> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Every package named by a `-p <name>` argument anywhere in `ci-gates.sh`.
fn ci_gated_packages() -> BTreeSet<String> {
    let script = read("scripts/ci-gates.sh");
    let mut tokens = script.split_whitespace();
    let mut gated = BTreeSet::new();

    while let Some(token) = tokens.next() {
        if token == "-p"
            && let Some(name) = tokens.next()
        {
            gated.insert(name.to_string());
        }
    }

    assert!(
        !gated.is_empty(),
        "no `-p <package>` arguments parsed out of scripts/ci-gates.sh — \
         did the gate script change shape?"
    );
    gated
}

/// True when `needle` appears in `haystack` not glued to a longer identifier —
/// so `resource-server` does not match inside `session-aware-resource-server`.
fn mentions_name(haystack: &str, needle: &str) -> bool {
    let is_name_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before_ok = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_name_char(c));
        let after_ok = haystack[end..]
            .chars()
            .next()
            .is_none_or(|c| !is_name_char(c));
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// The bolded first cell of every `EXAMPLES.md` table row — the canonical name
/// column, e.g. `| **minimal-server** | ...`.
fn examples_md_row_names() -> BTreeSet<String> {
    let doc = read("EXAMPLES.md");
    let mut names = BTreeSet::new();

    for line in doc.lines() {
        let trimmed = line.trim_start();
        let Some(row) = trimmed.strip_prefix('|') else {
            continue;
        };
        let Some(first_cell) = row.split('|').next() else {
            continue;
        };
        let cell = first_cell.trim();
        let Some(bold) = cell.strip_prefix("**").and_then(|s| s.split("**").next()) else {
            continue;
        };
        // Kebab-case with at least one letter: excludes the lane column's
        // bolded spec dates (`**2026-07-28**`), which are not example names.
        let looks_like_a_package = bold.chars().any(|c| c.is_ascii_lowercase())
            && bold
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if looks_like_a_package {
            names.insert(bold.to_string());
        }
    }

    names
}

#[test]
fn every_example_directory_is_a_workspace_member() {
    let members = root_manifest_example_list("members");
    let mut missing: Vec<String> = example_packages()
        .into_keys()
        .filter(|dir| !members.contains(dir))
        .collect();
    missing.sort();

    assert!(
        missing.is_empty(),
        "{} example director(ies) exist but are not in `[workspace.members]`, so \
         no cargo invocation can reach them: {missing:?}",
        missing.len()
    );
}

#[test]
fn every_example_is_documented_in_examples_md() {
    let doc = read("EXAMPLES.md");
    let mut undocumented: Vec<String> = example_packages()
        .into_keys()
        .filter(|dir| !mentions_name(&doc, dir))
        .collect();
    undocumented.sort();

    assert!(
        undocumented.is_empty(),
        "{} example(s) are absent from EXAMPLES.md, so a reader browsing the docs \
         never learns they exist: {undocumented:?}. Add a row naming each one, or \
         delete the directory if it is superseded.",
        undocumented.len()
    );
}

/// Directory names under `examples/archived/`. These are retired examples: the
/// workspace `exclude` list keeps them out of every build, so they deliberately
/// do NOT appear in [`example_packages`] and are not required to be gated. They
/// are still in the tree, though, so EXAMPLES.md §Archived may name them.
fn archived_examples() -> BTreeSet<String> {
    let archived = repo_root().join("examples").join("archived");
    let Ok(entries) = std::fs::read_dir(&archived) else {
        return BTreeSet::new(); // no archive directory is fine
    };
    entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if !path.join("Cargo.toml").is_file() {
                return None;
            }
            Some(path.file_name()?.to_string_lossy().into_owned())
        })
        .collect()
}

#[test]
fn examples_md_names_no_example_that_no_longer_exists() {
    let mut dirs: BTreeSet<String> = example_packages().into_keys().collect();
    // An archived example still exists — it is excluded from the build, not
    // deleted — so a §Archived row naming one is correct, not stale.
    dirs.extend(archived_examples());
    let mut stale: Vec<String> = examples_md_row_names()
        .into_iter()
        .filter(|name| !dirs.contains(name))
        .collect();
    stale.sort();

    assert!(
        stale.is_empty(),
        "EXAMPLES.md has table row(s) for {} example(s) that are in neither \
         examples/ nor examples/archived/: {stale:?}. Remove the row in the same \
         change that removes the directory.",
        stale.len()
    );
}

#[test]
fn every_example_is_built_by_default_members_or_a_ci_gate() {
    let default_members = root_manifest_example_list("default-members");
    let gated = ci_gated_packages();

    let mut ungated: Vec<String> = example_packages()
        .into_iter()
        .filter(|(dir, package)| !default_members.contains(dir) && !gated.contains(package))
        .map(|(dir, package)| {
            if dir == package {
                dir
            } else {
                format!("{dir} (package {package})")
            }
        })
        .collect();
    ungated.sort();

    assert!(
        ungated.is_empty(),
        "{} example(s) are in [workspace.members] but neither in [default-members] \
         nor named on a `-p` gate line in scripts/ci-gates.sh, so nothing ever \
         compiles them and they rot silently: {ungated:?}. Add each to \
         [default-members] if it builds on the 2026-07-28 default lane, or to a \
         gate line if its manifest pins protocol-2025-11-25.",
        ungated.len()
    );
}

#[test]
fn default_members_examples_all_exist() {
    let root = repo_root();
    for dir in root_manifest_example_list("default-members") {
        let path = root.join("examples").join(&dir);
        assert!(
            path.is_dir(),
            "root Cargo.toml lists `examples/{dir}` in [default-members] but {} \
             does not exist",
            path.display()
        );
    }
}
