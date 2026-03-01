// turul-mcp-server v0.3
// Custom output_field — control the JSON key name in structuredContent
//
// By default, tool results are wrapped in {"result": <value>}.
// Use output_field to customize the key name.

use turul_mcp_derive::mcp_tool;
use turul_mcp_server::McpResult;

/// Default output field: "result"
/// Response structuredContent: {"result": 8}
#[mcp_tool(name = "add_default", description = "Add with default output field")]
async fn add_default(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

/// Custom output field: "sum"
/// Response structuredContent: {"sum": 8}
#[mcp_tool(
    name = "add_custom",
    description = "Add with custom output field",
    output_field = "sum"
)]
async fn add_custom(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

/// Custom output field: "wordCount"
/// Response structuredContent: {"wordCount": 5}
/// Note: use camelCase for JSON field names per MCP convention.
#[mcp_tool(
    name = "count_words",
    description = "Count words in text",
    output_field = "wordCount"
)]
async fn count_words(
    #[param(description = "Text to count words in")] text: String,
) -> McpResult<usize> {
    Ok(text.split_whitespace().count())
}
