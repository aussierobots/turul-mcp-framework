//! The Skills extension surface for the 2026-07-28 spec line (ADR-033).

pub mod capability;
pub mod entry;
pub mod frontmatter;
pub mod manifest;
pub mod types;
pub mod uri;

#[cfg(test)]
mod compliance_test;

pub use capability::{EXTENSION_IDENTIFIER, SkillsCapability, capability, declared_by_server};
pub use entry::{EntryError, SkillEntry, SkillFile};
pub use frontmatter::{Frontmatter, FrontmatterError};
pub use manifest::{
    MAX_RESOURCES_PER_SKILL, MAX_TOTAL_BYTES_PER_SKILL, Manifest, ManifestError, SkillResource,
};
pub use types::{
    GetSkillRequestParams, GetSkillResult, ListSkillsResult, META_PREFIX,
    METHOD_RESOURCES_DIRECTORY_READ, METHOD_SKILLS_GET, METHOD_SKILLS_LIST, MIME_TYPE_DIRECTORY,
    RESOURCES_DYNAMIC, ReadResourceDirectoryRequestParams, ReadResourceDirectoryResult, Skill,
    SkillResources,
};
pub use uri::{SkillPath, UriError};
