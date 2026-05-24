//! Round-trip a fixture through its Rust binding and semantically diff the
//! re-serialized JSON against the upstream original.
//!
//! Semantic (not byte-equal): object field order is irrelevant, absent ==
//! `null` for optional fields, arrays are order-significant, numbers compared
//! as `f64`. The harness asserts the diff is empty for every modeled fixture.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::coverage::{Case, Kind, CASES};
use super::fetch::{self, list_example_dirs, list_json_files, Pin, PIN};

/// Single fixture's result.
#[derive(Debug)]
pub struct FixtureResult {
    pub dir: &'static str,
    pub file: PathBuf,
    pub outcome: Outcome,
}

#[derive(Debug)]
pub enum Outcome {
    /// Round-trip matched semantically.
    Pass,
    /// Round-trip diff was non-empty — modeled binding does not match upstream.
    Diff(String),
    /// Parse failed.
    ParseError(String),
    /// Re-serialize failed (extremely unusual — would indicate a non-serde-friendly type).
    SerializeError(String),
}

/// Aggregated report for one harness run.
#[derive(Debug, Default)]
pub struct Report {
    pub modeled: usize,
    pub not_modeled: usize,
    pub passed: usize,
    pub failed: Vec<FixtureResult>,
    pub total_files: usize,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Run the harness end-to-end: ensure fixtures, walk CASES, return a Report.
/// Used by both `tests/upstream_fixtures.rs` and `src/bin/compliance.rs` —
/// identical code path is the bidirectional guarantee.
pub fn run_all(dest: &Path) -> Result<Report, super::fetch::FetchError> {
    run_all_with_pin(dest, &PIN)
}

pub fn run_all_with_pin(dest: &Path, pin: &Pin) -> Result<Report, super::fetch::FetchError> {
    let examples_dir = fetch::ensure_fixtures(dest, pin)?;
    let mut report = Report::default();

    for case in CASES {
        match case.kind {
            Kind::NotModeled => {
                report.not_modeled += 1;
                continue;
            }
            _ => report.modeled += 1,
        }
        let case_dir = examples_dir.join(case.dir);
        let files = list_json_files(&case_dir)?;
        for file in files {
            report.total_files += 1;
            let raw = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    report.failed.push(FixtureResult {
                        dir: case.dir,
                        file,
                        outcome: Outcome::ParseError(format!("read: {e}")),
                    });
                    continue;
                }
            };
            match check_fixture(case, &raw) {
                Outcome::Pass => report.passed += 1,
                other => report.failed.push(FixtureResult {
                    dir: case.dir,
                    file,
                    outcome: other,
                }),
            }
        }
    }
    Ok(report)
}

/// Verify the table is in sync with the fetched tree — exactly 86 entries,
/// one per upstream example directory, no extras, no missing.
pub fn assert_table_matches_upstream(dest: &Path) -> Result<(), String> {
    let examples_dir = fetch::ensure_fixtures(dest, &PIN).map_err(|e| e.to_string())?;
    let upstream = list_example_dirs(&examples_dir).map_err(|e| e.to_string())?;
    let table: Vec<String> = CASES.iter().map(|c| c.dir.to_string()).collect();

    let upstream_set: std::collections::BTreeSet<_> = upstream.iter().collect();
    let table_set: std::collections::BTreeSet<_> = table.iter().collect();

    let missing: Vec<_> = upstream_set.difference(&table_set).collect();
    let extra: Vec<_> = table_set.difference(&upstream_set).collect();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(format!(
            "coverage table out of sync with upstream tree: missing={missing:?} extra={extra:?}"
        ));
    }
    if table.len() != 86 {
        return Err(format!(
            "coverage table size {} != expected 86 (upstream has {})",
            table.len(),
            upstream.len()
        ));
    }
    Ok(())
}

fn check_fixture(case: &Case, raw: &str) -> Outcome {
    let upstream: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => return Outcome::ParseError(format!("upstream JSON: {e}")),
    };
    let reparsed = match (case.parse_and_reserialize)(raw) {
        Ok(v) => v,
        Err(e) => return Outcome::ParseError(e),
    };
    match semantic_diff(&upstream, &reparsed) {
        None => Outcome::Pass,
        Some(diff) => Outcome::Diff(diff),
    }
}

/// Compare two JSON values semantically. Returns `None` on match, or a
/// human-readable description of the first divergence.
pub fn semantic_diff(a: &Value, b: &Value) -> Option<String> {
    semantic_diff_at("$", a, b)
}

fn semantic_diff_at(path: &str, a: &Value, b: &Value) -> Option<String> {
    match (a, b) {
        (Value::Null, Value::Null) => None,
        (Value::Bool(x), Value::Bool(y)) if x == y => None,
        (Value::Number(x), Value::Number(y)) => {
            let xf = x.as_f64();
            let yf = y.as_f64();
            if xf == yf {
                None
            } else {
                Some(format!("{path}: number {x} != {y}"))
            }
        }
        (Value::String(x), Value::String(y)) if x == y => None,
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: array length {} != {}", x.len(), y.len()));
            }
            for (i, (xv, yv)) in x.iter().zip(y.iter()).enumerate() {
                if let Some(d) = semantic_diff_at(&format!("{path}[{i}]"), xv, yv) {
                    return Some(d);
                }
            }
            None
        }
        (Value::Object(x), Value::Object(y)) => {
            let xk: std::collections::BTreeSet<_> = x.keys().collect();
            let yk: std::collections::BTreeSet<_> = y.keys().collect();
            let missing_in_b: Vec<_> = xk.difference(&yk).collect();
            let extra_in_b: Vec<_> = yk.difference(&xk).collect();
            if !missing_in_b.is_empty() {
                return Some(format!(
                    "{path}: re-serialized output is missing upstream keys {missing_in_b:?}"
                ));
            }
            if !extra_in_b.is_empty() {
                return Some(format!(
                    "{path}: re-serialized output added extra keys {extra_in_b:?}"
                ));
            }
            for k in xk {
                let p = format!("{path}.{k}");
                if let Some(d) = semantic_diff_at(&p, &x[k], &y[k]) {
                    return Some(d);
                }
            }
            None
        }
        _ => Some(format!("{path}: type/value mismatch: {a} != {b}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn diff_identical_objects_returns_none() {
        let a = json!({"a": 1, "b": "x"});
        let b = json!({"b": "x", "a": 1});
        assert!(semantic_diff(&a, &b).is_none());
    }

    #[test]
    fn diff_extra_field_is_reported() {
        let a = json!({"a": 1});
        let b = json!({"a": 1, "extra": true});
        let d = semantic_diff(&a, &b).unwrap();
        assert!(d.contains("extra"));
    }

    #[test]
    fn diff_missing_field_is_reported() {
        let a = json!({"a": 1, "needed": true});
        let b = json!({"a": 1});
        let d = semantic_diff(&a, &b).unwrap();
        assert!(d.contains("needed"));
    }

    #[test]
    fn diff_array_order_matters() {
        let a = json!([1, 2, 3]);
        let b = json!([1, 3, 2]);
        assert!(semantic_diff(&a, &b).is_some());
    }

    #[test]
    fn diff_number_compared_as_f64() {
        let a = json!(1.0);
        let b = json!(1);
        assert!(semantic_diff(&a, &b).is_none());
    }
}
