//! Wire types for `skills/list` and `skills/get` (`skills.mdx` §Skill Entries,
//! §Listing Skills, §Getting a Skill).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use turul_mcp_protocol_2026_07_28::caching::{CacheScope, ttl_ms_serde};
use turul_mcp_protocol_2026_07_28::meta::{Cursor, RequestMetaObject};
use turul_mcp_protocol_2026_07_28::resources::Resource;
use turul_mcp_protocol_2026_07_28::result_type::ResultType;

use super::manifest::SkillResource;

/// `skills/list`. Its params are the core `PaginatedRequestParams`.
pub const METHOD_SKILLS_LIST: &str = "skills/list";
/// `skills/get`.
pub const METHOD_SKILLS_GET: &str = "skills/get";
/// `resources/directory/read`, defined by this extension.
pub const METHOD_RESOURCES_DIRECTORY_READ: &str = "resources/directory/read";
/// Reverse-domain prefix for `_meta` keys this extension defines.
pub const META_PREFIX: &str = "io.modelcontextprotocol.skills/";
/// `"dynamic"`: a skill whose content cannot carry stable digests.
pub const RESOURCES_DYNAMIC: &str = "dynamic";

/// `Skill.resources`: a complete manifest, or `"dynamic"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillResources {
    /// Every file of the skill, each once.
    Manifest(Vec<SkillResource>),
    /// Generated content; no integrity.
    Dynamic,
}

impl Serialize for SkillResources {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Manifest(entries) => entries.serialize(s),
            Self::Dynamic => s.serialize_str(RESOURCES_DYNAMIC),
        }
    }
}

impl<'de> Deserialize<'de> for SkillResources {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Value::deserialize(d)? {
            Value::String(s) if s == RESOURCES_DYNAMIC => Ok(Self::Dynamic),
            v @ Value::Array(_) => serde_json::from_value(v)
                .map(Self::Manifest)
                .map_err(serde::de::Error::custom),
            other => Err(serde::de::Error::custom(format!(
                "resources must be an array or \"dynamic\", got {other}"
            ))),
        }
    }
}

/// A skill entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    /// URI of the skill's `SKILL.md`.
    pub uri: String,
    /// The frontmatter as a JSON object.
    pub frontmatter: Map<String, Value>,
    /// REQUIRED.
    pub resources: SkillResources,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

/// `skills/list` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSkillsResult {
    /// MUST be `"complete"`.
    #[serde(default)]
    pub result_type: ResultType,
    pub skills: Vec<Skill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    #[serde(with = "ttl_ms_serde")]
    pub ttl_ms: f64,
    pub cache_scope: CacheScope,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

/// `skills/get` params.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSkillRequestParams {
    /// URI of the skill's `SKILL.md`.
    pub uri: String,
    /// Required request metadata.
    #[serde(rename = "_meta")]
    pub meta: RequestMetaObject,
}

/// `skills/get` result. Carries no cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSkillResult {
    /// MUST be `"complete"`.
    #[serde(default)]
    pub result_type: ResultType,
    pub skill: Skill,
    #[serde(with = "ttl_ms_serde")]
    pub ttl_ms: f64,
    pub cache_scope: CacheScope,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

/// `mimeType` of a directory resource (`skills.mdx` §Directory Resources).
pub const MIME_TYPE_DIRECTORY: &str = "inode/directory";

/// `resources/directory/read` params (`ReadResourceDirectoryRequestParams`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceDirectoryRequestParams {
    /// URI of the directory resource to read.
    pub uri: String,
    /// Pagination cursor from a previous page's `nextCursor`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Required request metadata.
    #[serde(rename = "_meta")]
    pub meta: RequestMetaObject,
}

/// `resources/directory/read` result: the directory's direct children.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceDirectoryResult {
    /// MUST be `"complete"`.
    #[serde(default)]
    pub result_type: ResultType,
    /// Files with their `Resource` metadata; subdirectories as directory resources.
    pub resources: Vec<Resource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    #[serde(with = "ttl_ms_serde")]
    pub ttl_ms: f64,
    pub cache_scope: CacheScope,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}
