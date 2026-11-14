//! [`SkillEntry::from_files`]: the only path from a skill's files to a
//! [`Skill`] wire entry (ADR-033 §1). It composes the owners: validation by
//! `turul-agent-skills`, then [`Frontmatter`], [`SkillPath`] and [`Manifest`].

use std::collections::BTreeMap;

use turul_agent_skills::{ErrorCode, load_str};

use super::frontmatter::{Frontmatter, FrontmatterError};
use super::manifest::{Manifest, ManifestError, SkillResource};
use super::types::{Skill, SkillResources};
use super::uri::{SkillPath, UriError};

/// One file of a skill: its `/`-separated path relative to the skill root and
/// its raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

impl SkillFile {
    pub fn new(relative_path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            relative_path: relative_path.into(),
            bytes: bytes.into(),
        }
    }
}

/// Why a skill cannot be published. Every variant names the skill.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EntryError {
    #[error("skill {skill}: {source}")]
    Uri { skill: String, source: UriError },
    #[error("skill {0}: no SKILL.md among its files")]
    NoSkillMd(String),
    #[error("skill {0}: SKILL.md is not UTF-8")]
    SkillMdNotUtf8(String),
    /// `turul-agent-skills` refused the skill, or loaded it only with diagnostics.
    #[error("skill {skill}: does not validate: {}", errors.join("; "))]
    Invalid { skill: String, errors: Vec<String> },
    #[error("skill {skill}: {source}")]
    Frontmatter {
        skill: String,
        source: FrontmatterError,
    },
    #[error("skill {skill}: {source}")]
    Manifest {
        skill: String,
        source: ManifestError,
    },
}

/// A publishable skill: its path, frontmatter, manifest and the exact bytes the
/// manifest describes.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    path: SkillPath,
    frontmatter: Frontmatter,
    manifest: Manifest,
    contents: BTreeMap<String, Vec<u8>>,
}

impl SkillEntry {
    /// Build the entry for the skill at `path` from all of its files.
    ///
    /// `files` must be the complete contents of the skill directory, nested
    /// skills included (`skills.mdx` §Resources); completeness is the caller's
    /// contract, everything else is checked here.
    pub fn from_files(path: SkillPath, files: Vec<SkillFile>) -> Result<Self, EntryError> {
        let skill = path.skill_md_uri();
        let skill_md = files
            .iter()
            .find(|f| f.relative_path == "SKILL.md")
            .ok_or_else(|| EntryError::NoSkillMd(skill.clone()))?;
        let text = std::str::from_utf8(&skill_md.bytes)
            .map_err(|_| EntryError::SkillMdNotUtf8(skill.clone()))?;

        // Validation is turul-agent-skills': name rules, name == directory name
        // (here: the final path segment), required fields.
        let loaded = load_str(path.name().as_str(), text).map_err(|skip| EntryError::Invalid {
            skill: skill.clone(),
            errors: skip.errors.iter().map(ToString::to_string).collect(),
        })?;
        // An unknown field is kept as an extension and passes through (ADR-033 §3).
        let refusals: Vec<String> = loaded
            .diagnostics
            .iter()
            .filter(|d| d.code() != ErrorCode::UnknownField)
            .map(ToString::to_string)
            .collect();
        if !refusals.is_empty() {
            return Err(EntryError::Invalid {
                skill,
                errors: refusals,
            });
        }

        let frontmatter =
            Frontmatter::from_skill_md(text).map_err(|source| EntryError::Frontmatter {
                skill: skill.clone(),
                source,
            })?;
        debug_assert_eq!(frontmatter.name(), path.name().as_str());

        let mut resources = Vec::with_capacity(files.len());
        let mut contents = BTreeMap::new();
        for file in files {
            let uri = path
                .file_uri(&file.relative_path)
                .map_err(|source| EntryError::Uri {
                    skill: skill.clone(),
                    source,
                })?;
            resources.push(SkillResource::of(uri.clone(), &file.bytes));
            contents.insert(uri, file.bytes);
        }
        let manifest = Manifest::new(&path, resources).map_err(|source| EntryError::Manifest {
            skill: skill.clone(),
            source,
        })?;
        Ok(Self {
            path,
            frontmatter,
            manifest,
            contents,
        })
    }

    pub fn path(&self) -> &SkillPath {
        &self.path
    }

    /// URI of the skill's `SKILL.md`.
    pub fn uri(&self) -> String {
        self.path.skill_md_uri()
    }

    pub fn frontmatter(&self) -> &Frontmatter {
        &self.frontmatter
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Each manifest entry with the bytes it describes, in manifest order.
    pub fn files(&self) -> impl Iterator<Item = (&SkillResource, &[u8])> {
        self.manifest
            .entries()
            .iter()
            .map(|r| (r, self.contents[r.uri()].as_slice()))
    }

    /// The wire entry.
    pub fn to_wire(&self) -> Skill {
        Skill {
            uri: self.uri(),
            frontmatter: self.frontmatter.as_map().clone(),
            resources: SkillResources::Manifest(self.manifest.entries().to_vec()),
            meta: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_agent_skills::SkillName;

    fn path(prefix: &[&str], name: &str) -> SkillPath {
        SkillPath::new(prefix.iter().copied(), SkillName::parse(name).unwrap()).unwrap()
    }

    fn md(name: &str) -> SkillFile {
        SkillFile::new(
            "SKILL.md",
            format!("---\nname: {name}\ndescription: Does {name}.\n---\nBody\n"),
        )
    }

    /// S-G4 / S-G5: the entry's uri ends in the frontmatter name, and the
    /// manifest covers every file with digests of the exact bytes kept.
    #[test]
    fn s_g4_entry_manifest_matches_bytes() {
        let e = SkillEntry::from_files(
            path(&["acme", "billing"], "refunds"),
            vec![md("refunds"), SkillFile::new("examples/email.md", "hi")],
        )
        .unwrap();
        let wire = e.to_wire();
        assert_eq!(wire.uri, "skill://acme/billing/refunds/SKILL.md");
        assert_eq!(wire.frontmatter["name"], "refunds");
        for (res, bytes) in e.files() {
            assert_eq!(res, &SkillResource::of(res.uri().to_string(), bytes));
        }
        assert_eq!(e.files().count(), 2);
    }

    /// S-G5: a skill whose frontmatter name is not the final segment is refused
    /// by turul-agent-skills' validation.
    #[test]
    fn s_g5_name_must_equal_final_segment() {
        let err = SkillEntry::from_files(path(&[], "refunds"), vec![md("other")]).unwrap_err();
        assert!(matches!(err, EntryError::Invalid { .. }), "{err}");
    }

    /// ADR-033 §3: a skill that only loads with lenient repair is refused.
    #[test]
    fn repaired_skill_is_refused() {
        let f = SkillFile::new(
            "SKILL.md",
            "---\nname: a\ndescription: Use when: asked\n---\n",
        );
        let err = SkillEntry::from_files(path(&[], "a"), vec![f]).unwrap_err();
        assert!(
            matches!(&err, EntryError::Invalid { errors, .. }
                if errors.iter().any(|e| e.contains("repaired"))),
            "{err}"
        );
    }

    /// S-G3 (:208) / ADR-033 §3: a field outside the Agent Skills set passes
    /// through to the entry unchanged.
    #[test]
    fn s_g3_extension_fields_pass_through() {
        let f = SkillFile::new(
            "SKILL.md",
            "---\nname: a\ndescription: d\ntags:\n  - x\nx-future: v\n---\n",
        );
        let wire = SkillEntry::from_files(path(&[], "a"), vec![f])
            .unwrap()
            .to_wire();
        assert_eq!(wire.frontmatter["tags"], serde_json::json!(["x"]));
        assert_eq!(wire.frontmatter["x-future"], "v");
    }

    /// Nested skills: the enclosing manifest lists the nested files; the nested
    /// skill is its own entry whose uri shares the prefix.
    #[test]
    fn nested_skill_files_are_supporting_files() {
        let outer = SkillEntry::from_files(
            path(&[], "outer"),
            vec![
                md("outer"),
                SkillFile::new("inner/SKILL.md", md("inner").bytes),
            ],
        )
        .unwrap();
        let inner = SkillEntry::from_files(path(&["outer"], "inner"), vec![md("inner")]).unwrap();
        let outer_uris: Vec<_> = outer.manifest().entries().iter().map(|r| r.uri()).collect();
        assert!(outer_uris.contains(&inner.uri().as_str()));
        assert!(inner.uri().starts_with(&outer.path().root_uri()));
    }
}
