//! Tests for code examples from turul-mcp-aws-lambda README.md
//!
//! These tests verify that the AWS Lambda integration examples from the
//! turul-mcp-aws-lambda README compile correctly.

use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};

/// Test basic Lambda MCP server example from turul-mcp-aws-lambda README
#[test]
fn test_basic_lambda_server_configuration() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "echo", description = "Echo back the provided message")]
    struct EchoTool {
        #[param(description = "Message to echo back")]
        message: String,
    }

    impl EchoTool {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
            Ok(format!("Echo: {}", self.message))
        }
    }

    // Test the LambdaMcpServerBuilder API compiles correctly
    async fn example_lambda_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _server = LambdaMcpServerBuilder::new()
            .name("echo-lambda-server")
            .version("1.0.0")
            .tool(EchoTool::default())
            .sse(true)
            .cors_allow_all_origins()
            .build()
            .await?;

        // We don't actually create the handler since that would require Lambda runtime
        Ok(())
    }

    // Just verify the async function compiles
    let _ = example_lambda_server;
}

/// Test DynamoDB session storage configuration from turul-mcp-aws-lambda README
#[test]
fn test_lambda_dynamodb_session_storage() {
    // Note: We can't actually test storage backends without setting up databases
    // This test verifies the configuration APIs compile correctly

    async fn example_dynamodb_storage() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Note: We can't actually create DynamoDB storage without AWS credentials
        // This just verifies the API compiles

        // let storage = Arc::new(DynamoDbSessionStorage::new().await?);

        let _server = LambdaMcpServerBuilder::new()
            .name("my-lambda-server")
            // .storage(storage)  // Would be uncommented in real usage
            .build()
            .await?;

        Ok(())
    }

    let _ = example_dynamodb_storage;
}

/// Test session persistence example from turul-mcp-aws-lambda README
#[test]
fn test_session_persistence() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "counter", description = "Session-persistent counter")]
    struct CounterTool {
        _marker: (), // Required for derive macro
    }

    impl CounterTool {
        async fn execute(&self, session: Option<SessionContext>) -> McpResult<i32> {
            if let Some(session) = session {
                let count: i32 = session.get_typed_state("count").await.unwrap_or(0);
                let new_count = count + 1;
                session.set_typed_state("count", new_count).await.unwrap();
                Ok(new_count)
            } else {
                Ok(0)
            }
        }
    }

    // Verify session-persistent tool compiles
    async fn example_with_session_tool() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _server = LambdaMcpServerBuilder::new()
            .name("counter-lambda-server")
            .version("1.0.0")
            .tool(CounterTool::default())
            .build()
            .await?;

        Ok(())
    }

    let _ = example_with_session_tool;
}

/// Test CORS configuration from turul-mcp-aws-lambda README
#[test]
fn test_lambda_cors_configuration() {
    use turul_mcp_aws_lambda::CorsConfig;

    async fn example_cors_config() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Test permissive CORS (development)
        let _server1 = LambdaMcpServerBuilder::new()
            .cors_allow_all_origins()
            .build()
            .await?;

        // Test custom CORS configuration (production)
        let mut cors = CorsConfig::for_origins(vec!["https://myapp.com".to_string()]);
        cors.allow_credentials = true;

        let _server2 = LambdaMcpServerBuilder::new().cors(cors).build().await?;

        Ok(())
    }

    let _ = example_cors_config;
}

/// Test SSE streaming in Lambda from turul-mcp-aws-lambda README
#[test]
fn test_lambda_sse_streaming() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "long_task", description = "Long-running task with progress")]
    struct LongTaskTool {
        _marker: (),
    }

    impl LongTaskTool {
        async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
            if let Some(session) = session {
                for i in 1..=3 {
                    // Limit for testing
                    // Send progress notification via SSE
                    session.notify_progress("long-task", i as u64).await;

                    // Don't actually sleep in tests
                    // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }

            Ok("Task completed".to_string())
        }
    }

    // Verify SSE streaming tool compiles
    async fn example_sse_tool() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _server = LambdaMcpServerBuilder::new()
            .name("sse-lambda-server")
            .version("1.0.0")
            .tool(LongTaskTool::default())
            .sse(true)
            .build()
            .await?;

        Ok(())
    }

    let _ = example_sse_tool;
}

/// Test Lambda handler creation from turul-mcp-aws-lambda README
#[test]
fn test_lambda_handler_creation() {
    async fn example_handler_creation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(McpTool, Clone, Default)]
        #[tool(name = "test", description = "Test tool")]
        struct TestTool {
            _marker: (),
        }

        impl TestTool {
            async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
                Ok("test".to_string())
            }
        }

        let server = LambdaMcpServerBuilder::new()
            .name("test-lambda-server")
            .version("1.0.0")
            .tool(TestTool::default())
            .build()
            .await?;

        // Create handler for Lambda runtime - just verify this compiles
        let _handler = server.handler().await?;

        // The actual service_fn usage would be:
        // run_with_streaming_response(service_fn(move |req| {
        //     let handler = handler.clone();
        //     async move {
        //         handler.handle(req).await
        //             .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        //     }
        // })).await

        Ok(())
    }

    let _ = example_handler_creation;
}

/// Test production Lambda server configuration from turul-mcp-aws-lambda README
#[test]
fn test_production_lambda_configuration() {
    use turul_mcp_aws_lambda::CorsConfig;
    // Production configuration test

    async fn example_production_config() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(McpTool, Clone, Default)]
        #[tool(name = "status", description = "Check server status")]
        struct StatusTool {
            _marker: (),
        }

        impl StatusTool {
            async fn execute(
                &self,
                _session: Option<SessionContext>,
            ) -> McpResult<serde_json::Value> {
                Ok(serde_json::json!({
                    "status": "healthy",
                    "version": "1.0.0"
                }))
            }
        }

        // Production CORS configuration
        let mut cors = CorsConfig::for_origins(vec!["https://myapp.com".to_string()]);
        cors.allow_credentials = true;

        let _server = LambdaMcpServerBuilder::new()
            .name("production-lambda-server")
            .version("1.0.0")
            .tool(StatusTool::default())
            .cors(cors)
            .sse(true)
            .build()
            .await?;

        Ok(())
    }

    let _ = example_production_config;
}

/// Test Lambda streaming feature with proper streaming setup
#[cfg(feature = "streaming")]
#[tokio::test]
async fn test_lambda_streaming_feature_e2e() {
    use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
    use turul_mcp_session_storage::InMemorySessionStorage;
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt::try_init();

    #[derive(McpTool, Clone, Default)]
    #[tool(name = "stream_test", description = "Test streaming functionality")]
    struct StreamTestTool {
        #[param(description = "Test message")]
        message: String,
    }

    impl StreamTestTool {
        async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
            if let Some(session) = session {
                // Send progress notifications that would be streamed
                session.notify_progress("stream-test", 1).await;
                session.notify_progress("stream-test", 2).await;
                session.notify_progress("stream-test", 3).await;
            }
            Ok(format!("Streamed: {}", self.message))
        }
    }

    // Test with streaming feature enabled
    let storage = Arc::new(InMemorySessionStorage::new());
    let server = LambdaMcpServerBuilder::new()
        .name("stream-test-server")
        .version("1.0.0")
        .tool(StreamTestTool::default())
        .storage(storage)
        .sse(true)
        .build()
        .await
        .expect("Failed to build Lambda MCP server");

    // Create handler - this should work with streaming enabled
    let handler = server.handler().await.expect("Failed to create handler");

    // Test basic handler properties - verify it was created successfully
    // (We can't test Debug since LambdaMcpHandler doesn't implement it)

    // For a real streaming test, we'd need to:
    // 1. Create a mock Lambda request with SSE headers
    // 2. Call handler.handle_streaming() instead of handle()
    // 3. Verify streaming response format

    // This test verifies the streaming setup compiles and creates handlers correctly
    println!("✅ Lambda streaming E2E test completed successfully");
}

/// Test Lambda SSE validation without streaming feature
#[tokio::test]
async fn test_lambda_sse_validation_without_streaming() {
    use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
    use turul_mcp_session_storage::InMemorySessionStorage;
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt::try_init();

    // Simple test tool for validation test
    use turul_mcp_derive::McpTool;

    #[derive(Debug, Default, Clone, McpTool)]
    struct ValidationTestTool {
        message: String,
    }

    impl ValidationTestTool {
        async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> Result<String, turul_mcp_protocol::McpError> {
            Ok("validation".to_string())
        }
    }

    // Test that SSE validation works correctly when streaming feature is not enabled
    let storage = Arc::new(InMemorySessionStorage::new());
    let result = LambdaMcpServerBuilder::new()
        .name("sse-validation-test")
        .version("1.0.0")
        .tool(ValidationTestTool::default())
        .storage(storage)
        .sse(true) // This should fail
        .build()
        .await;

    // This should fail with a configuration error about streaming feature not being active
    assert!(result.is_err());
    if let Err(error) = result {
        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("streaming"));
        assert!(error_msg.contains("feature"));
    }

    println!("✅ SSE validation test completed successfully");
}

/// Integration test that actually executes lambda example configurations
#[tokio::test]
async fn test_lambda_examples_execution() {
    use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
    use turul_mcp_session_storage::InMemorySessionStorage;
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt::try_init();

    // Test 1: Basic lambda server builder actually executes
    {
        #[derive(Debug, Default, Clone, McpTool)]
        struct TestTool {
            message: String,
        }

        impl TestTool {
            async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> Result<String, turul_mcp_protocol::McpError> {
                Ok("test".to_string())
            }
        }

        let server = LambdaMcpServerBuilder::new()
            .name("test-execution-server")
            .version("1.0.0")
            .tool(TestTool::default())
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false) // Disable SSE for test
            .build()
            .await;

        assert!(server.is_ok(), "Basic lambda server should build successfully");
        let server = server.unwrap();

        // Actually create a handler to verify full pipeline
        let handler = server.handler().await;
        assert!(handler.is_ok(), "Handler creation should succeed");
    }

    // Test 2: CORS configuration execution
    {
        #[derive(Debug, Default, Clone, McpTool)]
        struct CorsTestTool {
            message: String,
        }

        impl CorsTestTool {
            async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> Result<String, turul_mcp_protocol::McpError> {
                Ok("cors-test".to_string())
            }
        }

        #[cfg(feature = "cors")]
        {
            let server = LambdaMcpServerBuilder::new()
                .name("cors-test-server")
                .version("1.0.0")
                .tool(CorsTestTool::default())
                .storage(Arc::new(InMemorySessionStorage::new()))
                .cors_allow_all_origins()
                .sse(false) // Disable SSE for test
                .build()
                .await;

            assert!(server.is_ok(), "CORS-enabled server should build successfully");
        }
    }

    // Test 3: Validation paths execution (SSE without streaming)
    {
        #[derive(Debug, Default, Clone, McpTool)]
        struct ValidationTool {
            message: String,
        }

        impl ValidationTool {
            async fn execute(&self, _session: Option<turul_mcp_server::SessionContext>) -> Result<String, turul_mcp_protocol::McpError> {
                Ok("validation".to_string())
            }
        }

        // This should fail due to SSE validation
        let result = LambdaMcpServerBuilder::new()
            .name("validation-test-server")
            .version("1.0.0")
            .tool(ValidationTool::default())
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(true) // This should trigger validation error
            .build()
            .await;

        assert!(result.is_err(), "SSE validation should prevent build without streaming feature");
    }

    println!("✅ Lambda examples execution test completed successfully");
}
