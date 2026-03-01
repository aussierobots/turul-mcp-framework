// turul-mcp-server v0.3
// Parameter validation: missing_param, invalid_param_type, param_out_of_range

use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::McpResult;

#[mcp_tool(name = "search", description = "Search with filters")]
async fn search(
    #[param(description = "Search query")] query: String,
    #[param(description = "Max results (1-100)")] limit: Option<f64>,
    #[param(description = "Sort order: asc or desc")] sort: Option<String>,
) -> McpResult<Vec<String>> {
    // Missing parameter — MCP error code -32602
    if query.trim().is_empty() {
        return Err(McpError::missing_param("query"));
    }

    // Invalid parameter type — MCP error code -32602
    let limit = limit.unwrap_or(10.0);
    if limit.fract() != 0.0 {
        return Err(McpError::invalid_param_type(
            "limit",
            "integer",
            "float",
        ));
    }

    // Parameter out of range — MCP error code -32602
    if limit < 1.0 || limit > 100.0 {
        return Err(McpError::param_out_of_range(
            "limit",
            &limit.to_string(),
            "must be between 1 and 100",
        ));
    }

    // Validate enum-like parameters
    let sort = sort.unwrap_or_else(|| "asc".to_string());
    if sort != "asc" && sort != "desc" {
        return Err(McpError::invalid_param_type(
            "sort",
            "\"asc\" or \"desc\"",
            &format!("\"{}\"", sort),
        ));
    }

    Ok(vec![format!("Result for '{}' (limit={}, sort={})", query, limit, sort)])
}
