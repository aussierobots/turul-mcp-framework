---
name: new-mcp-server
description: Scaffold a new Turul MCP server project with dual validation
user_invocable: true
arguments:
  - name: project-name
    description: Name for the new MCP server project (lowercase, hyphens allowed)
    required: true
---

# /new-mcp-server

Scaffold a new Turul MCP server project and validate it.

## Steps

### 1. Scaffold the Project

Run the scaffold script to generate the project structure:

```bash
bash plugins/turul-mcp-skills/scripts/scaffold-mcp-server.sh "$ARGUMENTS"
```

If the script is not available locally, generate the files manually:

- `Cargo.toml` with dependencies: `turul-mcp-server`, `turul-mcp-derive`, `turul-mcp-protocol`, `tokio`, `serde`, `serde_json`, `schemars`, `tracing`, `tracing-subscriber` (all targeting v0.3.x for turul crates)
- `src/main.rs` with a starter tool using the `#[mcp_tool]` function macro pattern:
  - A simple `hello` tool that takes a `name: String` and returns `McpResult<String>`
  - Uses `McpServer::builder()` with `.tool_fn(hello)`
  - Includes `tracing_subscriber` initialization

### 2. Validate — Detect Environment

Determine if we're inside the Turul monorepo or an external project:

**Monorepo detection** (ALL must be true):
1. A `Cargo.toml` with `[workspace]` containing `turul-mcp-` members exists in a parent directory, OR
2. **Fallback**: The file `AGENTS.md` exists in the workspace root AND `tests/Cargo.toml` contains `name = "turul-mcp-framework-integration-tests"`

If either detection method succeeds → **Mode 1: Monorepo (Full Release Gates)**
Otherwise → **Mode 2: External Project (Local Checks Only)**

### 3a. Mode 1 — Monorepo Validation (Full Release Gates)

Run all release gate tests. These are copy-pastable commands:

```bash
# Gate 1: MCP specification compliance (JSON-RPC, _meta, pagination, sessions)
cargo test --test compliance

# Gate 2: Notification payload correctness (round-trip _meta and payload fields)
cargo test --test feature_tests notification_payload_correctness

# Gate 3: Vec/array output schemas (tools/list advertises "type": "array")
cargo test --test schema_tests mcp_vec_result_schema_test

# Gate 4: Schemars derive integration (detailed schemas via schema_for!)
cargo test -p turul-mcp-derive schemars_integration_test

# Gate 5: Lifecycle -32031 enforcement (pre-init access → SessionError)
cargo test --test compliance test_strict_lifecycle_rejects_before_initialized
cargo test --test compliance test_strict_lifecycle_rejects_tool_calls_before_initialized

# Gate 6: Capability truthfulness (capabilities match actual support)
cargo test --test feature_tests test_tools_capability_truthfulness
cargo test --test feature_tests test_prompts_capability_truthfulness
cargo test --test compliance test_runtime_capability_truthfulness

# Gate 7: E2E session lifecycle over streamable HTTP
cargo test --test e2e_tests test_strict_lifecycle_enforcement_over_streamable_http
```

**Source mapping** (consolidated test binary → source files):

| Binary | Source modules |
|---|---|
| `compliance` | `mcp_compliance_tests.rs`, `mcp_specification_compliance.rs`, `mcp_behavioral_compliance.rs`, `mcp_tool_compliance.rs` |
| `feature_tests` | `notification_payload_correctness.rs`, `mcp_runtime_capability_validation.rs`, `framework_integration_tests.rs` |
| `schema_tests` | `mcp_vec_result_schema_test.rs`, `schemars_detailed_schema_test.rs`, `schemars_optional_fields_test.rs`, `test_schemars_derive.rs` |
| `e2e_tests` | `streamable_http_e2e.rs`, `streamable_http_client_test.rs`, `e2e_sse_notification_roundtrip.rs` |

**Note**: Gate 4 runs against `turul-mcp-derive` directly (not the consolidated `schema_tests` binary) per [AGENTS.md release readiness requirements](https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01).

### 3b. Mode 2 — External Project (Local Checks Only)

Run what's available locally:

```bash
# Compile check
cargo check

# Lint check
cargo clippy -- -D warnings

# Run project's own tests
cargo test

# Preflight lint (grep-based, non-authoritative)
bash plugins/turul-mcp-skills/scripts/preflight-lint.sh .
```

After all checks pass, display:

> Local checks passed. For full MCP compliance certification, run the Turul framework's release gate tests in the monorepo. See: https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01

### 4. Report Results

Summarize what was created and what validation passed:
- Project location and files created
- Which validation mode was used (monorepo or external)
- Gate results (pass/fail for each)
- Next steps: how to add tools, configure output schemas, run the server
