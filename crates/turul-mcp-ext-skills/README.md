# turul-mcp-ext-skills

Types and producers for the MCP **Skills extension** (`io.modelcontextprotocol/skills`,
SEP-2640, upstream **Beta**): serve [Agent Skills](https://agentskills.io/) over the
Resources primitive with `skills/list` and `skills/get`.

Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server) with its
`ext-skills` feature:

```rust,ignore
use turul_mcp_ext_skills::{SkillEntry, SkillFile, SkillName, SkillPath};
use turul_mcp_server::ext_skills::SkillCatalog;

let mut catalog = SkillCatalog::new();
catalog.add(SkillEntry::from_files(
    SkillPath::new(["acme"], SkillName::parse("refunds")?)?,
    vec![SkillFile::new("SKILL.md", std::fs::read("refunds/SKILL.md")?)],
)?)?;
let server = McpServer::builder().name("skills").with_ext_skills(catalog).build()?;
```

## What this crate owns

| Artifact | Owner |
|---|---|
| Capability value | `capability(SkillsCapability)` |
| `skill://` URIs | `SkillPath` |
| `sha256:` digest and size | `SkillResource::of` |
| Complete manifest, 512-file / 16 MiB limits | `Manifest::new` |
| Verbatim frontmatter JSON | `Frontmatter::from_skill_md` |
| Skill entry | `SkillEntry::from_files` |

Parsing and validating `SKILL.md` belongs to
[`turul-agent-skills`](https://docs.rs/turul-agent-skills); this crate calls it and never
re-implements it.

## Limits of this release

- Frontmatter must be all strings: a plain scalar a YAML parser would read as null,
  bool, number or timestamp is refused (quote it).
- `"dynamic"` resources and `resources/directory/read` are not served.
- The caller supplies a skill's complete file list.

Design and test contract: [ADR-033](../../docs/adr/033-skills-over-mcp.md). Vendored
specification and provenance: [`schema/`](schema/README.md).
