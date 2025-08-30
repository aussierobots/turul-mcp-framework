use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpServer, McpResult};
use turul_mcp_protocol::McpError;

// Enhanced calculator using derive macro - demonstrates advanced parameters
#[derive(McpTool, Clone)]
#[tool(name = "enhanced_calculator", description = "Enhanced calculator with parameter validation")]
struct EnhancedCalculatorTool {
    #[param(description = "First number (0-1000)")]
    a: f64,
    #[param(description = "Second number (0-1000)")]
    b: f64,
    #[param(description = "Math operation (add, subtract, multiply, divide)")]
    operation: String,
    #[param(description = "Decimal precision for result")]
    precision: Option<i32>,
}

impl EnhancedCalculatorTool {
    async fn execute(&self) -> McpResult<String> {
        // Validate input ranges
        if self.a < 0.0 || self.a > 1000.0 {
            return Err(McpError::param_out_of_range("a", &self.a.to_string(), "must be between 0 and 1000"));
        }
        if self.b < 0.0 || self.b > 1000.0 {
            return Err(McpError::param_out_of_range("b", &self.b.to_string(), "must be between 0 and 1000"));
        }

        let result = match self.operation.as_str() {
            "add" => self.a + self.b,
            "subtract" => self.a - self.b,
            "multiply" => self.a * self.b,
            "divide" => {
                if self.b == 0.0 {
                    return Err(McpError::tool_execution("Division by zero"));
                }
                self.a / self.b
            },
            _ => return Err(McpError::invalid_param_type("operation", "add|subtract|multiply|divide", &self.operation)),
        };
        
        let precision = self.precision.unwrap_or(2) as usize;
        Ok(format!("{:.prec$}", result, prec = precision))
    }
}

// Math functions tool - demonstrates optional parameters with defaults
#[derive(McpTool, Clone)]
#[tool(name = "math_functions", description = "Advanced mathematical functions")]
struct MathFunctionsTool {
    #[param(description = "Function (sin, cos, tan, log, sqrt)")]
    function: String,
    #[param(description = "Input value")]
    value: f64,
    #[param(description = "Use degrees for trig functions")]
    degrees: Option<bool>,
    #[param(description = "Decimal precision")]
    precision: Option<i32>,
}

impl MathFunctionsTool {
    async fn execute(&self) -> McpResult<String> {
        let degrees = self.degrees.unwrap_or(false);
        let precision = self.precision.unwrap_or(4) as usize;
        
        let result = match self.function.as_str() {
            "sin" => {
                let val = if degrees { self.value.to_radians() } else { self.value };
                val.sin()
            },
            "cos" => {
                let val = if degrees { self.value.to_radians() } else { self.value };
                val.cos()
            },
            "tan" => {
                let val = if degrees { self.value.to_radians() } else { self.value };
                val.tan()
            },
            "log" => {
                if self.value <= 0.0 {
                    return Err(McpError::param_out_of_range("value", &self.value.to_string(), "must be positive for logarithm"));
                }
                self.value.ln()
            },
            "sqrt" => {
                if self.value < 0.0 {
                    return Err(McpError::param_out_of_range("value", &self.value.to_string(), "must be non-negative for square root"));
                }
                self.value.sqrt()
            },
            _ => return Err(McpError::invalid_param_type("function", "sin|cos|tan|log|sqrt", &self.function)),
        };
        
        Ok(format!("{:.prec$}", result, prec = precision))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Testing Enhanced Tool Macro Patterns");

    let server = McpServer::builder()
        .name("enhanced-tool-test")
        .version("1.0.0") 
        .title("Enhanced Tool Test")
        .instructions("Testing enhanced derive macro patterns with parameter validation")
        .tool(EnhancedCalculatorTool { 
            a: 0.0, b: 0.0, operation: String::new(), precision: None 
        })
        .tool(MathFunctionsTool { 
            function: String::new(), value: 0.0, degrees: None, precision: None 
        })
        .bind_address("127.0.0.1:8010".parse()?)
        .build()?;

    println!("Enhanced tool test server running at: http://127.0.0.1:8010/mcp");
    println!("Tools available:");
    println!("  - enhanced_calculator: Calculator with parameter validation");
    println!("  - math_functions: Advanced mathematical functions");
    
    server.run().await?;
    Ok(())
}