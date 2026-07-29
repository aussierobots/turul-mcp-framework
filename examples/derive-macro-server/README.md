# Derive Macro Server

A five-tool server built entirely with `#[derive(McpTool)]` — the scale-up from
`calculator-add-simple-server-derive`'s single tool. It registers itself as
`code-generation-server`; the crate name is the authoring style it demonstrates.

## Tools (as registered by `src/main.rs`)

| Tool | Purpose |
|---|---|
| `generate_code` | Render a named template (`rust_struct`, `rust_enum`, `api_endpoint`, `database_model`) with parameter substitution |
| `validate_project` | Check a project directory's structure and config against per-language rules |
| `transform_code` | Apply a transformation (`naming_convention`, `add_documentation`, `add_derives`, `error_handling`) |
| `validate_config` | Validate a config document (`database_config`, `api_config`, …) |
| `generate_tests` | Generate `unit_tests` / `integration_tests` / `property_tests` scaffolding |

Every tool's `execute` returns `McpResult<String>` and none declares
`#[tool(output = Type)]`, so **none of them advertises an `outputSchema`** and
none returns `structuredContent` — the JSON each tool builds is serialized to a
string and lands under the derive macro's default `output` field. For typed
outputs reaching `tools/list`, see `calculator-add-simple-server-derive`.

## What this demonstrates

- `#[derive(McpTool)]` at realistic scale — five structs, four or five fields each
- `#[param(description = ...)]` and `#[param(..., optional)]` over `Option<T>`
  fields, so most arguments stay out of `required`
- Typed deserialization of external data files into Rust structs, with the
  files embedded via `include_str!` rather than read from a relative path — a
  CWD-relative read silently finds nothing under
  `cargo run -p derive-macro-server` from the workspace root
- Domain errors (`McpError::tool_execution`, `McpError::invalid_param_type`)
  reaching the client as JSON-RPC errors without the handler ever constructing
  a `JsonRpcError`

## Data files

| File | Deserialized into | Used by |
|---|---|---|
| `data/code_templates.json` | `CodeTemplates` (serde_json) | `generate_code`, `transform_code` |
| `data/validation_schemas.yaml` | `ValidationSchemas` (serde_yml) | `validate_config`, `validate_project` |
| `data/transformation_rules.md` | not loaded — prose reference only | — |

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id`.

## Run

```bash
cargo run -p derive-macro-server
# → http://127.0.0.1:8765/mcp
```

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'
call() { curl -s -X POST http://127.0.0.1:8765/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H "Mcp-Name: $1" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"$1\",\"arguments\":$2}}"; }

curl -s -X POST http://127.0.0.1:8765/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{$META}}"

# `parameters` is a JSON *string*, hence the escaping inside the arguments object
call generate_code '{"template_type":"rust_struct","parameters":"{\"name\":\"User\",\"visibility\":\"pub\",\"attributes\":[\"derive(Debug)\"],\"fields\":[{\"name\":\"id\",\"type\":\"u64\"}]}"}'

call generate_tests '{"source_code":"pub fn add(a: i32, b: i32) -> i32 { a + b }","test_type":"unit_tests"}'
call transform_code '{"source_code":"fn foo() { let x = 1; }","transformation":"add_documentation"}'
call validate_config '{"config_input":"{\"host\":\"db\"}","config_type":"database_config","format":"json"}'
call validate_project '{"project_type":"rust_project","project_path":"."}'
```

## See also

- `calculator-add-simple-server-derive` — the same macro on one tool, with
  `output = Type` and a schemars-generated `outputSchema`
- `function-macro-server` — the `#[mcp_tool]` async-fn alternative
