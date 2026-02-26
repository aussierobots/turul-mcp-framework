# MCP Framework E2E Testing Suite

This directory contains comprehensive End-to-End (E2E) testing infrastructure for the MCP Framework, validating complete MCP 2025-11-25 specification compliance with real HTTP/SSE transport.

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
│
├── Protocol Test Suites (E2E):
├── shared/                               # Shared E2E utilities and fixtures
│   ├── src/e2e_utils.rs                 # Common test client, server manager, fixtures
│   ├── tests/
│   │   ├── concurrent_session_advanced.rs       # Concurrent session testing
│   │   └── session_validation_comprehensive.rs  # Session validation tests
│   └── README.md                        # Shared utilities documentation
├── resources/                           # Resource E2E tests
│   ├── tests/
│   │   ├── e2e_integration.rs                   # Resources E2E integration
│   │   ├── e2e_shared_integration.rs            # Shared resources integration
│   │   ├── mcp_resources_protocol_coverage.rs   # Protocol coverage tests
│   │   ├── mcp_resources_specification.rs       # Specification compliance
│   │   ├── resource_templates_e2e.rs            # Template variable testing
│   │   └── sse_notifications_test.rs            # SSE notifications
│   └── README.md                        # Resources testing guide
├── prompts/                             # Prompts E2E tests
│   ├── tests/
│   │   ├── e2e_integration.rs                   # Prompts E2E integration
│   │   ├── e2e_shared_integration.rs            # Shared prompts integration
│   │   ├── mcp_prompts_protocol_coverage.rs     # Protocol coverage tests
│   │   ├── mcp_prompts_specification.rs         # Specification compliance
│   │   ├── prompts_arguments_validation.rs      # Arguments validation
│   │   ├── prompts_endpoints_integration.rs     # Endpoints integration
│   │   ├── prompts_notifications.rs             # Notifications testing
│   │   └── sse_notifications_test.rs            # SSE notifications
│   └── README.md                        # Prompts testing guide
├── tools/                               # Tools E2E tests
│   ├── tests/
│   │   ├── e2e_integration.rs                   # Tools E2E integration
│   │   ├── large_message_handling.rs            # Large message testing
│   │   └── mcp_error_code_coverage.rs           # Error code coverage
│   └── README.md                        # Tools testing guide
├── sampling/                            # Sampling protocol E2E tests
│   ├── tests/
│   │   ├── sampling_protocol_e2e.rs             # Sampling protocol tests
│   │   └── sampling_validation_e2e.rs           # Sampling validation
│   └── README.md                        # Sampling testing guide
├── roots/                               # Roots protocol E2E tests
│   ├── tests/
│   │   ├── roots_protocol_e2e.rs                # Roots protocol tests
│   │   └── roots_security_e2e.rs                # Roots security tests
│   └── README.md                        # Roots testing guide
├── elicitation/                         # Elicitation protocol E2E tests
│   ├── tests/
│   │   └── elicitation_protocol_e2e.rs          # Elicitation protocol tests
│   └── README.md                        # Elicitation testing guide
│
├── Framework Test Files (Integration):
├── acronym_output_field_test.rs         # Acronym output field testing
├── basic_session_test.rs                # Basic session functionality
├── builders_examples.rs                 # Builder pattern examples
├── calculator_levels_integration.rs     # Tool creation levels
├── client_drop_test.rs                  # Client cleanup testing
├── client_examples.rs                   # Client usage examples
├── client_integration_test.rs           # Client integration tests
├── client_streaming_test.rs             # Client streaming tests
├── custom_output_field_test.rs          # Custom output testing
├── derive_examples.rs                   # Derive macro examples
├── e2e_sse_notification_roundtrip.rs    # SSE notification roundtrip
├── framework_integration_tests.rs       # Framework integration tests
├── http_server_examples.rs              # HTTP server examples
├── lambda_examples.rs                   # AWS Lambda examples
├── lambda_streaming_real.rs             # Real Lambda streaming tests
├── mcp_behavioral_compliance.rs         # Behavioral compliance
├── mcp_compliance_tests.rs              # Core MCP protocol compliance
├── mcp_derive_macro_bug_detection.rs    # Derive macro bug detection
├── mcp_explicit_vec_output_test.rs      # Explicit Vec output testing
├── mcp_runtime_capabilities_validation.rs # Runtime capabilities validation
├── mcp_runtime_capability_validation.rs # Runtime capability validation
├── mcp_specification_compliance.rs      # Full specification testing
├── mcp_tool_compliance.rs               # Tool compliance tests
├── mcp_tool_schema_runtime_sync_test.rs # Tool schema runtime sync
├── mcp_vec_badly_named_tool_test.rs     # Vec badly named tool testing
├── mcp_vec_result_runtime_schema_test.rs # Vec result runtime schema
├── mcp_vec_result_schema_test.rs        # Vec result schema
├── output_field_consistency_test.rs     # Output field consistency
├── phase5_regression_tests.rs           # Phase 5 regression tests
├── readme_examples.rs                   # README example validation
├── resources_integration_tests.rs       # Resource integration
├── server_examples.rs                   # Server configuration examples
├── session_context_macro_tests.rs       # Session context macros
├── session_id_compliance.rs             # Session ID compliance
├── sse_progress_delivery.rs             # SSE progress delivery
├── streamable_http_client_test.rs       # Streamable HTTP client
├── streamable_http_e2e.rs               # Streamable HTTP E2E
├── tasks_e2e_inmemory.rs                # Task lifecycle E2E (MCP 2025-11-25)
├── mcp_2025_11_25_features.rs           # MCP 2025-11-25 feature tests
├── working_examples_validation.rs       # Example validation tests
│
└── test_helpers/                        # Common test utilities
    └── README.md                        # Test helpers documentation
```

## Quick Start

### Prerequisites

1. **Rust Toolchain**: Ensure you have Rust 1.70+ installed
2. **Network Access**: Tests use random ports (22000-65535 range typically)
3. **Build Dependencies**: All test servers must be pre-built

### Test Execution Configuration

The project uses `.cargo/config.toml` to configure default test behavior:

```toml
# .cargo/config.toml
[test]
# Uncomment to force single-threaded execution by default:
# test-threads = 1

[profile.test]
debug = true      # Better backtraces in failures
opt-level = 0     # Faster test compilation
```

**Note**: Network-heavy tests use `#[serial]` attribute for automatic serialization without requiring global single-threaded mode

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
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# List resources (use session ID from above)
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}'
```

## Test Features Validated

### MCP Protocol Compliance
- ✅ **Protocol Version Negotiation**: 2025-11-25 specification compliance
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
- ✅ **Tasks Testing**: Task lifecycle, cancellation, result retrieval, and status polling (MCP 2025-11-25)

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

**Concurrent Server Startup Failures**:
```bash
# Symptom: "Failed to start test server" or health check timeouts when running tests in parallel
# Cause: Multiple tests trying to spawn server binaries simultaneously overwhelms health checks

# Solution 1: Run tests serially (automatic with #[serial] attribute)
cargo test --package mcp-tools-tests

# Solution 2: Force single-threaded execution
cargo test --package mcp-tools-tests -- --test-threads=1

# Solution 3: Run individual test files
cargo test --package mcp-tools-tests --test e2e_integration
cargo test --package mcp-tools-tests --test large_message_handling
cargo test --package mcp-tools-tests --test mcp_error_code_coverage

# Note: Tests that spawn servers use #[serial] attribute to prevent this issue
```

### Debug Mode

Enable detailed logging for debugging:
```bash
# Full debug output
RUST_LOG=debug cargo test -- --nocapture

# Specific module debugging
RUST_LOG=mcp_e2e_shared=debug,turul_mcp_server=info cargo test -- --nocapture
```

### Test Isolation and Execution

Each test runs its own server instance on a random port to ensure isolation:
- ✅ **No shared state** between tests
- ✅ **Automatic cleanup** when tests complete
- ✅ **Serial execution** for network-heavy tests to prevent server startup conflicts
- ✅ **Random port assignment** prevents conflicts

**Important**: Network-heavy tests (tools, resources, prompts, sampling, roots, elicitation) use the `#[serial]` attribute from the `serial_test` crate to prevent concurrent server startup issues. When multiple tests try to start servers simultaneously, health check timeouts can occur. Serial execution ensures reliable test runs.

**Running Tests Serially**:
```bash
# Option 1: Tests with #[serial] attribute run sequentially automatically
cargo test --package mcp-tools-tests

# Option 2: Force single-threaded execution for all tests
cargo test --package mcp-tools-tests -- --test-threads=1

# Option 3: Run specific test suite packages (automatically handles serialization)
cargo test --workspace --exclude turul-mcp-framework-integration-tests
```

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
- **Specification Compliance**: Full MCP 2025-11-25 protocol validation
- **Production Readiness**: Testing conditions mirror real-world usage
- **Comprehensive Coverage**: All MCP features and edge cases tested
- **Developer Experience**: Clear feedback and debugging capabilities

This ensures the MCP Framework is production-ready with complete specification compliance.