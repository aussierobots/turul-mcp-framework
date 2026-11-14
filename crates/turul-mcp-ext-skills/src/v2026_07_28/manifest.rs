//! Skill file manifests (`skills.mdx` §Resources, §Limits, §Integrity and
//! Verification). [`SkillResource::of`] is the only producer of a `sha256:`
//! digest; [`Manifest::new`] is the only producer of a complete manifest
//! (ADR-033 §4).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::uri::SkillPath;

/// Resources per skill a host must accept (`skills.mdx` §Limits).
pub const MAX_RESOURCES_PER_SKILL: usize = 512;
/// Total file size per skill a host must accept, in bytes (`skills.mdx` §Limits).
pub const MAX_TOTAL_BYTES_PER_SKILL: u64 = 16 * 1024 * 1024;

/// One file of a skill with the digest and size of its raw bytes.
///
/// Locally, [`SkillResource::of`] is the only producer; `Deserialize` exists to
/// read entries a peer sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillResource {
    uri: String,
    digest: String,
    size: u64,
}

impl SkillResource {
    /// Digest and size of `bytes`, the exact content `resources/read` serves.
    pub fn of(uri: String, bytes: &[u8]) -> Self {
        let hash = Sha256::digest(bytes);
        let mut digest = String::with_capacity(7 + 64);
        digest.push_str("sha256:");
        for b in hash.iter() {
            digest.push_str(&format!("{b:02x}"));
        }
        Self {
            uri,
            digest,
            size: bytes.len() as u64,
        }
    }

    /// The file's resource URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// `sha256:` followed by 64 lowercase hex characters.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Length in bytes of the file's raw content.
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Why a set of files is not a valid skill manifest.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// No entry for the skill's `SKILL.md`.
    #[error("manifest has no entry for {0}")]
    MissingSkillMd(String),
    /// The same URI appears twice.
    #[error("manifest lists {0} more than once")]
    Duplicate(String),
    /// A URI is not inside the skill's directory.
    #[error("{uri} is not inside skill {root}")]
    Foreign { uri: String, root: String },
    /// More than [`MAX_RESOURCES_PER_SKILL`] entries.
    #[error("skill {root} has {count} files; the limit is {MAX_RESOURCES_PER_SKILL}")]
    TooManyFiles { root: String, count: usize },
    /// Total size above [`MAX_TOTAL_BYTES_PER_SKILL`].
    #[error("skill {root} totals {total} bytes; the limit is {MAX_TOTAL_BYTES_PER_SKILL}")]
    TooLarge { root: String, total: u64 },
}

/// A complete, ordered manifest of one skill's files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Manifest(Vec<SkillResource>);

impl Manifest {
    /// Check completeness, uniqueness, containment and limits, and order entries
    /// by URI so equal file sets produce identical manifests.
    pub fn new(path: &SkillPath, mut entries: Vec<SkillResource>) -> Result<Self, ManifestError> {
        let root = path.root_uri();
        let mut seen = BTreeSet::new();
        for entry in &entries {
            if !path.contains(&entry.uri) {
                return Err(ManifestError::Foreign {
                    uri: entry.uri.clone(),
                    root,
                });
            }
            if !seen.insert(entry.uri.as_str()) {
                return Err(ManifestError::Duplicate(entry.uri.clone()));
            }
        }
        let skill_md = path.skill_md_uri();
        if !seen.contains(skill_md.as_str()) {
            return Err(ManifestError::MissingSkillMd(skill_md));
        }
        if entries.len() > MAX_RESOURCES_PER_SKILL {
            return Err(ManifestError::TooManyFiles {
                root,
                count: entries.len(),
            });
        }
        let total: u64 = entries.iter().map(|e| e.size).sum();
        if total > MAX_TOTAL_BYTES_PER_SKILL {
            return Err(ManifestError::TooLarge { root, total });
        }
        entries.sort_by(|a, b| a.uri.cmp(&b.uri));
        Ok(Self(entries))
    }

    /// The entries, ordered by URI.
    pub fn entries(&self) -> &[SkillResource] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_agent_skills::SkillName;

    fn path() -> SkillPath {
        SkillPath::new(["acme"], SkillName::parse("refunds").unwrap()).unwrap()
    }

    fn res(rel: &str, bytes: &[u8]) -> SkillResource {
        SkillResource::of(path().file_uri(rel).unwrap(), bytes)
    }

    /// S-G4: `sha256:` + 64 lowercase hex over the raw bytes; `size` is their length.
    #[test]
    fn s_g4_digest_format_and_size() {
        let r = SkillResource::of("skill://x/SKILL.md".into(), b"abc");
        assert_eq!(
            r.digest(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(r.size(), 3);
        let empty = SkillResource::of("skill://x/SKILL.md".into(), b"");
        assert_eq!(
            empty.digest(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(empty.size(), 0);
    }

    /// ADR-033 §Tests: distinct content produces distinct digests.
    #[test]
    fn distinct_bytes_produce_distinct_digests() {
        assert_ne!(
            res("SKILL.md", b"a").digest(),
            res("SKILL.md", b"b").digest()
        );
    }

    /// ADR-033 §4: the manifest producer is deterministic under input order.
    #[test]
    fn manifest_is_deterministic_under_input_order() {
        let files = [("SKILL.md", &b"s"[..]), ("b.md", b"b"), ("a/c.md", b"c")];
        let forward: Vec<_> = files.iter().map(|(p, b)| res(p, b)).collect();
        let reverse: Vec<_> = files.iter().rev().map(|(p, b)| res(p, b)).collect();
        let m1 = Manifest::new(&path(), forward).unwrap();
        let m2 = Manifest::new(&path(), reverse).unwrap();
        assert_eq!(
            serde_json::to_string(&m1).unwrap(),
            serde_json::to_string(&m2).unwrap()
        );
        let uris: Vec<_> = m1.entries().iter().map(SkillResource::uri).collect();
        let mut sorted = uris.clone();
        sorted.sort();
        assert_eq!(uris, sorted);
    }

    /// S-G4: the manifest includes an entry for the skill's own `SKILL.md`.
    #[test]
    fn s_g4_manifest_requires_skill_md() {
        assert_eq!(
            Manifest::new(&path(), vec![res("a.md", b"a")]),
            Err(ManifestError::MissingSkillMd(
                "skill://acme/refunds/SKILL.md".into()
            ))
        );
    }

    /// S-G4: each file exactly once, all within the skill's directory.
    #[test]
    fn s_g4_manifest_refuses_duplicates_and_foreign_uris() {
        let dup = vec![res("SKILL.md", b"s"), res("SKILL.md", b"s")];
        assert!(matches!(
            Manifest::new(&path(), dup),
            Err(ManifestError::Duplicate(_))
        ));
        let foreign = vec![
            res("SKILL.md", b"s"),
            SkillResource::of("skill://acme/other/a.md".into(), b"a"),
        ];
        assert!(matches!(
            Manifest::new(&path(), foreign),
            Err(ManifestError::Foreign { .. })
        ));
    }

    /// S-G8: at most 512 entries and 16 MiB in total; both bounds inclusive.
    #[test]
    fn s_g8_limits() {
        let at_limit: Vec<_> = std::iter::once(res("SKILL.md", b"s"))
            .chain((1..MAX_RESOURCES_PER_SKILL).map(|i| res(&format!("f{i}"), b"")))
            .collect();
        assert!(Manifest::new(&path(), at_limit.clone()).is_ok());
        let mut over = at_limit;
        over.push(res("extra", b""));
        assert!(matches!(
            Manifest::new(&path(), over),
            Err(ManifestError::TooManyFiles { count: 513, .. })
        ));

        let big = vec![0u8; MAX_TOTAL_BYTES_PER_SKILL as usize - 1];
        assert!(Manifest::new(&path(), vec![res("SKILL.md", b"s"), res("big", &big)]).is_ok());
        assert!(matches!(
            Manifest::new(&path(), vec![res("SKILL.md", b"ss"), res("big", &big)]),
            Err(ManifestError::TooLarge { .. })
        ));
    }
}
