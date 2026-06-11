# Derive Macro Server

A realistic multi-tool server built entirely with `#[derive(McpTool)]` —
five code-generation/validation tools with complex struct schemas, external
data files, and typed outputs.

## Tools (as registered by `src/main.rs`)

| Tool | Purpose |
|---|---|
| `generate_code` | Generate code from named templates (`data/code_templates.md`) |
| `validate_project` | Validate a project structure description |
| `transform_code` | Apply source-to-source transformations |
| `validate_config` | Validate configuration documents |
| `generate_tests` | Generate test scaffolding for a given module |

## Run

```bash
cargo run -p derive-macro-server
# → http://127.0.0.1:8765/mcp
```

## Try it (2026-07-28 stateless)

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8765/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"
```

(Check the actual port in `src/main.rs` if it differs.)

## What this demonstrates

- `#[derive(McpTool)]` with `#[tool(name, description, output = Type)]`
- `#[param(description = ...)]` field schemas, including nested structs
- schemars-derived output schemas reaching `tools/list` losslessly
- Loading template/config data from `data/` files at startup
