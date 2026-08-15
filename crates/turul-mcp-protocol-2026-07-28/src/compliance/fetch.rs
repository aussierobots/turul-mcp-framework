//! Fetch + cache the upstream MCP example fixtures pinned to a known SHA.
//!
//! Uses `git clone --depth=1 --filter=blob:none --no-checkout` followed by
//! sparse-checkout of just `schema/2026-07-28/examples`, then `git checkout <SHA>`.
//! Atomic and cheap: one clone gives us the entire pinned tree without paying
//! for blob bandwidth until we ask. No new Cargo dependencies.
//!
//! [`resolve_subpath_head`] answers the separate question of *which* SHA to pin
//! to, and deliberately fetches without `--depth` — see its own note.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The pinned upstream tree. **Single source of truth** for which examples we
/// assert compliance against. `schema/EXAMPLES_PIN.md` is human documentation
/// regenerated from this constant by `refresh --write`.
pub const PIN: Pin = Pin {
    repo: "https://github.com/modelcontextprotocol/modelcontextprotocol.git",
    // Commit SHA (not tree SHA) of the last upstream commit that changed
    // `subpath`. `refresh --write` rewrites this line and nothing else, so keep
    // any capture-specific detail out of this comment — it would go stale on
    // the next bump without anyone noticing. The same commit pins
    // `schema/schema.ts`; see `schema/README.md`.
    sha: "271ecc9accafdd9b83a3c869fa67c22953b2af80",
    subpath: "schema/2026-07-28/examples",
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
        let recorded = fs::read_to_string(&pin_marker).map_err(|e| FetchError::Io {
            path: pin_marker.clone(),
            source: e,
        })?;
        if recorded.trim() == pin.sha {
            return Ok(examples_dir);
        }
    }

    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| FetchError::Io {
            path: dest.to_path_buf(),
            source: e,
        })?;
    }
    fs::create_dir_all(dest).map_err(|e| FetchError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;

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

    fs::write(&pin_marker, pin.sha).map_err(|e| FetchError::Io {
        path: pin_marker.clone(),
        source: e,
    })?;

    if !examples_dir.exists() {
        return Err(FetchError::CachePoisoned(examples_dir));
    }
    Ok(examples_dir)
}

/// Resolve the most recent commit on `branch` that actually changed
/// `pin.subpath`, using `workdir` as scratch space.
///
/// The fetch is blobless but **not** shallow: `--depth=1` would leave no
/// history for `git log` to walk, so the answer would collapse to the branch
/// tip regardless of which commits touched the subpath. Trees are enough to
/// decide whether a commit changed a path, so file contents stay unfetched.
pub fn resolve_subpath_head(pin: &Pin, branch: &str, workdir: &Path) -> Result<String, FetchError> {
    if workdir.exists() {
        fs::remove_dir_all(workdir).map_err(|e| FetchError::Io {
            path: workdir.to_path_buf(),
            source: e,
        })?;
    }
    fs::create_dir_all(workdir).map_err(|e| FetchError::Io {
        path: workdir.to_path_buf(),
        source: e,
    })?;

    run_git(workdir, &["init", "--quiet"])?;
    run_git(workdir, &["remote", "add", "origin", pin.repo])?;
    run_git(workdir, &["config", "extensions.partialClone", "origin"])?;
    run_git(
        workdir,
        &["fetch", "--quiet", "--filter=blob:none", "origin", branch],
    )?;

    let sha = run_git_stdout(
        workdir,
        &["log", "-1", "--format=%H", "FETCH_HEAD", "--", pin.subpath],
    )?;
    let sha = sha.trim();

    if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(FetchError::GitFailed {
            status: 0,
            stderr: format!(
                "no commit found touching {} on {branch} (got {sha:?})",
                pin.subpath
            ),
        });
    }
    Ok(sha.to_string())
}

/// Enumerate every example sub-directory under the fetched tree (i.e. the
/// upstream PascalCase dirs), sorted lexicographically for stable iteration.
pub fn list_example_dirs(examples_dir: &Path) -> Result<Vec<String>, FetchError> {
    let mut names: Vec<String> = fs::read_dir(examples_dir)
        .map_err(|e| FetchError::Io {
            path: examples_dir.to_path_buf(),
            source: e,
        })?
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
        .map_err(|e| FetchError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?
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
    run_git_stdout(cwd, args).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "turul-fetch-test-{}-{}-{}",
            std::process::id(),
            label,
            n
        ))
    }

    fn git(dir: &Path, args: &[&str]) {
        run_git(dir, args).unwrap_or_else(|e| panic!("git {args:?} failed: {e}"));
    }

    fn commit_file(repo: &Path, rel: &str, msg: &str) -> String {
        let path = repo.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, msg).unwrap();
        git(repo, &["add", "."]);
        git(repo, &["commit", "--quiet", "-m", msg]);
        run_git_stdout(repo, &["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string()
    }

    fn init_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        git(path, &["init", "--quiet"]);
        git(path, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(path, &["config", "user.email", "fixture@example.invalid"]);
        git(path, &["config", "user.name", "Fixture"]);
        git(path, &["config", "commit.gpgsign", "false"]);
        git(path, &["config", "uploadpack.allowFilter", "true"]);
    }

    /// The resolver must return the last commit that *changed the subpath*, not
    /// the branch tip. The fixture deliberately ends with commits that leave the
    /// subpath untouched: an implementation that reports the branch tip returns
    /// `tip` here and fails, which is the regression being guarded.
    #[test]
    fn resolve_subpath_head_ignores_later_unrelated_commits() {
        let origin = scratch("origin");
        init_repo(&origin);

        commit_file(&origin, "other/a.txt", "unrelated-first");
        let expected = commit_file(&origin, "schema/examples/Case/x.json", "touches-subpath");
        commit_file(&origin, "other/b.txt", "unrelated-after");
        let tip = commit_file(&origin, "docs/readme.md", "tip-unrelated");

        let pin = Pin {
            repo: Box::leak(origin.to_string_lossy().into_owned().into_boxed_str()),
            sha: "unused",
            subpath: "schema/examples",
        };

        let work = scratch("work");
        let resolved = resolve_subpath_head(&pin, "main", &work).expect("resolve failed");

        assert_eq!(
            resolved, expected,
            "resolver must return the last subpath-changing commit"
        );
        assert_ne!(resolved, tip, "resolver must not return the branch tip");

        let _ = fs::remove_dir_all(&origin);
        let _ = fs::remove_dir_all(&work);
    }
}

fn run_git_stdout(cwd: &Path, args: &[&str]) -> Result<String, FetchError> {
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
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
