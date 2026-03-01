//! # MCP Tools Test Server
//!
//! Comprehensive test server providing various types of tools for E2E testing.
//! This server implements all MCP tools patterns and edge cases to validate
//! framework compliance with the MCP 2025-11-25 specification.
//!
//! ## Test Tools Available:
//!
//! ### Basic Tools (Coverage)
//! - `calculator` - Basic arithmetic operations with parameter validation
//! - `string_processor` - Text manipulation with various input types
//! - `data_transformer` - JSON/data transformation operations
//! - `session_counter` - Session-aware state management
//! - `progress_tracker` - Long-running operation with progress updates
//! - `error_generator` - Controlled error conditions for testing
//! - `parameter_validator` - Complex parameter validation scenarios
//!
//! ## Usage:
//! ```bash
//! # Start server on random port
//! cargo run --package tools-test-server
//!
//! # Test with curl
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//!
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Mcp-Session-Id: SESSION_ID" \
//!   -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
//!
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Mcp-Session-Id: SESSION_ID" \
//!   -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"calculator","arguments":{"operation":"add","a":5,"b":3}}}'
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
// Note: Removed LazyLock and Mutex imports as we now use proper SessionStorage integration
use std::time::Duration;

use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::info;
use uuid::Uuid;

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::ResourceContents;
use turul_mcp_protocol::schema::{JsonSchema, JsonSchemaGenerator};
use turul_mcp_protocol::tools::{ToolAnnotations, ToolSchema};
// Server prelude re-exports builders prelude + protocol types
use turul_mcp_server::prelude::*;

// ===== BASIC TOOLS (Core functionality testing) =====

/// Result type for calculator operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CalculatorResult {
    result: f64,
    operation: String,
    a: f64,
    b: f64,
}

impl JsonSchemaGenerator for CalculatorResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("result".to_string(), JsonSchema::number()),
                ("operation".to_string(), JsonSchema::string()),
                ("a".to_string(), JsonSchema::number()),
                ("b".to_string(), JsonSchema::number()),
            ]))
            .with_required(vec![
                "result".to_string(),
                "operation".to_string(),
                "a".to_string(),
                "b".to_string(),
            ])
    }
}

/// Basic calculator tool for testing arithmetic operations with parameter validation
#[derive(McpTool, Clone)]
#[tool(
    name = "calculator",
    description = "Performs basic arithmetic operations (add, subtract, multiply, divide) with validation",
    output = CalculatorResult
)]
pub struct CalculatorTool {
    /// The operation to perform
    #[param(description = "Operation: add, subtract, multiply, divide")]
    pub operation: String,

    /// First number
    #[param(description = "First number")]
    pub a: f64,

    /// Second number
    #[param(description = "Second number")]
    pub b: f64,
}

impl CalculatorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CalculatorResult> {
        let result = match self.operation.as_str() {
            "add" => self.a + self.b,
            "subtract" => self.a - self.b,
            "multiply" => self.a * self.b,
            "divide" => {
                if self.b == 0.0 {
                    return Err(McpError::tool_execution("Division by zero"));
                }
                self.a / self.b
            }
            _ => {
                return Err(McpError::tool_execution(&format!(
                    "Invalid operation: {}",
                    self.operation
                )));
            }
        };

        Ok(CalculatorResult {
            result,
            operation: self.operation.clone(),
            a: self.a,
            b: self.b,
        })
    }
}

/// Result type for string processing operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct StringResult {
    result: String,
    operation: String,
    original: String,
    metadata: HashMap<String, serde_json::Value>,
}

impl JsonSchemaGenerator for StringResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("result".to_string(), JsonSchema::string()),
                ("operation".to_string(), JsonSchema::string()),
                ("original".to_string(), JsonSchema::string()),
                ("metadata".to_string(), JsonSchema::object()),
            ]))
            .with_required(vec![
                "result".to_string(),
                "operation".to_string(),
                "original".to_string(),
            ])
    }
}

/// String processing tool for text manipulation operations
#[derive(McpTool, Clone)]
#[tool(
    name = "string_processor",
    description = "Processes text with operations like uppercase, lowercase, reverse, length",
    output = StringResult
)]
pub struct StringProcessorTool {
    /// Text to process
    #[param(description = "Input text to process")]
    pub text: String,

    /// Operation to perform
    #[param(description = "Operation: uppercase, lowercase, reverse, length, trim")]
    pub operation: String,
}

impl StringProcessorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<StringResult> {
        let mut metadata = HashMap::new();

        let result = match self.operation.as_str() {
            "uppercase" => self.text.to_uppercase(),
            "lowercase" => self.text.to_lowercase(),
            "reverse" => self.text.chars().rev().collect(),
            "length" => {
                let char_count = self.text.chars().count();
                let byte_count = self.text.len();
                metadata.insert("char_count".to_string(), serde_json::json!(char_count));
                metadata.insert("byte_count".to_string(), serde_json::json!(byte_count));
                format!("{} characters, {} bytes", char_count, byte_count)
            }
            "trim" => self.text.trim().to_string(),
            _ => {
                return Err(McpError::tool_execution(&format!(
                    "Invalid operation: {}",
                    self.operation
                )));
            }
        };

        Ok(StringResult {
            result,
            operation: self.operation.clone(),
            original: self.text.clone(),
            metadata,
        })
    }
}

/// Result type for data transformation operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct DataResult {
    result: serde_json::Value,
    operation: String,
    input_type: String,
    metadata: HashMap<String, serde_json::Value>,
}

impl JsonSchemaGenerator for DataResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("result".to_string(), JsonSchema::object()),
                ("operation".to_string(), JsonSchema::string()),
                ("input_type".to_string(), JsonSchema::string()),
                ("metadata".to_string(), JsonSchema::object()),
            ]))
            .with_required(vec![
                "result".to_string(),
                "operation".to_string(),
                "input_type".to_string(),
            ])
    }
}

/// Data transformation tool for JSON operations
#[derive(McpTool, Clone)]
#[tool(
    name = "data_transformer",
    description = "Transforms JSON data with operations like extract, merge, validate",
    output = DataResult
)]
pub struct DataTransformerTool {
    /// JSON data to transform
    #[param(description = "JSON data to transform")]
    pub data: serde_json::Value,

    /// Operation to perform
    #[param(description = "Operation: extract_keys, count_items, validate, pretty_print")]
    pub operation: String,
}

impl DataTransformerTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<DataResult> {
        let mut metadata = HashMap::new();

        let input_type = match &self.data {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
        .to_string();

        let result = match self.operation.as_str() {
            "extract_keys" => {
                if let serde_json::Value::Object(obj) = &self.data {
                    let keys: Vec<&String> = obj.keys().collect();
                    metadata.insert("key_count".to_string(), serde_json::json!(keys.len()));
                    serde_json::json!({"keys": keys})
                } else {
                    return Err(McpError::tool_execution(
                        "Data must be an object to extract keys",
                    ));
                }
            }
            "count_items" => {
                let count = match &self.data {
                    serde_json::Value::Object(obj) => obj.len(),
                    serde_json::Value::Array(arr) => arr.len(),
                    _ => 1,
                };
                serde_json::json!({"count": count, "type": input_type})
            }
            "validate" => {
                serde_json::json!({
                    "valid": true,
                    "type": input_type,
                    "serialized_size": serde_json::to_string(&self.data).unwrap_or_default().len()
                })
            }
            "pretty_print" => {
                let pretty = serde_json::to_string_pretty(&self.data).map_err(|e| {
                    McpError::tool_execution(&format!("Failed to pretty print: {}", e))
                })?;
                serde_json::json!({"formatted": pretty})
            }
            _ => {
                return Err(McpError::tool_execution(&format!(
                    "Invalid operation: {}",
                    self.operation
                )));
            }
        };

        Ok(DataResult {
            result,
            operation: self.operation.clone(),
            input_type,
            metadata,
        })
    }
}

/// Result type for session counter operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CounterResult {
    session_id: String,
    operation: String,
    current_value: i64,
    amount: i64,
    total_sessions: usize,
}

impl JsonSchemaGenerator for CounterResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("session_id".to_string(), JsonSchema::string()),
                ("operation".to_string(), JsonSchema::string()),
                ("current_value".to_string(), JsonSchema::integer()),
                ("amount".to_string(), JsonSchema::integer()),
                ("total_sessions".to_string(), JsonSchema::integer()),
            ]))
            .with_required(vec![
                "session_id".to_string(),
                "operation".to_string(),
                "current_value".to_string(),
            ])
    }
}

/// Session-aware counter tool that maintains state per session using proper SessionStorage integration
#[derive(McpTool, Clone)]
#[tool(
    name = "session_counter",
    description = "Maintains a counter per session, demonstrating proper SessionStorage integration",
    output = CounterResult
)]
pub struct SessionCounterTool {
    /// Operation to perform on counter
    #[param(description = "Operation: increment, decrement, get, reset")]
    pub operation: String,

    /// Amount to increment/decrement by (default 1)
    #[param(description = "Amount to change counter by (default 1)")]
    pub amount: Option<i64>,
}

impl SessionCounterTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<CounterResult> {
        let session_context = session.ok_or_else(|| {
            McpError::tool_execution("SessionCounterTool requires session context")
        })?;

        let session_id = session_context.session_id.clone();
        let amount = self.amount.unwrap_or(1);
        let counter_key = "counter_value";

        // Get current counter value from session storage
        let state_value = (session_context.get_state)(counter_key).await;
        let current_value = state_value.and_then(|v| v.as_i64()).unwrap_or(0);

        let new_value = match self.operation.as_str() {
            "increment" => {
                let new_val = current_value + amount;
                (session_context.set_state)(counter_key, json!(new_val)).await;
                new_val
            }
            "decrement" => {
                let new_val = current_value - amount;
                (session_context.set_state)(counter_key, json!(new_val)).await;
                new_val
            }
            "get" => current_value,
            "reset" => {
                (session_context.set_state)(counter_key, json!(0)).await;
                0
            }
            _ => {
                return Err(McpError::tool_execution(&format!(
                    "Invalid operation: {}",
                    self.operation
                )));
            }
        };

        // Note: total_sessions is not readily available from SessionContext
        // This would require additional API calls to the SessionStorage
        // For now, we'll set it to 1 to indicate single session isolation
        let total_sessions = 1;

        Ok(CounterResult {
            session_id,
            operation: self.operation.clone(),
            current_value: new_value,
            amount,
            total_sessions,
        })
    }
}

/// Result type for progress tracking operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ProgressResult {
    operation: String,
    duration: f64,
    steps: u32,
    progress_token: String,
    status: String,
    completed_at: String,
}

impl JsonSchemaGenerator for ProgressResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("operation".to_string(), JsonSchema::string()),
                ("duration".to_string(), JsonSchema::number()),
                ("steps".to_string(), JsonSchema::integer()),
                ("progress_token".to_string(), JsonSchema::string()),
                ("status".to_string(), JsonSchema::string()),
                ("completed_at".to_string(), JsonSchema::string()),
            ]))
            .with_required(vec![
                "operation".to_string(),
                "duration".to_string(),
                "steps".to_string(),
                "status".to_string(),
            ])
    }
}

/// Progress tracking tool for long-running operations with progress notifications
#[derive(McpTool, Clone)]
#[tool(
    name = "progress_tracker",
    description = "Simulates long-running operation with progress notifications",
    output = ProgressResult
)]
pub struct ProgressTrackerTool {
    /// Duration in seconds for the operation
    #[param(description = "Duration of operation in seconds")]
    pub duration: f64,

    /// Number of progress updates to send
    #[param(description = "Number of progress steps (default 5)")]
    pub steps: Option<u32>,
}

impl ProgressTrackerTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<ProgressResult> {
        let steps = self.steps.unwrap_or(5).max(1);
        let step_duration = Duration::from_secs_f64(self.duration / steps as f64);
        let progress_token = Uuid::now_v7().as_simple().to_string();

        info!(
            "Starting progress tracking operation: {} seconds, {} steps",
            self.duration, steps
        );

        // Send initial progress notification
        if let Some(session_context) = &session {
            session_context.notify_progress(&progress_token, 0).await;
            info!(
                "Starting progress tracking operation: {} seconds, {} steps",
                self.duration, steps
            );
        }

        // Simulate work with progress updates
        for step in 1..=steps {
            sleep(step_duration).await;

            if let Some(session_context) = &session {
                let progress = (step as f64 / steps as f64 * 100.0) as u64;
                session_context
                    .notify_progress(&progress_token, progress)
                    .await;
                info!(
                    "Progress: {}/{} steps completed ({}%)",
                    step, steps, progress
                );
            } else {
                info!("Progress: {}/{} steps completed (no session)", step, steps);
            }
        }

        Ok(ProgressResult {
            operation: "progress_tracker".to_string(),
            duration: self.duration,
            steps,
            progress_token,
            status: "completed".to_string(),
            completed_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Result type for error generator operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ErrorResult {
    message: String,
}

impl JsonSchemaGenerator for ErrorResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([(
                "message".to_string(),
                JsonSchema::string(),
            )]))
            .with_required(vec!["message".to_string()])
    }
}

/// Error generator tool for testing error handling
#[derive(McpTool, Clone)]
#[tool(
    name = "error_generator",
    description = "Generates specific types of errors for testing error handling",
    output = ErrorResult
)]
pub struct ErrorGeneratorTool {
    /// Type of error to generate
    #[param(description = "Error type: invalid_params, tool_execution, timeout, resource_error")]
    pub error_type: String,

    /// Custom error message
    #[param(description = "Custom error message (optional)")]
    pub message: Option<String>,
}

impl ErrorGeneratorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<ErrorResult> {
        let message = self
            .message
            .clone()
            .unwrap_or_else(|| format!("Test error: {}", self.error_type));

        match self.error_type.as_str() {
            "invalid_params" => Err(McpError::tool_execution(&message)),
            "tool_execution" => Err(McpError::tool_execution(&message)),
            "timeout" => {
                sleep(Duration::from_secs(10)).await; // This should timeout in tests
                Ok(ErrorResult {
                    message: "should not reach here".to_string(),
                })
            }
            "resource_error" => Err(McpError::resource_execution(&message)),
            "validation" => Err(McpError::validation(&message)),
            _ => Err(McpError::tool_execution(&format!(
                "Unknown error type: {}",
                self.error_type
            ))),
        }
    }
}

/// Result type for parameter validator operations
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ValidationResult {
    validation_result: String,
    email: String,
    age: u32,
    config_keys: Vec<String>,
    tag_count: usize,
    validated_at: String,
}

impl JsonSchemaGenerator for ValidationResult {
    fn json_schema() -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("validation_result".to_string(), JsonSchema::string()),
                ("email".to_string(), JsonSchema::string()),
                ("age".to_string(), JsonSchema::integer()),
                (
                    "config_keys".to_string(),
                    JsonSchema::array(JsonSchema::string()),
                ),
                ("tag_count".to_string(), JsonSchema::integer()),
                ("validated_at".to_string(), JsonSchema::string()),
            ]))
            .with_required(vec![
                "validation_result".to_string(),
                "email".to_string(),
                "age".to_string(),
            ])
    }
}

/// Parameter validator tool for complex schema validation
#[derive(McpTool, Clone)]
#[tool(
    name = "parameter_validator",
    description = "Tests complex parameter validation scenarios",
    output = ValidationResult
)]
pub struct ParameterValidatorTool {
    /// Email address to validate
    #[param(description = "Email address")]
    pub email: String,

    /// Age (must be 0-150)
    #[param(description = "Age in years (0-150)")]
    pub age: u32,

    /// Configuration object
    #[param(description = "Configuration object")]
    pub config: serde_json::Value,

    /// Optional tags
    #[param(description = "Optional tags array")]
    pub tags: Option<Vec<String>>,
}

impl ParameterValidatorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<ValidationResult> {
        // Validate email format
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(McpError::validation("Invalid email format"));
        }

        // Validate age range
        if self.age > 150 {
            return Err(McpError::tool_execution(&format!(
                "Age {} is out of range (0-150)",
                self.age
            )));
        }

        // Validate config is an object
        if !self.config.is_object() {
            return Err(McpError::tool_execution("config must be an object"));
        }

        let config_keys = self
            .config
            .as_object()
            .ok_or_else(|| McpError::tool_execution("config object validation failed"))?
            .keys()
            .cloned()
            .collect();
        let tag_count = self.tags.as_ref().map(|t| t.len()).unwrap_or(0);

        Ok(ValidationResult {
            validation_result: "passed".to_string(),
            email: self.email.clone(),
            age: self.age,
            config_keys,
            tag_count,
            validated_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Legacy calculator tool (deprecated in favor of the main calculator)
struct LegacyCalculatorTool;

impl HasBaseMetadata for LegacyCalculatorTool {
    fn name(&self) -> &str {
        "legacy_calculator"
    }
}

impl HasDescription for LegacyCalculatorTool {
    fn description(&self) -> Option<&str> {
        Some("Basic addition only (deprecated - use calculator instead)")
    }
}

impl HasInputSchema for LegacyCalculatorTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    (
                        "a".to_string(),
                        JsonSchema::Number {
                            description: Some("First number".to_string()),
                            minimum: None,
                            maximum: None,
                        },
                    ),
                    (
                        "b".to_string(),
                        JsonSchema::Number {
                            description: Some("Second number".to_string()),
                            minimum: None,
                            maximum: None,
                        },
                    ),
                ]))
                .with_required(vec!["a".to_string(), "b".to_string()])
        })
    }
}

impl HasOutputSchema for LegacyCalculatorTool {}

impl HasAnnotations for LegacyCalculatorTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        static ANNOTATIONS: std::sync::OnceLock<ToolAnnotations> = std::sync::OnceLock::new();
        Some(ANNOTATIONS.get_or_init(|| {
            ToolAnnotations::new()
                .with_title("Legacy Calculator (Add Only)")
                .with_read_only_hint(true)
        }))
    }
}

impl HasToolMeta for LegacyCalculatorTool {}

impl HasIcons for LegacyCalculatorTool {}
impl HasExecution for LegacyCalculatorTool {}

#[async_trait::async_trait]
impl McpTool for LegacyCalculatorTool {
    async fn call(
        &self,
        args: serde_json::Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::missing_param("a"))?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::missing_param("b"))?;

        let result = a + b;

        Ok(CallToolResult::success(vec![
            ToolResult::text(
                "⚠️ DEPRECATED: legacy_calculator is deprecated since v0.1.0. Use 'calculator' instead.",
            ),
            ToolResult::text(format!("{} + {} = {}", a, b, result)),
            ToolResult::resource(ResourceContents::text(
                "file:///calculation/result.json",
                serde_json::to_string_pretty(&serde_json::json!({
                    "result": result,
                    "operation": "add",
                    "inputs": {"a": a, "b": b},
                    "deprecation_warning": {
                        "deprecated": true,
                        "since": "0.1.0",
                        "replacement": "calculator",
                        "removal_date": "2025-12-31"
                    }
                }))
                .unwrap(),
            )),
        ]))
    }
}

// ===== CUSTOM OUTPUT FIELD TOOL (MCP Compliance Testing) =====

use turul_mcp_derive::mcp_tool;

/// Word count result for MCP compliance testing
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct WordCountResult {
    word_count: u32,
    character_count: u32,
    sentence_count: u32,
}

impl std::fmt::Display for WordCountResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Words: {}, Characters: {}, Sentences: {}",
            self.word_count, self.character_count, self.sentence_count
        )
    }
}

#[mcp_tool(
    name = "word_count_analyzer",
    description = "Analyze text and return word, character, and sentence counts",
    output_field = "analysisResult"  // Custom field name for testing MCP compliance
)]
async fn word_count_analyzer(
    #[param(description = "Text to analyze")] text: String,
) -> McpResult<WordCountResult> {
    let word_count = text.split_whitespace().count() as u32;
    let character_count = text.chars().count() as u32;
    let sentence_count = text.split('.').filter(|s| !s.trim().is_empty()).count() as u32;

    Ok(WordCountResult {
        word_count,
        character_count,
        sentence_count,
    })
}

/// Simple addition tool with custom output field
#[mcp_tool(
    name = "custom_calculator",
    description = "Add two numbers with custom output field name",
    output_field = "calculationResult"  // Custom field name instead of default "result"
)]
async fn custom_calculator(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

// ===== BUG REPRODUCTION TOOLS =====

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CountAnnouncementsResult {
    pub count: u32,
}

/// Test tool that reproduces the output_field schema bug
#[derive(McpTool, Clone)]
#[tool(
    name = "count_announcements_struct",
    description = "Count announcements using struct macro with custom output field",
    output = CountAnnouncementsResult,
    output_field = "countResult"  // This should show up in schema, but doesn't
)]
pub struct CountAnnouncementsTool {
    #[param(description = "Text to analyze")]
    pub text: String,
}

impl CountAnnouncementsTool {
    async fn execute(
        &self,
        _session: Option<SessionContext>,
    ) -> McpResult<CountAnnouncementsResult> {
        let count = self.text.matches("announcement").count() as u32;
        Ok(CountAnnouncementsResult { count })
    }
}

/// Result type for simple counting
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CountResult {
    pub count: u32,
}

/// Simple tool like user's example - no #tool attribute, just derive
#[derive(McpTool, Default)]
pub struct CountWords {
    #[param(description = "Optional word to count (e.g. 'hello')")]
    word: Option<String>,
}

impl CountWords {
    pub async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CountResult> {
        let count = if let Some(word) = &self.word {
            // Count specific word occurrences
            format!(
                "The quick brown fox jumps over the lazy dog. The {} was amazing.",
                word
            )
            .matches(word)
            .count() as u32
        } else {
            // Count total words
            "The quick brown fox jumps over the lazy dog"
                .split_whitespace()
                .count() as u32
        };

        Ok(CountResult { count })
    }
}

// ===== SERVER IMPLEMENTATION =====

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to bind to (0 for random port)
    #[arg(short, long, default_value_t = 0)]
    port: u16,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing - respect RUST_LOG or use default levels
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new(format!(
                    "tools_test_server={},turul_mcp_server={},turul_http_mcp_server={}",
                    log_level, log_level, log_level
                ))
            }),
        )
        .init();

    info!("🔧 MCP Tools Test Server starting...");

    // Create server with comprehensive tool collection (with strict lifecycle for testing)
    let server = McpServer::builder()
        .name("tools-test-server")
        .version("0.2.0")
        .title("MCP Tools Test Server")
        .instructions("Comprehensive test tools for E2E validation")
        .with_strict_lifecycle() // Enable strict lifecycle enforcement for E2E testing
        // Basic tools
        .tool(CalculatorTool {
            operation: "add".to_string(),
            a: 0.0,
            b: 0.0,
        })
        .tool(StringProcessorTool {
            text: "".to_string(),
            operation: "uppercase".to_string(),
        })
        .tool(DataTransformerTool {
            data: serde_json::json!({}),
            operation: "validate".to_string(),
        })
        // Advanced tools
        .tool(SessionCounterTool {
            operation: "get".to_string(),
            amount: None,
        })
        .tool(ProgressTrackerTool {
            duration: 1.0,
            steps: Some(3),
        })
        // Error testing tools
        .tool(ErrorGeneratorTool {
            error_type: "tool_execution".to_string(),
            message: None,
        })
        .tool(ParameterValidatorTool {
            email: "test@example.com".to_string(),
            age: 25,
            config: serde_json::json!({}),
            tags: None,
        })
        // Deprecated tool for testing deprecation annotations
        .tool(LegacyCalculatorTool)
        // Bug reproduction tool - demonstrates output_field schema mismatch
        .tool(CountAnnouncementsTool {
            text: "".to_string(),
        })
        .tool(CountWords::default())
        // Custom output field tools for MCP compliance testing
        .tool_fn(word_count_analyzer)
        .tool_fn(custom_calculator)
        .bind_address(SocketAddr::from(([0, 0, 0, 0], args.port)))
        .build()?;

    info!("🚀 Tools Test Server running on port {}", args.port);
    info!(
        "📋 Available tools: calculator, string_processor, data_transformer, session_counter, progress_tracker, error_generator, parameter_validator, legacy_calculator (deprecated), word_count_analyzer (custom output: analysisResult), custom_calculator (custom output: calculationResult)"
    );

    server.run().await?;
    Ok(())
}
