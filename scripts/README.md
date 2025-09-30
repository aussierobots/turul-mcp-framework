# Example Verification Scripts

Intent-based verification scripts for all 44+ examples in the Turul MCP Framework.

## Overview

These scripts test the **actual intent and functionality** of each example, not just generic protocol compliance. Each phase verifies that examples work as designed with proper business logic validation.

## Scripts

### Individual Phase Scripts

- **`verify_phase1.sh`** - Calculator Learning Progression (5 examples)
  - Tests all 4 tool creation patterns (function, derive, builder, manual)
  - Verifies actual math: `5.0 + 3.0 = 8.0`
  - Validates pattern equivalence

- **`verify_phase2.sh`** - Resource Servers (6 examples)
  - Tests `resources/list` and `resources/read`
  - Verifies template variable substitution
  - Validates session-aware resource behavior

- **`verify_phase3.sh`** - Prompts & Special Features (7 examples)
  - Tests `prompts/get` with template substitution
  - Validates completion, sampling, elicitation features
  - Verifies pagination and notification patterns

- **`verify_phase4.sh`** - Session Storage Backends (4 examples)
  - Tests SQLite, PostgreSQL, DynamoDB, stateful operations
  - Verifies session persistence across requests
  - Validates storage-specific behavior

- **`verify_phase5.sh`** - Advanced/Composite Servers (9 examples)
  - Tests real business logic (alerts, audit, logging)
  - Verifies multi-capability servers
  - Validates complex workflows

- **`verify_phase6.sh`** - Clients & Test Utilities (5 examples)
  - Tests CLIENT behavior (not servers!)
  - Verifies session management, SSE streaming
  - Validates client-server integration

- **`verify_phase7.sh`** - Lambda Examples (3 examples)
  - Tests AWS Lambda deployment patterns (compilation only)
  - Verifies serverless MCP builds correctly
  - Note: Full testing requires AWS deployment

- **`verify_phase8.sh`** - Meta Examples (3 examples)
  - Tests builders showcase, performance testing
  - Verifies demonstration and tutorial examples
  - Validates educational content

### Master Script

- **`run_all_phases.sh`** - Runs all 8 phases sequentially
  - Generates comprehensive report
  - Interactive prompts between phases
  - Provides pass/fail summary

## Usage

### Run Individual Phase

```bash
# Run a specific phase
./scripts/verify_phase1.sh

# Run with output capture
./scripts/verify_phase1.sh 2>&1 | tee phase1_results.log
```

### Run All Phases

```bash
# Interactive mode (prompts between phases)
./scripts/run_all_phases.sh

# Non-interactive (remove read prompts first)
./scripts/run_all_phases.sh 2>&1 | tee full_verification.log
```

### Run Individual Examples Manually

```bash
# Start a server
RUST_LOG=error cargo run --bin minimal-server -- --port 8641 &
SERVER_PID=$!
sleep 3

# Initialize and get session ID (from header in lenient mode)
SESSION_ID=$(curl -i -s -X POST "http://127.0.0.1:8641/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
  | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')

echo "Session ID: $SESSION_ID"

# List tools
curl -s -X POST "http://127.0.0.1:8641/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "MCP-Session-ID: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | jq

# Call tool
curl -s -X POST "http://127.0.0.1:8641/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "MCP-Session-ID: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello"}}}' | jq

# Cleanup
kill $SERVER_PID
```

## Key Implementation Details

### Session ID Handling

The framework uses **lenient mode** where:
- Session IDs are created automatically on `initialize`
- Session ID is returned in the `Mcp-Session-Id` response **header** (not JSON body)
- All subsequent requests must include `MCP-Session-ID: <session-id>` header

**Incorrect** (old approach):
```bash
SESSION_ID=$(curl -s ... | jq -r '.result.sessionId')
```

**Correct** (current approach):
```bash
SESSION_ID=$(curl -i -s ... | grep -i 'mcp-session-id:' | sed 's/.*: //' | tr -d '\r\n ')
```

### Intent-Based Testing Philosophy

These scripts test **what the example is designed to demonstrate**, not just protocol compliance:

- **Calculator servers**: Verify math is correct (`5 + 3 = 8`)
- **Resource servers**: Verify content is returned and templates work
- **Prompt servers**: Verify messages are generated with proper substitution
- **Storage servers**: Verify sessions persist across requests
- **Client utilities**: Verify CLIENT behavior, not server behavior

### External Dependencies

Some examples require external services:
- **PostgreSQL servers**: Require running PostgreSQL instance
- **DynamoDB servers**: Require AWS credentials
- **Lambda examples**: Require AWS Lambda runtime

Scripts gracefully skip these with `SKIPPED` status when dependencies are unavailable.

## Test Coverage

| Phase | Examples | Status |
|-------|----------|--------|
| Phase 1 | 5 | ✅ Ready |
| Phase 2 | 6 | ✅ Ready |
| Phase 3 | 7 | ✅ Ready |
| Phase 4 | 4 | ✅ Ready |
| Phase 5 | 9 | ✅ Ready |
| Phase 6 | 5 | ✅ Ready |
| Phase 7 | 3 | ✅ Ready |
| Phase 8 | 3 | ✅ Ready |
| **Total** | **42** | ✅ Ready |

## Troubleshooting

### Port Conflicts

```bash
# Kill stuck servers
pkill -f minimal-server
pkill -f calculator-add
pkill -f resource-server

# Check port usage
lsof -i :8641
```

### Session ID Not Found

Ensure you're using the **header-based** extraction method, not JSON parsing.

### Server Fails to Start

1. Check if port is already in use
2. Verify binary compiled: `cargo build --bin <server-name>`
3. Check for panics in logs: `RUST_LOG=debug cargo run --bin <server-name>`

### Test Timeouts

Increase timeout values in scripts:
```bash
# Change from 10s to 30s
RUST_LOG=error timeout 30s cargo run --bin ...
```

## Next Steps

1. Run all phases: `./scripts/run_all_phases.sh`
2. Review results and update `EXAMPLE_VERIFICATION_LOG.md`
3. Fix any failing examples
4. Re-run verification until all pass
5. Document final results

## See Also

- `../EXAMPLE_VERIFICATION_LOG.md` - Campaign tracking document
- `../examples/` - Example source code
- `../CLAUDE.md` - Development guidelines