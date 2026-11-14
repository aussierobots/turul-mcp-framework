//! `Skill.frontmatter` (`skills.mdx` §Frontmatter). [`Frontmatter::from_skill_md`]
//! projects `turul-agent-skills`' parse into JSON and refuses anything it cannot
//! render faithfully (ADR-033 §3).

use serde_json::{Map, Value};
use turul_agent_skills::{RawValue, parse_skill_md};

/// Why a `SKILL.md` frontmatter cannot be served verbatim.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrontmatterError {
    /// The document has no parseable frontmatter.
    #[error("SKILL.md frontmatter does not parse: {0}")]
    Parse(String),
    /// The YAML only parsed after lenient repair, so a strict host parse disagrees.
    #[error("SKILL.md frontmatter needed lenient repair ({0}); quote the value")]
    Repaired(String),
    /// A plain scalar a YAML parser would read as null, bool, number or timestamp.
    #[error(
        "frontmatter value at {0} would not be a string to a YAML parser; quote it (typed values are not served yet)"
    )]
    Typed(String),
    /// `name` or `description` is absent or not a string.
    #[error("frontmatter field {0:?} is required and must be a string")]
    Required(&'static str),
}

/// The frontmatter as a JSON object, `name` and `description` guaranteed strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter(Map<String, Value>);

impl Frontmatter {
    /// Project the frontmatter of `skill_md`.
    pub fn from_skill_md(skill_md: &str) -> Result<Self, FrontmatterError> {
        let raw = parse_skill_md(skill_md).map_err(|e| FrontmatterError::Parse(e.to_string()))?;
        if let Some(original) = raw.repaired {
            return Err(FrontmatterError::Repaired(original));
        }
        let mut map = Map::new();
        for (key, value) in &raw.entries {
            map.insert(key.clone(), project(value, key)?);
        }
        for field in ["name", "description"] {
            if !map.get(field).is_some_and(Value::is_string) {
                return Err(FrontmatterError::Required(field));
            }
        }
        Ok(Self(map))
    }

    /// The `name` field.
    pub fn name(&self) -> &str {
        self.0["name"].as_str().expect("checked at construction")
    }

    /// The `description` field.
    pub fn description(&self) -> &str {
        self.0["description"]
            .as_str()
            .expect("checked at construction")
    }

    /// The JSON object.
    pub fn as_map(&self) -> &Map<String, Value> {
        &self.0
    }
}

fn project(value: &RawValue, at: &str) -> Result<Value, FrontmatterError> {
    match value {
        RawValue::Str { typed: true, .. } => Err(FrontmatterError::Typed(at.to_string())),
        RawValue::Str { text, .. } => Ok(Value::String(text.clone())),
        RawValue::Seq(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| project(v, &format!("{at}[{i}]")))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        RawValue::Map(entries) => entries
            .iter()
            .map(|(k, v)| Ok((k.clone(), project(v, &format!("{at}.{k}"))?)))
            .collect::<Result<Map<_, _>, _>>()
            .map(Value::Object),
        // `RawValue` is non_exhaustive: a shape this crate does not know cannot be
        // rendered faithfully.
        _ => Err(FrontmatterError::Typed(at.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DOC: &str = "---\nname: refunds\ndescription: Process refunds\nlicense: Apache-2.0\nmetadata:\n  author: acme\n  version: \"1.0\"\ntags:\n  - a\n  - 'b'\n---\nBody\n";

    /// S-G3: every field the author wrote passes through, nested shapes included.
    #[test]
    fn s_g3_string_fields_round_trip() {
        let fm = Frontmatter::from_skill_md(DOC).unwrap();
        assert_eq!(
            Value::Object(fm.as_map().clone()),
            json!({
                "name": "refunds",
                "description": "Process refunds",
                "license": "Apache-2.0",
                "metadata": { "author": "acme", "version": "1.0" },
                "tags": ["a", "b"]
            })
        );
        assert_eq!(fm.name(), "refunds");
        assert_eq!(fm.description(), "Process refunds");
    }

    /// S-G3 / ADR-033 §3: a value a YAML parser would type is refused, with its path.
    #[test]
    fn s_g3_typed_scalar_refused() {
        for (doc, at) in [
            (
                "---\nname: a\ndescription: d\nversion: 1.0\n---\n",
                "version",
            ),
            ("---\nname: a\ndescription: d\nflag: yes\n---\n", "flag"),
            ("---\nname: a\ndescription: d\nempty:\n---\n", "empty"),
            (
                "---\nname: a\ndescription: d\nmetadata:\n  n: 3\n---\n",
                "metadata.n",
            ),
            (
                "---\nname: a\ndescription: d\nt:\n  - x\n  - 2026-01-01\n---\n",
                "t[1]",
            ),
        ] {
            assert_eq!(
                Frontmatter::from_skill_md(doc),
                Err(FrontmatterError::Typed(at.to_string())),
                "{doc}"
            );
        }
    }

    /// S-G3 / ADR-033 §3: lenient repair means a strict host parse would disagree.
    #[test]
    fn s_g3_repaired_refused() {
        let doc = "---\nname: a\ndescription: Use when: asked\n---\n";
        assert!(matches!(
            Frontmatter::from_skill_md(doc),
            Err(FrontmatterError::Repaired(_))
        ));
    }

    #[test]
    fn s_g3_required_fields() {
        assert_eq!(
            Frontmatter::from_skill_md("---\nname: a\n---\n"),
            Err(FrontmatterError::Required("description"))
        );
    }

    /// S-G3 (:496): what we serve equals an independent YAML parse of the same
    /// bytes — `serde_yml`, not `turul-agent-skills`' parser.
    #[test]
    fn s_g3_independent_parse_agrees() {
        let yaml = DOC
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n"))
            .unwrap()
            .0;
        let independent: Value = serde_yml::from_str(yaml).unwrap();
        let ours = Value::Object(Frontmatter::from_skill_md(DOC).unwrap().as_map().clone());
        assert_eq!(ours, independent);
    }
}
