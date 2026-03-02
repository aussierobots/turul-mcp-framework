# Test Utilities Reference

Complete API reference for `McpTestClient`, `TestServerManager`, and `TestFixtures` from the `mcp-e2e-shared` crate (`mcp_e2e_shared` lib name).

## McpTestClient

HTTP client for E2E testing with automatic session management.

### Construction

```rust
let client = McpTestClient::new(port);  // Connect to server on given port
```

### Session Lifecycle

```rust
// Initialize with default capabilities (empty)
let init_result = client.initialize().await?;
// Returns: HashMap<String, Value> with "result" containing server info

// Initialize with specific client capabilities
let init_result = client.initialize_with_capabilities(json!({
    "tools": { "listChanged": false }
})).await?;

// Initialize with explicit protocol version (for testing version negotiation)
let init_result = client.initialize_with_capabilities_and_version(
    json!({}),
    "2025-06-18"  // Intentional: testing legacy backward compat
).await?;

// Complete strict lifecycle handshake (REQUIRED for strict mode servers)
let ack = client.send_initialized_notification().await?;

// Get current session ID
let session_id: Option<&String> = client.session_id();
```

### MCP Operations

```rust
// List tools
let result = client.list_tools().await?;
// Returns: {"result": {"tools": [...]}}

// Call a tool
let result = client.call_tool("calculator_add", json!({"a": 5.0, "b": 3.0})).await?;
// Returns: {"result": {"content": [...], "structuredContent": {...}}}

// Call a tool with SSE streaming (for progress notifications)
let response: reqwest::Response = client.call_tool_with_sse("slow_op", json!({"input": "test"})).await?;
// Returns raw Response — parse SSE events from body

// List resources
let result = client.list_resources().await?;
// Returns: {"result": {"resources": [...]}}

// Read a resource
let result = client.read_resource("file:///data/config.json").await?;
// Returns: {"result": {"contents": [{"uri": "...", "text": "..."}]}}

// Subscribe to resource changes
let result = client.subscribe_resource("file:///data/config.json").await?;

// List prompts
let result = client.list_prompts().await?;
// Returns: {"result": {"prompts": [...]}}

// Get a prompt with arguments
let mut args = HashMap::new();
args.insert("name".to_string(), json!("Alice"));
let result = client.get_prompt("greeting", Some(args)).await?;
// Returns: {"result": {"messages": [...]}}

// Generic JSON-RPC request
let result = client.make_request("custom/method", json!({"key": "value"}), 42).await?;

// Send a notification (no response expected)
let ack = client.send_notification(json!({
    "jsonrpc": "2.0",
    "method": "notifications/cancelled",
    "params": {"requestId": 5, "reason": "timeout"}
})).await?;

// Connect to SSE stream for real-time events
let response: reqwest::Response = client.connect_sse().await?;
```

### SSE Event Testing

```rust
// Test SSE notifications (connects for 2 seconds, collects events)
let events: Vec<String> = client.test_sse_notifications().await?;
```

## TestServerManager

Manages server process lifecycle for E2E tests. Auto-kills on `Drop`.

### Starting Servers

```rust
// Start by server binary name (auto-builds if not compiled)
let server = TestServerManager::start("tools-test-server").await?;

// Convenience methods for known test servers
let server = TestServerManager::start_resource_server().await?;
let server = TestServerManager::start_prompts_server().await?;
let server = TestServerManager::start_tools_server().await?;
let server = TestServerManager::start_sampling_server().await?;
let server = TestServerManager::start_roots_server().await?;
let server = TestServerManager::start_elicitation_server().await?;

// Get allocated port
let port: u16 = server.port();
```

### Server Binary Mapping

| Binary Name | Package |
|---|---|
| `resource-test-server` | `resource-test-server` |
| `prompts-test-server` | `prompts-test-server` |
| `tools-test-server` | `tools-test-server` |
| `sampling-server` | `sampling-server` |
| `roots-server` | `roots-server` |
| `elicitation-server` | `elicitation-server` |
| `tasks-e2e-inmemory-server` | `tasks-e2e-inmemory-server` |

### Port Allocation

`TestServerManager` uses OS ephemeral port allocation (`TcpListener::bind("127.0.0.1:0")`) for reliable port assignment. No hardcoded ports, no third-party port pickers.

### Health Check

After spawning the server process, `TestServerManager::start()` polls the server with `POST /mcp` (initialize request) every 300ms for up to 15 seconds. If the server doesn't respond, it kills the process and returns an error.

## TestFixtures

Pre-built capability objects and assertion helpers.

### Capabilities

```rust
TestFixtures::resource_capabilities()  // {"resources": {"subscribe": true, "listChanged": false}}
TestFixtures::tools_capabilities()     // {"tools": {"listChanged": false}}
TestFixtures::prompts_capabilities()   // {"prompts": {"listChanged": false}}
```

### Initialization Assertions

```rust
// Verify standard MCP initialization response (expects 2025-11-25)
TestFixtures::verify_initialization_response(&result);

// Verify legacy backward-compat response (accepts 2025-11-25 or 2025-06-18)
TestFixtures::verify_initialization_response_legacy(&result);
```

### Response Assertions

```rust
TestFixtures::verify_error_response(&result);            // Assert JSON-RPC error structure
TestFixtures::verify_resource_list_response(&result);     // Assert valid resources/list
TestFixtures::verify_resource_content_response(&result);  // Assert valid resources/read
TestFixtures::verify_prompt_response(&result);            // Assert valid prompts/get
TestFixtures::verify_prompts_list_response(&result);      // Assert valid prompts/list
```

### Content Extraction

```rust
// Extract structured content from tool result
let structured: Option<&Value> = TestFixtures::extract_tool_structured_content(&result);

// Extract text content from tool result
let text: String = TestFixtures::extract_tool_content_text(&result);

// Extract first result object from structured content
let obj: Option<Value> = TestFixtures::extract_tool_result_object(&result);

// Extract tools array from tools/list response
let tools: Option<Vec<Value>> = TestFixtures::extract_tools_list(&result);
```

### Prompt Test Arguments

```rust
TestFixtures::create_string_args()    // {"required_text": "test string", "optional_text": "optional value"}
TestFixtures::create_number_args()    // {"count": "42", "multiplier": "3.14"}  (strings per MCP spec)
TestFixtures::create_boolean_args()   // {"enable_feature": "true", "debug_mode": "false"}
TestFixtures::create_template_args()  // {"name": "Alice", "topic": "machine learning", "style": "casual"}
```

## SessionTestUtils

Utilities for testing session-aware behavior.

```rust
// Verify session consistency across multiple requests
SessionTestUtils::verify_session_consistency(&client).await?;

// Test session-aware resource behavior
SessionTestUtils::test_session_aware_resource(&client).await?;

// Test session-aware prompt behavior
SessionTestUtils::test_session_aware_prompt(&client).await?;
```
