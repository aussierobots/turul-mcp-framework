use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer};
use serde::{Deserialize, Serialize};
use tracing::info;

// Test various primitive types with f64 output
#[derive(McpTool, Clone)]
#[tool(name = "math_operations", description = "Perform math operations on various numeric types")]
#[output_type(f64)]
struct MathOperationsTool {
    #[param(description = "Integer value")]
    int_val: i32,
    #[param(description = "Long integer value")]
    long_val: i64,
    #[param(description = "Unsigned integer")]
    uint_val: u32,
    #[param(description = "Float value")]
    float_val: f32,
    #[param(description = "Double value")]
    double_val: f64,
}

impl MathOperationsTool {
    async fn execute(&self) -> McpResult<f64> {
        let result = self.int_val as f64 + 
                    self.long_val as f64 + 
                    self.uint_val as f64 + 
                    self.float_val as f64 + 
                    self.double_val;
        Ok(result)
    }
}

// Test string processing with string output
#[derive(McpTool, Clone)]
#[tool(name = "string_processor", description = "Process strings in various ways")]
#[output_type(String)]
struct StringProcessorTool {
    #[param(description = "Input text to process")]
    text: String,
    #[param(description = "Number of repetitions")]
    repeat_count: u32,
    #[param(description = "Convert to uppercase")]
    uppercase: bool,
}

impl StringProcessorTool {
    async fn execute(&self) -> McpResult<String> {
        let mut result = self.text.repeat(self.repeat_count as usize);
        if self.uppercase {
            result = result.to_uppercase();
        }
        Ok(result)
    }
}

// Test array/vector processing
#[derive(McpTool, Clone)]
#[tool(name = "array_processor", description = "Process arrays of numbers")]
#[output_type(f64)]
struct ArrayProcessorTool {
    #[param(description = "Array of numbers to sum")]
    numbers: Vec<f64>,
    #[param(description = "Optional multiplier")]
    multiplier: Option<f64>,
}

impl ArrayProcessorTool {
    async fn execute(&self) -> McpResult<f64> {
        let sum: f64 = self.numbers.iter().sum();
        let multiplier = self.multiplier.unwrap_or(1.0);
        Ok(sum * multiplier)
    }
}

// Test boolean logic
#[derive(McpTool, Clone)]
#[tool(name = "boolean_logic", description = "Perform boolean operations")]
#[output_type(bool)]
struct BooleanLogicTool {
    #[param(description = "First boolean value")]
    a: bool,
    #[param(description = "Second boolean value")]
    b: bool,
    #[param(description = "Operation type: and, or, xor")]
    operation: String,
}

impl BooleanLogicTool {
    async fn execute(&self) -> McpResult<bool> {
        let result = match self.operation.as_str() {
            "and" => self.a && self.b,
            "or" => self.a || self.b,
            "xor" => self.a ^ self.b,
            _ => return Err(turul_mcp_protocol::McpError::invalid_param_type("operation", "and|or|xor", &self.operation)),
        };
        Ok(result)
    }
}

// Test struct-based output (complex type)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisResult {
    count: usize,
    average: f64,
    min: f64,
    max: f64,
    has_negatives: bool,
}

impl std::fmt::Display for AnalysisResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnalysisResult {{ count: {}, average: {:.2}, min: {:.2}, max: {:.2}, has_negatives: {} }}", 
               self.count, self.average, self.min, self.max, self.has_negatives)
    }
}

#[derive(McpTool, Clone)]
#[tool(name = "data_analyzer", description = "Analyze numerical data")]
struct DataAnalyzerTool {
    #[param(description = "Data points to analyze")]
    data: Vec<f64>,
    #[param(description = "Include detailed stats")]
    detailed: Option<bool>,
}

// Note: For struct outputs, we don't use #[output_type] - it will use generic schema
impl DataAnalyzerTool {
    async fn execute(&self) -> McpResult<AnalysisResult> {
        tracing::debug!("DataAnalyzerTool executing with {} data points, detailed: {:?}", self.data.len(), self.detailed);
        
        if self.data.is_empty() {
            return Err(turul_mcp_protocol::McpError::invalid_param_type("data", "non-empty array", "empty array"));
        }

        let count = self.data.len();
        let sum: f64 = self.data.iter().sum();
        let average = sum / count as f64;
        let min = self.data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = self.data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let has_negatives = self.data.iter().any(|&x| x < 0.0);

        Ok(AnalysisResult {
            count,
            average,
            min,
            max,
            has_negatives,
        })
    }
}

// Test optional parameters extensively
#[derive(McpTool, Clone)]
#[tool(name = "optional_params_test", description = "Test various optional parameter types")]
#[output_type(String)]
struct OptionalParamsTool {
    #[param(description = "Required string")]
    required_text: String,
    #[param(description = "Optional string")]
    optional_text: Option<String>,
    #[param(description = "Optional number")]
    optional_number: Option<f64>,
    #[param(description = "Optional boolean")]
    optional_bool: Option<bool>,
    #[param(description = "Optional array")]
    optional_array: Option<Vec<i32>>,
}

impl OptionalParamsTool {
    async fn execute(&self) -> McpResult<String> {
        let mut parts = vec![format!("required: {}", self.required_text)];
        
        if let Some(ref text) = self.optional_text {
            parts.push(format!("text: {}", text));
        }
        
        if let Some(num) = self.optional_number {
            parts.push(format!("number: {}", num));
        }
        
        if let Some(b) = self.optional_bool {
            parts.push(format!("bool: {}", b));
        }
        
        if let Some(ref arr) = self.optional_array {
            parts.push(format!("array: {:?}", arr));
        }
        
        Ok(parts.join(", "))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting comprehensive types example server");
    let server = McpServer::builder()
        .name("comprehensive_types_example")
        .version("0.1.0")
        .title("Comprehensive Types Example Server")
        .instructions("Demonstrates various parameter and output types for MCP tools")
        .tool(MathOperationsTool {
            int_val: 0,
            long_val: 0,
            uint_val: 0,
            float_val: 0.0,
            double_val: 0.0,
        })
        .tool(StringProcessorTool {
            text: String::new(),
            repeat_count: 0,
            uppercase: false,
        })
        .tool(ArrayProcessorTool {
            numbers: Vec::new(),
            multiplier: None,
        })
        .tool(BooleanLogicTool {
            a: false,
            b: false,
            operation: String::new(),
        })
        .tool(DataAnalyzerTool {
            data: Vec::new(),
            detailed: None,
        })
        .tool(OptionalParamsTool {
            required_text: String::new(),
            optional_text: None,
            optional_number: None,
            optional_bool: None,
            optional_array: None,
        })
        .bind_address("127.0.0.1:8647".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}