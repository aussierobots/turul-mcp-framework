//! MCP 2025-06-18 Specification Compliance Validator
//!
//! This module provides comprehensive validation of MCP server responses against 
//! the official MCP 2025-06-18 specification requirements.

#![allow(unused_imports)]
#![allow(dead_code)]

use anyhow::{Result, Context};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{debug, warn};

/// MCP Specification compliance validator
#[derive(Debug)]
pub struct McpSpecValidator {
    /// Expected protocol version
    protocol_version: String,
    /// Validation rules for different message types
    rules: ValidationRules,
}

/// Validation rules for different MCP message types
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationRules {
    /// JSON-RPC 2.0 base requirements
    pub jsonrpc_base: JsonRpcRules,
    /// Initialization protocol rules
    pub initialization: InitializationRules,
    /// Tool calling protocol rules
    pub tools: ToolRules,
    /// Resource access protocol rules
    pub resources: ResourceRules,
    /// Notification protocol rules
    pub notifications: NotificationRules,
    /// SSE streaming rules
    pub streaming: StreamingRules,
}

/// JSON-RPC 2.0 base requirements
#[derive(Debug)]
#[allow(dead_code)]
pub struct JsonRpcRules {
    /// Required fields for all requests
    pub required_request_fields: Vec<&'static str>,
    /// Required fields for all responses
    pub required_response_fields: Vec<&'static str>,
    /// Valid JSON-RPC version
    pub jsonrpc_version: String,
}

/// Initialization protocol validation rules
#[derive(Debug)]
#[allow(dead_code)]
pub struct InitializationRules {
    /// Required initialize request fields
    pub required_init_fields: Vec<&'static str>,
    /// Required capabilities structure
    pub required_capabilities: Vec<&'static str>,
    /// Required client info fields
    pub required_client_info: Vec<&'static str>,
}

/// Tool calling protocol validation rules
#[derive(Debug)]
#[allow(dead_code)]
pub struct ToolRules {
    /// Required tools/list response fields
    pub required_list_fields: Vec<&'static str>,
    /// Required tool definition fields
    pub required_tool_fields: Vec<&'static str>,
    /// Required tools/call request fields
    pub required_call_fields: Vec<&'static str>,
}

/// Resource access protocol validation rules
#[derive(Debug)]
#[allow(dead_code)]
pub struct ResourceRules {
    /// Required resources/list response fields
    pub required_list_fields: Vec<&'static str>,
    /// Required resource definition fields
    pub required_resource_fields: Vec<&'static str>,
    /// Required resources/read request fields
    pub required_read_fields: Vec<&'static str>,
}

/// Notification protocol validation rules
#[derive(Debug)]
#[allow(dead_code)]
pub struct NotificationRules {
    /// Valid notification method names
    pub valid_methods: Vec<&'static str>,
    /// Required notification fields
    pub required_fields: Vec<&'static str>,
}

/// SSE streaming validation rules
#[derive(Debug)]
#[allow(dead_code)]
pub struct StreamingRules {
    /// Required SSE headers
    pub required_headers: Vec<&'static str>,
    /// Valid SSE event types
    pub valid_event_types: Vec<&'static str>,
}

/// Validation result for a specific check
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the validation passed
    pub passed: bool,
    /// Rule that was validated
    pub rule_name: String,
    /// Detailed error message if failed
    pub error_message: Option<String>,
    /// Additional context or metadata
    pub context: Option<Value>,
}

/// Comprehensive validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Overall compliance status
    pub overall_passed: bool,
    /// Individual validation results
    pub results: Vec<ValidationResult>,
    /// Summary statistics
    pub summary: ValidationSummary,
}

/// Validation summary statistics
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    /// Total number of validations performed
    pub total_checks: usize,
    /// Number of passed validations
    pub passed_checks: usize,
    /// Number of failed validations
    pub failed_checks: usize,
    /// Compliance percentage
    pub compliance_percentage: f64,
}

impl McpSpecValidator {
    /// Create a new MCP specification validator
    pub fn new() -> Self {
        Self {
            protocol_version: "2025-06-18".to_string(),
            rules: ValidationRules::default(),
        }
    }

    /// Create validator for specific protocol version
    pub fn for_version(version: &str) -> Self {
        Self {
            protocol_version: version.to_string(),
            rules: ValidationRules::default(),
        }
    }

    /// Validate JSON-RPC 2.0 request format
    pub fn validate_jsonrpc_request(&self, request: &Value) -> ValidationResult {
        let rule_name = "jsonrpc_2.0_request_format".to_string();
        
        // Check required fields
        for field in &self.rules.jsonrpc_base.required_request_fields {
            if request.get(field).is_none() {
                return ValidationResult {
                    passed: false,
                    rule_name,
                    error_message: Some(format!("Missing required field: {}", field)),
                    context: Some(json!({"missing_field": field})),
                };
            }
        }
        
        // Check JSON-RPC version
        if let Some(version) = request.get("jsonrpc") {
            if version.as_str() != Some(&self.rules.jsonrpc_base.jsonrpc_version) {
                return ValidationResult {
                    passed: false,
                    rule_name,
                    error_message: Some(format!("Invalid jsonrpc version: expected '{}', got '{}'", 
                        self.rules.jsonrpc_base.jsonrpc_version, version)),
                    context: Some(json!({"expected": self.rules.jsonrpc_base.jsonrpc_version, "actual": version})),
                };
            }
        }

        ValidationResult {
            passed: true,
            rule_name,
            error_message: None,
            context: None,
        }
    }

    /// Validate JSON-RPC 2.0 response format
    pub fn validate_jsonrpc_response(&self, response: &Value) -> ValidationResult {
        let rule_name = "jsonrpc_2.0_response_format".to_string();
        
        // Check required fields
        for field in &self.rules.jsonrpc_base.required_response_fields {
            if response.get(field).is_none() {
                return ValidationResult {
                    passed: false,
                    rule_name,
                    error_message: Some(format!("Missing required field: {}", field)),
                    context: Some(json!({"missing_field": field})),
                };
            }
        }

        // Check for result or error (mutually exclusive)
        let has_result = response.get("result").is_some();
        let has_error = response.get("error").is_some();
        
        if !has_result && !has_error {
            return ValidationResult {
                passed: false,
                rule_name,
                error_message: Some("Response must have either 'result' or 'error' field".to_string()),
                context: Some(json!({"has_result": has_result, "has_error": has_error})),
            };
        }

        if has_result && has_error {
            return ValidationResult {
                passed: false,
                rule_name,
                error_message: Some("Response cannot have both 'result' and 'error' fields".to_string()),
                context: Some(json!({"has_result": has_result, "has_error": has_error})),
            };
        }

        ValidationResult {
            passed: true,
            rule_name,
            error_message: None,
            context: None,
        }
    }

    /// Validate initialization handshake protocol
    pub fn validate_initialization(&self, init_request: &Value, init_response: &Value) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        
        // Validate initialization request
        let mut init_result = ValidationResult {
            passed: true,
            rule_name: "initialization_request".to_string(),
            error_message: None,
            context: None,
        };

        // Check required initialization fields
        for field in &self.rules.initialization.required_init_fields {
            if init_request.get("params").and_then(|p| p.get(field)).is_none() {
                init_result.passed = false;
                init_result.error_message = Some(format!("Missing required initialization field: {}", field));
                init_result.context = Some(json!({"missing_field": field}));
                break;
            }
        }

        // Check protocol version
        if let Some(params) = init_request.get("params") {
            if let Some(protocol_version) = params.get("protocolVersion") {
                if protocol_version.as_str() != Some(&self.protocol_version) {
                    init_result.passed = false;
                    init_result.error_message = Some(format!("Protocol version mismatch: expected '{}', got '{}'", 
                        self.protocol_version, protocol_version));
                    init_result.context = Some(json!({"expected": self.protocol_version, "actual": protocol_version}));
                }
            }
        }

        results.push(init_result);

        // Validate initialization response
        let response_result = ValidationResult {
            passed: init_response.get("result").is_some(),
            rule_name: "initialization_response".to_string(),
            error_message: if init_response.get("result").is_none() {
                Some("Initialization response must contain 'result' field".to_string())
            } else {
                None
            },
            context: None,
        };

        results.push(response_result);
        results
    }

    /// Validate tools/list response
    pub fn validate_tools_list(&self, response: &Value) -> ValidationResult {
        let rule_name = "tools_list_response".to_string();
        
        if let Some(result) = response.get("result") {
            if let Some(tools) = result.get("tools") {
                if let Some(tools_array) = tools.as_array() {
                    // Validate each tool definition
                    for (index, tool) in tools_array.iter().enumerate() {
                        for field in &self.rules.tools.required_tool_fields {
                            if tool.get(field).is_none() {
                                return ValidationResult {
                                    passed: false,
                                    rule_name,
                                    error_message: Some(format!("Tool at index {} missing required field: {}", index, field)),
                                    context: Some(json!({"tool_index": index, "missing_field": field})),
                                };
                            }
                        }
                    }
                    
                    return ValidationResult {
                        passed: true,
                        rule_name,
                        error_message: None,
                        context: Some(json!({"tool_count": tools_array.len()})),
                    };
                } else {
                    return ValidationResult {
                        passed: false,
                        rule_name,
                        error_message: Some("'tools' field must be an array".to_string()),
                        context: Some(json!({"tools_type": tools.get_type()})),
                    };
                }
            } else {
                return ValidationResult {
                    passed: false,
                    rule_name,
                    error_message: Some("Missing 'tools' field in result".to_string()),
                    context: None,
                };
            }
        }

        ValidationResult {
            passed: false,
            rule_name,
            error_message: Some("Missing 'result' field in response".to_string()),
            context: None,
        }
    }

    /// Validate tool execution call
    pub fn validate_tool_call(&self, call_request: &Value, call_response: &Value) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        
        // Validate call request
        let mut call_result = ValidationResult {
            passed: true,
            rule_name: "tool_call_request".to_string(),
            error_message: None,
            context: None,
        };

        if let Some(params) = call_request.get("params") {
            for field in &self.rules.tools.required_call_fields {
                if params.get(field).is_none() {
                    call_result.passed = false;
                    call_result.error_message = Some(format!("Missing required tool call field: {}", field));
                    call_result.context = Some(json!({"missing_field": field}));
                    break;
                }
            }
        } else {
            call_result.passed = false;
            call_result.error_message = Some("Tool call request missing 'params' field".to_string());
        }

        results.push(call_result);

        // Validate call response
        let response_result = ValidationResult {
            passed: call_response.get("result").is_some() || call_response.get("error").is_some(),
            rule_name: "tool_call_response".to_string(),
            error_message: if call_response.get("result").is_none() && call_response.get("error").is_none() {
                Some("Tool call response must contain either 'result' or 'error' field".to_string())
            } else {
                None
            },
            context: None,
        };

        results.push(response_result);
        results
    }

    /// Validate SSE streaming compliance
    pub fn validate_sse_stream(&self, headers: &HashMap<String, String>, event_data: &str) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        
        // Check required SSE headers
        let mut header_result = ValidationResult {
            passed: true,
            rule_name: "sse_headers".to_string(),
            error_message: None,
            context: None,
        };

        for required_header in &self.rules.streaming.required_headers {
            if !headers.contains_key(*required_header) {
                header_result.passed = false;
                header_result.error_message = Some(format!("Missing required SSE header: {}", required_header));
                header_result.context = Some(json!({"missing_header": required_header}));
                break;
            }
        }

        results.push(header_result);

        // Validate SSE event format
        let event_result = ValidationResult {
            passed: event_data.starts_with("data: ") || event_data.starts_with("event: ") || event_data.starts_with("id: "),
            rule_name: "sse_event_format".to_string(),
            error_message: if !(event_data.starts_with("data: ") || event_data.starts_with("event: ") || event_data.starts_with("id: ")) {
                Some("SSE events must start with 'data:', 'event:', or 'id:'".to_string())
            } else {
                None
            },
            context: Some(json!({"event_preview": event_data.chars().take(50).collect::<String>()})),
        };

        results.push(event_result);
        results
    }

    /// Generate comprehensive validation report
    pub fn generate_report(&self, results: Vec<ValidationResult>) -> ValidationReport {
        let total_checks = results.len();
        let passed_checks = results.iter().filter(|r| r.passed).count();
        let failed_checks = total_checks - passed_checks;
        let compliance_percentage = if total_checks > 0 {
            (passed_checks as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        };

        ValidationReport {
            overall_passed: failed_checks == 0,
            results,
            summary: ValidationSummary {
                total_checks,
                passed_checks,
                failed_checks,
                compliance_percentage,
            },
        }
    }
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            jsonrpc_base: JsonRpcRules {
                required_request_fields: vec!["jsonrpc", "method", "id"],
                required_response_fields: vec!["jsonrpc", "id"],
                jsonrpc_version: "2.0".to_string(),
            },
            initialization: InitializationRules {
                required_init_fields: vec!["protocolVersion", "capabilities", "clientInfo"],
                required_capabilities: vec!["tools", "resources"],
                required_client_info: vec!["name", "version"],
            },
            tools: ToolRules {
                required_list_fields: vec!["tools"],
                required_tool_fields: vec!["name", "description"],
                required_call_fields: vec!["name"],
            },
            resources: ResourceRules {
                required_list_fields: vec!["resources"],
                required_resource_fields: vec!["uri", "name"],
                required_read_fields: vec!["uri"],
            },
            notifications: NotificationRules {
                valid_methods: vec![
                    "notifications/initialized",
                    "notifications/progress",
                    "notifications/message",
                    "notifications/resources/updated",
                    "logging/message"
                ],
                required_fields: vec!["jsonrpc", "method"],
            },
            streaming: StreamingRules {
                required_headers: vec!["content-type", "cache-control"],
                valid_event_types: vec!["message", "progress", "error", "resource_updated"],
            },
        }
    }
}

/// Helper trait to get type name of JSON values for error reporting
trait JsonValueType {
    fn get_type(&self) -> &'static str;
}

impl JsonValueType for Value {
    fn get_type(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number", 
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = McpSpecValidator::new();
        assert_eq!(validator.protocol_version, "2025-06-18");
    }

    #[test]
    fn test_jsonrpc_request_validation_success() {
        let validator = McpSpecValidator::new();
        let request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1
        });

        let result = validator.validate_jsonrpc_request(&request);
        assert!(result.passed);
    }

    #[test]
    fn test_jsonrpc_request_validation_missing_field() {
        let validator = McpSpecValidator::new();
        let request = json!({
            "jsonrpc": "2.0",
            "method": "initialize"
            // Missing "id" field
        });

        let result = validator.validate_jsonrpc_request(&request);
        assert!(!result.passed);
        assert!(result.error_message.unwrap().contains("Missing required field: id"));
    }

    #[test]
    fn test_jsonrpc_response_validation_success() {
        let validator = McpSpecValidator::new();
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"status": "ok"}
        });

        let result = validator.validate_jsonrpc_response(&response);
        assert!(result.passed);
    }

    #[test]
    fn test_tools_list_validation() {
        let validator = McpSpecValidator::new();
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "test_tool",
                        "description": "A test tool"
                    }
                ]
            }
        });

        let result = validator.validate_tools_list(&response);
        assert!(result.passed);
    }
}