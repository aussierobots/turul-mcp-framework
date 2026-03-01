# turul-mcp-skills

Skills and tools for building MCP servers and clients with the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework) (Rust).

## What's Included (v0.2.0)

| Component | Type | Purpose |
|---|---|---|
| `tool-creation-patterns` | Skill | Decision tree: function macro vs derive vs builder |
| `resource-prompt-patterns` | Skill | Resource creation (4 patterns) and prompt creation (3 patterns) with decision flowcharts |
| `output-schemas` | Skill | The `output = Type` requirement, schemars, Vec\<T\>, structuredContent |
| `mcp-client-patterns` | Skill | Transport selection, connection lifecycle, tool invocation, task workflows, error handling |
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

### resource-prompt-patterns

Triggers on: "create a resource", "MCP resource", "McpResource", "mcp_resource", "resource!", "ResourceBuilder", "create a prompt", "MCP prompt", "McpPrompt", "prompt!", "PromptBuilder"

Guides you through choosing the right resource or prompt creation approach:
- **Resources** — 4 patterns: Function Macro (`#[mcp_resource]`), Derive (`#[derive(McpResource)]`), Declarative (`resource!{}`), Builder (`ResourceBuilder`)
- **Prompts** — 3 patterns: Derive (`#[derive(McpPrompt)]`), Declarative (`prompt!{}`), Builder (`PromptBuilder`)
- Decision flowcharts, comparison table, common mistakes, cross-references

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

### mcp-client-patterns

Triggers on: "MCP client", "McpClient", "McpClientBuilder", "connect to MCP server", "HttpTransport", "SseTransport", "client session", "ToolCallResponse"

Covers building MCP client applications with `turul-mcp-client`:
- Transport selection (auto-detect, HttpTransport, SseTransport)
- Connection lifecycle (connect, disconnect, session states)
- Tool/resource/prompt invocation from the client side
- Task workflows (call_tool_with_task, polling, all TaskStatus variants)
- Error handling (McpClientError variants, retryability, backoff)
- Configuration (ClientConfig, timeouts, retries, connection settings)

## Command

### /new-mcp-server

Scaffolds a new Turul MCP server project with:
- Storage backend selection (`--storage inmemory|sqlite|postgres|dynamodb`)
- Cargo.toml with correct dependencies and feature flags for the chosen backend
- A starter tool using the function macro pattern
- `.env.example` with connection string template (non-inmemory backends)
- Dual validation: full release gates in monorepo, local checks for external projects

## Version Compatibility

This plugin targets **turul-mcp-server v0.3** (MCP 2025-11-25).

## Changelog

### v0.2.0
- Added `resource-prompt-patterns` skill: 4 resource patterns (function macro, derive, declarative, builder) + 3 prompt patterns (derive, declarative, builder) with decision flowcharts, comparison table, and 7 example files
- Added Resource creation and Prompt creation rows to `server-patterns-index`
- Updated `tool-creation-patterns` "Beyond This Skill" with resource/prompt hand-off

### v0.1.2
- Added `mcp-client-patterns` skill: transport selection, connection lifecycle, tool invocation, task workflows, error handling for `turul-mcp-client`
- Added MCP Client row to `server-patterns-index`

### v0.1.1
- Added `storage-backend-matrix` reference: decision matrix, feature flags, Cargo.toml patterns, config structs for all storage backends
- Updated `/new-mcp-server` scaffold with `--storage` flag (inmemory/sqlite/postgres/dynamodb) and `.env.example` generation
- Added storage backend row to `server-patterns-index`

### v0.1.0
- Initial release: tool-creation-patterns skill, output-schemas skill, /new-mcp-server command, server-patterns-index reference
