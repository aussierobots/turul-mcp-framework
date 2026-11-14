//! ADR-036: every `HashMap` field of a wire type serializes in sorted key
//! order. A source scan, so a new field cannot skip `serde_maps` unnoticed.

use std::path::Path;

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A struct field whose type is `HashMap<..>` or `Option<HashMap<..>>`.
fn is_map_field(line: &str) -> bool {
    let t = line.trim_start();
    let t = t
        .strip_prefix("pub(crate) ")
        .or(t.strip_prefix("pub "))
        .unwrap_or(t);
    let Some((name, ty)) = t.split_once(':') else {
        return false;
    };
    let ty = ty.trim_start();
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && (ty.starts_with("HashMap<")
            || ty.starts_with("Option<HashMap<")
            || ty.starts_with("std::collections::HashMap<"))
        && t.trim_end().ends_with(',')
}

/// True when the nearest enclosing item above `i` is a `fn` (a parameter),
/// not a `struct`/`enum` (a field).
fn in_fn_signature(lines: &[&str], i: usize) -> bool {
    lines[..i]
        .iter()
        .rev()
        .find_map(|l| {
            let t = l.trim_start();
            if t.contains("fn ") {
                Some(true)
            } else if t.contains("struct ") || t.contains("enum ") {
                Some(false)
            } else {
                None
            }
        })
        .unwrap_or(false)
}

#[test]
fn every_wire_hashmap_field_is_sorted() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut unsorted = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !is_map_field(line) || in_fn_signature(&lines, i) {
                continue;
            }
            // Attributes and docs directly above the field.
            let mut k = i;
            let mut annotated = false;
            while k > 0 {
                k -= 1;
                let above = lines[k].trim_start();
                if !(above.starts_with("#[")
                    || above.starts_with("//")
                    || above.starts_with(')')
                    || above.starts_with('"')
                    || above.starts_with("skip")
                    || above.starts_with("rename")
                    || above.starts_with("default")
                    || above.starts_with("deserialize_with"))
                {
                    break;
                }
                annotated |= above.contains("serde_maps::");
            }
            // Fields of a struct with a hand-written `Serialize` must sort
            // through `serde_maps::sorted_entries` instead; allow those.
            if !annotated && !text.contains("serde_maps::sorted_entries(&self.") {
                unsorted.push(format!("{}:{}", file.display(), i + 1));
            }
        }
    }
    assert!(
        unsorted.is_empty(),
        "HashMap fields without serde_maps: {unsorted:?}"
    );
}
