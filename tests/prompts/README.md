# MCP Prompts E2E Testing

This directory contains comprehensive End-to-End testing for MCP Prompts functionality, validating complete prompt protocol compliance with real HTTP transport.

## Overview

The prompts E2E testing validates:
- **Prompt Discovery**: `prompts/list` endpoint functionality
- **Prompt Getting**: `prompts/get` with various argument types
- **Prompt Arguments**: String, number, boolean, and template arguments
- **Multi-Message Prompts**: User/assistant conversation patterns
- **Session Context**: Session-aware prompt behavior
- **Error Handling**: Proper error responses and edge cases
- **SSE Notifications**: Real-time prompt list change notifications

## Test Structure

```
tests/prompts/
├── Cargo.toml                           # Test dependencies and configuration
├── README.md                            # This documentation
├── src/                                 # Test utilities (if any)
└── tests/                               # All E2E test files
    ├── e2e_integration.rs               # Original comprehensive E2E tests
    ├── e2e_shared_integration.rs        # Tests using shared utilities
    ├── sse_notifications_test.rs        # SSE-specific functionality tests
    ├── mcp_prompts_protocol_coverage.rs      # Protocol compliance tests
    └── mcp_prompts_specification.rs           # Specification validation tests
```

## Prompts Test Server

The tests use `prompts-test-server` which provides 11 comprehensive test prompts:

### Basic Prompts (Coverage)
- `simple_prompt` - Basic single-message prompt with no arguments
- `string_args_prompt` - String argument validation
- `number_args_prompt` - Numeric argument validation
- `boolean_args_prompt` - Boolean argument validation
- `template_prompt` - URI template variables in prompt content

### Advanced Prompts (Features)  
- `multi_message_prompt` - User/assistant conversation pattern
- `session_aware_prompt` - Session-dependent prompt generation
- `dynamic_prompt` - Context-dependent dynamic content

### Edge Case Prompts
- `validation_failure_prompt` - Always returns validation errors
- `complex_args_prompt` - Multiple mixed argument types
- `all_features_prompt` - All optional fields and capabilities

## Setup

### Prerequisites

1. **Build Prompts Test Server**:
```bash
# From project root
cargo build --package prompts-test-server
```

2. **Verify Server Binary**:
```bash
ls -la target/debug/prompts-test-server
```

### Dependencies

The test suite requires:
- `mcp-e2e-shared` - Shared testing utilities
- `reqwest` - HTTP client for real transport testing
- `tokio` - Async runtime for concurrent testing
- `serde_json` - JSON handling and validation
- `tracing` - Logging and debugging

## Running Tests

### All Prompt Tests
```bash
cd tests/prompts
cargo test -- --nocapture
```

### Test Categories

**Protocol Compliance Tests**:
```bash
cargo test e2e_integration -- --nocapture
```

**Shared Utilities Tests**:
```bash
cargo test e2e_shared_integration -- --nocapture
```

**SSE Notification Tests**:
```bash
cargo test sse_notifications -- --nocapture
```

**MCP Specification Tests**:
```bash
cargo test mcp_prompts_specification -- --nocapture
cargo test mcp_prompts_protocol_coverage -- --nocapture
```

### Individual Test Examples

**Basic Functionality**:
```bash
cargo test test_mcp_initialize_session -- --nocapture
cargo test test_prompts_list -- --nocapture
cargo test test_simple_prompt_get -- --nocapture
cargo test test_string_args_prompt_get -- --nocapture
```

**Advanced Features**:
```bash
cargo test test_session_aware_prompt -- --nocapture
cargo test test_multi_message_prompt -- --nocapture
cargo test test_template_prompt_with_variables -- --nocapture
```

**Error Handling**:
```bash
cargo test test_validation_failure_prompt -- --nocapture
cargo test test_nonexistent_prompt -- --nocapture
```

**SSE Notifications**:
```bash
cargo test test_sse_connection_establishment -- --nocapture
cargo test test_sse_prompts_list_changed_notification -- --nocapture
cargo test test_sse_prompts_session_isolation -- --nocapture
```

## Test Validation Points

### MCP Protocol Compliance
- ✅ **JSON-RPC 2.0**: Proper request/response structure
- ✅ **Protocol Version**: 2025-06-18 specification compliance
- ✅ **HTTP Headers**: Correct Content-Type and session headers
- ✅ **Session Management**: UUID v7 session IDs with proper isolation
- ✅ **Capability Truthfulness**: Server capabilities accurately reflect functionality

### Prompt Operations
- ✅ **Prompt Listing**: Proper `prompts/list` response structure
- ✅ **Prompt Getting**: Valid `prompts/get` with name parameter
- ✅ **Argument Validation**: String, number, boolean argument handling
- ✅ **Template Variables**: Variable substitution in prompt content
- ✅ **Multi-Message Patterns**: User/assistant conversation flows

### Content Validation
- ✅ **PromptMessage Structure**: Proper `role`, `content` fields
- ✅ **Message Roles**: Correct User/Assistant role handling
- ✅ **Argument Structure**: Valid `name`, `description`, `required` fields
- ✅ **Dynamic Content**: Session-aware and context-dependent prompts
- ✅ **Metadata Fields**: Optional fields like `title`, annotations

### Error Handling
- ✅ **HTTP Errors**: Proper status codes and error responses
- ✅ **MCP Errors**: Correct JSON-RPC error structure
- ✅ **Invalid Arguments**: Graceful handling of malformed arguments
- ✅ **Missing Prompts**: NotFound error responses
- ✅ **Validation Errors**: Argument validation error handling
- ✅ **Production Safety**: Zero panic! statements in production code paths

### Session & SSE
- ✅ **Session Consistency**: Same session across multiple requests
- ✅ **Session Isolation**: Different clients get different sessions
- ✅ **SSE Event Bridge**: Real-time notification delivery
- ✅ **Event Format**: Proper SSE event structure (`data:`, `event:`)
- ✅ **List Change Notifications**: `listChanged` events (camelCase)

## Manual Testing

### Start Test Server
```bash
# Start on specific port
cargo run --package prompts-test-server -- --port 8081

# Start on random port (check logs for actual port)
cargo run --package prompts-test-server
```

### Manual API Testing

**Initialize Session**:
```bash
curl -X POST http://127.0.0.1:8081/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {
        "prompts": {
          "listChanged": true
        }
      },
      "clientInfo": {
        "name": "manual-test",
        "version": "1.0.0"
      }
    }
  }'
```

**List Prompts** (use session ID from initialization):
```bash
curl -X POST http://127.0.0.1:8081/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "prompts/list",
    "params": {}
  }'
```

**Get Prompt**:
```bash
curl -X POST http://127.0.0.1:8081/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "prompts/get",
    "params": {
      "name": "simple_prompt"
    }
  }'
```

**Get Prompt with Arguments**:
```bash
curl -X POST http://127.0.0.1:8081/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "prompts/get",
    "params": {
      "name": "string_args_prompt",
      "arguments": {
        "user_input": "Hello World"
      }
    }
  }'
```

**SSE Stream** (in separate terminal):
```bash
curl -N -H 'Accept: text/event-stream' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  http://127.0.0.1:8081/mcp
```

## Troubleshooting

### Common Issues

**Server Build Failure**:
```bash
# Check if prompts-test-server builds
cargo build --package prompts-test-server

# Look for compilation errors
cargo check --package prompts-test-server
```

**Port Binding Errors**:
```bash
# Check for processes using ports
lsof -i :8081

# Kill if necessary
pkill -f prompts-test-server
```

**Test Timeout Issues**:
```bash
# Run with debug logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Check server startup logs
RUST_LOG=info cargo test -- --nocapture 2>&1 | grep "Server started"
```

**Session Errors**:
```bash
# Verify session header propagation
RUST_LOG=mcp_e2e_shared=debug cargo test test_session -- --nocapture
```

### Debug Output

Enable detailed logging for debugging:
```bash
# Full debug logging
RUST_LOG=debug cargo test -- --nocapture

# Specific components
RUST_LOG=prompts_test_server=info,mcp_e2e_shared=debug cargo test -- --nocapture
```

### Expected Log Output

Successful test run should show:
```
INFO prompts_test_server: 🚀 Starting MCP Prompts Test Server on port XXXX
INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
INFO turul_mcp_server::builder:    - Prompts: true
INFO turul_mcp_server::server: ✅ SSE event bridge established successfully
INFO mcp_e2e_shared::e2e_utils: Server prompts-test-server started successfully on port XXXX
```

## Test Development

### Adding New Prompt Tests

1. **Use Shared Utilities**:
```rust
use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};

#[tokio::test]
async fn test_new_prompt_feature() {
    tracing_subscriber::fmt::init();
    
    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());
    
    client.initialize().await.expect("Failed to initialize");
    
    // Your test logic here
}
```

2. **Follow Test Patterns**:
- Initialize with appropriate capabilities
- Use descriptive test names
- Validate response structure with `TestFixtures::verify_*` methods
- Include error case testing
- Test session consistency where relevant

3. **Validate Responses**:
```rust
// For prompt list responses
TestFixtures::verify_prompt_list_response(&result);

// For prompt get responses  
TestFixtures::verify_prompt_get_response(&result);

// For error responses
TestFixtures::verify_error_response(&result);
```

### Test Organization

- **e2e_integration.rs**: Original comprehensive tests (can be deprecated)
- **e2e_shared_integration.rs**: Tests using shared utilities (preferred)
- **sse_notifications_test.rs**: SSE-specific functionality
- **protocol_***: MCP specification compliance tests

Keep tests focused and use descriptive names that explain what is being validated.

## Architecture

The prompts E2E testing architecture:

```
Test Client (HTTP) ←→ Prompts Test Server (Real MCP Server)
     ↓                         ↓
Session Management      11 Test Prompts
SSE Event Stream        Full MCP Compliance
Protocol Validation     Argument Validation
```

This provides real-world testing conditions that mirror production usage while validating complete MCP 2025-11-25 specification compliance.