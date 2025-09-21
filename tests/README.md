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
├── README.md                              # This file - main testing guide
├── Cargo.toml                            # Test workspace configuration
├── E2E_TEST_IMPLEMENTATION_STATUS.md     # Implementation status tracking
│
├── Protocol Test Suites (E2E):
├── shared/                               # Shared E2E utilities and fixtures
│   ├── src/e2e_utils.rs                 # Common test client, server manager, fixtures
│   ├── tests/                           # Comprehensive session validation tests
│   └── README.md                        # Shared utilities documentation
├── resources/                           # Resource E2E tests
│   ├── tests/                           # All resource-related E2E tests
│   └── README.md                        # Resources testing guide
├── prompts/                             # Prompts E2E tests
│   ├── tests/                           # All prompts-related E2E tests
│   └── README.md                        # Prompts testing guide
├── tools/                               # Tools E2E tests
│   ├── tests/                           # All tools-related E2E tests
│   └── README.md                        # Tools testing guide
├── sampling/                            # Sampling protocol E2E tests
│   ├── tests/                           # Sampling-related E2E tests
│   └── README.md                        # Sampling testing guide
├── roots/                               # Roots protocol E2E tests
│   ├── tests/                           # Roots-related E2E tests
│   └── README.md                        # Roots testing guide
├── elicitation/                         # Elicitation protocol E2E tests
│   ├── tests/                           # Elicitation-related E2E tests
│   └── README.md                        # Elicitation testing guide
│
├── Framework Test Files (Integration):
├── mcp_runtime_capability_validation.rs # Runtime capability validation
├── mcp_compliance_tests.rs             # Core MCP protocol compliance
├── mcp_specification_compliance.rs     # Full specification testing
├── framework_integration_tests.rs      # Framework integration tests
├── working_examples_validation.rs      # Example validation tests
├── basic_session_test.rs               # Basic session functionality
├── client_drop_test.rs                 # Client cleanup testing
├── session_context_macro_tests.rs      # Session context macros
├── builders_examples.rs                # Builder pattern examples
├── derive_examples.rs                  # Derive macro examples
├── server_examples.rs                  # Server configuration examples
├── http_server_examples.rs             # HTTP server examples
├── lambda_examples.rs                  # AWS Lambda examples
├── client_examples.rs                  # Client usage examples
├── calculator_levels_integration.rs    # Tool creation levels
├── resources_integration_tests.rs      # Resource integration
├── custom_output_field_test.rs         # Custom output testing
│
└── test_helpers/                        # Common test utilities
    └── README.md                        # Test helpers documentation
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

# Or run each protocol test suite individually
cargo test -p mcp-sampling-tests --test sampling_protocol_e2e -- --nocapture
cargo test -p mcp-roots-tests --test roots_protocol_e2e -- --nocapture
cargo test -p mcp-elicitation-tests --test elicitation_protocol_e2e -- --nocapture
cargo test -p mcp-resources-tests --test e2e_integration -- --nocapture
cargo test -p mcp-prompts-tests --test e2e_integration -- --nocapture
cargo test -p turul-mcp-framework-tools-integration-tests --test e2e_integration -- --nocapture
```

#### Run Specific Test Categories

**Protocol E2E Tests**:
```bash
# All major protocol areas
cargo test -p mcp-sampling-tests --test sampling_protocol_e2e
cargo test -p mcp-roots-tests --test roots_protocol_e2e
cargo test -p mcp-elicitation-tests --test elicitation_protocol_e2e
cargo test -p mcp-resources-tests --test e2e_integration
cargo test -p mcp-prompts-tests --test e2e_integration
cargo test -p turul-mcp-framework-tools-integration-tests --test e2e_integration
```

**Core Compliance Tests**:
```bash
cargo test -p turul-mcp-framework-integration-tests --test mcp_runtime_capability_validation
cargo test -p turul-mcp-framework-integration-tests --test mcp_compliance_tests
cargo test -p turul-mcp-framework-integration-tests --test mcp_specification_compliance
```

**Session and Framework Tests**:
```bash
cargo test -p turul-mcp-framework-integration-tests --test basic_session_test
cargo test -p mcp-e2e-shared --test concurrent_session_advanced
cargo test -p turul-mcp-framework-integration-tests --test framework_integration_tests
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

### Complete Protocol Coverage
- ✅ **Tools Testing**: 8+ test tools covering all execution patterns and progress tracking
- ✅ **Resources Testing**: 17+ test resources covering all MCP resource patterns
- ✅ **Prompts Testing**: 12+ test prompts covering all argument types and validation
- ✅ **Sampling Testing**: Message generation with parameter validation and content formats
- ✅ **Roots Testing**: Directory discovery, URI validation, and security boundaries
- ✅ **Elicitation Testing**: Workflow management, form generation, and data validation

### Advanced Testing Features
- ✅ **URI Templates**: Template variable substitution (e.g., `file:///template/items/{id}`)
- ✅ **Error Handling**: Comprehensive error responses and edge cases across all protocols
- ✅ **Binary Content**: Base64-encoded binary data support
- ✅ **Large Content**: Performance testing with large datasets
- ✅ **Session-Aware Operations**: Context-dependent behavior across all protocol areas
- ✅ **Concurrent Sessions**: 50+ concurrent client testing with session isolation
- ✅ **SSE Event Bridge**: Real-time notification delivery across all protocol areas
- ✅ **Format Compliance**: Proper SSE event formatting and protocol adherence

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