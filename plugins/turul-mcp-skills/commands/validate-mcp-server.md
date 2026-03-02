---
name: validate-mcp-server
description: Validate an existing Turul MCP server project for correctness, compliance, and best practices
user_invocable: true
arguments: []
---

# /validate-mcp-server

Validate an existing Turul MCP server project. Detects monorepo vs external projects and runs appropriate checks.

## Steps

### 1. Detect Environment

Determine if we're inside the Turul monorepo or an external project:

**Monorepo detection** (either method suffices):
1. A `Cargo.toml` with `[workspace]` containing `turul-mcp-` members exists in a parent directory, OR
2. **Fallback**: The file `AGENTS.md` exists in the workspace root AND `tests/Cargo.toml` contains `name = "turul-mcp-framework-integration-tests"`

If either detection method succeeds: **Mode 1: Monorepo (Full Release Gates)**
Otherwise: **Mode 2: External Project (Local Checks Only)**

### 2a. Mode 1 — Monorepo Validation (Full Release Gates)

Run all 7 release gate tests from the workspace root. Report pass/fail for each:

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

### 2b. Mode 2 — External Project (Local Checks Only)

Run local validation from the project directory:

```bash
# Build check
cargo check

# Lint
cargo clippy -- -D warnings

# Tests
cargo test

# Preflight lint (grep-based, non-authoritative)
bash plugins/turul-mcp-skills/scripts/preflight-lint.sh .
```

### 3. Additional Checks (Both Modes)

Run these checks by reading project files (no compilation needed):

**Check 1: Turul dependency exists**
- Read `Cargo.toml` and verify `turul-mcp-server` is listed under `[dependencies]`
- **FAIL** if missing — this is not a Turul MCP server project

**Check 2: MCP components registered**
- Search `src/` for `.tool(`, `.tool_fn(`, `.resource(`, `.resource_fn(`, `.prompt(`, `.prompt_fn(`
- **WARN** (not fail) if none found — valid but likely unintentional empty server

**Check 3: Direct protocol imports (forbidden)**
- Search `src/` for `use turul_mcp_protocol_2025_11_25` or `use turul_mcp_protocol_2025_06_18`
- **FAIL** if found — must use `turul_mcp_protocol` re-export crate
- See: [CLAUDE.md — Protocol Re-export Rule](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#protocol-re-export-rule-mandatory)

**Check 4: Derive macro output attribute**
- Search `src/` for `#[derive(McpTool)]` blocks that have a custom return type in `execute()` but no `output = Type` attribute
- **WARN** (not fail) if detected — framework has zero-config heuristics, but explicit output types produce more accurate schemas
- See: [CLAUDE.md — Output Types and Schemas](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#output-types-and-schemas)

**Check 5: Builder name**
- Search `src/` for `McpServer::builder()` or `LambdaMcpServerBuilder::new()` and verify `.name(` is called
- **FAIL** if `.name()` is missing — required by the builder
- **WARN** if `.version()` is missing — recommended but optional

**Check 6: JsonRpcError in handlers**
- Search `src/` for `JsonRpcError` construction in tool/resource/prompt handler code
- **WARN** if found — handlers should return `McpError` variants; framework handles conversion
- See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

### 4. Report

Print a summary table with pass/fail/warn status per check:

```
Validation Results for: my-mcp-server
Mode: External Project

  ✓ Turul dependency          turul-mcp-server = "0.3" found
  ✓ MCP components            3 tools, 1 resource registered
  ✓ Protocol imports           No direct versioned protocol imports
  ⚠ Output attribute          DoubleTool uses derive(McpTool) without output = Type
  ✓ Builder name              .name("my-server") found
  ⚠ Builder version           .version() not found (recommended)
  ✓ Error handling            No direct JsonRpcError construction
  ✓ cargo check               Passed
  ✓ cargo clippy              Passed (0 warnings)
  ✓ cargo test                Passed (12 tests)

Result: PASS (2 warnings)
```

For each warning or failure, include a one-line actionable fix suggestion referencing the relevant CLAUDE.md section or skill.
