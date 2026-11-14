# turul-mcp-ext-skills

Types and producers for the MCP **Skills extension** (`io.modelcontextprotocol/skills`,
SEP-2640, upstream **Beta**): serve [Agent Skills](https://agentskills.io/) over the
Resources primitive with `skills/list` and `skills/get`.

Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server) with its
`ext-skills` feature:

```rust,ignore
use std::path::Path;
use turul_mcp_ext_skills::{SkillEntry, SkillName, SkillPath};
use turul_mcp_server::ext_skills::SkillCatalog;

let mut catalog = SkillCatalog::new();
catalog.add(SkillEntry::from_dir(
    SkillPath::new(["acme"], SkillName::parse("refunds")?)?,
    Path::new("skills/refunds"),
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
| Skill entry | `SkillEntry::from_dir`, `SkillEntry::from_files` |

Parsing and validating `SKILL.md` belongs to
[`turul-agent-skills`](https://docs.rs/turul-agent-skills); this crate calls it and never
re-implements it.

## Limits of this release

- Frontmatter must be all strings: a plain scalar a YAML parser would read as null,
  bool, number or timestamp is refused (quote it).
- Directory reads (`resources/directory/read`) and generated (`"dynamic"`) skills are
  served by `turul-mcp-server` (ADR-033 §7, §8); this crate supplies their wire types.
- `from_dir` refuses a skill with a symlink whose target is outside the skill directory.

Design and test contract: [ADR-033](../../docs/adr/033-skills-over-mcp.md). Vendored
specification and provenance: [`schema/`](schema/README.md).
