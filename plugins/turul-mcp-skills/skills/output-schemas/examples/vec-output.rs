// turul-mcp-server v0.3
// Vec<T> output schema — the most common gotcha
//
// Tools returning arrays need:
// - Derive macro: output = Vec<ItemType> (explicit)
// - Function macro: auto-detected from return type

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::prelude::*;

/// Item type — must derive JsonSchema for detailed array item schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    /// Title of the matching item
    pub title: String,
    /// Relevance score (0.0 to 1.0)
    pub score: f64,
    /// Optional snippet showing the match context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// Derive macro with Vec<T> output.
/// output = Vec<SearchResult> is REQUIRED — without it, schema shows inputs.
/// Generated schema: {"type": "array", "items": {"type": "object", ...}}
#[derive(McpTool, Default)]
#[tool(
    name = "search_derive",
    description = "Search with array results (derive)",
    output = Vec<SearchResult>
)]
pub struct SearchDeriveTool {
    #[param(description = "Search query")]
    pub query: String,
    #[param(description = "Maximum number of results")]
    pub limit: Option<usize>,
}

impl SearchDeriveTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<SearchResult>> {
        let limit = self.limit.unwrap_or(10);
        let results = vec![
            SearchResult {
                title: format!("Result for: {}", self.query),
                score: 0.95,
                snippet: Some("...matching content...".to_string()),
            },
        ];
        Ok(results.into_iter().take(limit).collect())
    }
}

/// Function macro with Vec<T> output — auto-detected from return type.
/// No explicit output attribute needed.
#[mcp_tool(name = "search_function", description = "Search with array results (function)")]
async fn search_fn(
    #[param(description = "Search query")] query: String,
) -> McpResult<Vec<SearchResult>> {
    Ok(vec![SearchResult {
        title: format!("Result for: {}", query),
        score: 0.95,
        snippet: None,
    }])
}
