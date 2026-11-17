//! `Skill.frontmatter` (`skills.mdx` §Frontmatter). [`Frontmatter::from_skill_md`]
//! projects `turul-agent-skills`' parse into JSON — scalars typed by its YAML 1.2
//! core resolver — and refuses anything a host could read differently.

use serde_json::{Map, Value};
use turul_agent_skills::{CoreScalar, RawValue, parse_skill_md};

/// Why a `SKILL.md` frontmatter cannot be served verbatim.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrontmatterError {
    /// The document has no parseable frontmatter.
    #[error("SKILL.md frontmatter does not parse: {0}")]
    Parse(String),
    /// The YAML only parsed after lenient repair, so a strict host parse disagrees.
    #[error("SKILL.md frontmatter needed lenient repair ({0}); quote the value")]
    Repaired(String),
    /// A plain scalar YAML 1.1 and YAML 1.2 read differently (`yes`, `0777`, timestamps).
    #[error("frontmatter value at {0} reads differently under YAML 1.1 and 1.2; quote it")]
    SchemaDisagreement(String),
    /// A value JSON cannot carry exactly: an integer outside `i64`, `.inf` or `.nan`.
    #[error("frontmatter value at {0} has no exact JSON form; quote it")]
    Unrepresentable(String),
    /// `name` or `description` is absent or not a string.
    #[error("frontmatter field {0:?} is required and must be a string")]
    Required(&'static str),
}

/// The frontmatter as a JSON object, `name` and `description` guaranteed strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter(Map<String, Value>);

impl Frontmatter {
    /// Projects the frontmatter of `skill_md`.
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
        RawValue::Str { text, .. } => {
            let unrepresentable = || FrontmatterError::Unrepresentable(at.to_string());
            let resolved = value.core_scalar().ok_or_else(unrepresentable)?;
            if !resolved.yaml_1_1_agrees {
                return Err(FrontmatterError::SchemaDisagreement(at.to_string()));
            }
            match resolved.value {
                CoreScalar::Str => Ok(Value::String(text.clone())),
                CoreScalar::Null => Ok(Value::Null),
                CoreScalar::Bool(b) => Ok(Value::Bool(b)),
                CoreScalar::Int(i) => Ok(Value::from(i)),
                CoreScalar::Float(f) => serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .ok_or_else(unrepresentable),
                // IntOutOfRange, and shapes of the non_exhaustive enum this crate
                // does not know: no exact JSON form.
                _ => Err(unrepresentable()),
            }
        }
        // `RawValue` is non_exhaustive: a shape this crate does not know cannot be
        // rendered faithfully.
        _ => Err(FrontmatterError::Unrepresentable(at.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DOC: &str = "---\nname: refunds\ndescription: Process refunds\nlicense: Apache-2.0\nmetadata:\n  author: acme\n  version: \"1.0\"\ntags:\n  - a\n  - 'b'\n---\nBody\n";

    /// Every field the author wrote passes through, nested shapes included.
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

    fn doc(extra: &str) -> String {
        format!("---\nname: a\ndescription: d\n{extra}\n---\n")
    }

    /// Plain scalars both schemas agree on are served with their
    /// YAML 1.2 core type; quoted scalars stay strings.
    #[test]
    fn s_g3_typed_scalars_served() {
        for (extra, key, want) in [
            ("version: 1.0", "version", json!(1.0)),
            ("n: 3", "n", json!(3)),
            ("n: -42", "n", json!(-42)),
            ("n: 0x1F", "n", json!(31)),
            ("flag: true", "flag", json!(true)),
            ("flag: False", "flag", json!(false)),
            ("empty:", "empty", Value::Null),
            ("tilde: ~", "tilde", Value::Null),
            ("q: \"1.0\"", "q", json!("1.0")),
            ("q: 'yes'", "q", json!("yes")),
            ("metadata:\n  n: 3", "metadata", json!({ "n": 3 })),
        ] {
            let fm =
                Frontmatter::from_skill_md(&doc(extra)).unwrap_or_else(|e| panic!("{extra}: {e}"));
            assert_eq!(fm.as_map()[key], want, "{extra}");
        }
    }

    /// A plain scalar YAML 1.1 reads differently is refused, with its path.
    #[test]
    fn s_g3_schema_disagreement_refused() {
        for (extra, at) in [
            ("flag: yes", "flag"),
            ("flag: off", "flag"),
            ("mode: 0777", "mode"),
            ("t: 1:30", "t"),
            ("n: 1e3", "n"),
            ("metadata:\n  big: 1_000", "metadata.big"),
            ("t:\n  - x\n  - 2026-01-01", "t[1]"),
        ] {
            assert_eq!(
                Frontmatter::from_skill_md(&doc(extra)),
                Err(FrontmatterError::SchemaDisagreement(at.to_string())),
                "{extra}"
            );
        }
    }

    /// Values JSON cannot carry exactly are refused, with their path.
    #[test]
    fn s_g3_unrepresentable_refused() {
        for (extra, at) in [
            ("n: 9223372036854775808", "n"),
            ("f: .inf", "f"),
            ("f: .nan", "f"),
        ] {
            assert_eq!(
                Frontmatter::from_skill_md(&doc(extra)),
                Err(FrontmatterError::Unrepresentable(at.to_string())),
                "{extra}"
            );
        }
    }

    /// `name` and `description` must still be strings once typed.
    #[test]
    fn s_g3_required_fields_must_be_strings() {
        assert_eq!(
            Frontmatter::from_skill_md("---\nname: true\ndescription: d\n---\n"),
            Err(FrontmatterError::Required("name"))
        );
    }

    /// Lenient repair means a strict host parse would disagree.
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

    /// What we serve equals an independent YAML parse of the same
    /// bytes — `serde_yml`, not `turul-agent-skills`' parser.
    #[test]
    fn s_g3_independent_parse_agrees() {
        const TYPED: &str = "---\nname: a\ndescription: d\nversion: 1.5\nn: 3\nflag: true\nnone:\nq: \"2\"\nmeta:\n  xs:\n    - 1\n    - two\n    - false\n---\n";
        for doc in [DOC, TYPED] {
            let yaml = doc
                .strip_prefix("---\n")
                .and_then(|s| s.split_once("\n---\n"))
                .unwrap()
                .0;
            let independent: Value = serde_yml::from_str(yaml).unwrap();
            let ours = Value::Object(Frontmatter::from_skill_md(doc).unwrap().as_map().clone());
            assert_eq!(ours, independent, "{doc}");
        }
    }
}
