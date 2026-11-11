//! Generated code must name crate roots through `utils::*_path()` only.
//!
//! Source scan: skips comment lines and `use` lines, and cannot see paths built
//! at runtime by string concatenation. `utils.rs` owns the literals.

use std::path::Path;

const FORBIDDEN: [&str; 3] = [
    "turul_mcp_protocol::",
    "turul_mcp_builders::",
    "serde_json::",
];

fn scan(dir: &Path, hits: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scan(&path, hits);
        } else if path.extension().is_some_and(|e| e == "rs")
            && path.file_name().is_some_and(|n| n != "utils.rs")
        {
            for (i, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") || t.starts_with("use ") {
                    continue;
                }
                if FORBIDDEN.iter().any(|p| {
                    line.match_indices(p)
                        .any(|(at, _)| !line[..at].ends_with("::") && !line[..at].ends_with('#'))
                }) {
                    hits.push(format!("{}:{}: {}", path.display(), i + 1, t));
                }
            }
        }
    }
}

#[test]
fn no_literal_crate_roots_in_generated_code() {
    let mut hits = Vec::new();
    scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut hits,
    );
    assert!(hits.is_empty(), "literal crate roots:\n{}", hits.join("\n"));
}
