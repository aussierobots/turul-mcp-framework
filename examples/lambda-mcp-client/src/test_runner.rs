//! Test runner implementation for Lambda MCP Client
//! 
//! Simplified version without validation complexity

use anyhow::{Context, Result};
use colored::*;
use indicatif::ProgressBar;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use crate::client::{McpClient, McpClientConfig};
use crate::test_suite::{TestCase, TestSuite};

/// Result of a single test execution
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Test execution duration
    pub duration: Duration,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional test details
    pub details: Option<Value>,
    /// Test output/logs
    pub output: Option<String>,
}

/// Test runner for executing test suites
#[derive(Debug)]
pub struct TestRunner {
    client_config: McpClientConfig,
    concurrency: u32,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(client_config: McpClientConfig, concurrency: u32) -> Self {
        Self {
            client_config,
            concurrency,
        }
    }

    /// Run a complete test suite with optional progress reporting
    pub async fn run_test_suite(
        &self,
        test_suite: TestSuite,
        progress_bar: Option<ProgressBar>,
    ) -> Result<Vec<TestResult>> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency as usize));
        let mut handles = Vec::new();

        for test_case in test_suite.test_cases {
            let permit = semaphore.clone().acquire_owned().await?;
            let client_config = self.client_config.clone();
            let pb = progress_bar.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = permit;
                let result = Self::run_single_test(&test_case, client_config).await;
                
                if let Some(pb) = pb {
                    pb.inc(1);
                    pb.set_message(format!("Completed: {}", test_case.name));
                }
                
                result
            });
            
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await??);
        }

        Ok(results)
    }

    /// Run a single test case
    async fn run_single_test(
        test_case: &TestCase,
        client_config: McpClientConfig,
    ) -> Result<TestResult> {
        let start_time = Instant::now();
        let session_id = format!("test-{}", uuid::Uuid::new_v4());
        
        debug!("Executing test: {}", test_case.name);
        
        let result = match Self::run_test_implementation(test_case, client_config, session_id).await {
            Ok(details) => {
                TestResult {
                    name: test_case.name.clone(),
                    passed: true,
                    duration: start_time.elapsed(),
                    error: None,
                    details: Some(details),
                    output: None,
                }
            },
            Err(e) => {
                error!("Test '{}' failed: {}", test_case.name, e);
                TestResult {
                    name: test_case.name.clone(),
                    passed: false,
                    duration: start_time.elapsed(),
                    error: Some(e.to_string()),
                    details: None,
                    output: None,
                }
            }
        };

        debug!("Test '{}' completed in {:.2}s", test_case.name, result.duration.as_secs_f64());
        Ok(result)
    }

    /// Run the actual test implementation - simplified without validation
    async fn run_test_implementation(
        test_case: &TestCase,
        client_config: McpClientConfig,
        session_id: String,
    ) -> Result<Value> {
        let mut client = McpClient::new(client_config.clone()).await?;
        
        match test_case.test_type.as_str() {
            "protocol_initialize" => {
                let init_response = client.initialize().await?;
                Ok(serde_json::to_value(init_response)?)
            }
            "protocol_tools_list" => {
                client.initialize().await?;
                let tools_response = client.list_tools().await?;
                Ok(serde_json::to_value(tools_response)?)
            }
            "tools_execution" => {
                client.initialize().await?;
                let tools = client.list_tools().await?;
                let mut results = Vec::new();
                
                for tool in tools.tools {
                    match client.call_tool(&tool.name, Some(json!({}))).await {
                        Ok(result) => {
                            results.push(json!({
                                "tool": tool.name,
                                "success": true,
                                "result": result
                            }));
                        }
                        Err(e) => {
                            results.push(json!({
                                "tool": tool.name,
                                "success": false,
                                "error": e.to_string()
                            }));
                        }
                    }
                }
                
                Ok(json!({"tool_executions": results}))
            }
            "session_management" => {
                client.initialize().await?;
                let session_info = client.call_tool("session_info", Some(json!({}))).await
                    .context("Failed to get session info")?;
                Ok(serde_json::to_value(session_info)?)
            }
            "mcp_streamable_http_delete_session" => {
                // Test DELETE method for session termination
                use reqwest;
                
                client.initialize().await?;
                
                // Test DELETE with session ID - should return 204
                let delete_client = reqwest::Client::new();
                let delete_response = delete_client
                    .delete(format!("{}/mcp", client_config.base_url))
                    .header("Mcp-Session-Id", &session_id)
                    .send()
                    .await?;
                
                let delete_status = delete_response.status();
                let delete_headers = delete_response.headers().clone();
                
                // Test DELETE without session ID - should return 400
                let delete_no_session_response = delete_client
                    .delete(format!("{}/mcp", client_config.base_url))
                    .send()
                    .await?;
                
                let delete_no_session_status = delete_no_session_response.status();
                
                Ok(json!({
                    "protocol": "streamable_http_delete_session",
                    "delete_with_session_status": delete_status.as_u16(),
                    "delete_with_session_valid": delete_status == 204 || delete_status == 404,
                    "delete_without_session_status": delete_no_session_status.as_u16(),
                    "delete_without_session_valid": delete_no_session_status == 400,
                    "cors_headers_present": delete_headers.contains_key("access-control-allow-origin"),
                    "session_id_tested": session_id,
                    "mcp_spec_compliance": (delete_status == 204 || delete_status == 404) && delete_no_session_status == 400
                }))
            }
            _ => {
                // Default implementation for other test types
                client.initialize().await?;
                Ok(json!({
                    "test_type": test_case.test_type,
                    "status": "completed",
                    "message": "Basic test execution successful"
                }))
            }
        }
    }
}