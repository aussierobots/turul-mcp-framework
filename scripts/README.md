# Example Verification Scripts

Intent-based verification scripts for all 44+ examples in the Turul MCP Framework.

## Overview

These scripts test the **actual intent and functionality** of each example, not just generic protocol compliance. Each phase verifies that examples work as designed with proper business logic validation.

## Scripts

### Individual Verification Scripts

- **`verify_calculator_examples.sh`** - Calculator Learning Progression (5 examples)
  - Tests all 4 tool creation patterns (function, derive, builder, manual)
  - Verifies actual math: `5.0 + 3.0 = 8.0`
  - Validates pattern equivalence

- **`verify_resource_servers.sh`** - Resource Servers (6 examples)
  - Tests `resources/list` and `resources/read`
  - Verifies template variable substitution
  - Validates session-aware resource behavior

- **`verify_prompts_examples.sh`** - Prompts & Special Features (7 examples)
  - Tests `prompts/get` with template substitution
  - Validates completion, sampling, elicitation features
  - Verifies pagination and notification patterns

- **`verify_storage_backends.sh`** - Session Storage Backends (4 examples)
  - Tests SQLite, PostgreSQL, DynamoDB, stateful operations
  - Verifies session persistence across requests
  - Validates storage-specific behavior

- **`verify_advanced_servers.sh`** - Advanced/Composite Servers (9 examples)
  - Tests real business logic (alerts, audit, logging)
  - Verifies multi-capability servers
  - Validates complex workflows

- **`verify_client_examples.sh`** - Clients & Test Utilities (5 examples)
  - Tests CLIENT behavior (not servers!)
  - Verifies session management, SSE streaming
  - Validates client-server integration

- **`verify_lambda_examples.sh`** - Lambda Examples (3 examples)
  - Tests AWS Lambda deployment patterns (compilation only)
  - Verifies serverless MCP builds correctly
  - Note: Full testing requires AWS deployment

- **`verify_meta_examples.sh`** - Meta Examples (3 examples)
  - Tests builders showcase, performance testing
  - Verifies demonstration and tutorial examples
  - Validates educational content

### Master Scripts

- **`verify_all_examples.sh`** - Runs all 8 verification phases sequentially
  - Generates comprehensive report
  - Interactive prompts between phases
  - Provides pass/fail summary

- **`verify_all_examples_unattended.sh`** - Runs all 8 verification phases non-interactively
  - Generates comprehensive report without prompts
  - Collects results to temporary files
  - Suitable for CI/CD pipelines

## Usage

### Run Individual Verification

```bash
# Run a specific verification script
./scripts/verify_calculator_examples.sh

# Run with output capture
./scripts/verify_calculator_examples.sh 2>&1 | tee calculator_results.log
```

### Run All Examples

```bash
# Interactive mode (prompts between phases)
./scripts/verify_all_examples.sh

# Non-interactive (unattended)
./scripts/verify_all_examples_unattended.sh 2>&1 | tee full_verification.log
```

### Run Individual Examples Manually

```bash
# Start a server
RUST_LOG=error cargo run --bin minimal-server -- --port 8641 &
SERVER_PID=$!
sleep 3

# MCP 2026-07-28 is stateless: no `initialize` handshake and no session header.
# The protocol version and client capabilities ride in `_meta` on every request.

# Every request carries the MCP-Protocol-Version and Mcp-Method headers, and a
# params._meta block. `Mcp-Name` is additionally required on named calls
# (tools/call, prompts/get, resources/read) and MUST equal the name in the body.

# Discover what the server supports (optional — clients MAY call this first)
curl -s -X POST "http://127.0.0.1:8641/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: server/discover" \
  -d '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' | jq

# List tools
curl -s -X POST "http://127.0.0.1:8641/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/list" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' | jq

# Call a tool — Mcp-Name must equal params.name
curl -s -X POST "http://127.0.0.1:8641/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/call" \
  -H "Mcp-Name: echo" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"text":"Hello, MCP!"},"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' | jq

# Cleanup
kill $SERVER_PID
```

## Key Implementation Details

### Stateless requests (2026-07-28)

There is no `initialize` handshake and no `Mcp-Session-Id`. Every request carries
`MCP-Protocol-Version` and `Mcp-Method` headers plus a `params._meta` block; named
calls (`tools/call`, `prompts/get`, `resources/read`) also carry `Mcp-Name`, which
must equal the name in the body. See the worked commands above.

On the opt-in 2025-11-25 lane the old session handshake still applies.


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

| Suite | Examples | Status |
|-------|----------|--------|
| Calculator progression | 5 | ✅ Ready |
| Resource servers | 6 | ✅ Ready |
| Prompts & special features | 7 | ✅ Ready |
| Session storage backends | 4 | ✅ Ready |
| Advanced/composite servers | 9 | ✅ Ready |
| Clients & test utilities | 5 | ✅ Ready |
| Lambda examples | 3 | ✅ Ready |
| Meta examples | 3 | ✅ Ready |
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

1. Run all examples: `./scripts/verify_all_examples.sh`
2. Review results
3. Fix any failing examples
4. Re-run verification until all pass
5. Document final results

## See Also

- `../examples/` - Example source code
- `../CLAUDE.md` - Development guidelines