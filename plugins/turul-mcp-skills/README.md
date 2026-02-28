# turul-mcp-skills

Skills and tools for building MCP servers with the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework) (Rust).

## What's Included (v0.1.1)

| Component | Type | Purpose |
|---|---|---|
| `tool-creation-patterns` | Skill | Decision tree: function macro vs derive vs builder |
| `output-schemas` | Skill | The `output = Type` requirement, schemars, Vec\<T\>, structuredContent |
| `/new-mcp-server` | Command | Scaffold a Turul MCP server project with storage backend selection and dual validation |
| `server-patterns-index` | Reference | Pointer index to CLAUDE.md/AGENTS.md authoritative sections |
| `storage-backend-matrix` | Reference | Feature flags, Cargo.toml patterns, and config for InMemory/SQLite/PostgreSQL/DynamoDB |

## Installation

**From plugin registry:**
```bash
claude plugin install turul-mcp-skills
```

**From repository:**
```bash
claude plugin install --url https://github.com/aussierobots/turul-mcp-framework --path plugins/turul-mcp-skills
```

**Local development:**
```bash
claude --plugin-dir plugins/turul-mcp-skills
```

## Skills

### tool-creation-patterns

Triggers on: "create a tool", "mcp_tool macro", "derive McpTool", "ToolBuilder", "which tool pattern"

Guides you through choosing the right tool creation approach:
- **Level 1 — Function Macro** (`#[mcp_tool]`): Quick-start for simple, stateless tools
- **Level 2 — Derive Macro** (`#[derive(McpTool)]`): Complex tools needing session access, custom output types
- **Level 3 — Builder** (`ToolBuilder`): Dynamic/runtime tool construction

### output-schemas

Triggers on: "output schema", "structuredContent", "schemars", "output_field", "Vec output"

Covers the most common gotchas with MCP tool output:
- Why `output = Type` is mandatory on derive macros
- Automatic schemars detection
- Vec\<T\> output patterns
- `output_field` customization
- structuredContent auto-generation

## Command

### /new-mcp-server

Scaffolds a new Turul MCP server project with:
- Storage backend selection (`--storage inmemory|sqlite|postgres|dynamodb`)
- Cargo.toml with correct dependencies and feature flags for the chosen backend
- A starter tool using the function macro pattern
- `.env.example` with connection string template (non-inmemory backends)
- Dual validation: full release gates in monorepo, local checks for external projects

## Version Compatibility

This plugin targets **turul-mcp-server v0.3.0** (MCP 2025-11-25).

## Changelog

### v0.1.1
- Added `storage-backend-matrix` reference: decision matrix, feature flags, Cargo.toml patterns, config structs for all storage backends
- Updated `/new-mcp-server` scaffold with `--storage` flag (inmemory/sqlite/postgres/dynamodb) and `.env.example` generation
- Added storage backend row to `server-patterns-index`

### v0.1.0
- Initial release: tool-creation-patterns skill, output-schemas skill, /new-mcp-server command, server-patterns-index reference
