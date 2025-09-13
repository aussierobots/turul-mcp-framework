# MCP E2E Shared Testing Utilities

This directory contains shared testing infrastructure and utilities used across all E2E test suites in the MCP Framework. It provides common functionality for HTTP client operations, server management, test fixtures, and session validation.

## Overview

The shared utilities crate (`mcp-e2e-shared`) provides:
- **McpTestClient**: HTTP client with session management and MCP protocol support
- **TestServerManager**: Automated test server lifecycle management
- **TestFixtures**: Common test data and response validation utilities
- **SessionTestUtils**: Session management and validation helpers

## Crate Structure

```
tests/shared/
├── Cargo.toml                                # Shared utilities crate configuration
├── README.md                                 # This documentation
├── src/
│   ├── lib.rs                               # Public API exports
│   └── e2e_utils.rs                         # Core utilities implementation
└── tests/
    └── session_validation_comprehensive.rs  # Session validation tests
```

## Core Components

### McpTestClient

HTTP client with built-in MCP protocol support and session management:

```rust
use mcp_e2e_shared::McpTestClient;

let mut client = McpTestClient::new(server_port);

// Initialize MCP session
client.initialize().await?;

// Access session ID
let session_id = client.session_id().unwrap();

// Make MCP requests
let resources = client.list_resources().await?;
let prompts = client.list_prompts().await?;
let content = client.read_resource("memory://data").await?;
```

**Key Features**:
- Automatic session management with proper headers
- Built-in JSON-RPC 2.0 request/response handling
- SSE (Server-Sent Events) testing capabilities
- Error handling and timeout management

### TestServerManager

Automated server lifecycle management for test isolation:

```rust
use mcp_e2e_shared::TestServerManager;

// Start resource test server
let resource_server = TestServerManager::start_resource_server().await?;
let port = resource_server.port();

// Start prompts test server  
let prompts_server = TestServerManager::start_prompts_server().await?;
let port = prompts_server.port();

// Servers automatically shut down when dropped
```

**Key Features**:
- Random port assignment to avoid conflicts
- Automatic server startup and shutdown
- Health checks and startup validation
- Process isolation and cleanup

### TestFixtures

Common test data and validation helpers:

```rust
use mcp_e2e_shared::TestFixtures;

// Create test capabilities
let resource_caps = TestFixtures::resource_capabilities();
let prompts_caps = TestFixtures::prompts_capabilities();

// Create test arguments
let string_args = TestFixtures::create_string_args();
let number_args = TestFixtures::create_number_args();

// Validate responses
TestFixtures::verify_initialization_response(&response);
TestFixtures::verify_resource_list_response(&response);
TestFixtures::verify_prompt_get_response(&response);
TestFixtures::verify_error_response(&response);
```

**Available Fixtures**:
- Protocol capabilities for different MCP features
- Test arguments for various data types
- Response validation for all MCP operations
- Error response validation

### SessionTestUtils

Session management and validation utilities:

```rust
use mcp_e2e_shared::SessionTestUtils;

// Verify session consistency across requests
SessionTestUtils::verify_session_consistency(&client).await?;

// Test session-aware resources
SessionTestUtils::test_session_aware_resource(&client).await?;

// Test session-aware prompts
SessionTestUtils::test_session_aware_prompt(&client).await?;
```

**Session Features**:
- Cross-request session validation
- Multi-client session isolation testing
- Session-aware resource/prompt testing
- Concurrent session operation validation

## Dependencies

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
portpicker = "0.1"
futures = "0.3"
```

## Usage Patterns

### Basic E2E Test Structure

```rust
use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};

#[tokio::test]
async fn test_mcp_operation() {
    tracing_subscriber::fmt::init();
    
    // Start appropriate test server
    let server = TestServerManager::start_resource_server().await
        .expect("Failed to start server");
    
    // Create and initialize client
    let mut client = McpTestClient::new(server.port());
    client.initialize().await.expect("Failed to initialize");
    
    // Perform test operations
    let result = client.list_resources().await.expect("Failed to list resources");
    
    // Validate results
    TestFixtures::verify_resource_list_response(&result);
}
```

### Multi-Client Session Testing

```rust
use mcp_e2e_shared::{McpTestClient, TestServerManager};

#[tokio::test]
async fn test_session_isolation() {
    let server = TestServerManager::start_resource_server().await?;
    
    // Create multiple clients
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());
    
    // Initialize both
    client1.initialize().await?;
    client2.initialize().await?;
    
    // Verify different session IDs
    assert_ne!(client1.session_id(), client2.session_id());
    
    // Test independent operations
    let result1 = client1.list_resources().await?;
    let result2 = client2.list_resources().await?;
    
    // Both should succeed independently
    assert!(result1.contains_key("result"));
    assert!(result2.contains_key("result"));
}
```

### SSE Notification Testing

```rust
use mcp_e2e_shared::{McpTestClient, TestServerManager};

#[tokio::test]
async fn test_sse_notifications() {
    let server = TestServerManager::start_resource_server().await?;
    let mut client = McpTestClient::new(server.port());
    
    client.initialize().await?;
    
    // Test SSE event stream
    let events = client.test_sse_notifications().await?;
    
    // Validate SSE format
    for event in &events {
        if event.contains("data:") {
            assert!(event.starts_with("data:"));
        }
        if event.contains("event:") {
            assert!(event.starts_with("event:"));
        }
    }
}
```

## Running Shared Utility Tests

### Session Validation Tests
```bash
cd tests/shared
cargo test session_validation -- --nocapture
```

### Specific Session Tests
```bash
cargo test test_session_id_generation_and_persistence -- --nocapture
cargo test test_cross_request_session_consistency -- --nocapture
cargo test test_session_isolation_between_clients -- --nocapture
cargo test test_concurrent_session_operations -- --nocapture
```

### Debug Mode
```bash
# Full debug output
RUST_LOG=debug cargo test -- --nocapture

# Specific module debugging
RUST_LOG=mcp_e2e_shared=debug cargo test -- --nocapture
```

## Test Development Guidelines

### Adding New Utilities

1. **Add to e2e_utils.rs**:
```rust
impl TestFixtures {
    pub fn new_validation_method(response: &serde_json::Value) {
        // Validation logic
        assert!(response.contains_key("result"));
        // Add specific validations
    }
}
```

2. **Export in lib.rs**:
```rust
pub use e2e_utils::{McpTestClient, TestServerManager, TestFixtures, SessionTestUtils};
```

3. **Document in README** (this file)

### Best Practices

1. **Use Descriptive Names**: Test methods should clearly indicate what they validate
2. **Provide Context**: Include tracing logs for debugging
3. **Validate Structure**: Check both success and error response formats
4. **Test Isolation**: Each test should be independent
5. **Resource Cleanup**: Rely on automatic server shutdown via Drop trait

### Error Handling Patterns

```rust
// Prefer Result types for operations that may fail
async fn perform_operation(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Implementation
}

// Use expect() for setup operations that should always succeed
let server = TestServerManager::start_resource_server().await
    .expect("Failed to start resource server");

// Use proper error propagation in tests
let result = client.list_resources().await
    .map_err(|e| format!("Failed to list resources: {}", e))?;
```

## Integration with Test Suites

### Resources Tests
```rust
// In tests/resources/tests/*.rs
use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};

#[tokio::test]
async fn test_resource_operation() {
    let server = TestServerManager::start_resource_server().await?;
    // Use shared utilities...
}
```

### Prompts Tests
```rust  
// In tests/prompts/tests/*.rs
use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};

#[tokio::test]
async fn test_prompt_operation() {
    let server = TestServerManager::start_prompts_server().await?;
    // Use shared utilities...
}
```

## Troubleshooting

### Common Issues

**Compilation Errors**:
```bash
# Build shared utilities first
cd tests/shared
cargo build

# Check for errors
cargo check
```

**Missing Dependencies**:
```bash
# Ensure all required features are enabled
grep -r "mcp-e2e-shared" tests/*/Cargo.toml
```

**Port Binding Issues**:
```bash
# Random port assignment should prevent conflicts
# If issues persist, check for hanging processes
pkill -f "resource-test-server\|prompts-test-server"
```

**Session Errors**:
```bash
# Debug session management
RUST_LOG=mcp_e2e_shared=debug cargo test session -- --nocapture
```

### Debug Logging

The shared utilities support comprehensive logging:

```bash
# Enable all debug logging
RUST_LOG=debug cargo test -- --nocapture

# Focus on shared utilities
RUST_LOG=mcp_e2e_shared=debug cargo test -- --nocapture

# Focus on specific components
RUST_LOG=mcp_e2e_shared::e2e_utils=trace cargo test -- --nocapture
```

## Architecture

The shared utilities provide a common foundation:

```
┌─────────────────────────────────────────┐
│           Test Suites                   │
│  ┌─────────────┐    ┌─────────────────┐ │
│  │  Resources  │    │    Prompts      │ │
│  │    Tests    │    │     Tests       │ │
│  └─────────────┘    └─────────────────┘ │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│         mcp-e2e-shared                  │
│  ┌──────────────┐  ┌─────────────────┐  │
│  │ McpTestClient│  │TestServerManager│  │
│  └──────────────┘  └─────────────────┘  │
│  ┌──────────────┐  ┌─────────────────┐  │
│  │ TestFixtures │  │SessionTestUtils │  │
│  └──────────────┘  └─────────────────┘  │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│        Test Servers                     │
│  ┌─────────────────┐ ┌────────────────┐ │
│  │resource-test-   │ │prompts-test-   │ │
│  │server           │ │server          │ │
│  └─────────────────┘ └────────────────┘ │
└─────────────────────────────────────────┘
```

This architecture ensures:
- **Code Reuse**: Common functionality shared across test suites
- **Consistency**: Uniform testing patterns and validation
- **Maintainability**: Centralized utilities for easier updates
- **Isolation**: Independent test execution with proper cleanup