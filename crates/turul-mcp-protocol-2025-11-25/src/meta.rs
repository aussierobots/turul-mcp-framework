//! _meta Field Support for MCP 2025-11-25
//!
//! This module provides comprehensive support for the structured _meta fields
//! introduced in MCP 2025-11-25 specification.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Annotations for resources, prompts, and tools (matches TypeScript Annotations per MCP 2025-11-25).
/// See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// Target audience for this item: "user", "assistant", or both
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    /// Priority hint (0.0 = lowest, 1.0 = highest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    /// ISO 8601 datetime when this item was last modified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

impl Annotations {
    pub fn new() -> Self {
        Self {
            audience: None,
            priority: None,
            last_modified: None,
        }
    }

    pub fn with_audience(mut self, audience: Vec<String>) -> Self {
        self.audience = Some(audience);
        self
    }

    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = Some(priority.clamp(0.0, 1.0));
        self
    }

    pub fn with_last_modified(mut self, last_modified: impl Into<String>) -> Self {
        self.last_modified = Some(last_modified.into());
        self
    }
}

impl Default for Annotations {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress token for tracking long-running operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ProgressToken(pub String);

impl ProgressToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProgressToken {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProgressToken {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Cursor for pagination support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Cursor(pub String);

impl Cursor {
    pub fn new(cursor: impl Into<String>) -> Self {
        Self(cursor.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Cursor {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Cursor {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Structured _meta field for MCP 2025-11-25
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// Progress token for tracking long-running operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,

    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,

    /// Total number of items (for pagination context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,

    /// Whether there are more items available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,

    /// Estimated remaining time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_remaining_seconds: Option<f64>,

    /// Current progress (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,

    /// Current step in a multi-step process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<u64>,

    /// Total steps in a multi-step process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_steps: Option<u64>,

    /// Additional metadata as key-value pairs
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Meta {
    /// Create a new empty Meta
    pub fn new() -> Self {
        Self::default()
    }

    /// Create Meta with progress token
    pub fn with_progress_token(token: impl Into<ProgressToken>) -> Self {
        Self {
            progress_token: Some(token.into()),
            ..Default::default()
        }
    }

    /// Create Meta with cursor for pagination
    pub fn with_cursor(cursor: impl Into<Cursor>) -> Self {
        Self {
            cursor: Some(cursor.into()),
            ..Default::default()
        }
    }

    /// Create Meta with pagination info
    pub fn with_pagination(cursor: Option<Cursor>, total: Option<u64>, has_more: bool) -> Self {
        Self {
            cursor,
            total,
            has_more: Some(has_more),
            ..Default::default()
        }
    }

    /// Create Meta with progress information
    pub fn with_progress(
        progress: f64,
        current_step: Option<u64>,
        total_steps: Option<u64>,
    ) -> Self {
        Self {
            progress: Some(progress.clamp(0.0, 1.0)),
            current_step,
            total_steps,
            ..Default::default()
        }
    }

    /// Add progress token
    pub fn set_progress_token(mut self, token: impl Into<ProgressToken>) -> Self {
        self.progress_token = Some(token.into());
        self
    }

    /// Add cursor
    pub fn set_cursor(mut self, cursor: impl Into<Cursor>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Set pagination info
    pub fn set_pagination(
        mut self,
        cursor: Option<Cursor>,
        total: Option<u64>,
        has_more: bool,
    ) -> Self {
        self.cursor = cursor;
        self.total = total;
        self.has_more = Some(has_more);
        self
    }

    /// Set progress info
    pub fn set_progress(
        mut self,
        progress: f64,
        current_step: Option<u64>,
        total_steps: Option<u64>,
    ) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self.current_step = current_step;
        self.total_steps = total_steps;
        self
    }

    /// Set estimated remaining time
    pub fn set_estimated_remaining(mut self, seconds: f64) -> Self {
        self.estimated_remaining_seconds = Some(seconds);
        self
    }

    /// Add extra metadata
    pub fn add_extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// Check if meta has any content
    pub fn is_empty(&self) -> bool {
        self.progress_token.is_none()
            && self.cursor.is_none()
            && self.total.is_none()
            && self.has_more.is_none()
            && self.estimated_remaining_seconds.is_none()
            && self.progress.is_none()
            && self.current_step.is_none()
            && self.total_steps.is_none()
            && self.extra.is_empty()
    }

    /// Merge request extras from incoming request _meta into this Meta
    /// This helper preserves pagination context while adding request metadata
    pub fn merge_request_extras(mut self, request_meta: Option<&HashMap<String, Value>>) -> Self {
        if let Some(request_extras) = request_meta {
            for (key, value) in request_extras {
                // Don't override structured fields - only merge into extra
                match key.as_str() {
                    "progressToken"
                    | "cursor"
                    | "total"
                    | "hasMore"
                    | "estimatedRemainingSeconds"
                    | "progress"
                    | "currentStep"
                    | "totalSteps" => {
                        // Skip reserved fields - these should be set explicitly
                    }
                    _ => {
                        self.extra.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        self
    }
}

/// Trait for types that can include _meta fields
pub trait WithMeta {
    /// Get the _meta field
    fn meta(&self) -> Option<&Meta>;

    /// Set the _meta field
    fn set_meta(&mut self, meta: Option<Meta>);

    /// Add or update _meta field with builder pattern
    fn with_meta(mut self, meta: Meta) -> Self
    where
        Self: Sized,
    {
        self.set_meta(Some(meta));
        self
    }
}

/// Helper for pagination responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The actual response data
    #[serde(flatten)]
    pub data: T,

    /// Pagination metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data, meta: None }
    }

    pub fn with_pagination(
        data: T,
        cursor: Option<Cursor>,
        total: Option<u64>,
        has_more: bool,
    ) -> Self {
        Self {
            data,
            meta: Some(Meta::with_pagination(cursor, total, has_more)),
        }
    }
}

impl<T> WithMeta for PaginatedResponse<T> {
    fn meta(&self) -> Option<&Meta> {
        self.meta.as_ref()
    }

    fn set_meta(&mut self, meta: Option<Meta>) {
        self.meta = meta;
    }
}

/// Helper for progress responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressResponse<T> {
    /// The actual response data
    #[serde(flatten)]
    pub data: T,

    /// Progress metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

impl<T> ProgressResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data, meta: None }
    }

    pub fn with_progress(
        data: T,
        progress_token: Option<ProgressToken>,
        progress: f64,
        current_step: Option<u64>,
        total_steps: Option<u64>,
    ) -> Self {
        let mut meta = Meta::with_progress(progress, current_step, total_steps);
        if let Some(token) = progress_token {
            meta = meta.set_progress_token(token);
        }

        Self {
            data,
            meta: Some(meta),
        }
    }
}

impl<T> WithMeta for ProgressResponse<T> {
    fn meta(&self) -> Option<&Meta> {
        self.meta.as_ref()
    }

    fn set_meta(&mut self, meta: Option<Meta>) {
        self.meta = meta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_progress_token() {
        let token = ProgressToken::new("task-123");
        assert_eq!(token.as_str(), "task-123");

        let from_string: ProgressToken = "task-456".into();
        assert_eq!(from_string.as_str(), "task-456");
    }

    #[test]
    fn test_cursor() {
        let cursor = Cursor::new("page-2");
        assert_eq!(cursor.as_str(), "page-2");

        let from_string: Cursor = "page-3".into();
        assert_eq!(from_string.as_str(), "page-3");
    }

    #[test]
    fn test_meta_creation() {
        let meta = Meta::new()
            .set_progress_token("task-123")
            .set_progress(0.5, Some(5), Some(10))
            .add_extra("custom_field", "custom_value");

        assert_eq!(meta.progress_token.as_ref().unwrap().as_str(), "task-123");
        assert_eq!(meta.progress, Some(0.5));
        assert_eq!(meta.current_step, Some(5));
        assert_eq!(meta.total_steps, Some(10));
        assert_eq!(meta.extra.get("custom_field"), Some(&json!("custom_value")));
    }

    #[test]
    fn test_meta_serialization() {
        let meta = Meta::with_progress_token("task-123")
            .set_cursor("page-1")
            .set_progress(0.75, Some(3), Some(4));

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: Meta = serde_json::from_str(&json).unwrap();

        assert_eq!(meta.progress_token, deserialized.progress_token);
        assert_eq!(meta.cursor, deserialized.cursor);
        assert_eq!(meta.progress, deserialized.progress);
    }

    #[test]
    fn test_paginated_response() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            items: Vec<String>,
        }

        let data = TestData {
            items: vec!["item1".to_string(), "item2".to_string()],
        };

        let response =
            PaginatedResponse::with_pagination(data, Some("next-page".into()), Some(100), true);

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: PaginatedResponse<TestData> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.data.items.len(), 2);
        assert!(deserialized.meta.is_some());
        assert_eq!(
            deserialized
                .meta
                .as_ref()
                .unwrap()
                .cursor
                .as_ref()
                .unwrap()
                .as_str(),
            "next-page"
        );
        assert_eq!(deserialized.meta.as_ref().unwrap().total, Some(100));
        assert_eq!(deserialized.meta.as_ref().unwrap().has_more, Some(true));
    }

    #[test]
    fn test_progress_response() {
        #[derive(Serialize, Deserialize)]
        struct TaskResult {
            status: String,
        }

        let data = TaskResult {
            status: "processing".to_string(),
        };

        let response =
            ProgressResponse::with_progress(data, Some("task-456".into()), 0.8, Some(8), Some(10));

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ProgressResponse<TaskResult> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.data.status, "processing");
        assert!(deserialized.meta.is_some());
        assert_eq!(
            deserialized
                .meta
                .as_ref()
                .unwrap()
                .progress_token
                .as_ref()
                .unwrap()
                .as_str(),
            "task-456"
        );
        assert_eq!(deserialized.meta.as_ref().unwrap().progress, Some(0.8));
        assert_eq!(deserialized.meta.as_ref().unwrap().current_step, Some(8));
        assert_eq!(deserialized.meta.as_ref().unwrap().total_steps, Some(10));
    }

    #[test]
    fn test_meta_is_empty() {
        let empty_meta = Meta::new();
        assert!(empty_meta.is_empty());

        let non_empty_meta = Meta::new().set_progress_token("test");
        assert!(!non_empty_meta.is_empty());
    }

    #[test]
    fn test_merge_request_extras() {
        let mut request_meta = HashMap::new();
        request_meta.insert("customField".to_string(), json!("custom_value"));
        request_meta.insert("userContext".to_string(), json!("user_123"));
        request_meta.insert("progressToken".to_string(), json!("should_be_ignored"));
        request_meta.insert("cursor".to_string(), json!("should_be_ignored"));

        let meta = Meta::with_pagination(Some("page-1".into()), Some(100), true)
            .merge_request_extras(Some(&request_meta));

        // Should preserve existing structured fields
        assert_eq!(meta.cursor.as_ref().unwrap().as_str(), "page-1");
        assert_eq!(meta.total, Some(100));
        assert_eq!(meta.has_more, Some(true));

        // Should add custom fields to extra
        assert_eq!(meta.extra.get("customField"), Some(&json!("custom_value")));
        assert_eq!(meta.extra.get("userContext"), Some(&json!("user_123")));

        // Should not override structured fields in extra
        assert!(!meta.extra.contains_key("progressToken"));
        assert!(!meta.extra.contains_key("cursor"));
    }

    #[test]
    fn test_merge_request_extras_empty() {
        let meta = Meta::with_cursor("test-cursor").merge_request_extras(None);

        assert_eq!(meta.cursor.as_ref().unwrap().as_str(), "test-cursor");
        assert!(meta.extra.is_empty());
    }
}
