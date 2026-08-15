//! Wire-format compliance CLI for `turul-mcp-protocol-2026-07-28`.
//!
//! Fetches the pinned upstream `schema/2026-07-28/examples` tree and round-trips
//! every modeled fixture through its Rust binding. Exits 0 if all modeled
//! cases match, non-zero otherwise. The same `compliance::roundtrip::run_all`
//! is called by `tests/upstream_fixtures.rs` — green tests ⇒ green binary.
//!
//! Usage:
//!   mcp-compliance-2026-07-28
//!   mcp-compliance-2026-07-28 refresh             # dry-run: probe upstream HEAD, run, diff
//!   mcp-compliance-2026-07-28 refresh --write     # also rewrite fetch.rs PIN + EXAMPLES_PIN.md

use std::path::PathBuf;
use std::process::ExitCode;

use turul_mcp_protocol_2026_07_28::compliance::fetch::{PIN, Pin, resolve_subpath_head};
use turul_mcp_protocol_2026_07_28::compliance::roundtrip::{self, Outcome};

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("mcp-compliance-2026-07-28")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let subcommand = args.first().map(String::as_str).unwrap_or("run");

    match subcommand {
        "refresh" => refresh(&args),
        _ => run(),
    }
}

fn run() -> ExitCode {
    let dest = cache_dir();
    println!("Pinned upstream: {} @ {}", PIN.repo, PIN.sha);

    let report = match roundtrip::run_all(&dest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("fetch failed: {e}");
            return ExitCode::from(2);
        }
    };

    let total_cases = report.modeled + report.not_modeled;
    println!(
        "Modeled:   {} / {}    Not modeled: {}",
        report.modeled, total_cases, report.not_modeled
    );
    println!(
        "Files:     {} fixtures, {} passed, {} failed",
        report.total_files,
        report.passed,
        report.failed.len()
    );

    if report.is_clean() {
        println!("All modeled cases match upstream — compliance OK.");
        ExitCode::SUCCESS
    } else {
        println!("\nFAILED ({}):", report.failed.len());
        for f in &report.failed {
            let detail = match &f.outcome {
                Outcome::Pass => unreachable!(),
                Outcome::Diff(d) => format!("diff: {d}"),
                Outcome::ParseError(e) => format!("parse error: {e}"),
                Outcome::SerializeError(e) => format!("serialize error: {e}"),
            };
            println!(
                "  X {}/{}",
                f.dir,
                f.file.file_name().unwrap().to_string_lossy()
            );
            println!("    {detail}");
        }
        ExitCode::from(1)
    }
}

/// Refresh: probe upstream `main` for the latest commit touching the pinned
/// subpath, fetch that commit into a side cache, run the harness against it,
/// and (with `--write`) atomically rewrite the PIN in `fetch.rs` and the
/// markdown in `schema/EXAMPLES_PIN.md`. Exits non-zero if any modeled case
/// would regress under the new pin.
///
/// Probing `main` is safe because `PIN.subpath` names the dated
/// `schema/2026-07-28/` directory, which upstream only touches to publish
/// errata against the released spec. Resolving a *floating* subpath such as
/// `schema/draft/` against `main` would instead walk onto the next spec
/// cycle's content while still claiming to implement 2026-07-28.
fn refresh(args: &[String]) -> ExitCode {
    let write = args.iter().any(|a| a == "--write");
    println!("refresh (write={write})");
    println!("Current pin: {}", PIN.sha);

    let resolve_dir = std::env::temp_dir().join("mcp-compliance-2026-07-28-resolve");
    let new_sha = match resolve_subpath_head(&PIN, "main", &resolve_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("refresh: failed to resolve upstream HEAD: {e}");
            return ExitCode::from(2);
        }
    };
    println!("Last commit touching {} on main: {}", PIN.subpath, new_sha);

    if new_sha == PIN.sha {
        println!("Already at HEAD — nothing to do.");
        return ExitCode::SUCCESS;
    }

    // Validate the candidate pin by running the full harness with it in a
    // side cache so we never poison the primary cache mid-rotation.
    let probe_dest = std::env::temp_dir().join(format!("mcp-compliance-probe-{new_sha}"));
    let candidate_pin = Pin {
        repo: PIN.repo,
        sha: leak_string(new_sha.clone()),
        subpath: PIN.subpath,
    };
    let report = match roundtrip::run_all_with_pin(&probe_dest, &candidate_pin) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("refresh: probe fetch failed: {e}");
            return ExitCode::from(2);
        }
    };

    println!(
        "Probe: modeled={} files={} passed={} failed={}",
        report.modeled,
        report.total_files,
        report.passed,
        report.failed.len()
    );
    if !report.is_clean() {
        println!("\nRegressions under candidate pin {new_sha}:");
        for f in &report.failed {
            let detail = match &f.outcome {
                Outcome::Pass => unreachable!(),
                Outcome::Diff(d) => format!("diff: {d}"),
                Outcome::ParseError(e) => format!("parse error: {e}"),
                Outcome::SerializeError(e) => format!("serialize error: {e}"),
            };
            println!(
                "  X {}/{}\n    {}",
                f.dir,
                f.file.file_name().unwrap().to_string_lossy(),
                detail
            );
        }
        eprintln!("\nrefresh refusing to bump pin while modeled cases regress.");
        return ExitCode::from(1);
    }

    if !write {
        println!("\nDry-run OK. Re-run with `refresh --write` to bump the pin.");
        return ExitCode::SUCCESS;
    }

    // Atomic-pair update: rewrite both files; if either fails, leave both intact.
    if let Err(e) = atomic_pin_rewrite(&new_sha) {
        eprintln!("refresh --write failed: {e}");
        return ExitCode::from(2);
    }
    println!("PIN rewritten to {new_sha} in src/compliance/fetch.rs + schema/EXAMPLES_PIN.md");
    ExitCode::SUCCESS
}

/// `Pin` requires `&'static str`. For runtime-discovered SHAs we leak — this
/// is a CLI binary that exits immediately after, so the leak is benign and
/// keeps the trait shape simple.
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Rewrite the PIN constant in `src/compliance/fetch.rs` AND the SHA lines in
/// `schema/EXAMPLES_PIN.md` in one operation. If either rewrite fails after
/// the other succeeded, this leaves the repo in a mixed state — caller should
/// re-run the refresh or revert via git. We don't fsync because both files
/// are tracked: any inconsistency is recoverable via `git checkout`.
fn atomic_pin_rewrite(new_sha: &str) -> Result<(), String> {
    // Locate the crate root: this binary lives in <crate>/src/bin/, so the
    // crate root is two ancestors up from CARGO_MANIFEST_DIR's parent.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fetch_path = crate_dir.join("src/compliance/fetch.rs");
    let pin_md = crate_dir.join("schema/EXAMPLES_PIN.md");

    let fetch_src =
        std::fs::read_to_string(&fetch_path).map_err(|e| format!("read fetch.rs: {e}"))?;
    let pin_md_src =
        std::fs::read_to_string(&pin_md).map_err(|e| format!("read EXAMPLES_PIN.md: {e}"))?;

    let new_fetch = replace_pin_in_fetch(&fetch_src, PIN.sha, new_sha)?;
    let new_pin_md = replace_pin_in_md(&pin_md_src, PIN.sha, new_sha)?;

    std::fs::write(&fetch_path, new_fetch).map_err(|e| format!("write fetch.rs: {e}"))?;
    if let Err(e) = std::fs::write(&pin_md, new_pin_md) {
        // Best-effort rollback of fetch.rs to keep the pair consistent.
        let _ = std::fs::write(&fetch_path, fetch_src);
        return Err(format!("write EXAMPLES_PIN.md (fetch.rs rolled back): {e}"));
    }
    Ok(())
}

fn replace_pin_in_fetch(src: &str, old_sha: &str, new_sha: &str) -> Result<String, String> {
    let needle = format!("sha: \"{old_sha}\"");
    let replacement = format!("sha: \"{new_sha}\"");
    if !src.contains(&needle) {
        return Err(format!(
            "could not find `{needle}` in fetch.rs — refusing to write blindly"
        ));
    }
    Ok(src.replacen(&needle, &replacement, 1))
}

fn replace_pin_in_md(src: &str, old_sha: &str, new_sha: &str) -> Result<String, String> {
    if !src.contains(old_sha) {
        return Err(format!(
            "could not find `{old_sha}` in EXAMPLES_PIN.md — refusing to write blindly"
        ));
    }
    Ok(src.replacen(old_sha, new_sha, 1))
}
