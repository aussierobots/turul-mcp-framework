//! # Working Examples Validation Test Suite
//!
//! This test suite validates that documented working examples actually compile and function correctly.
//! This serves as a comprehensive integration test for the MCP Framework.

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Test that documented working examples compile successfully
#[tokio::test]
async fn test_working_examples_compilation() {
    let working_examples = vec![
        "minimal-server",
        "derive-macro-server", 
        "function-macro-server",
        "macro-calculator",
        "notification-server",
        "stateful-server",
        "client-initialise-report",
    ];
    
    for example in working_examples {
        println!("Testing compilation of {}", example);
        
        let output = Command::new("cargo")
            .args(&["check", "-p", example, "--quiet"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect(&format!("Failed to execute cargo check for {}", example));
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Example {} failed to compile:\n{}", example, stderr);
        }
        
        println!("✅ {} compiles successfully", example);
    }
}

/// Test that MCP Streamable HTTP compliance test passes
#[tokio::test] 
async fn test_mcp_streamable_http_compliance() {
    // Start server in background
    let mut server = Command::new("cargo")
        .args(&["run", "--example", "client-initialise-server", "--", "--port", "52940"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start test server");
    
    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(2000)).await;
    
    // Run compliance test with timeout
    let test_result = timeout(Duration::from_secs(30), async {
        Command::new("cargo")
            .args(&["run", "--example", "client-initialise-report", "--", "--url", "http://127.0.0.1:52940/mcp"])
            .env("RUST_LOG", "info")
            .output()
            .expect("Failed to execute compliance test")
    }).await;
    
    // Clean up server
    let _ = server.kill();
    
    match test_result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!("MCP compliance test failed:\nstdout: {}\nstderr: {}", stdout, stderr);
            }
            
            // Check for compliance success message
            if stdout.contains("🎆 FULLY MCP COMPLIANT") {
                println!("✅ MCP Streamable HTTP compliance test passed");
                println!("Test output: {}", stdout.lines().last().unwrap_or(""));
            } else {
                panic!("MCP compliance test did not show success message:\n{}", stdout);
            }
        }
        Err(_) => {
            panic!("MCP compliance test timed out after 30 seconds");
        }
    }
}

/// Test that examples can be started without immediate crashes
#[tokio::test]
async fn test_examples_startup() {
    let examples_with_ports = vec![
        ("minimal-server", None),
        ("notification-server", Some("8005")),
        ("derive-macro-server", Some("8765")),
    ];
    
    for (example, port) in examples_with_ports {
        println!("Testing startup of {}", example);
        
        let mut cmd = Command::new("cargo");
        cmd.args(&["run", "-p", example]);
        
        if let Some(p) = port {
            // Use a different port to avoid conflicts
            let test_port = format!("808{}", p.chars().last().unwrap());
            cmd.env("PORT", test_port);
        }
        
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect(&format!("Failed to start {}", example));
        
        // Let it run for 2 seconds
        tokio::time::sleep(Duration::from_millis(2000)).await;
        
        // Check if it's still running (not crashed)
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let stderr = child.stderr.take().map(|mut s| {
                        let mut buf = Vec::new();
                        std::io::Read::read_to_end(&mut s, &mut buf).ok();
                        String::from_utf8_lossy(&buf).to_string()
                    }).unwrap_or_default();
                    
                    panic!("{} crashed during startup: {}", example, stderr);
                }
            }
            Ok(None) => {
                // Still running, kill it
                let _ = child.kill();
                println!("✅ {} started successfully", example);
            }
            Err(e) => {
                panic!("Error checking status of {}: {}", example, e);
            }
        }
    }
}

/// Test that the framework handles basic tool operations correctly
#[tokio::test] 
async fn test_basic_framework_functionality() {
    // This test validates core framework functionality without requiring examples
    
    // Test 1: Verify that derive macros work
    use turul_mcp_derive::McpTool;
    use turul_mcp_server::McpTool;
    use serde_json::json;
    
    #[derive(McpTool, Clone)]
    #[tool(name = "test_add", description = "Test addition")]
    struct TestAddTool {
        #[param(description = "First number")]
        a: f64,
        #[param(description = "Second number")]  
        b: f64,
    }
    
    impl TestAddTool {
        async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> turul_mcp_server::McpResult<String> {
            Ok(format!("{}", self.a + self.b))
        }
    }
    
    let tool = TestAddTool { a: 5.0, b: 3.0 };
    let args = json!({"a": 10.0, "b": 5.0});
    let result = tool.call(args, None).await;
    
    assert!(result.is_ok(), "Basic tool execution failed");
    
    let response = result.unwrap();
    assert!(!response.content.is_empty(), "Tool response should have content");
    assert_eq!(response.is_error, Some(false), "Tool should not report error");
    
    println!("✅ Basic framework functionality working");
}

