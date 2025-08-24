//! Schema Validation Module
//!
//! This module provides comprehensive schema validation for MCP protocol
//! messages and tool responses, ensuring compliance with declared schemas.

#![allow(unused_imports)]
#![allow(dead_code)]

use anyhow::{Context, Result};
use jsonschema::{Validator, ValidationError};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// Schema validator for MCP protocol and tool responses
#[derive(Debug)]
pub struct SchemaValidator {
    /// Compiled JSON schemas for tools
    tool_schemas: HashMap<String, Validator>,
    /// Compiled JSON schemas for protocol messages
    protocol_schemas: HashMap<String, Validator>,
    /// Enable strict validation mode
    strict_mode: bool,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new(strict_mode: bool) -> Self {
        let mut validator = Self {
            tool_schemas: HashMap::new(),
            protocol_schemas: HashMap::new(),
            strict_mode,
        };

        // Initialize protocol schemas
        validator.initialize_protocol_schemas();
        
        validator
    }

    /// Add a tool schema for validation
    pub fn add_tool_schema(&mut self, tool_name: String, schema: Value) -> Result<()> {
        let compiled_schema = Validator::new(&schema)
            .context("Failed to compile tool schema")?;
        
        debug!("Added tool schema for: {}", tool_name);
        self.tool_schemas.insert(tool_name, compiled_schema);
        Ok(())
    }

    /// Validate a tool response against its schema
    pub fn validate_tool_response(&self, tool_name: &str, response: &Value) -> Result<()> {
        if let Some(schema) = self.tool_schemas.get(tool_name) {
            debug!("Validating response for tool: {}", tool_name);
            
            match schema.validate(response) {
                Ok(_) => {
                    debug!("Tool response validation passed for: {}", tool_name);
                    Ok(())
                }
                Err(_) => {
                    error!("Tool response validation failed for {}", tool_name);
                    
                    if self.strict_mode {
                        Err(anyhow::anyhow!(
                            "Tool response validation failed for {}",
                            tool_name
                        ))
                    } else {
                        warn!("Tool response validation failed (non-strict mode): {}", tool_name);
                        Ok(())
                    }
                }
            }
        } else {
            let message = format!("No schema found for tool: {}", tool_name);
            if self.strict_mode {
                Err(anyhow::anyhow!(message))
            } else {
                warn!("{}", message);
                Ok(())
            }
        }
    }

    /// Validate a protocol message
    pub fn validate_protocol_message(&self, message_type: &str, message: &Value) -> Result<()> {
        if let Some(schema) = self.protocol_schemas.get(message_type) {
            debug!("Validating protocol message: {}", message_type);
            
            match schema.validate(message) {
                Ok(_) => {
                    debug!("Protocol message validation passed for: {}", message_type);
                    Ok(())
                }
                Err(_) => {
                    error!("Protocol message validation failed for {}", message_type);
                    
                    if self.strict_mode {
                        Err(anyhow::anyhow!(
                            "Protocol message validation failed for {}",
                            message_type
                        ))
                    } else {
                        warn!("Protocol message validation failed (non-strict mode): {}", message_type);
                        Ok(())
                    }
                }
            }
        } else {
            let message = format!("No schema found for protocol message: {}", message_type);
            if self.strict_mode {
                Err(anyhow::anyhow!(message))
            } else {
                warn!("{}", message);
                Ok(())
            }
        }
    }

    /// Extract and add tool schemas from tools list response
    pub fn extract_tool_schemas_from_list(&mut self, tools_response: &Value) -> Result<()> {
        if let Some(result) = tools_response.get("result") {
            if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                for tool in tools {
                    if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                        // Extract schema if present in tool definition
                        if let Some(output_schema) = tool.get("outputSchema") {
                            self.add_tool_schema(name.to_string(), output_schema.clone())?;
                        } else {
                            // Create a basic schema if none provided
                            let basic_schema = json!({
                                "type": "object",
                                "properties": {
                                    "content": {"type": "array"},
                                    "isError": {"type": "boolean"}
                                },
                                "required": ["content", "isError"],
                                "additionalProperties": true
                            });
                            self.add_tool_schema(name.to_string(), basic_schema)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Initialize protocol schemas for MCP messages
    fn initialize_protocol_schemas(&mut self) {
        // Initialize request schema
        let initialize_request_schema = json!({
            "type": "object",
            "properties": {
                "jsonrpc": {"type": "string", "const": "2.0"},
                "id": {},
                "method": {"type": "string", "const": "initialize"},
                "params": {
                    "type": "object",
                    "properties": {
                        "protocolVersion": {"type": "string"},
                        "capabilities": {"type": "object"},
                        "clientInfo": {"type": "object"}
                    },
                    "required": ["protocolVersion", "capabilities"]
                }
            },
            "required": ["jsonrpc", "id", "method", "params"],
            "additionalProperties": false
        });

        let initialize_response_schema = json!({
            "type": "object",
            "properties": {
                "jsonrpc": {"type": "string", "const": "2.0"},
                "id": {},
                "result": {
                    "type": "object",
                    "properties": {
                        "protocolVersion": {"type": "string"},
                        "capabilities": {"type": "object"},
                        "serverInfo": {"type": "object"}
                    },
                    "required": ["protocolVersion", "capabilities"]
                }
            },
            "required": ["jsonrpc", "id", "result"],
            "additionalProperties": false
        });

        let tools_list_response_schema = json!({
            "type": "object",
            "properties": {
                "jsonrpc": {"type": "string", "const": "2.0"},
                "id": {},
                "result": {
                    "type": "object",
                    "properties": {
                        "tools": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "description": {"type": "string"},
                                    "inputSchema": {"type": "object"},
                                    "outputSchema": {"type": "object"}
                                },
                                "required": ["name", "description"],
                                "additionalProperties": true
                            }
                        }
                    },
                    "required": ["tools"],
                    "additionalProperties": false
                }
            },
            "required": ["jsonrpc", "id", "result"],
            "additionalProperties": false
        });

        let tool_call_response_schema = json!({
            "type": "object", 
            "properties": {
                "jsonrpc": {"type": "string", "const": "2.0"},
                "id": {},
                "result": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "array",
                            "items": {"type": "object"}
                        },
                        "isError": {"type": "boolean"}
                    },
                    "required": ["content", "isError"],
                    "additionalProperties": true
                }
            },
            "required": ["jsonrpc", "id", "result"],
            "additionalProperties": false
        });

        let error_response_schema = json!({
            "type": "object",
            "properties": {
                "jsonrpc": {"type": "string", "const": "2.0"},
                "id": {},
                "error": {
                    "type": "object",
                    "properties": {
                        "code": {"type": "integer"},
                        "message": {"type": "string"},
                        "data": {}
                    },
                    "required": ["code", "message"],
                    "additionalProperties": false
                }
            },
            "required": ["jsonrpc", "id", "error"],
            "additionalProperties": false
        });

        // Compile and store schemas
        let schemas = vec![
            ("initialize_request", initialize_request_schema),
            ("initialize_response", initialize_response_schema),
            ("tools_list_response", tools_list_response_schema),
            ("tool_call_response", tool_call_response_schema),
            ("error_response", error_response_schema),
        ];

        for (name, schema) in schemas {
            match Validator::new(&schema) {
                Ok(compiled_schema) => {
                    self.protocol_schemas.insert(name.to_string(), compiled_schema);
                    debug!("Added protocol schema: {}", name);
                }
                Err(e) => {
                    error!("Failed to compile protocol schema {}: {}", name, e);
                }
            }
        }
    }

    /// Get detailed validation errors for a response
    pub fn get_detailed_validation_errors(
        &self,
        tool_name: &str,
        response: &Value,
    ) -> Option<Vec<String>> {
        if let Some(schema) = self.tool_schemas.get(tool_name) {
            match schema.validate(response) {
                Ok(_) => None,
                Err(_) => {
                    // For now, return a simple error message since the ValidationError API is complex
                    Some(vec![format!("Validation failed for tool: {}", tool_name)])
                }
            }
        } else {
            None
        }
    }

    /// Check if a tool schema exists
    pub fn has_tool_schema(&self, tool_name: &str) -> bool {
        self.tool_schemas.contains_key(tool_name)
    }

    /// Get list of tools with schemas
    pub fn get_tools_with_schemas(&self) -> Vec<String> {
        self.tool_schemas.keys().cloned().collect()
    }

    /// Enable or disable strict mode
    pub fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
        debug!("Schema validation strict mode: {}", strict);
    }
}

/// Utility function to create a basic tool response schema
pub fn create_basic_tool_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "content": {
                "type": "array",
                "items": {"type": "object"}
            },
            "isError": {"type": "boolean"}
        },
        "required": ["content", "isError"],
        "additionalProperties": true
    })
}

/// Utility function to validate JSON-RPC 2.0 message structure
pub fn validate_jsonrpc_message(message: &Value) -> Result<()> {
    // Check for required jsonrpc field
    if let Some(jsonrpc) = message.get("jsonrpc") {
        if jsonrpc != "2.0" {
            return Err(anyhow::anyhow!("Invalid JSON-RPC version: expected '2.0', got '{}'", jsonrpc));
        }
    } else {
        return Err(anyhow::anyhow!("Missing 'jsonrpc' field"));
    }

    // Check for id field (required for requests and responses)
    if message.get("id").is_none() && message.get("method").is_some() {
        // This is a notification, which doesn't require an id
    } else if message.get("id").is_none() {
        return Err(anyhow::anyhow!("Missing 'id' field"));
    }

    // Check for either method (request) or result/error (response)
    let has_method = message.get("method").is_some();
    let has_result = message.get("result").is_some();
    let has_error = message.get("error").is_some();

    if !has_method && !has_result && !has_error {
        return Err(anyhow::anyhow!("Message must have either 'method' (request) or 'result'/'error' (response)"));
    }

    if has_result && has_error {
        return Err(anyhow::anyhow!("Message cannot have both 'result' and 'error'"));
    }

    Ok(())
}