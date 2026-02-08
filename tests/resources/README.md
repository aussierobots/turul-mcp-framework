# MCP Resources E2E Testing

This directory contains comprehensive End-to-End testing for MCP Resources functionality, validating complete resource protocol compliance with real HTTP transport.

## Overview

The resources E2E testing validates:
- **Resource Discovery**: `resources/list` endpoint functionality
- **Resource Reading**: `resources/read` with various content types
- **Resource Subscriptions**: Tests verify `resources/subscribe` is correctly unimplemented
- **URI Templates**: Variable substitution in resource URIs
- **Session Context**: Session-aware resource behavior
- **Error Handling**: Proper error responses and edge cases
- **SSE Notifications**: Real-time resource change notifications

## Test Structure

```
tests/resources/
├── Cargo.toml                           # Test dependencies and configuration
├── README.md                            # This documentation
├── src/                                 # Test utilities (if any)
└── tests/                               # All E2E test files
    ├── e2e_integration.rs               # Original comprehensive E2E tests
    ├── e2e_shared_integration.rs        # Tests using shared utilities
    ├── sse_notifications_test.rs        # SSE-specific functionality tests
    ├── mcp_resources_protocol_coverage.rs    # Protocol compliance tests
    └── mcp_resources_specification.rs         # Specification validation tests
```

## Resource Test Server

The tests use `resource-test-server` which provides 17 comprehensive test resources:

### Basic Resources (Coverage)
- `file:///tmp/test.txt` - File reading with error handling  
- `memory://data` - Fast in-memory JSON data
- `error://not_found` - Always returns NotFound errors
- `slow://delayed` - Configurable delay simulation
- `template://items/{id}` - URI template variables
- `empty://content` - Empty content edge case
- `large://dataset` - Large content (configurable size)
- `binary://image` - Binary data with MIME types

### Advanced Resources (Features)  
- `session://info` - Session-aware resource
- `subscribe://updates` - Subscription support
- `notify://trigger` - SSE notification triggers
- `multi://contents` - Multiple ResourceContent items
- `paginated://items` - Cursor-based pagination

### Edge Case Resources
- `invalid://bad-chars-and-spaces` - Intentionally non-compliant URI for error testing
- `long://very-long-path...` - Very long URI testing
- `meta://dynamic` - _meta field behavior changes
- `complete://all-fields` - All optional fields populated

## Setup

### Prerequisites

1. **Build Resource Test Server**:
```bash
# From project root
cargo build --package resource-test-server
```

2. **Verify Server Binary**:
```bash
ls -la target/debug/resource-test-server
```

### Dependencies

The test suite requires:
- `mcp-e2e-shared` - Shared testing utilities
- `reqwest` - HTTP client for real transport testing
- `tokio` - Async runtime for concurrent testing
- `serde_json` - JSON handling and validation
- `tracing` - Logging and debugging

## Running Tests

### All Resource Tests
```bash
cd tests/resources
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
cargo test mcp_resources_specification -- --nocapture
cargo test mcp_resources_protocol_coverage -- --nocapture
```

### Individual Test Examples

**Basic Functionality**:
```bash
cargo test test_mcp_initialize_session -- --nocapture
cargo test test_resources_list -- --nocapture
cargo test test_file_resource_read -- --nocapture
cargo test test_memory_resource_read -- --nocapture
```

**Advanced Features**:
```bash
cargo test test_session_aware_resource -- --nocapture
cargo test test_resource_subscription -- --nocapture
cargo test test_template_resource_with_variables -- --nocapture
```

**Error Handling**:
```bash
cargo test test_error_resource_handling -- --nocapture
cargo test test_validation_failure -- --nocapture
```

**SSE Notifications**:
```bash
cargo test test_sse_connection_establishment -- --nocapture
cargo test test_sse_resource_list_changed_notification -- --nocapture
cargo test test_sse_session_isolation -- --nocapture
```

## Test Validation Points

### MCP Protocol Compliance
- ✅ **JSON-RPC 2.0**: Proper request/response structure
- ✅ **Protocol Version**: 2025-06-18 specification compliance
- ✅ **HTTP Headers**: Correct Content-Type and session headers
- ✅ **Session Management**: UUID v7 session IDs with proper isolation
- ✅ **Capability Truthfulness**: Server capabilities accurately reflect functionality

### Resource Operations
- ✅ **Resource Listing**: Proper `resources/list` response structure
- ✅ **Resource Reading**: Valid `resources/read` with URI parameter
- ❌ **Resource Subscription**: `resources/subscribe` not yet implemented (advertises `subscribe: false`)
- ✅ **Content Types**: Text, binary, JSON, and empty content handling
- ✅ **URI Templates**: Variable substitution in resource URIs
- ✅ **URI Validation**: Robust error collection for invalid resource URIs

### Content Validation
- ✅ **ResourceContent Structure**: Proper `uri`, `text`/`blob` fields
- ✅ **MIME Types**: Correct content type detection and reporting
- ✅ **Binary Data**: Base64 encoding for binary content
- ✅ **Large Content**: Performance with large datasets
- ✅ **Metadata Fields**: Optional `mimeType`, `size`, annotations

### Error Handling
- ✅ **HTTP Errors**: Proper status codes and error responses
- ✅ **MCP Errors**: Correct JSON-RPC error structure
- ✅ **Invalid URIs**: Graceful handling of malformed URIs
- ✅ **Missing Resources**: NotFound error responses
- ✅ **Permission Errors**: Access control error handling
- ✅ **Production Safety**: Zero panic! statements in production code paths

### Session & SSE
- ✅ **Session Consistency**: Same session across multiple requests
- ✅ **Session Isolation**: Different clients get different sessions
- ✅ **SSE Event Bridge**: Real-time notification delivery
- ✅ **Event Format**: Proper SSE event structure (`data:`, `event:`)
- ✅ **List Change Notifications**: `listChanged` events (camelCase)
- ✅ **Resource Templates**: `resources/templates/list` endpoint for URI template discovery

## Manual Testing

### Start Test Server
```bash
# Start on specific port
cargo run --package resource-test-server -- --port 8080

# Start on random port (check logs for actual port)
cargo run --package resource-test-server
```

### Manual API Testing

**Initialize Session**:
```bash
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {
        "resources": {
          "subscribe": true,
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

**List Resources** (use session ID from initialization):
```bash
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "resources/list",
    "params": {}
  }'
```

**Read Resource**:
```bash
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "resources/read",
    "params": {
      "uri": "memory://data"
    }
  }'
```

**Subscribe to Resource**:
```bash
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "resources/subscribe",
    "params": {
      "uri": "subscribe://updates"
    }
  }'
```

**SSE Stream** (in separate terminal):
```bash
curl -N -H 'Accept: text/event-stream' \
  -H 'Mcp-Session-Id: YOUR_SESSION_ID' \
  http://127.0.0.1:8080/mcp
```

## Troubleshooting

### Common Issues

**Server Build Failure**:
```bash
# Check if resource-test-server builds
cargo build --package resource-test-server

# Look for compilation errors
cargo check --package resource-test-server
```

**Port Binding Errors**:
```bash
# Check for processes using ports
lsof -i :8080

# Kill if necessary
pkill -f resource-test-server
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
RUST_LOG=resource_test_server=info,mcp_e2e_shared=debug cargo test -- --nocapture
```

### Expected Log Output

Successful test run should show:
```
INFO resource_test_server: 🚀 Starting MCP Resource Test Server on port XXXX
INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
INFO turul_mcp_server::builder:    - Resources: true
INFO turul_mcp_server::server: ✅ SSE event bridge established successfully
INFO mcp_e2e_shared::e2e_utils: Server resource-test-server started successfully on port XXXX
```

## Test Development

### Adding New Resource Tests

1. **Use Shared Utilities**:
```rust
use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};

#[tokio::test]
async fn test_new_resource_feature() {
    tracing_subscriber::fmt::init();
    
    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
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
// For resource list responses
TestFixtures::verify_resource_list_response(&result);

// For resource content responses  
TestFixtures::verify_resource_content_response(&result);

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

The resource E2E testing architecture:

```
Test Client (HTTP) ←→ Resource Test Server (Real MCP Server)
     ↓                         ↓
Session Management      17 Test Resources
SSE Event Stream        Full MCP Compliance
Protocol Validation     Error Simulation
```

This provides real-world testing conditions that mirror production usage while validating complete MCP 2025-11-25 specification compliance.