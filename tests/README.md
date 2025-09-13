# MCP Framework E2E Testing Suite

This directory contains comprehensive End-to-End (E2E) testing infrastructure for the MCP Framework, validating complete MCP 2025-06-18 specification compliance with real HTTP/SSE transport.

## Overview

The E2E testing suite consists of:

- **Test Servers**: Real MCP servers (`resource-test-server`, `prompts-test-server`) with comprehensive test data
- **E2E Test Suites**: HTTP client tests that validate MCP protocol compliance
- **Shared Utilities**: Reusable test infrastructure and validation helpers
- **SSE Testing**: Server-Sent Events validation for real-time notifications
- **Session Testing**: Session management, consistency, and isolation validation

## Directory Structure

```
tests/
├── README.md                 # This file - main testing guide
├── shared/                   # Shared E2E utilities and fixtures
│   ├── src/e2e_utils.rs     # Common test client, server manager, fixtures
│   ├── tests/               # Comprehensive session validation tests
│   └── README.md            # Shared utilities documentation
├── resources/               # Resource E2E tests
│   ├── tests/               # All resource-related E2E tests
│   └── README.md            # Resources testing guide
└── prompts/                 # Prompts E2E tests
    ├── tests/               # All prompts-related E2E tests
    └── README.md            # Prompts testing guide
```

## Quick Start

### Prerequisites

1. **Rust Toolchain**: Ensure you have Rust 1.70+ installed
2. **Network Access**: Tests use random ports (22000-65535 range typically)
3. **Build Dependencies**: All test servers must be pre-built

### Setup

1. **Build Test Servers** (Required):
```bash
# From project root
cargo build --package resource-test-server
cargo build --package prompts-test-server
```

2. **Build Test Infrastructure**:
```bash
# Build shared utilities
cd tests/shared && cargo build

# Build all test suites
cd ../resources && cargo build
cd ../prompts && cargo build
```

### Running Tests

#### Run All E2E Tests
```bash
# From project root - runs all E2E tests
cargo test --workspace --test "*e2e*" -- --nocapture

# Or run each test suite individually
cd tests/resources && cargo test -- --nocapture
cd tests/prompts && cargo test -- --nocapture
cd tests/shared && cargo test -- --nocapture
```

#### Run Specific Test Categories

**Session Validation Tests**:
```bash
cd tests/shared && cargo test session -- --nocapture
```

**SSE Notification Tests**:
```bash
cd tests/resources && cargo test sse -- --nocapture
cd tests/prompts && cargo test sse -- --nocapture
```

**Protocol Compliance Tests**:
```bash
cd tests/resources && cargo test e2e_integration -- --nocapture
cd tests/prompts && cargo test e2e_integration -- --nocapture
```

**Shared Utilities Tests**:
```bash
cd tests/resources && cargo test e2e_shared_integration -- --nocapture
cd tests/prompts && cargo test e2e_shared_integration -- --nocapture
```

#### Run Individual Tests
```bash
# Specific test examples
cargo test test_mcp_initialize_session -- --nocapture
cargo test test_resources_list -- --nocapture
cargo test test_sse_connection_establishment -- --nocapture
cargo test test_session_consistency -- --nocapture
```

### Manual Server Testing

You can also run the test servers manually for debugging:

**Resource Test Server**:
```bash
# Run on specific port
cargo run --package resource-test-server -- --port 8080

# Run on random port
cargo run --package resource-test-server
```

**Prompts Test Server**:
```bash
# Run on specific port  
cargo run --package prompts-test-server -- --port 8081

# Run on random port
cargo run --package prompts-test-server
```

**Manual Testing Examples**:
```bash
# Initialize session
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# List resources (use session ID from above)
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}'
```

## Test Features Validated

### MCP Protocol Compliance
- ✅ **Protocol Version Negotiation**: 2025-06-18 specification compliance
- ✅ **JSON-RPC 2.0**: Proper request/response structure validation
- ✅ **HTTP Transport**: Real HTTP client/server communication
- ✅ **Session Management**: UUID v7 session IDs with proper isolation
- ✅ **Server-Sent Events**: SSE notifications with proper formatting

### Resource Testing
- ✅ **17 Test Resources**: Comprehensive coverage of all MCP resource patterns
- ✅ **URI Templates**: Template variable substitution (e.g., `template://items/{id}`)
- ✅ **Error Handling**: Proper error responses and edge cases
- ✅ **Binary Content**: Base64-encoded binary data support
- ✅ **Large Content**: Performance testing with large datasets
- ✅ **Session-Aware Resources**: Context-dependent resource behavior

### Prompts Testing  
- ✅ **11 Test Prompts**: All MCP prompt patterns and argument types
- ✅ **Argument Validation**: String, number, boolean, and template arguments
- ✅ **Multi-Message Prompts**: User/assistant conversation patterns
- ✅ **Dynamic Prompts**: Context-dependent prompt generation
- ✅ **Validation Errors**: Proper error handling for invalid arguments

### Session & SSE Testing
- ✅ **Session Consistency**: Cross-request session maintenance
- ✅ **Session Isolation**: Multi-client session separation
- ✅ **SSE Event Bridge**: Real-time notification delivery
- ✅ **Format Compliance**: Proper SSE event formatting
- ✅ **Concurrent Operations**: Multi-client concurrent testing

## Troubleshooting

### Common Issues

**Port Conflicts**:
```bash
# If tests fail with port binding errors, check for running processes
lsof -i :PORT_NUMBER
kill PID_OF_PROCESS
```

**Test Server Build Failures**:
```bash
# Ensure test servers are built first
cargo build --package resource-test-server
cargo build --package prompts-test-server

# Check build status
ls -la target/debug/resource-test-server
ls -la target/debug/prompts-test-server
```

**Session Timeout Issues**:
```bash
# Run with debug logging
RUST_LOG=debug cargo test test_name -- --nocapture
```

**Network Connectivity**:
```bash
# Test local connectivity
curl -v http://127.0.0.1:8080/mcp
```

### Debug Mode

Enable detailed logging for debugging:
```bash
# Full debug output
RUST_LOG=debug cargo test -- --nocapture

# Specific module debugging
RUST_LOG=mcp_e2e_shared=debug,turul_mcp_server=info cargo test -- --nocapture
```

### Test Isolation

Each test runs its own server instance on a random port to ensure isolation:
- ✅ **No shared state** between tests
- ✅ **Automatic cleanup** when tests complete
- ✅ **Parallel execution** supported
- ✅ **Random port assignment** prevents conflicts

## Teardown

Tests automatically clean up resources, but you can manually ensure cleanup:

```bash
# Kill any remaining test server processes
pkill -f "resource-test-server\|prompts-test-server"

# Clean build artifacts if needed
cargo clean
```

## Test Results Interpretation

### Success Indicators
- ✅ **"test result: ok"** - All assertions passed
- ✅ **"Server started successfully"** - Test server initialization worked
- ✅ **"Session ID: ..."** - Session management working
- ✅ **"SSE event bridge established"** - Real-time notifications enabled

### Failure Patterns
- ❌ **"Failed to start server"** - Build or port binding issues
- ❌ **"Session error"** - Session management problems  
- ❌ **"Protocol version"** - MCP specification compliance issues
- ❌ **"assertion failed"** - Test expectation not met

## Contributing

When adding new E2E tests:

1. **Use Shared Utilities**: Import from `mcp_e2e_shared` crate
2. **Follow Naming**: Use descriptive test names with `test_` prefix
3. **Add Documentation**: Include test purpose and expected behavior
4. **Validate Cleanup**: Ensure tests don't leave running processes
5. **Test Isolation**: Each test should be independent

See individual README files in each test directory for specific guidelines.

## Architecture

The E2E testing architecture provides:

- **Real Transport**: Actual HTTP/SSE communication (not mocks)
- **Specification Compliance**: Full MCP 2025-06-18 protocol validation
- **Production Readiness**: Testing conditions mirror real-world usage
- **Comprehensive Coverage**: All MCP features and edge cases tested
- **Developer Experience**: Clear feedback and debugging capabilities

This ensures the MCP Framework is production-ready with complete specification compliance.