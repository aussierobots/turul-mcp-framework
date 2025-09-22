//! E2E Runtime Capabilities Validation
//!
//! This test performs actual initialize() calls to verify that servers
//! advertise truthful capabilities at runtime, not just in code.

use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

/// Test actual runtime capability truthfulness via real HTTP initialize endpoint
#[tokio::test]
async fn test_runtime_initialize_capabilities_truthfulness() {
    // Start a real test server
    let server_handle = start_test_server().await;
    let port = server_handle.port;

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create HTTP client
    let client = reqwest::Client::new();
    let server_url = format!("http://127.0.0.1:{}/mcp", port);

    // Perform actual initialize() call
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "mcp-runtime-test",
                "version": "1.0.0"
            }
        }
    });

    let response = timeout(Duration::from_secs(5), client
        .post(&server_url)
        .json(&initialize_request)
        .send())
        .await
        .expect("Request timed out")
        .expect("Failed to send request");

    assert!(response.status().is_success(), "Initialize request failed: {}", response.status());

    let body: Value = response.json().await.expect("Failed to parse JSON response");

    // Verify JSON-RPC structure
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(body["result"].is_object(), "Missing result object");

    // Extract actual runtime capabilities
    let capabilities = &body["result"]["capabilities"];

    // CRITICAL: Verify actual runtime behavior for static framework
    assert_eq!(
        capabilities["prompts"]["listChanged"],
        false,
        "❌ COMPLIANCE VIOLATION: prompts.listChanged should be false for static framework, got: {}",
        capabilities["prompts"]["listChanged"]
    );

    assert_eq!(
        capabilities["resources"]["listChanged"],
        false,
        "❌ COMPLIANCE VIOLATION: resources.listChanged should be false for static framework, got: {}",
        capabilities["resources"]["listChanged"]
    );

    assert_eq!(
        capabilities["resources"]["subscribe"],
        false,
        "❌ COMPLIANCE VIOLATION: resources.subscribe should be false until implemented, got: {}",
        capabilities["resources"]["subscribe"]
    );

    assert_eq!(
        capabilities["tools"]["listChanged"],
        false,
        "❌ COMPLIANCE VIOLATION: tools.listChanged should be false for static framework, got: {}",
        capabilities["tools"]["listChanged"]
    );

    // Verify server info
    assert_eq!(body["result"]["protocolVersion"], "2025-06-18");
    assert!(body["result"]["serverInfo"].is_object());

    println!("✅ RUNTIME VALIDATION PASSED: All capabilities are truthfully advertised");
    println!("Actual capabilities: {}", serde_json::to_string_pretty(capabilities).unwrap());

    // Cleanup
    server_handle.shutdown().await;
}

/// Test with resource test server specifically to verify URI validation
#[tokio::test]
async fn test_resource_server_runtime_capabilities() {
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::thread;

    // Try to start resource-test-server
    let mut cmd = Command::new("cargo")
        .args(&["run", "--bin", "resource-test-server", "--", "--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start resource-test-server");

    // Give it time to either start or fail
    thread::sleep(Duration::from_millis(500));

    // Check if it's still running (URI validation passed)
    match cmd.try_wait() {
        Ok(Some(status)) => {
            let stderr = cmd.stderr.take().unwrap();
            let mut stderr_content = String::new();
            std::io::Read::read_to_string(&mut stderr, &mut stderr_content).ok();

            if stderr_content.contains("Invalid resource URI") {
                panic!("❌ URI VALIDATION FAILED: Resource server couldn't start due to invalid URI: {}", stderr_content);
            } else {
                panic!("❌ Resource server exited unexpectedly: {} - stderr: {}", status, stderr_content);
            }
        }
        Ok(None) => {
            // Still running, that means URI validation passed
            println!("✅ URI VALIDATION PASSED: Resource server started successfully");
            cmd.kill().ok();
        }
        Err(e) => panic!("❌ Failed to check server status: {}", e),
    }
}

// Minimal test server for capability validation
struct TestServerHandle {
    port: u16,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServerHandle {
    async fn shutdown(self) {
        self.handle.abort();
    }
}

async fn start_test_server() -> TestServerHandle {
    use turul_mcp_server::McpServer;

    // Create a minimal server with prompts, resources, and tools
    let server = McpServer::builder()
        .name("runtime-test-server")
        .version("1.0.0")
        .title("Runtime Capabilities Test Server")
        .instructions("Test server for validating runtime capability truthfulness")
        .with_prompts()
        .with_resources()
        .with_tools()
        .bind_address("127.0.0.1:0".parse().unwrap())
        .build()
        .expect("Failed to build test server");

    let port = server.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        server.run().await.ok();
    });

    TestServerHandle { port, handle }
}