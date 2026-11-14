//! Types for the MCP Skills extension (`io.modelcontextprotocol/skills`), which
//! lets a server publish [Agent Skills](https://agentskills.io/) as resources.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server): enable its
//! `ext-skills` feature and call `.with_ext_skills(catalog)` on the builder. This
//! crate supplies the wire types and the MCP-specific producers: `skill://` URIs
//! ([`SkillPath`]), `sha256:` manifests ([`Manifest`]), verbatim frontmatter
//! ([`Frontmatter`]) and entries ([`SkillEntry`]). Parsing and validating
//! `SKILL.md` belongs to [`turul-agent-skills`](https://docs.rs/turul-agent-skills).
//!
//! # Example
//!
//! ```rust
//! use turul_mcp_ext_skills::{SkillEntry, SkillFile, SkillName, SkillPath};
//!
//! let path = SkillPath::new(["acme"], SkillName::parse("refunds")?)?;
//! let entry = SkillEntry::from_files(
//!     path,
//!     vec![SkillFile::new(
//!         "SKILL.md",
//!         "---\nname: refunds\ndescription: Process refunds.\n---\nSteps.\n",
//!     )],
//! )?;
//! assert_eq!(entry.uri(), "skill://acme/refunds/SKILL.md");
//! assert!(entry.manifest().entries()[0].digest().starts_with("sha256:"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `protocol-2026-07-28` | yes | The extension for MCP 2026-07-28 |

/// The skill-name type, owned by `turul-agent-skills`; re-exported because
/// [`SkillPath::new`] takes one.
pub use turul_agent_skills::SkillName;

#[cfg(feature = "protocol-2026-07-28")]
pub mod v2026_07_28;

#[cfg(feature = "protocol-2026-07-28")]
pub use v2026_07_28::*;
