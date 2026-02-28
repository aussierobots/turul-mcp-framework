# turul-mcp-skills

Skills and tools for building MCP servers with the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework) (Rust).

## What's Included (v0.1.0)

| Component | Type | Purpose |
|---|---|---|
| `tool-creation-patterns` | Skill | Decision tree: function macro vs derive vs builder |
| `output-schemas` | Skill | The `output = Type` requirement, schemars, Vec\<T\>, structuredContent |
| `/new-mcp-server` | Command | Scaffold a Turul MCP server project with dual validation |
| `server-patterns-index` | Reference | Pointer index to CLAUDE.md/AGENTS.md authoritative sections |

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
- Cargo.toml with correct dependencies
- A starter tool using the function macro pattern
- Dual validation: full release gates in monorepo, local checks for external projects

## Version Compatibility

This plugin targets **turul-mcp-server v0.3.0** (MCP 2025-11-25).
