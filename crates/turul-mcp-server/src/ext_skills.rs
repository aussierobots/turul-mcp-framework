//! Server-side runtime for the Skills extension (`io.modelcontextprotocol/skills`)
//! on the 2026-07-28 lane (ADR-033 §5).
//!
//! A [`SkillCatalog`] holds fully built [`SkillEntry`] values; entries,
//! manifests, digests and URIs all come from `turul-mcp-ext-skills`, and this
//! module only serves them: `skills/list`, `skills/get`, and one static resource
//! per manifest file serving exactly the hashed bytes.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use serde_json::Value;
use turul_mcp_ext_skills::v2026_07_28::{
    GetSkillRequestParams, GetSkillResult, ListSkillsResult, METHOD_SKILLS_GET, METHOD_SKILLS_LIST,
    SkillEntry,
};
use turul_mcp_protocol::caching::CacheScope;
use turul_mcp_protocol::json_rpc::PaginatedRequestParams;
use turul_mcp_protocol::meta::Cursor;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_protocol::result_type::ResultType;

use crate::handlers::McpHandler;
use crate::prelude::*;

/// Entries per `skills/list` page.
const PAGE_SIZE: usize = 50;

/// The skills a server publishes, keyed by `SKILL.md` URI.
#[derive(Debug, Clone, Default)]
pub struct SkillCatalog {
    entries: BTreeMap<String, SkillEntry>,
}

impl SkillCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a skill. Two skills at the same path are a configuration error.
    pub fn add(&mut self, entry: SkillEntry) -> McpResult<()> {
        let uri = entry.uri();
        if self.entries.contains_key(&uri) {
            return Err(McpError::configuration(&format!(
                "skill {uri} is added more than once"
            )));
        }
        self.entries.insert(uri, entry);
        Ok(())
    }

    pub fn entries(&self) -> impl Iterator<Item = &SkillEntry> {
        self.entries.values()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One manifest file served through `resources/read`.
pub(crate) struct SkillFileResource {
    uri: String,
    digest: String,
    /// This file is the entry's own `SKILL.md`, not a supporting file.
    own_skill_md: bool,
    name: String,
    description: Option<String>,
    mime_type: &'static str,
    bytes: Arc<[u8]>,
}

impl SkillFileResource {
    /// One resource per manifest file of `entry`. `SKILL.md` carries the skill's
    /// name and description (`skills.mdx` §Resource Metadata).
    pub(crate) fn for_entry(entry: &SkillEntry) -> Vec<Self> {
        let skill_md = entry.uri();
        let root = entry.path().root_uri();
        entry
            .files()
            .map(|(res, bytes)| {
                let is_skill_md = res.uri() == skill_md;
                let relative = res.uri()[root.len() + 1..].to_string();
                Self {
                    uri: res.uri().to_string(),
                    digest: res.digest().to_string(),
                    own_skill_md: is_skill_md,
                    name: if is_skill_md {
                        entry.frontmatter().name().to_string()
                    } else {
                        relative.clone()
                    },
                    description: is_skill_md.then(|| entry.frontmatter().description().to_string()),
                    mime_type: mime_type(&relative, bytes),
                    bytes: bytes.into(),
                }
            })
            .collect()
    }

    pub(crate) fn uri(&self) -> &str {
        &self.uri
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn own_skill_md(&self) -> bool {
        self.own_skill_md
    }
}

fn mime_type(relative: &str, bytes: &[u8]) -> &'static str {
    let ext = relative
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        _ if std::str::from_utf8(bytes).is_ok() => "text/plain",
        _ => "application/octet-stream",
    }
}

impl HasResourceMetadata for SkillFileResource {
    fn name(&self) -> &str {
        &self.name
    }
}
impl HasResourceUri for SkillFileResource {
    fn uri(&self) -> &str {
        &self.uri
    }
}
impl HasResourceDescription for SkillFileResource {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
impl HasResourceMimeType for SkillFileResource {
    fn mime_type(&self) -> Option<&str> {
        Some(self.mime_type)
    }
}
impl HasResourceSize for SkillFileResource {
    fn size(&self) -> Option<u64> {
        Some(self.bytes.len() as u64)
    }
}
impl HasResourceAnnotations for SkillFileResource {}
impl HasResourceMeta for SkillFileResource {}
impl HasIcons for SkillFileResource {}

#[async_trait]
impl McpResource for SkillFileResource {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        // Exactly the bytes the manifest digest covers: UTF-8 as text, else blob.
        let content = match std::str::from_utf8(&self.bytes) {
            Ok(text) => ResourceContent::text(&self.uri, text),
            Err(_) => ResourceContent::blob(
                &self.uri,
                base64::engine::general_purpose::STANDARD.encode(&self.bytes),
                self.mime_type,
            ),
        };
        Ok(vec![content.with_mime_type(self.mime_type)])
    }
}

fn params<T: serde::de::DeserializeOwned>(params: Option<Value>, what: &str) -> McpResult<T> {
    let params = params.ok_or_else(|| McpError::missing_param(what))?;
    serde_json::from_value(params)
        .map_err(|e| McpError::InvalidParameters(format!("invalid {what}: {e}")))
}

/// `skills/list`: stable URI order, entries never split across pages.
pub(crate) struct SkillsListHandler(pub(crate) Arc<SkillCatalog>);

#[async_trait]
impl McpHandler for SkillsListHandler {
    async fn handle(&self, p: Option<Value>) -> McpResult<Value> {
        let p: PaginatedRequestParams = params(p, "skills/list params")?;
        let skip = match &p.cursor {
            None => 0,
            // A cursor is the last URI of a page this server served.
            Some(Cursor(after)) => {
                self.0
                    .entries
                    .keys()
                    .position(|k| k == after)
                    .ok_or_else(|| {
                        McpError::InvalidParameters(format!("invalid pagination cursor {after:?}"))
                    })?
                    + 1
            }
        };
        let page: Vec<&SkillEntry> = self.0.entries().skip(skip).take(PAGE_SIZE).collect();
        let next_cursor = (skip + page.len() < self.0.entries.len())
            .then(|| Cursor(page.last().expect("non-empty page").uri()));
        let result = ListSkillsResult {
            result_type: ResultType::Complete,
            skills: page.iter().map(|e| e.to_wire()).collect(),
            next_cursor,
            ttl_ms: 0.0,
            cache_scope: CacheScope::Public,
            meta: None,
        };
        Ok(serde_json::to_value(result)?)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![METHOD_SKILLS_LIST.to_string()]
    }
}

/// `skills/get`: any served skill by its `SKILL.md` URI, listed or not.
pub(crate) struct SkillsGetHandler(pub(crate) Arc<SkillCatalog>);

#[async_trait]
impl McpHandler for SkillsGetHandler {
    async fn handle(&self, p: Option<Value>) -> McpResult<Value> {
        let p: GetSkillRequestParams = params(p, "skills/get params")?;
        let entry = self.0.entries.get(&p.uri).ok_or_else(|| {
            McpError::InvalidParameters(format!("No skill is served at {}", p.uri))
        })?;
        let result = GetSkillResult {
            result_type: ResultType::Complete,
            skill: entry.to_wire(),
            ttl_ms: 0.0,
            cache_scope: CacheScope::Public,
            meta: None,
        };
        Ok(serde_json::to_value(result)?)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![METHOD_SKILLS_GET.to_string()]
    }
}
