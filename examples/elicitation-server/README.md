# Customer Onboarding and Data Collection Platform

Five tools that read workflow, compliance-form, preference and survey
definitions out of `data/`, turn each set of field definitions into a JSON
Schema, and return a description of the form the caller should render.

## This is not the elicitation protocol

Despite the package name, **no MCP elicitation happens here**. Nothing is
server-initiated: `elicitation/create` is never issued, and each generated form
travels back inside an ordinary `tools/call` result. What the example actually
demonstrates is config-driven JSON Schema generation and validation-rule
loading from external files.

For real user-input round trips see **`examples/mrtr-elicitation-server`** — on
the 2026-07-28 stateless core a tool returns `InputRequiredResult` and the
client retries the original call carrying `inputResponses` plus the echoed
`requestState`.

The package's main job is being the fixture behind `tests/elicitation`
(`cargo test -p mcp-elicitation-tests`), which drives it over `tools/list` and
`tools/call`.

## Spec lane

Pinned to **2025-11-25** in `Cargo.toml` (`protocol-2025-11-25` on every
framework dependency), independent of the workspace 2026-07-28 default. It
therefore uses the stateful lane's `initialize` → `notifications/initialized` →
`Mcp-Session-Id` handshake.

## Tools (as registered by `src/main.rs`)

| Tool | Key arguments | Returns |
|---|---|---|
| `start_onboarding_workflow` | `workflow_type`, `step_index?` | The step's fields, generated schema, and position in the workflow |
| `compliance_form` | `form_type` | GDPR/CCPA form fields and the regulatory framework they map to |
| `collect_user_preferences` | `collection_type` | Notification / accessibility preference categories and settings |
| `customer_satisfaction_survey` | `survey_type` | Survey questions plus scoring and follow-up metadata |
| `data_validation_demo` | field values | Validation outcome against `data/validation_rules.yaml` |

`tools/list` is authoritative — run it rather than trusting this table if the
code has moved on.

## Data files

| File | Drives |
|---|---|
| `data/onboarding_workflows.json` | Workflow steps, compliance forms, preference sets, survey templates |
| `data/validation_rules.yaml` | Field-type rules, age verification, KYC requirements |
| `data/reference_data.md` | Geographic and industry reference lists |

The data directory is resolved relative to the working directory, falling back
to `examples/elicitation-server/data` and `../elicitation-server/data` so the
E2E harness can start the server from the workspace root.

## Run

```bash
# --port 0 (the default) takes an ephemeral port from the OS; the chosen
# port is printed at startup
cargo run -p elicitation-server -- --port 8053
```

## Try it (2025-11-25 stateful)

```bash
# 1. handshake — capture Mcp-Session-Id from the response headers
curl -i -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"curl","version":"1.0"}}}'

# 2. enable the session
curl -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' -H 'Mcp-Session-Id: SESSION_ID' \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'

# 3. start a workflow
curl -s -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' -H 'Mcp-Session-Id: SESSION_ID' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"start_onboarding_workflow","arguments":{"workflow_type":"personal_account","step_index":0}}}'

# 4. an out-of-range step → -32602
curl -s -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' -H 'Mcp-Session-Id: SESSION_ID' \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"start_onboarding_workflow","arguments":{"workflow_type":"personal_account","step_index":99}}}'
```

## See also

- `examples/mrtr-elicitation-server` — the actual user-input round trip
- `crates/turul-mcp-server/tests/mrtr_2026.rs` — its wire contract tests
