//! MCP 2025-06-18 Specification Compliance Test Suite
//!
//! This module defines comprehensive test suites that validate lambda-mcp-server
//! compliance with the official MCP 2025-06-18 specification. Tests focus on
//! specification conformance rather than implementation-specific behavior.

use serde_json::{json, Value};
use std::collections::HashMap;

/// A single test case
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Unique test name
    pub name: String,
    /// Test description
    pub description: String,
    /// Test type/category
    pub test_type: String,
    /// Expected test duration (for timeouts)
    pub expected_duration_secs: u64,
    /// Test parameters
    pub parameters: Option<Value>,
    /// Test priority (0 = highest, 100 = lowest)
    pub priority: u8,
}

/// Collection of test cases
#[derive(Debug, Clone)]
pub struct TestSuite {
    /// Suite name
    pub name: String,
    /// Suite description  
    pub description: String,
    /// Test cases in this suite
    pub test_cases: Vec<TestCase>,
}

impl TestSuite {
    /// Create a comprehensive MCP 2025-06-18 specification compliance test suite
    pub fn comprehensive() -> Self {
        let mut test_cases = Vec::new();
        
        // Core MCP protocol compliance tests (highest priority)
        test_cases.extend(Self::core_mcp_compliance_tests());
        
        // JSON-RPC 2.0 specification tests
        test_cases.extend(Self::jsonrpc_compliance_tests());
        
        // MCP Streamable HTTP specification tests
        test_cases.extend(Self::streamable_http_tests());
        
        // MCP tool protocol compliance tests
        test_cases.extend(Self::tool_protocol_tests());
        
        // MCP resource protocol compliance tests  
        test_cases.extend(Self::resource_protocol_tests());
        
        // MCP notification protocol compliance tests
        test_cases.extend(Self::notification_protocol_tests());
        
        // Error handling per MCP specification
        test_cases.extend(Self::mcp_error_handling_tests());

        Self {
            name: "MCP 2025-06-18 Specification Compliance Test Suite".to_string(),
            description: "Comprehensive validation of server compliance with official MCP 2025-06-18 specification requirements. Tests validate specification conformance as the source of truth.".to_string(),
            test_cases,
        }
    }

    /// Protocol-only test suite
    pub fn protocol_only() -> Self {
        Self {
            name: "MCP Protocol Compliance Test Suite".to_string(),
            description: "Validates MCP 2025-06-18 Streamable HTTP protocol implementation".to_string(),
            test_cases: Self::core_mcp_compliance_tests(),
        }
    }

    /// Tools-only test suite
    pub fn tools_only() -> Self {
        Self {
            name: "MCP Tools Test Suite".to_string(),
            description: "Validates all available MCP tools and their functionality".to_string(),
            test_cases: Self::tool_protocol_tests(),
        }
    }

    /// Session management test suite
    pub fn session_only() -> Self {
        Self {
            name: "MCP Session Management Test Suite".to_string(),
            description: "Validates session lifecycle, state management, and concurrency".to_string(),
            test_cases: Self::session_tests(),
        }
    }

    /// Infrastructure integration test suite
    pub fn infrastructure_only() -> Self {
        Self {
            name: "Infrastructure Integration Test Suite".to_string(),
            description: "Validates AWS Lambda, DynamoDB, SNS, and other infrastructure components".to_string(),
            test_cases: Self::infrastructure_tests(),
        }
    }

    /// SSE streaming and new architecture test suite
    pub fn streaming_only() -> Self {
        Self {
            name: "SSE Streaming & New Architecture Test Suite".to_string(),
            description: "Validates SSE streaming, tokio broadcast, multiple connections, and clean SNS architecture".to_string(),
            test_cases: Self::streaming_tests(),
        }
    }

    /// Get the total number of test cases
    pub fn test_count(&self) -> usize {
        self.test_cases.len()
    }

    /// Convert into a vector of test cases
    pub fn into_test_cases(self) -> Vec<TestCase> {
        self.test_cases
    }

    /// Core MCP protocol compliance tests (MCP specification requirements)
    fn core_mcp_compliance_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "MCP Initialization Protocol".to_string(),
                description: "Validate MCP 2025-06-18 initialization handshake per specification section 4.1".to_string(),
                test_type: "mcp_spec_initialization".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "validate_protocol_version": "2025-06-18",
                    "validate_capabilities_structure": true,
                    "validate_client_info_required": true,
                    "validate_initialized_notification": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP Session Management Protocol".to_string(),
                description: "Validate session lifecycle per MCP specification section 5.2".to_string(),
                test_type: "mcp_spec_session_lifecycle".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "test_session_isolation": true,
                    "validate_session_headers": true,
                    "test_session_cleanup": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP Protocol Version Negotiation".to_string(),
                description: "Validate protocol version negotiation per MCP specification section 3.1".to_string(),
                test_type: "mcp_spec_version_negotiation".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "test_supported_versions": ["2025-06-18"],
                    "test_unsupported_version_handling": true,
                    "validate_version_in_responses": true
                })),
                priority: 1,
            },
        ]
    }

    /// JSON-RPC 2.0 specification compliance tests  
    fn jsonrpc_compliance_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "JSON-RPC 2.0 Request Format".to_string(),
                description: "Validate JSON-RPC 2.0 request format per RFC specification".to_string(),
                test_type: "jsonrpc_spec_request_format".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "validate_required_fields": ["jsonrpc", "method", "id"],
                    "validate_jsonrpc_version": "2.0",
                    "test_notification_requests": true,
                    "test_batch_requests": false
                })),
                priority: 0,
            },
            TestCase {
                name: "JSON-RPC 2.0 Response Format".to_string(),
                description: "Validate JSON-RPC 2.0 response format per RFC specification".to_string(),
                test_type: "jsonrpc_spec_response_format".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "validate_required_fields": ["jsonrpc", "id"],
                    "validate_result_or_error_exclusive": true,
                    "validate_error_object_format": true,
                    "test_id_matching": true
                })),
                priority: 0,
            },
            TestCase {
                name: "JSON-RPC 2.0 Error Handling".to_string(),
                description: "Validate JSON-RPC 2.0 error response format per RFC specification".to_string(),
                test_type: "jsonrpc_spec_error_handling".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "test_parse_error": true,
                    "test_invalid_request": true,
                    "test_method_not_found": true,
                    "test_invalid_params": true,
                    "validate_error_codes": [-32700, -32600, -32601, -32602]
                })),
                priority: 1,
            },
        ]
    }

    /// MCP Streamable HTTP specification tests
    fn streamable_http_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "MCP Streamable HTTP POST Requests".to_string(),
                description: "Validate POST requests for JSON-RPC tools per MCP Streamable HTTP spec".to_string(),
                test_type: "mcp_streamable_http_post".to_string(),
                expected_duration_secs: 10,
                parameters: Some(json!({
                    "validate_content_type": "application/json",
                    "validate_mcp_session_header": true,
                    "test_cors_headers": true,
                    "test_tool_execution": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP Streamable HTTP GET SSE Streaming".to_string(),
                description: "Validate GET requests for SSE streaming per MCP Streamable HTTP spec".to_string(),
                test_type: "mcp_streamable_http_get_sse".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "validate_content_type": "text/event-stream",
                    "validate_cache_control": "no-cache",
                    "validate_sse_format": true,
                    "stream_duration_seconds": 10,
                    "expect_mcp_notifications": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP Session Header Compliance".to_string(),
                description: "Validate Mcp-Session-Id header handling per specification".to_string(),
                test_type: "mcp_session_header_compliance".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "test_session_isolation": true,
                    "test_multiple_sessions": 3,
                    "validate_session_scoping": true
                })),
                priority: 1,
            },
        ]
    }

    /// MCP tool protocol compliance tests
    fn tool_protocol_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "MCP tools/list Protocol".to_string(),
                description: "Validate tools/list method response per MCP specification section 6.1".to_string(),
                test_type: "mcp_spec_tools_list".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "validate_response_structure": true,
                    "validate_tool_fields": ["name", "description"],
                    "validate_optional_fields": ["inputSchema"],
                    "test_empty_tools_list": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP tools/call Protocol".to_string(),
                description: "Validate tools/call method request/response per MCP specification section 6.2".to_string(),
                test_type: "mcp_spec_tools_call".to_string(),
                expected_duration_secs: 10,
                parameters: Some(json!({
                    "validate_call_format": true,
                    "test_required_params": ["name"],
                    "test_optional_params": ["arguments"],
                    "validate_response_content": true,
                    "test_tool_error_handling": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP Tool Input Schema Validation".to_string(),
                description: "Validate tool input schemas conform to JSON Schema specification".to_string(),
                test_type: "mcp_spec_tool_schemas".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "validate_json_schema_format": true,
                    "test_schema_validation": true,
                    "test_invalid_arguments": true,
                    "validate_error_responses": true
                })),
                priority: 1,
            },
            TestCase {
                name: "MCP Tool Response Content Types".to_string(),
                description: "Validate tool response content types per MCP specification section 6.3".to_string(),
                test_type: "mcp_spec_tool_content_types".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "test_text_content": true,
                    "test_json_content": true,
                    "validate_content_structure": true,
                    "test_error_content": true
                })),
                priority: 2,
            },
        ]
    }

    /// MCP resource protocol compliance tests  
    fn resource_protocol_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "MCP resources/list Protocol".to_string(),
                description: "Validate resources/list method response per MCP specification section 7.1".to_string(),
                test_type: "mcp_spec_resources_list".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "validate_response_structure": true,
                    "validate_resource_fields": ["uri", "name"],
                    "validate_optional_fields": ["description", "mimeType"],
                    "test_empty_resources_list": true
                })),
                priority: 1,
            },
            TestCase {
                name: "MCP resources/read Protocol".to_string(),
                description: "Validate resources/read method request/response per MCP specification section 7.2".to_string(),
                test_type: "mcp_spec_resources_read".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "validate_read_format": true,
                    "test_required_params": ["uri"],
                    "validate_response_content": true,
                    "test_resource_not_found": true
                })),
                priority: 2,
            },
        ]
    }

    /// MCP notification protocol compliance tests
    fn notification_protocol_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "MCP notifications/initialized Protocol".to_string(),
                description: "Validate notifications/initialized method per MCP specification section 8.1".to_string(),
                test_type: "mcp_spec_notifications_initialized".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "validate_notification_format": true,
                    "test_method_name": "notifications/initialized",
                    "validate_no_response_required": true
                })),
                priority: 0,
            },
            TestCase {
                name: "MCP Progress Notifications".to_string(),
                description: "Validate progress notification protocol per MCP specification section 8.2".to_string(),
                test_type: "mcp_spec_progress_notifications".to_string(),
                expected_duration_secs: 10,
                parameters: Some(json!({
                    "test_progress_structure": true,
                    "validate_progress_fields": ["progress", "total"],
                    "test_cancellation_support": true
                })),
                priority: 2,
            },
            TestCase {
                name: "MCP Resource Update Notifications".to_string(),
                description: "Validate resource update notifications per MCP specification section 8.3".to_string(),
                test_type: "mcp_spec_resource_notifications".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "test_resource_updated_notifications": true,
                    "validate_resource_uri_field": true,
                    "test_notification_delivery": true
                })),
                priority: 3,
            },
        ]
    }

    /// MCP error handling specification compliance tests
    fn mcp_error_handling_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "MCP Standard Error Codes".to_string(),
                description: "Validate MCP-specific error codes per specification section 9.1".to_string(),
                test_type: "mcp_spec_error_codes".to_string(),
                expected_duration_secs: 10,
                parameters: Some(json!({
                    "test_tool_not_found": true,
                    "test_resource_not_found": true,
                    "test_invalid_tool_params": true,
                    "validate_error_structure": true,
                    "test_internal_server_errors": true
                })),
                priority: 1,
            },
            TestCase {
                name: "MCP Error Response Format".to_string(),
                description: "Validate error response format per MCP and JSON-RPC specifications".to_string(),
                test_type: "mcp_spec_error_format".to_string(),
                expected_duration_secs: 8,
                parameters: Some(json!({
                    "validate_error_object_required_fields": ["code", "message"],
                    "validate_optional_data_field": true,
                    "test_error_message_clarity": true,
                    "validate_no_result_with_error": true
                })),
                priority: 1,
            },
        ]
    }

    /// Session management tests
    fn session_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "Session Lifecycle".to_string(),
                description: "Test complete session lifecycle from initialization to cleanup".to_string(),
                test_type: "session_lifecycle".to_string(),
                expected_duration_secs: 10,
                parameters: None,
                priority: 0,
            },
            TestCase {
                name: "Concurrent Sessions".to_string(),
                description: "Test multiple concurrent sessions and isolation".to_string(),
                test_type: "concurrent_sessions".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "session_count": 3,
                    "operations_per_session": 5
                })),
                priority: 1,
            },
            TestCase {
                name: "Session State Persistence".to_string(),
                description: "Test session state persistence across requests".to_string(),
                test_type: "session_persistence".to_string(),
                expected_duration_secs: 10,
                parameters: None,
                priority: 2,
            },
            TestCase {
                name: "Session Timeout Handling".to_string(),
                description: "Test session timeout and cleanup behavior".to_string(),
                test_type: "session_timeout".to_string(),
                expected_duration_secs: 20,
                parameters: Some(json!({
                    "timeout_seconds": 5
                })),
                priority: 3,
            },
        ]
    }

    /// Infrastructure integration tests
    fn infrastructure_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "DynamoDB Session Storage".to_string(),
                description: "Test DynamoDB session storage and retrieval".to_string(),
                test_type: "dynamodb_integration".to_string(),
                expected_duration_secs: 15,
                parameters: None,
                priority: 1,
            },
            TestCase {
                name: "SNS Global Event Integration".to_string(),
                description: "Test SNS global event publishing and tokio broadcast distribution".to_string(),
                test_type: "sns_integration".to_string(),
                expected_duration_secs: 20,
                parameters: Some(json!({
                    "test_event_count": 3,
                    "verify_broadcast": true
                })),
                priority: 1,
            },
            TestCase {
                name: "Lambda Execution Context".to_string(),
                description: "Test Lambda function execution context and metadata".to_string(),
                test_type: "lambda_context".to_string(),
                expected_duration_secs: 5,
                parameters: None,
                priority: 2,
            },
            TestCase {
                name: "CloudWatch Integration".to_string(),
                description: "Test CloudWatch logging and metrics integration".to_string(),
                test_type: "cloudwatch_integration".to_string(),
                expected_duration_secs: 10,
                parameters: None,
                priority: 3,
            },
        ]
    }

    /// Performance and reliability tests
    fn performance_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "Basic Performance".to_string(),
                description: "Test basic response times and throughput".to_string(),
                test_type: "performance_basic".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "operation_count": 10,
                    "max_duration_ms": 5000
                })),
                priority: 2,
            },
            TestCase {
                name: "Load Testing".to_string(),
                description: "Test server behavior under sustained load".to_string(),
                test_type: "performance_load".to_string(),
                expected_duration_secs: 60,
                parameters: Some(json!({
                    "concurrent_requests": 10,
                    "duration_seconds": 30
                })),
                priority: 4,
            },
            TestCase {
                name: "Memory Usage".to_string(),
                description: "Test memory usage patterns and potential leaks".to_string(),
                test_type: "performance_memory".to_string(),
                expected_duration_secs: 30,
                parameters: None,
                priority: 3,
            },
        ]
    }

    /// Error handling tests
    fn error_handling_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "Invalid Method Handling".to_string(),
                description: "Test server response to invalid MCP methods".to_string(),
                test_type: "error_handling".to_string(),
                expected_duration_secs: 5,
                parameters: Some(json!({
                    "test_type": "invalid_method"
                })),
                priority: 1,
            },
            TestCase {
                name: "Malformed Request Handling".to_string(),
                description: "Test server response to malformed JSON-RPC requests".to_string(),
                test_type: "error_malformed".to_string(),
                expected_duration_secs: 5,
                parameters: None,
                priority: 1,
            },
            TestCase {
                name: "Tool Error Handling".to_string(),
                description: "Test tool error conditions and error response format".to_string(),
                test_type: "error_tool_failures".to_string(),
                expected_duration_secs: 10,
                parameters: None,
                priority: 2,
            },
            TestCase {
                name: "Network Error Recovery".to_string(),
                description: "Test recovery from network errors and timeouts".to_string(),
                test_type: "error_network".to_string(),
                expected_duration_secs: 20,
                parameters: None,
                priority: 3,
            },
        ]
    }

    /// New architecture streaming and multiple connection tests
    fn streaming_tests() -> Vec<TestCase> {
        vec![
            TestCase {
                name: "SSE Streaming Basic".to_string(),
                description: "Test basic Server-Sent Events streaming according to MCP 2025-06-18".to_string(),
                test_type: "sse_streaming_basic".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "stream_duration_seconds": 10,
                    "expect_heartbeat": true,
                    "expect_connection_event": true
                })),
                priority: 0,
            },
            TestCase {
                name: "Multiple SSE Connections".to_string(),
                description: "Test multiple concurrent SSE connections receiving tokio broadcast events".to_string(),
                test_type: "sse_multiple_connections".to_string(),
                expected_duration_secs: 20,
                parameters: Some(json!({
                    "connection_count": 3,
                    "stream_duration_seconds": 10,
                    "verify_all_receive_events": true
                })),
                priority: 1,
            },
            TestCase {
                name: "Global Event Broadcasting".to_string(),
                description: "Test internal global event broadcasting via tokio channels".to_string(),
                test_type: "global_event_broadcast".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "test_events": ["server_startup", "tool_execution", "system_health"],
                    "verify_mcp_format": true
                })),
                priority: 1,
            },
            TestCase {
                name: "MCP Streamable HTTP Compliance".to_string(),
                description: "Test MCP 2025-06-18 Streamable HTTP specification compliance".to_string(),
                test_type: "mcp_streamable_http".to_string(),
                expected_duration_secs: 20,
                parameters: Some(json!({
                    "test_post_jsonrpc": true,
                    "test_get_sse": true,
                    "test_session_headers": true,
                    "test_cors": true
                })),
                priority: 0,
            },
            TestCase {
                name: "PUT vs GET Tool Invocation".to_string(),
                description: "Test both PUT and GET methods for tool invocation per MCP specification".to_string(),
                test_type: "http_methods_compliance".to_string(),
                expected_duration_secs: 15,
                parameters: Some(json!({
                    "test_put_tools": true,
                    "test_get_streaming": true,
                    "compare_responses": true
                })),
                priority: 2,
            },
            TestCase {
                name: "Session Isolation in Streaming".to_string(),
                description: "Test that session-specific events only go to correct SSE connections".to_string(),
                test_type: "session_isolation_streaming".to_string(),
                expected_duration_secs: 25,
                parameters: Some(json!({
                    "session_count": 3,
                    "targeted_events_per_session": 2,
                    "verify_isolation": true
                })),
                priority: 1,
            },
            TestCase {
                name: "Legacy vs Clean Architecture".to_string(),
                description: "Compare legacy SQS polling vs new tokio broadcast performance".to_string(),
                test_type: "architecture_comparison".to_string(),
                expected_duration_secs: 30,
                parameters: Some(json!({
                    "test_legacy_sqs": false,
                    "test_tokio_broadcast": true,
                    "measure_latency": true,
                    "verify_no_message_loss": true
                })),
                priority: 3,
            },
        ]
    }
}

/// Test case builder for custom test creation
pub struct TestCaseBuilder {
    name: String,
    description: String,
    test_type: String,
    expected_duration_secs: u64,
    parameters: Option<Value>,
    priority: u8,
}

impl TestCaseBuilder {
    /// Create a new test case builder
    pub fn new(name: impl Into<String>, test_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            test_type: test_type.into(),
            expected_duration_secs: 10,
            parameters: None,
            priority: 5,
        }
    }

    /// Set test description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set expected duration
    pub fn duration_secs(mut self, duration: u64) -> Self {
        self.expected_duration_secs = duration;
        self
    }

    /// Set test parameters
    pub fn parameters(mut self, parameters: Value) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// Set test priority
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Build the test case
    pub fn build(self) -> TestCase {
        TestCase {
            name: self.name,
            description: self.description,
            test_type: self.test_type,
            expected_duration_secs: self.expected_duration_secs,
            parameters: self.parameters,
            priority: self.priority,
        }
    }
}