// turul-mcp-server v0.3
// Unit testing a tool with the framework-native call() API

use serde::{Deserialize, Serialize};
use serde_json::json;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct WordCountResult {
    count: usize,
    unique_count: usize,
}

#[derive(McpTool, Default)]
#[tool(
    name = "word_count",
    description = "Count words in text",
    output = WordCountResult
)]
struct WordCountTool {
    #[param(description = "Text to count words in")]
    text: String,
}

impl WordCountTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<WordCountResult> {
        let words: Vec<&str> = self.text.split_whitespace().collect();
        let unique: std::collections::HashSet<&str> = words.iter().copied().collect();
        Ok(WordCountResult {
            count: words.len(),
            unique_count: unique.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_word_count_basic() {
        let tool = WordCountTool {
            text: "hello world hello".to_string(),
        };
        // Framework-native API: call() with JSON params and no session
        let result = tool.call(json!({"text": "hello world hello"}), None).await.unwrap();
        let parsed: WordCountResult = serde_json::from_value(result).unwrap();

        assert_eq!(parsed.count, 3);
        assert_eq!(parsed.unique_count, 2);
    }

    #[tokio::test]
    async fn test_word_count_empty() {
        let tool = WordCountTool {
            text: String::new(),
        };
        let result = tool.call(json!({"text": ""}), None).await.unwrap();
        let parsed: WordCountResult = serde_json::from_value(result).unwrap();

        assert_eq!(parsed.count, 0);
        assert_eq!(parsed.unique_count, 0);
    }

    #[tokio::test]
    async fn test_word_count_whitespace_handling() {
        let tool = WordCountTool {
            text: "  multiple   spaces   between  ".to_string(),
        };
        let result = tool
            .call(json!({"text": "  multiple   spaces   between  "}), None)
            .await
            .unwrap();
        let parsed: WordCountResult = serde_json::from_value(result).unwrap();

        assert_eq!(parsed.count, 3);
    }
}
