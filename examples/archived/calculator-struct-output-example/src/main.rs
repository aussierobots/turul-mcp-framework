use turul_mcp_derive::{McpTool, JsonSchema};
use turul_mcp_server::{McpResult, McpServer};
use serde::{Deserialize, Serialize};
use tracing::info;

// Define the output struct for calculation results
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalculationResult {
    pub operation: String,
    pub operand1: f64,
    pub operand2: f64,
    pub result: f64,
    pub timestamp: String,
    pub metadata: CalculationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalculationMetadata {
    pub precision: String,
    pub calculation_time_ms: f64,
    pub is_exact: bool,
}

// Calculator tool that returns a struct
#[derive(McpTool, Clone)]
#[tool(name = "calculator_struct", description = "Advanced calculator that returns structured calculation results",
    output = CalculationResult
)]
struct CalculatorStructTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
    #[param(description = "Operation: add, subtract, multiply, divide")]
    operation: String,
}

impl CalculatorStructTool {
    async fn execute(&self) -> McpResult<CalculationResult> {
        let start_time = std::time::Instant::now();

        let result = match self.operation.as_str() {
            "add" => self.a + self.b,
            "subtract" => self.a - self.b,
            "multiply" => self.a * self.b,
            "divide" => {
                if self.b == 0.0 {
                    return Err(turul_mcp_protocol::McpError::tool_execution("Division by zero"));
                }
                self.a / self.b
            }
            _ => return Err(turul_mcp_protocol::McpError::invalid_param_type("operation", "add|subtract|multiply|divide", &self.operation)),
        };

        let calculation_time = start_time.elapsed().as_micros() as f64 / 1000.0;

        // Determine if the result is "exact" (no precision loss)
        let is_exact = match self.operation.as_str() {
            "add" | "subtract" => true,
            "multiply" => self.a.fract() == 0.0 && self.b.fract() == 0.0,
            "divide" => self.b != 0.0 && (self.a % self.b) == 0.0,
            _ => false,
        };

        Ok(CalculationResult {
            operation: self.operation.clone(),
            operand1: self.a,
            operand2: self.b,
            result,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: CalculationMetadata {
                precision: "f64".to_string(),
                calculation_time_ms: calculation_time,
                is_exact,
            },
        })
    }
}

// Another example with a different struct - statistical operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StatisticsResult {
    pub values: Vec<f64>,
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    pub range: f64,
}

#[derive(McpTool, Clone)]
#[tool(name = "statistics_calculator", description = "Calculate comprehensive statistics for a list of numbers",
    output = StatisticsResult
)]
struct StatisticsCalculatorTool {
    #[param(description = "List of numbers to analyze")]
    numbers: Vec<f64>,
}

impl StatisticsCalculatorTool {
    async fn execute(&self) -> McpResult<StatisticsResult> {
        if self.numbers.is_empty() {
            return Err(turul_mcp_protocol::McpError::invalid_param_type("numbers", "non-empty array", "empty array"));
        }

        let count = self.numbers.len();
        let sum: f64 = self.numbers.iter().sum();
        let mean = sum / count as f64;

        let min = self.numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = self.numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let range = max - min;

        // Calculate standard deviation
        let variance: f64 = self.numbers.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        Ok(StatisticsResult {
            values: self.numbers.clone(),
            count,
            sum,
            mean,
            min,
            max,
            std_dev,
            range,
        })
    }
}

// Complex calculation result with nested structures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuadraticResult {
    pub equation: String,
    pub discriminant: f64,
    pub solutions: QuadraticSolutions,
    pub vertex: Point2D,
    pub y_intercept: f64,
    pub properties: QuadraticProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuadraticSolutions {
    pub solution_type: String, // "two_real", "one_real", "complex"
    pub x1: Option<f64>,
    pub x2: Option<f64>,
    pub complex_part: Option<f64>, // imaginary component if complex
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuadraticProperties {
    pub opens_upward: bool,
    pub has_real_roots: bool,
    pub axis_of_symmetry: f64,
}

#[derive(McpTool, Clone)]
#[tool(name = "quadratic_solver", description = "Solve quadratic equations and provide comprehensive analysis",
    output = QuadraticResult
)]
struct QuadraticSolverTool {
    #[param(description = "Coefficient a in ax² + bx + c = 0")]
    a: f64,
    #[param(description = "Coefficient b in ax² + bx + c = 0")]
    b: f64,
    #[param(description = "Coefficient c in ax² + bx + c = 0")]
    c: f64,
}

impl QuadraticSolverTool {
    async fn execute(&self) -> McpResult<QuadraticResult> {
        if self.a == 0.0 {
            return Err(turul_mcp_protocol::McpError::invalid_param_type("a", "non-zero", "zero"));
        }

        let discriminant = self.b * self.b - 4.0 * self.a * self.c;

        let solutions = if discriminant > 0.0 {
            let sqrt_discriminant = discriminant.sqrt();
            let x1 = (-self.b + sqrt_discriminant) / (2.0 * self.a);
            let x2 = (-self.b - sqrt_discriminant) / (2.0 * self.a);
            QuadraticSolutions {
                solution_type: "two_real".to_string(),
                x1: Some(x1),
                x2: Some(x2),
                complex_part: None,
            }
        } else if discriminant == 0.0 {
            let x = -self.b / (2.0 * self.a);
            QuadraticSolutions {
                solution_type: "one_real".to_string(),
                x1: Some(x),
                x2: None,
                complex_part: None,
            }
        } else {
            let real_part = -self.b / (2.0 * self.a);
            let imaginary_part = (-discriminant).sqrt() / (2.0 * self.a);
            QuadraticSolutions {
                solution_type: "complex".to_string(),
                x1: Some(real_part),
                x2: Some(real_part),
                complex_part: Some(imaginary_part),
            }
        };

        // Calculate vertex (h, k) where h = -b/(2a)
        let vertex_x = -self.b / (2.0 * self.a);
        let vertex_y = self.a * vertex_x * vertex_x + self.b * vertex_x + self.c;
        let vertex = Point2D { x: vertex_x, y: vertex_y };

        let y_intercept = self.c;
        let opens_upward = self.a > 0.0;
        let has_real_roots = discriminant >= 0.0;
        let axis_of_symmetry = vertex_x;

        let equation = format!("{}x² + {}x + {} = 0", self.a, self.b, self.c);

        Ok(QuadraticResult {
            equation,
            discriminant,
            solutions,
            vertex,
            y_intercept,
            properties: QuadraticProperties {
                opens_upward,
                has_real_roots,
                axis_of_symmetry,
            },
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator struct output example server");
    let server = McpServer::builder()
        .name("calculator_struct_output")
        .version("0.1.0")
        .title("Calculator Struct Output Example Server")
        .instructions("Demonstrates struct-based output_type with derive macros for complex calculation results")
        .tool(CalculatorStructTool {
            a: 0.0,
            b: 0.0,
            operation: String::new(),
        })
        .tool(StatisticsCalculatorTool {
            numbers: Vec::new(),
        })
        .tool(QuadraticSolverTool {
            a: 0.0,
            b: 0.0,
            c: 0.0,
        })
        .bind_address("127.0.0.1:8650".parse()?)
        .build()?;

    println!("🧮 Calculator Struct Output Examples:");
    println!("  📊 calculator_struct - Basic arithmetic with detailed results");
    println!("  📈 statistics_calculator - Statistical analysis of number arrays");
    println!("  🔢 quadratic_solver - Comprehensive quadratic equation analysis");
    println!();
    println!("🌟 Features demonstrated:");
    println!("  ✨ Struct-based output_type with derive macros");
    println!("  ✨ Complex nested data structures");
    println!("  ✨ Automatic JSON schema generation for structs");
    println!("  ✨ Structured content matching complex output schemas");
    println!("  ✨ Rich calculation metadata and analysis");

    server.run().await?;
    Ok(())
}