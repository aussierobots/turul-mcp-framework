// turul-mcp-server v0.3
// Vec<T> output — ALWAYS wrap in a response struct
//
// Bare Vec<T> has schema issues with schemars 1.x:
// - ToolBuilder: schema_for!(Vec<T>) generates non-object root → from_schemars() REJECTS it
// - Derive macro without schemars: static schema shows "object" not "array"
// - Derive macro with schemars: works, but wrapper structs are still cleaner
//
// RECOMMENDED: Wrap Vec<T> in a response struct. This works reliably
// with all tool patterns (macros, derive, builder).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::prelude::*;

/// Item type — derive JsonSchema for detailed array item schema.
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

/// RECOMMENDED: Wrap Vec<T> in a response struct.
/// This produces a clean object schema that works with all patterns.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResponse {
    /// The matching results
    pub results: Vec<SearchResult>,
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Derive macro with wrapper struct — RECOMMENDED pattern.
/// Schema: {"type": "object", "properties": {"results": {"type": "array", ...}}}
#[derive(McpTool, Default)]
#[tool(
    name = "search_derive",
    description = "Search with array results (derive)",
    output = SearchResponse
)]
pub struct SearchDeriveTool {
    #[param(description = "Search query")]
    pub query: String,
    #[param(description = "Maximum number of results")]
    pub limit: Option<usize>,
}

impl SearchDeriveTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<SearchResponse> {
        let limit = self.limit.unwrap_or(10);
        let results = vec![
            SearchResult {
                title: format!("Result for: {}", self.query),
                score: 0.95,
                snippet: Some("...matching content...".to_string()),
            },
        ];
        Ok(SearchResponse {
            results: results.into_iter().take(limit).collect(),
            next_cursor: None,
        })
    }
}

/// Function macro with wrapper struct — RECOMMENDED pattern.
#[mcp_tool(name = "search_function", description = "Search with array results (function)")]
async fn search_fn(
    #[param(description = "Search query")] query: String,
) -> McpResult<SearchResponse> {
    Ok(SearchResponse {
        results: vec![SearchResult {
            title: format!("Result for: {}", query),
            score: 0.95,
            snippet: None,
        }],
        next_cursor: None,
    })
}

// ── NOT RECOMMENDED: bare Vec<T> ───────────────────────────────────
//
// Bare Vec<T> works with macros IF the item type derives JsonSchema,
// but hits bugs with ToolBuilder (from_schemars rejects non-object root).
// Use wrapper structs instead for consistency.
//
// #[derive(McpTool, Default)]
// #[tool(name = "search", description = "...", output = Vec<SearchResult>)]
// struct SearchTool { ... }
//
// #[mcp_tool(name = "search_fn", description = "...")]
// async fn search(query: String) -> McpResult<Vec<SearchResult>> { ... }
