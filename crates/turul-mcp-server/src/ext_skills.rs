//! Server-side runtime for the Skills extension (`io.modelcontextprotocol/skills`)
//! on the 2026-07-28 lane (ADR-033 §5, §7, §8).
//!
//! A [`SkillCatalog`] holds built [`SkillEntry`] values and [`DynamicSkill`]
//! generators; entries, manifests, digests, frontmatter and URIs all come from
//! `turul-mcp-ext-skills`. This module serves them: `skills/list`, `skills/get`,
//! `resources/directory/read`, one static resource per manifest file, and one
//! `{+file}` template resource per dynamic skill.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use serde_json::Value;
use turul_mcp_ext_skills::v2026_07_28::{
    GetSkillRequestParams, GetSkillResult, ListSkillsResult, METHOD_RESOURCES_DIRECTORY_READ,
    METHOD_SKILLS_GET, METHOD_SKILLS_LIST, MIME_TYPE_DIRECTORY, ReadResourceDirectoryRequestParams,
    ReadResourceDirectoryResult, Skill, SkillEntry, SkillFile, SkillPath, SkillResources,
};
use turul_mcp_protocol::caching::CacheScope;
use turul_mcp_protocol::json_rpc::PaginatedRequestParams;
use turul_mcp_protocol::meta::Cursor;
use turul_mcp_protocol::resources::{Resource, ResourceContent};
use turul_mcp_protocol::result_type::ResultType;

use crate::handlers::McpHandler;
use crate::prelude::*;
use crate::security::SecurityMiddleware;

/// Entries per `skills/list` and `resources/directory/read` page.
const PAGE_SIZE: usize = 50;

/// A skill whose files are generated on demand (`resources: "dynamic"`,
/// ADR-033 §8). The path is fixed; `files()` returns the skill's current
/// files and is called for every operation that needs them.
#[async_trait]
pub trait DynamicSkill: Send + Sync {
    /// Where the skill sits in the namespace; its final segment is its name.
    fn path(&self) -> SkillPath;
    /// The skill's current files, `SKILL.md` included.
    async fn files(&self) -> McpResult<Vec<SkillFile>>;
}

/// The skills a server publishes, keyed by `SKILL.md` URI.
#[derive(Clone, Default)]
pub struct SkillCatalog {
    entries: BTreeMap<String, SkillEntry>,
    dynamic: BTreeMap<String, (SkillPath, Arc<dyn DynamicSkill>)>,
}

impl std::fmt::Debug for SkillCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillCatalog")
            .field("entries", &self.entries.keys().collect::<Vec<_>>())
            .field("dynamic", &self.dynamic.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl SkillCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a skill. Two skills at the same path are a configuration error.
    pub fn add(&mut self, entry: SkillEntry) -> McpResult<()> {
        let uri = entry.uri();
        self.ensure_new(&uri)?;
        self.entries.insert(uri, entry);
        Ok(())
    }

    /// Add a generated skill (ADR-033 §8).
    pub fn add_dynamic(&mut self, skill: Arc<dyn DynamicSkill>) -> McpResult<()> {
        let path = skill.path();
        let uri = path.skill_md_uri();
        self.ensure_new(&uri)?;
        self.dynamic.insert(uri, (path, skill));
        Ok(())
    }

    fn ensure_new(&self, uri: &str) -> McpResult<()> {
        if self.entries.contains_key(uri) || self.dynamic.contains_key(uri) {
            return Err(McpError::configuration(&format!(
                "skill {uri} is added more than once"
            )));
        }
        Ok(())
    }

    pub fn entries(&self) -> impl Iterator<Item = &SkillEntry> {
        self.entries.values()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.dynamic.is_empty()
    }

    /// Roots of dynamic skills, for template registration.
    pub(crate) fn dynamic_skills(
        &self,
    ) -> impl Iterator<Item = &(SkillPath, Arc<dyn DynamicSkill>)> {
        self.dynamic.values()
    }

    /// A dynamic skill may not share or nest with another skill's directory
    /// (ADR-033 §8): its template would answer for the other skill's files.
    pub(crate) fn overlap_errors(&self) -> Vec<String> {
        let roots: Vec<(String, bool)> = self
            .entries
            .values()
            .map(|e| (e.path().root_uri(), false))
            .chain(self.dynamic.values().map(|(p, _)| (p.root_uri(), true)))
            .collect();
        let inside = |a: &str, b: &str| a == b || a.starts_with(&format!("{b}/"));
        let mut errors = Vec::new();
        for (i, (a, a_dyn)) in roots.iter().enumerate() {
            for (b, b_dyn) in roots.iter().skip(i + 1) {
                if (*a_dyn || *b_dyn) && (inside(a, b) || inside(b, a)) {
                    errors.push(format!(
                        "dynamic skill directory {a} overlaps {b} (ADR-033 §8)"
                    ));
                }
            }
        }
        errors
    }
}

/// Directory children: child URI → its `Resource` metadata (ADR-033 §7).
pub(crate) type DirectoryIndex = BTreeMap<String, BTreeMap<String, Resource>>;

/// Index every directory of `files` under its skill roots. File children carry
/// the metadata `resources/list` reports (`resource_to_descriptor`), so the two
/// cannot disagree.
pub(crate) fn directory_index<'a>(
    roots: impl IntoIterator<Item = String>,
    files: impl IntoIterator<Item = &'a SkillFileResource>,
) -> DirectoryIndex {
    let roots: Vec<String> = roots.into_iter().collect();
    let mut index: DirectoryIndex = BTreeMap::new();
    for root in &roots {
        index.entry(root.clone()).or_default();
    }
    for file in files {
        // Walk from the outermost root containing the file, so a nested
        // skill's directory is also a child of its enclosing skill's.
        let Some(root) = roots
            .iter()
            .filter(|r| file.uri().starts_with(&format!("{r}/")))
            .min_by_key(|r| r.len())
        else {
            continue;
        };
        let relative = &file.uri()[root.len() + 1..];
        let mut dir = root.clone();
        let segments: Vec<&str> = relative.split('/').collect();
        for segment in &segments[..segments.len() - 1] {
            let child = format!("{dir}/{segment}");
            index.entry(dir.clone()).or_default().insert(
                child.clone(),
                Resource::new(child.clone(), *segment).with_mime_type(MIME_TYPE_DIRECTORY),
            );
            index.entry(child.clone()).or_default();
            dir = child;
        }
        index.entry(dir).or_default().insert(
            file.uri().to_string(),
            crate::resource::resource_to_descriptor(file),
        );
    }
    index
}

/// Everything the Skills handlers share.
pub(crate) struct SkillsRuntime {
    pub(crate) catalog: SkillCatalog,
    /// Static directory index (ADR-033 §7).
    pub(crate) directories: DirectoryIndex,
    /// The `resources/read` policy: a generated file it would refuse is never
    /// served (ADR-033 §5 Q10, §8).
    pub(crate) policy: Arc<SecurityMiddleware>,
}

impl SkillsRuntime {
    /// Generate a dynamic skill's current entry, through the same owners as a
    /// static one; a generated URI the read policy refuses is an error.
    async fn generate(
        &self,
        path: &SkillPath,
        skill: &Arc<dyn DynamicSkill>,
    ) -> McpResult<SkillEntry> {
        let entry = SkillEntry::from_files(path.clone(), skill.files().await?)
            .map_err(|e| McpError::tool_execution(&format!("dynamic skill: {e}")))?;
        for (res, _) in entry.files() {
            let probe = serde_json::json!({ "uri": res.uri() });
            self.policy
                .validate_request("resources/read", Some(&probe), None)
                .map_err(|e| {
                    McpError::tool_execution(&format!(
                        "dynamic skill file {} would be unreadable ({e})",
                        res.uri()
                    ))
                })?;
        }
        Ok(entry)
    }

    /// Every served skill's `SKILL.md` URI, in order.
    fn uris(&self) -> Vec<&String> {
        let mut uris: Vec<&String> = self
            .catalog
            .entries
            .keys()
            .chain(self.catalog.dynamic.keys())
            .collect();
        uris.sort();
        uris
    }

    /// The wire entry for `uri`, or `None` if no skill is served there.
    async fn skill(&self, uri: &str) -> McpResult<Option<Skill>> {
        if let Some(e) = self.catalog.entries.get(uri) {
            return Ok(Some(e.to_wire()));
        }
        let Some((path, skill)) = self.catalog.dynamic.get(uri) else {
            return Ok(None);
        };
        let mut wire = self.generate(path, skill).await?.to_wire();
        wire.resources = SkillResources::Dynamic;
        Ok(Some(wire))
    }

    /// A directory's children, or `None` if `uri` is not a directory resource.
    async fn children(&self, uri: &str) -> McpResult<Option<BTreeMap<String, Resource>>> {
        if let Some(children) = self.directories.get(uri) {
            return Ok(Some(children.clone()));
        }
        for (path, skill) in self.catalog.dynamic.values() {
            let root = path.root_uri();
            if uri == root || uri.starts_with(&format!("{root}/")) {
                let entry = self.generate(path, skill).await?;
                let files = SkillFileResource::for_entry(&entry);
                return Ok(directory_index([root], files.iter()).remove(uri));
            }
        }
        Ok(None)
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
        Ok(vec![content_of(&self.uri, &self.bytes, self.mime_type)])
    }
}

fn params<T: serde::de::DeserializeOwned>(params: Option<Value>, what: &str) -> McpResult<T> {
    let params = params.ok_or_else(|| McpError::missing_param(what))?;
    serde_json::from_value(params)
        .map_err(|e| McpError::InvalidParameters(format!("invalid {what}: {e}")))
}

/// Exactly the given bytes: UTF-8 as text, else blob.
fn content_of(uri: &str, bytes: &[u8], mime_type: &str) -> ResourceContent {
    let content = match std::str::from_utf8(bytes) {
        Ok(text) => ResourceContent::text(uri, text),
        Err(_) => ResourceContent::blob(
            uri,
            base64::engine::general_purpose::STANDARD.encode(bytes),
            mime_type,
        ),
    };
    content.with_mime_type(mime_type)
}

/// Page `items` after `cursor`, which must be an item a previous page ended on.
fn page<'a, T>(
    items: impl Iterator<Item = (&'a String, T)>,
    cursor: Option<&Cursor>,
) -> McpResult<(Vec<T>, Option<Cursor>)> {
    let items: Vec<(&String, T)> = items.collect();
    let skip = match cursor {
        None => 0,
        Some(Cursor(after)) => {
            items.iter().position(|(k, _)| *k == after).ok_or_else(|| {
                McpError::InvalidParameters(format!("invalid pagination cursor {after:?}"))
            })? + 1
        }
    };
    let total = items.len();
    let mut out = Vec::new();
    let mut last = None;
    for (k, v) in items.into_iter().skip(skip).take(PAGE_SIZE) {
        last = Some(k.clone());
        out.push(v);
    }
    let next = (skip + out.len() < total).then(|| Cursor(last.expect("non-empty page")));
    Ok((out, next))
}

/// `skills/list`: stable URI order, entries never split across pages.
pub(crate) struct SkillsListHandler(pub(crate) Arc<SkillsRuntime>);

#[async_trait]
impl McpHandler for SkillsListHandler {
    async fn handle(&self, p: Option<Value>) -> McpResult<Value> {
        let p: PaginatedRequestParams = params(p, "skills/list params")?;
        let uris = self.0.uris();
        let (page_uris, next_cursor) = page(uris.into_iter().map(|u| (u, u)), p.cursor.as_ref())?;
        let mut skills = Vec::with_capacity(page_uris.len());
        for uri in page_uris {
            skills.push(self.0.skill(uri).await?.expect("listed skill is served"));
        }
        let result = ListSkillsResult {
            result_type: ResultType::Complete,
            skills,
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
pub(crate) struct SkillsGetHandler(pub(crate) Arc<SkillsRuntime>);

#[async_trait]
impl McpHandler for SkillsGetHandler {
    async fn handle(&self, p: Option<Value>) -> McpResult<Value> {
        let p: GetSkillRequestParams = params(p, "skills/get params")?;
        let skill = self.0.skill(&p.uri).await?.ok_or_else(|| {
            McpError::InvalidParameters(format!("No skill is served at {}", p.uri))
        })?;
        let result = GetSkillResult {
            result_type: ResultType::Complete,
            skill,
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

/// `resources/directory/read` (ADR-033 §7): a directory's direct children.
pub(crate) struct DirectoryReadHandler(pub(crate) Arc<SkillsRuntime>);

#[async_trait]
impl McpHandler for DirectoryReadHandler {
    async fn handle(&self, p: Option<Value>) -> McpResult<Value> {
        let p: ReadResourceDirectoryRequestParams = params(p, "resources/directory/read params")?;
        let children = self.0.children(&p.uri).await?.ok_or_else(|| {
            McpError::InvalidParameters(format!("{} is not a directory resource", p.uri))
        })?;
        let (resources, next_cursor) = page(children.iter(), p.cursor.as_ref())?;
        let result = ReadResourceDirectoryResult {
            result_type: ResultType::Complete,
            resources: resources.into_iter().cloned().collect(),
            next_cursor,
            ttl_ms: 0.0,
            cache_scope: CacheScope::Public,
            meta: None,
        };
        Ok(serde_json::to_value(result)?)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![METHOD_RESOURCES_DIRECTORY_READ.to_string()]
    }
}

/// Filled once `with_ext_skills` has assembled the runtime; dynamic template
/// resources are registered before the read policy that covers them exists.
pub(crate) type RuntimeSlot = Arc<std::sync::OnceLock<Arc<SkillsRuntime>>>;

/// The `<root>/{+file}` template resource of one dynamic skill (ADR-033 §8):
/// each read regenerates the skill and serves the file at the requested URI.
pub(crate) struct DynamicSkillResource {
    template: String,
    name: String,
    path: SkillPath,
    skill: Arc<dyn DynamicSkill>,
    runtime: RuntimeSlot,
}

impl DynamicSkillResource {
    pub(crate) fn new(path: SkillPath, skill: Arc<dyn DynamicSkill>, runtime: RuntimeSlot) -> Self {
        Self {
            template: format!("{}/{{+file}}", path.root_uri()),
            name: path.name().as_str().to_string(),
            path,
            skill,
            runtime,
        }
    }
}

impl HasResourceMetadata for DynamicSkillResource {
    fn name(&self) -> &str {
        &self.name
    }
}
impl HasResourceUri for DynamicSkillResource {
    fn uri(&self) -> &str {
        &self.template
    }
}
impl HasResourceDescription for DynamicSkillResource {}
impl HasResourceMimeType for DynamicSkillResource {}
impl HasResourceSize for DynamicSkillResource {}
impl HasResourceAnnotations for DynamicSkillResource {}
impl HasResourceMeta for DynamicSkillResource {}
impl HasIcons for DynamicSkillResource {}

#[async_trait]
impl McpResource for DynamicSkillResource {
    async fn read(
        &self,
        params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let uri = params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::missing_param("uri"))?;
        let runtime = self
            .runtime
            .get()
            .ok_or_else(|| McpError::configuration("dynamic skill runtime not bound"))?;
        let entry = runtime.generate(&self.path, &self.skill).await?;
        let file = SkillFileResource::for_entry(&entry)
            .into_iter()
            .find(|f| f.uri() == uri)
            .ok_or_else(|| {
                McpError::InvalidParameters(format!("No skill file is served at {uri}"))
            })?;
        Ok(vec![content_of(&file.uri, &file.bytes, file.mime_type)])
    }
}
