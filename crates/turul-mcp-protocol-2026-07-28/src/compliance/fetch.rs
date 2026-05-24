//! Fetch + cache the upstream MCP example fixtures pinned to a known SHA.
//!
//! Uses `git clone --depth=1 --filter=blob:none --no-checkout` followed by
//! sparse-checkout of just `schema/draft/examples`, then `git checkout <SHA>`.
//! Atomic and cheap: one clone gives us the entire pinned tree without paying
//! for blob bandwidth until we ask. No new Cargo dependencies.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The pinned upstream tree. **Single source of truth** for which examples we
/// assert compliance against. `schema/EXAMPLES_PIN.md` is human documentation
/// regenerated from this constant by `refresh --write`.
pub const PIN: Pin = Pin {
    repo: "https://github.com/modelcontextprotocol/modelcontextprotocol.git",
    // Commit SHA (not tree SHA) — the last commit that touched
    // `schema/draft/examples` at the time of the 2026-05-24 capture.
    // The tree SHA for that subpath at this commit is
    // `9f9415b427c4db6f7ad375ca7b86d1a5ee955072` (recorded for audit only).
    sha: "c3e3f09eb5d271407afac0f0bb6ee2dae5813d1d",
    subpath: "schema/draft/examples",
};

/// Pin record for an upstream tree fragment.
#[derive(Debug, Clone, Copy)]
pub struct Pin {
    pub repo: &'static str,
    pub sha: &'static str,
    pub subpath: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("git not available on PATH: {0}")]
    GitNotFound(io::Error),
    #[error("git command failed (status={status}): {stderr}")]
    GitFailed { status: i32, stderr: String },
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cache appears poisoned (missing {0:?}); delete and retry")]
    CachePoisoned(PathBuf),
}

/// Ensure the pinned example tree is materialised under `dest`. Returns the
/// absolute path to the `examples/` directory inside the cache.
///
/// Idempotent: if `dest/.pin` already matches `pin.sha`, no network is hit.
/// On a stale pin, the cache is wiped and re-cloned.
pub fn ensure_fixtures(dest: &Path, pin: &Pin) -> Result<PathBuf, FetchError> {
    let examples_dir = dest.join(pin.subpath);
    let pin_marker = dest.join(".pin");

    if pin_marker.exists() && examples_dir.exists() {
        let recorded = fs::read_to_string(&pin_marker)
            .map_err(|e| FetchError::Io { path: pin_marker.clone(), source: e })?;
        if recorded.trim() == pin.sha {
            return Ok(examples_dir);
        }
    }

    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| FetchError::Io { path: dest.to_path_buf(), source: e })?;
    }
    fs::create_dir_all(dest).map_err(|e| FetchError::Io { path: dest.to_path_buf(), source: e })?;

    run_git(dest, &["init", "--quiet"])?;
    run_git(dest, &["remote", "add", "origin", pin.repo])?;
    run_git(dest, &["config", "extensions.partialClone", "origin"])?;
    run_git(dest, &["sparse-checkout", "init", "--cone"])?;
    run_git(dest, &["sparse-checkout", "set", pin.subpath])?;
    run_git(
        dest,
        &[
            "fetch",
            "--depth=1",
            "--filter=blob:none",
            "origin",
            pin.sha,
        ],
    )?;
    run_git(dest, &["checkout", "--quiet", pin.sha])?;

    fs::write(&pin_marker, pin.sha)
        .map_err(|e| FetchError::Io { path: pin_marker.clone(), source: e })?;

    if !examples_dir.exists() {
        return Err(FetchError::CachePoisoned(examples_dir));
    }
    Ok(examples_dir)
}

/// Enumerate every example sub-directory under the fetched tree (i.e. the 86
/// upstream PascalCase dirs), sorted lexicographically for stable iteration.
pub fn list_example_dirs(examples_dir: &Path) -> Result<Vec<String>, FetchError> {
    let mut names: Vec<String> = fs::read_dir(examples_dir)
        .map_err(|e| FetchError::Io { path: examples_dir.to_path_buf(), source: e })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                entry.file_name().into_string().ok()
            } else {
                None
            }
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Enumerate every `*.json` file directly under one example directory, sorted.
pub fn list_json_files(dir: &Path) -> Result<Vec<PathBuf>, FetchError> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| FetchError::Io { path: dir.to_path_buf(), source: e })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    Ok(files)
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<(), FetchError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(FetchError::GitNotFound)?;
    if !output.status.success() {
        return Err(FetchError::GitFailed {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}
