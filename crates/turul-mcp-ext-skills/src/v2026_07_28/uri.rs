//! `skill://` URIs (`skills.mdx` §Resource Mapping). [`SkillPath`] is the only
//! place a `skill://` string is formatted (ADR-033 §2).

use turul_agent_skills::SkillName;

const SCHEME: &str = "skill://";
const SKILL_MD: &str = "SKILL.md";

/// Why a skill path or file path cannot form a URI.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UriError {
    /// A prefix segment is empty, `.`/`..`, or has a character outside the RFC 3986
    /// unreserved set.
    #[error("invalid skill path prefix segment {0:?}: use only A-Z a-z 0-9 - . _ ~")]
    PrefixSegment(String),
    /// A file path is empty or has an empty, `.` or `..` segment.
    #[error("invalid skill file path {0:?}: segments must be non-empty and not '.' or '..'")]
    FilePath(String),
}

/// Where a skill sits in a server's skill namespace: an optional organisational
/// prefix and the skill's name, which is always the final segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillPath {
    prefix: Vec<String>,
    name: SkillName,
}

impl SkillPath {
    /// A skill path from prefix segments and the skill name.
    ///
    /// The first segment (prefix or name) becomes the URI authority; restricting
    /// prefix segments to unreserved characters makes it a valid `reg-name`.
    pub fn new<'a>(
        prefix: impl IntoIterator<Item = &'a str>,
        name: SkillName,
    ) -> Result<Self, UriError> {
        let prefix: Vec<String> = prefix.into_iter().map(str::to_string).collect();
        if let Some(bad) = prefix.iter().find(|s| !is_prefix_segment(s)) {
            return Err(UriError::PrefixSegment(bad.clone()));
        }
        Ok(Self { prefix, name })
    }

    /// The skill's name, the final path segment.
    pub fn name(&self) -> &SkillName {
        &self.name
    }

    /// The skill root directory URI, `skill://<skill-path>`, with no trailing slash.
    pub fn root_uri(&self) -> String {
        let mut uri = String::from(SCHEME);
        for segment in &self.prefix {
            uri.push_str(segment);
            uri.push('/');
        }
        uri.push_str(self.name.as_str());
        uri
    }

    /// The `SKILL.md` URI, the skill's identity within a server.
    pub fn skill_md_uri(&self) -> String {
        self.file_uri(SKILL_MD)
            .expect("SKILL.md is a valid file path")
    }

    /// The URI of a file at `relative` (`/`-separated) inside the skill.
    /// Characters outside the unreserved set are percent-encoded.
    pub fn file_uri(&self, relative: &str) -> Result<String, UriError> {
        let mut uri = self.root_uri();
        if relative.is_empty() {
            return Err(UriError::FilePath(relative.to_string()));
        }
        for segment in relative.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(UriError::FilePath(relative.to_string()));
            }
            uri.push('/');
            for byte in segment.bytes() {
                if is_unreserved(byte) {
                    uri.push(byte as char);
                } else {
                    uri.push_str(&format!("%{byte:02X}"));
                }
            }
        }
        Ok(uri)
    }

    /// True when `uri` names a file inside this skill's directory.
    pub fn contains(&self, uri: &str) -> bool {
        uri.strip_prefix(&self.root_uri())
            .is_some_and(|rest| rest.len() > 1 && rest.starts_with('/'))
    }
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

fn is_prefix_segment(s: &str) -> bool {
    !s.is_empty() && s != "." && s != ".." && s.bytes().all(is_unreserved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> SkillName {
        SkillName::parse(s).unwrap()
    }

    /// S-G5: the final segment of `<skill-path>` is the skill name, and the spec's
    /// resource-mapping examples are reproduced exactly.
    #[test]
    fn s_g5_final_segment_is_skill_name() {
        let flat = SkillPath::new([], name("git-workflow")).unwrap();
        assert_eq!(flat.skill_md_uri(), "skill://git-workflow/SKILL.md");
        assert_eq!(flat.root_uri(), "skill://git-workflow");

        let nested = SkillPath::new(["acme", "billing"], name("refunds")).unwrap();
        assert_eq!(
            nested.skill_md_uri(),
            "skill://acme/billing/refunds/SKILL.md"
        );
        assert_eq!(
            nested.file_uri("examples/email.md").unwrap(),
            "skill://acme/billing/refunds/examples/email.md"
        );
        assert_eq!(nested.name().as_str(), "refunds");
    }

    /// ADR-033 §Tests: distinct skill identities produce distinct URIs, including
    /// two skills sharing a name under different prefixes.
    #[test]
    fn distinct_paths_produce_distinct_uris() {
        let a = SkillPath::new(["acme", "billing"], name("refunds")).unwrap();
        let b = SkillPath::new(["acme", "support"], name("refunds")).unwrap();
        let c = SkillPath::new([], name("refunds")).unwrap();
        let uris = [a.skill_md_uri(), b.skill_md_uri(), c.skill_md_uri()];
        assert_ne!(uris[0], uris[1]);
        assert_ne!(uris[0], uris[2]);
        assert_ne!(uris[1], uris[2]);
        assert!(!a.contains(&b.skill_md_uri()));
    }

    #[test]
    fn prefix_segment_rules() {
        for bad in ["", ".", "..", "a b", "a/b", "ä", "a%20"] {
            assert_eq!(
                SkillPath::new([bad], name("x")),
                Err(UriError::PrefixSegment(bad.to_string())),
                "{bad:?}"
            );
        }
        assert!(SkillPath::new(["a-b.c_d~e"], name("x")).is_ok());
    }

    #[test]
    fn file_path_rules() {
        let p = SkillPath::new([], name("x")).unwrap();
        for bad in ["", "/a", "a/", "a//b", "./a", "a/../b", ".."] {
            assert!(p.file_uri(bad).is_err(), "{bad:?}");
        }
        assert_eq!(p.file_uri("a b.md").unwrap(), "skill://x/a%20b.md");
        assert_eq!(p.file_uri("{x}.md").unwrap(), "skill://x/%7Bx%7D.md");
        assert!(p.contains("skill://x/SKILL.md"));
        assert!(!p.contains("skill://xy/SKILL.md"));
        assert!(!p.contains("skill://x"));
        assert!(!p.contains("skill://x/"));
    }
}
