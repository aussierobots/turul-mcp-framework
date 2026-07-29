//! The crate's own documentation ships to crates.io and docs.rs, so a number
//! that drifts in `COMPLIANCE.md` is a false claim delivered to consumers, not
//! an internal untidiness.
//!
//! Every count stated in prose here was wrong at some point: the `@see` table
//! said 8 when the schema carried 13, and the fixture tree was described as 86
//! directories while `coverage.rs` asserted 88 in the same crate. These tests
//! recompute each figure from the artifact it describes, so prose and owner
//! cannot disagree silently.

const COMPLIANCE_MD: &str = include_str!("../COMPLIANCE.md");
const SCHEMA_TS: &str = include_str!("../schema/schema.ts");

/// Pulls the integer immediately preceding `suffix` out of `haystack`.
fn stated_count(haystack: &str, suffix: &str) -> usize {
    let idx = haystack
        .find(suffix)
        .unwrap_or_else(|| panic!("COMPLIANCE.md no longer contains {suffix:?}"));
    let digits: String = haystack[..idx]
        .chars()
        .rev()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("no integer before {suffix:?} in COMPLIANCE.md"))
}

#[test]
fn compliance_md_see_count_matches_the_vendored_schema() {
    let stated = stated_count(COMPLIANCE_MD, " `@see` block-tags");
    let actual = SCHEMA_TS.matches("@see").count();
    assert_eq!(
        stated, actual,
        "COMPLIANCE.md claims {stated} @see block-tags; schema/schema.ts carries {actual}"
    );
}

#[cfg(feature = "compliance")]
#[test]
fn compliance_md_fixture_count_matches_the_case_table() {
    use turul_mcp_protocol_2026_07_28::compliance::coverage::CASES;
    let stated = stated_count(COMPLIANCE_MD, " directories, ");
    assert_eq!(
        stated,
        CASES.len(),
        "COMPLIANCE.md claims {stated} fixture directories; the CASES table has {}",
        CASES.len()
    );
}

/// The pin moved to the released dated path. Naming `schema/draft/` in prose is
/// legitimate — it is what upstream calls the next cycle's floating pointer —
/// but a doc that locates one of *this crate's own artifacts* under it is
/// asserting the wrong provenance. These two spellings are the ones that were
/// actually wrong, and both survived a re-pin unnoticed.
#[test]
fn shipped_docs_do_not_locate_this_crates_artifacts_under_the_draft_path() {
    for (name, body) in [
        ("COMPLIANCE.md", COMPLIANCE_MD),
        ("README.md", include_str!("../README.md")),
        ("Cargo.toml", include_str!("../Cargo.toml")),
    ] {
        for stale in ["schema/draft/examples", "draft-schema.ts"] {
            assert!(
                !body.contains(stale),
                "{name} names {stale}; this crate's artifacts live under \
                 schema/2026-07-28 (fixtures) and schema/schema.ts (vendored copy)"
            );
        }
    }
}
