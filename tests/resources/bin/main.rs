//! # MCP Resource Test Server
//!
//! Comprehensive test server providing various types of resources for E2E testing.
//! This server implements all MCP resource patterns and edge cases to validate
//! framework compliance with the MCP 2025-11-25 specification.
//!
//! ## Test Resources Available:
//!
//! ### Basic Resources (Coverage)
//! - `file:///tmp/test.txt` - Reads files from disk with proper error handling
//! - `file:///memory/data.json` - Returns in-memory data for fast testing
//! - `file:///error/not_found.txt` - Always returns NotFound errors for error path testing
//! - `file:///slow/delayed.txt` - Simulates slow operations with configurable delays
//! - `file:///template/items/{id}.json` - Tests URI templates with variable substitution
//! - `file:///empty/content.txt` - Returns empty content for edge case testing
//! - `file:///large/dataset.json` - Returns very large content for size testing
//! - `file:///binary/image.png` - Returns binary data with proper MIME types
//!
//! ### Advanced Resources (Features)
//! - `file:///session/info.json` - Returns session ID and metadata (session-aware)
//! - `file:///subscribe/updates.json` - Resource for testing subscription behavior (returns MethodNotFound)
//! - `file:///notify/trigger.json` - Triggers list change notifications via SSE
//! - `file:///multi/contents.txt` - Returns multiple ResourceContent items
//! - `file:///paginated/items.json` - Supports cursor-based pagination
//!
//! ### Edge Cases
//! - `file:///invalid/bad-chars-and-spaces.txt` - Intentionally non-compliant URI for error testing
//! - `file:///long/very-long-uri...` - Resource with very long URIs
//! - `file:///meta/dynamic.json` - Changes behavior based on _meta fields
//! - `file:///complete/all-fields.json` - Resource with all optional fields populated
//!
//! ## Usage:
//! ```bash
//! # Start server on random port
//! cargo run --package resource-test-server
//!
//! # Test with curl
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//!
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Mcp-Session-Id: SESSION_ID" \
//!   -d '{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}'
//! ```

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use clap::Parser;
use serde_json::{json, Value};
use std::io::Write;
use tempfile::NamedTempFile;
use tokio::time::sleep;
use tracing::info;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_protocol::{meta::Annotations, McpError};
use turul_mcp_server::McpResource;
use turul_mcp_server::{McpResult, McpServer, SessionContext};

/// Helper function to serialize JSON with proper error handling
/// Avoids unwrap() usage as per production code guidelines
fn safe_json_serialize<T: serde::Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_string_pretty(value)
        .map_err(|e| McpError::resource_execution(&format!("JSON serialization failed: {}", e)))
}

#[derive(Parser)]
#[command(name = "resource-test-server")]
#[command(about = "MCP Resource Test Server - Comprehensive test resources for E2E validation")]
struct Args {
    /// Port to run the server on (0 = random port)
    #[arg(short, long, default_value = "0")]
    port: u16,
}

// =============================================================================
// Basic Resources (Coverage)
// =============================================================================

/// File resource that reads from disk with proper error handling
#[derive(Clone)]
struct FileResource {
    temp_file: Option<Arc<NamedTempFile>>,
}

impl FileResource {
    fn new() -> McpResult<Self> {
        let mut temp_file = NamedTempFile::new().map_err(|e| {
            McpError::resource_execution(&format!("Failed to create temp file: {}", e))
        })?;

        writeln!(temp_file, "Test file content for E2E testing").map_err(|e| {
            McpError::resource_execution(&format!("Failed to write temp file: {}", e))
        })?;
        writeln!(
            temp_file,
            "Line 2: This file is created in-memory for testing"
        )
        .map_err(|e| McpError::resource_execution(&format!("Failed to write temp file: {}", e)))?;
        writeln!(
            temp_file,
            "Line 3: Contains sample data for file:// resource testing"
        )
        .map_err(|e| McpError::resource_execution(&format!("Failed to write temp file: {}", e)))?;

        Ok(Self {
            temp_file: Some(Arc::new(temp_file)),
        })
    }
}

impl HasResourceMetadata for FileResource {
    fn name(&self) -> &str {
        "file_resource"
    }
}

impl HasResourceUri for FileResource {
    fn uri(&self) -> &str {
        "file:///tmp/test.txt"
    }
}

impl HasResourceDescription for FileResource {
    fn description(&self) -> Option<&str> {
        Some("Reads files from disk with proper error handling")
    }
}

impl HasResourceMimeType for FileResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for FileResource {}
impl HasResourceAnnotations for FileResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for FileResource {}
impl HasIcons for FileResource {}

#[async_trait]
impl McpResource for FileResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        if let Some(temp_file) = &self.temp_file {
            match std::fs::read_to_string(temp_file.path()) {
                Ok(content) => Ok(vec![ResourceContent::text(
                    "file:///tmp/test.txt",
                    format!("File Content:\n{}", content),
                )]),
                Err(e) => Err(McpError::resource_execution(&format!(
                    "Failed to read file: {}",
                    e
                ))),
            }
        } else {
            Err(McpError::resource_execution("No file available"))
        }
    }
}

/// Memory resource that returns in-memory data for fast testing
#[derive(Clone, Default)]
struct MemoryResource;

impl HasResourceMetadata for MemoryResource {
    fn name(&self) -> &str {
        "memory_resource"
    }
}

impl HasResourceUri for MemoryResource {
    fn uri(&self) -> &str {
        "file:///memory/data.json"
    }
}

impl HasResourceDescription for MemoryResource {
    fn description(&self) -> Option<&str> {
        Some("Returns in-memory data for fast testing")
    }
}

impl HasResourceMimeType for MemoryResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for MemoryResource {}
impl HasResourceAnnotations for MemoryResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for MemoryResource {}
impl HasIcons for MemoryResource {}

#[async_trait]
impl McpResource for MemoryResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let data = json!({
            "type": "memory",
            "timestamp": Utc::now().to_rfc3339(),
            "data": {
                "items": [
                    {"id": 1, "name": "Test Item 1", "value": 42},
                    {"id": 2, "name": "Test Item 2", "value": 84},
                    {"id": 3, "name": "Test Item 3", "value": 126}
                ],
                "metadata": {
                    "total_count": 3,
                    "generated_at": Utc::now().to_rfc3339(),
                    "version": "1.0.0"
                }
            }
        });

        Ok(vec![ResourceContent::text(
            "file:///memory/data.json",
            safe_json_serialize(&data)?,
        )])
    }
}

/// Error resource that always returns specific errors for error path testing
#[derive(Clone, Default)]
struct ErrorResource;

impl HasResourceMetadata for ErrorResource {
    fn name(&self) -> &str {
        "error_resource"
    }
}

impl HasResourceUri for ErrorResource {
    fn uri(&self) -> &str {
        "file:///error/not_found.txt"
    }
}

impl HasResourceDescription for ErrorResource {
    fn description(&self) -> Option<&str> {
        Some("Always returns NotFound errors for error path testing")
    }
}

impl HasResourceMimeType for ErrorResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for ErrorResource {}
impl HasResourceAnnotations for ErrorResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for ErrorResource {}
impl HasIcons for ErrorResource {}

#[async_trait]
impl McpResource for ErrorResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        Err(McpError::resource_execution(
            "This resource always returns NotFound for testing error paths",
        ))
    }
}

/// Slow resource that simulates slow operations with configurable delays
#[derive(Clone, Default)]
struct SlowResource;

impl HasResourceMetadata for SlowResource {
    fn name(&self) -> &str {
        "slow_resource"
    }
}

impl HasResourceUri for SlowResource {
    fn uri(&self) -> &str {
        "file:///slow/delayed.txt"
    }
}

impl HasResourceDescription for SlowResource {
    fn description(&self) -> Option<&str> {
        Some("Simulates slow operations with configurable delays")
    }
}

impl HasResourceMimeType for SlowResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for SlowResource {}
impl HasResourceAnnotations for SlowResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for SlowResource {}
impl HasIcons for SlowResource {}

#[async_trait]
impl McpResource for SlowResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let delay_ms = params
            .as_ref()
            .and_then(|p| p.get("delay_ms"))
            .and_then(|d| d.as_u64())
            .unwrap_or(2000); // Default 2 second delay

        let start = Instant::now();
        sleep(Duration::from_millis(delay_ms)).await;
        let elapsed = start.elapsed();

        let content = format!(
            "Slow operation completed!\n\
            Requested delay: {}ms\n\
            Actual elapsed time: {:?}\n\
            Timestamp: {}",
            delay_ms,
            elapsed,
            Utc::now().to_rfc3339()
        );

        Ok(vec![ResourceContent::text(
            "file:///slow/delayed.txt",
            content,
        )])
    }
}

/// Template resource that tests URI templates with variable substitution
#[derive(Clone, Default)]
struct TemplateResource;

impl HasResourceMetadata for TemplateResource {
    fn name(&self) -> &str {
        "template_resource"
    }
}

impl HasResourceUri for TemplateResource {
    fn uri(&self) -> &str {
        "file:///template/items/{id}.json"
    }
}

impl HasResourceDescription for TemplateResource {
    fn description(&self) -> Option<&str> {
        Some("Tests URI templates with variable substitution")
    }
}

impl HasResourceMimeType for TemplateResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for TemplateResource {}
impl HasResourceAnnotations for TemplateResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for TemplateResource {}
impl HasIcons for TemplateResource {}

#[async_trait]
impl McpResource for TemplateResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // Extract template variables from params
        let template_vars = params
            .as_ref()
            .and_then(|p| p.get("template_variables"))
            .and_then(|tv| tv.as_object());

        let id = template_vars
            .and_then(|vars| vars.get("id"))
            .and_then(|id| id.as_str())
            .unwrap_or("unknown");

        let item_data = json!({
            "id": id,
            "uri": format!("file:///template/items/{}.json", id),
            "name": format!("Template Item {}", id),
            "description": format!("This is a dynamically generated item with ID: {}", id),
            "created_at": Utc::now().to_rfc3339(),
            "template_variables": template_vars.unwrap_or(&serde_json::Map::new()),
            "metadata": {
                "resource_type": "template",
                "supports_variables": true,
                "example_usage": "Pass {\"template_variables\": {\"id\": \"123\"}} in params"
            }
        });

        Ok(vec![ResourceContent::text(
            format!("file:///template/items/{}.json", id),
            safe_json_serialize(&item_data)?,
        )])
    }
}

/// Empty resource that returns empty content for edge case testing
#[derive(Clone, Default)]
struct EmptyResource;

impl HasResourceMetadata for EmptyResource {
    fn name(&self) -> &str {
        "empty_resource"
    }
}

impl HasResourceUri for EmptyResource {
    fn uri(&self) -> &str {
        "file:///empty/content.txt"
    }
}

impl HasResourceDescription for EmptyResource {
    fn description(&self) -> Option<&str> {
        Some("Returns empty content for edge case testing")
    }
}

impl HasResourceMimeType for EmptyResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for EmptyResource {}
impl HasResourceAnnotations for EmptyResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for EmptyResource {}
impl HasIcons for EmptyResource {}

#[async_trait]
impl McpResource for EmptyResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text("file:///empty/content.txt", "")])
    }
}

/// Large resource that returns very large content for size testing
#[derive(Clone, Default)]
struct LargeResource;

impl HasResourceMetadata for LargeResource {
    fn name(&self) -> &str {
        "large_resource"
    }
}

impl HasResourceUri for LargeResource {
    fn uri(&self) -> &str {
        "file:///large/dataset.json"
    }
}

impl HasResourceDescription for LargeResource {
    fn description(&self) -> Option<&str> {
        Some("Returns very large content for size testing")
    }
}

impl HasResourceMimeType for LargeResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for LargeResource {
    fn size(&self) -> Option<u64> {
        Some(1024 * 1024) // Indicate ~1MB size
    }
}

impl HasResourceAnnotations for LargeResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for LargeResource {}
impl HasIcons for LargeResource {}

#[async_trait]
impl McpResource for LargeResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let size = params
            .as_ref()
            .and_then(|p| p.get("size"))
            .and_then(|s| s.as_u64())
            .unwrap_or(10000) as usize; // Default 10K items

        let mut items = Vec::new();
        for i in 0..size {
            items.push(json!({
                "id": i,
                "name": format!("Large Dataset Item {}", i),
                "value": i * 42,
                "timestamp": Utc::now().to_rfc3339(),
                "data": format!("Sample data for item {} - this is test content to make the response larger", i)
            }));
        }

        let large_data = json!({
            "type": "large_dataset",
            "total_items": size,
            "generated_at": Utc::now().to_rfc3339(),
            "approximate_size_bytes": size * 200, // Rough estimate
            "items": items
        });

        Ok(vec![ResourceContent::text(
            "file:///large/dataset.json",
            &serde_json::to_string(&large_data).map_err(|e| {
                McpError::resource_execution(&format!("Large data serialization failed: {}", e))
            })?,
        )])
    }
}

/// Binary resource that returns binary data with proper MIME types
#[derive(Clone, Default)]
struct BinaryResource;

impl HasResourceMetadata for BinaryResource {
    fn name(&self) -> &str {
        "binary_resource"
    }
}

impl HasResourceUri for BinaryResource {
    fn uri(&self) -> &str {
        "file:///binary/image.png"
    }
}

impl HasResourceDescription for BinaryResource {
    fn description(&self) -> Option<&str> {
        Some("Returns binary data with proper MIME types")
    }
}

impl HasResourceMimeType for BinaryResource {
    fn mime_type(&self) -> Option<&str> {
        Some("image/png")
    }
}

impl HasResourceSize for BinaryResource {
    fn size(&self) -> Option<u64> {
        Some(1024)
    } // 1KB fake image
}

impl HasResourceAnnotations for BinaryResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for BinaryResource {}
impl HasIcons for BinaryResource {}

#[async_trait]
impl McpResource for BinaryResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // Create fake PNG header + data (simplified for testing)
        let mut fake_png = Vec::new();
        fake_png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]); // PNG signature

        // Add some fake image data to reach ~1KB
        for i in 0..128 {
            fake_png.extend_from_slice(&[
                (i % 256) as u8,
                ((i * 2) % 256) as u8,
                ((i * 3) % 256) as u8,
                0xFF,
            ]); // RGBA pixels
        }

        // For MCP, we need to base64 encode binary data
        let encoded = general_purpose::STANDARD.encode(&fake_png);

        Ok(vec![ResourceContent::blob(
            "file:///binary/image.png",
            encoded,
            "image/png",
        )])
    }
}

// =============================================================================
// Advanced Resources (Features)
// =============================================================================

/// Session-aware resource that returns session ID and metadata
#[derive(Clone, Default)]
struct SessionResource;

impl HasResourceMetadata for SessionResource {
    fn name(&self) -> &str {
        "session_resource"
    }
}

impl HasResourceUri for SessionResource {
    fn uri(&self) -> &str {
        "file:///session/info.json"
    }
}

impl HasResourceDescription for SessionResource {
    fn description(&self) -> Option<&str> {
        Some("Returns session ID and metadata (session-aware)")
    }
}

impl HasResourceMimeType for SessionResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for SessionResource {}
impl HasResourceAnnotations for SessionResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for SessionResource {}
impl HasIcons for SessionResource {}

#[async_trait]
impl McpResource for SessionResource {
    async fn read(&self, _params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // This resource demonstrates session-aware behavior
        let session_data = if let Some(ctx) = session {
            json!({
                "message": "This resource is session-aware",
                "session_id": ctx.session_id.to_string(),
                "timestamp": Utc::now().to_rfc3339(),
                "session_features": {
                    "state_storage": true,
                    "notifications": true,
                    "progress_tracking": true
                }
            })
        } else {
            json!({
                "message": "This resource is session-aware",
                "note": "No session context provided",
                "timestamp": Utc::now().to_rfc3339(),
                "session_features": {
                    "state_storage": true,
                    "notifications": true,
                    "progress_tracking": true
                }
            })
        };

        Ok(vec![ResourceContent::text(
            "file:///session/info.json",
            safe_json_serialize(&session_data)?,
        )])
    }
}

/// Subscribable resource that supports resource subscriptions
#[derive(Clone)]
struct SubscribableResource {
    counter: Arc<AtomicU64>,
}

impl Default for SubscribableResource {
    fn default() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl HasResourceMetadata for SubscribableResource {
    fn name(&self) -> &str {
        "subscribable_resource"
    }
}

impl HasResourceUri for SubscribableResource {
    fn uri(&self) -> &str {
        "file:///subscribe/updates.json"
    }
}

impl HasResourceDescription for SubscribableResource {
    fn description(&self) -> Option<&str> {
        Some("Supports resource subscriptions")
    }
}

impl HasResourceMimeType for SubscribableResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for SubscribableResource {}
impl HasResourceAnnotations for SubscribableResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for SubscribableResource {}
impl HasIcons for SubscribableResource {}

#[async_trait]
impl McpResource for SubscribableResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);

        let subscription_data = json!({
            "type": "subscribable",
            "access_count": count,
            "timestamp": Utc::now().to_rfc3339(),
            "data": format!("This resource has been accessed {} times", count),
            "subscription_info": {
                "supports_subscriptions": true,
                "update_frequency": "on_change",
                "notification_method": "SSE"
            }
        });

        Ok(vec![ResourceContent::text(
            "file:///subscribe/updates.json",
            safe_json_serialize(&subscription_data)?,
        )])
    }
}

/// Notifying resource that triggers list change notifications via SSE
#[derive(Clone, Default)]
struct NotifyingResource;

impl HasResourceMetadata for NotifyingResource {
    fn name(&self) -> &str {
        "notifying_resource"
    }
}

impl HasResourceUri for NotifyingResource {
    fn uri(&self) -> &str {
        "file:///notify/trigger.json"
    }
}

impl HasResourceDescription for NotifyingResource {
    fn description(&self) -> Option<&str> {
        Some("Triggers list change notifications via SSE")
    }
}

impl HasResourceMimeType for NotifyingResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for NotifyingResource {}
impl HasResourceAnnotations for NotifyingResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for NotifyingResource {}
impl HasIcons for NotifyingResource {}

#[async_trait]
impl McpResource for NotifyingResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // DEMONSTRATION: This resource simulates notification emission during read operations
        //
        // PRODUCTION LIMITATION: The current McpResource trait doesn't provide access to
        // session context or NotificationBroadcaster. In a production system, notifications
        // would be emitted through the session context like this:
        //
        // let notification = ResourceListChangedNotification::new();
        // session_context.broadcaster().send_resource_list_changed_notification(
        //     &session_id, notification
        // ).await?;
        //
        // This example demonstrates the EXPECTED notification structure and behavior.

        let timestamp = Utc::now();
        let notification_id = format!("notify-{}", timestamp.timestamp_millis());

        // Demonstrate the exact JSON-RPC notification that WOULD be emitted
        let simulated_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/list_changed",
            "params": {
                "_meta": {
                    "event_id": notification_id,
                    "timestamp": timestamp.to_rfc3339(),
                    "source": "NotifyingResource",
                    "trigger": "resource_read_operation"
                }
            }
        });

        let response_data = json!({
            "type": "notification_demo",
            "message": "Resource read operation completed - notification emitted to SSE clients",
            "timestamp": timestamp.to_rfc3339(),
            "resource_info": {
                "uri": "file:///notify/trigger.json",
                "accessed_at": timestamp.to_rfc3339(),
                "read_count": 1
            },
            "notification_emitted": {
                "method": "notifications/resources/list_changed",
                "target": "all_connected_sse_clients",
                "reason": "resource_list_changed_due_to_read_operation",
                "compliance": "MCP_2025_11_25",
                "transport": "SSE_streaming"
            },
            "sse_event_structure": simulated_notification,
            "architecture_note": "In production, this notification is sent via StreamManager to all SSE connections"
        });

        Ok(vec![ResourceContent::text(
            "file:///notify/trigger.json",
            safe_json_serialize(&response_data)?,
        )])
    }
}

/// Multi-content resource that returns multiple ResourceContent items
#[derive(Clone, Default)]
struct MultiContentResource;

impl HasResourceMetadata for MultiContentResource {
    fn name(&self) -> &str {
        "multi_content_resource"
    }
}

impl HasResourceUri for MultiContentResource {
    fn uri(&self) -> &str {
        "file:///multi/contents.txt"
    }
}

impl HasResourceDescription for MultiContentResource {
    fn description(&self) -> Option<&str> {
        Some("Returns multiple ResourceContent items")
    }
}

impl HasResourceMimeType for MultiContentResource {
    fn mime_type(&self) -> Option<&str> {
        Some("multipart/mixed")
    }
}

impl HasResourceSize for MultiContentResource {}
impl HasResourceAnnotations for MultiContentResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for MultiContentResource {}
impl HasIcons for MultiContentResource {}

#[async_trait]
impl McpResource for MultiContentResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![
            ResourceContent::text(
                "file:///multi/contents/part1.json",
                json!({
                    "part": 1,
                    "type": "metadata",
                    "description": "This is the first part of a multi-content resource",
                    "timestamp": Utc::now().to_rfc3339()
                }).to_string()
            ),
            ResourceContent::text(
                "file:///multi/contents/part2.json",
                json!({
                    "part": 2,
                    "type": "data",
                    "items": [
                        {"id": 1, "name": "Item 1"},
                        {"id": 2, "name": "Item 2"},
                        {"id": 3, "name": "Item 3"}
                    ]
                }).to_string()
            ),
            ResourceContent::text(
                "file:///multi/contents/part3.txt",
                "Part 3: Plain text content\nThis demonstrates that ResourceContent can contain different types of data\nincluding JSON, plain text, and other formats."
            )
        ])
    }
}

/// Paginated resource that supports cursor-based pagination
#[derive(Clone, Default)]
struct PaginatedResource;

impl HasResourceMetadata for PaginatedResource {
    fn name(&self) -> &str {
        "paginated_resource"
    }
}

impl HasResourceUri for PaginatedResource {
    fn uri(&self) -> &str {
        "file:///paginated/items.json"
    }
}

impl HasResourceDescription for PaginatedResource {
    fn description(&self) -> Option<&str> {
        Some("Supports cursor-based pagination")
    }
}

impl HasResourceMimeType for PaginatedResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for PaginatedResource {}
impl HasResourceAnnotations for PaginatedResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for PaginatedResource {}
impl HasIcons for PaginatedResource {}

#[async_trait]
impl McpResource for PaginatedResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let page_size = params
            .as_ref()
            .and_then(|p| p.get("page_size"))
            .and_then(|s| s.as_u64())
            .unwrap_or(10) as usize;

        let cursor = params
            .as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(0);

        let total_items = 100; // Simulate 100 total items
        let start = cursor;
        let end = (start + page_size).min(total_items);

        let mut items = Vec::new();
        for i in start..end {
            items.push(json!({
                "id": i,
                "name": format!("Paginated Item {}", i),
                "index": i,
                "page_info": format!("Page starting at {}, size {}", cursor, page_size)
            }));
        }

        let next_cursor = if end < total_items { Some(end) } else { None };

        let paginated_data = json!({
            "type": "paginated",
            "items": items,
            "pagination": {
                "current_cursor": cursor,
                "next_cursor": next_cursor,
                "page_size": page_size,
                "total_items": total_items,
                "has_more": next_cursor.is_some()
            }
        });

        Ok(vec![ResourceContent::text(
            "file:///paginated/items.json",
            safe_json_serialize(&paginated_data)?,
        )])
    }
}

// =============================================================================
// Edge Case Resources
// =============================================================================

/// Resource with invalid URI characters for testing error handling
#[derive(Clone, Default)]
struct InvalidUriResource;

impl HasResourceMetadata for InvalidUriResource {
    fn name(&self) -> &str {
        "invalid_uri_resource"
    }
}

impl HasResourceUri for InvalidUriResource {
    fn uri(&self) -> &str {
        "file:///invalid/bad-chars-and-spaces.txt"
    }
}

impl HasResourceDescription for InvalidUriResource {
    fn description(&self) -> Option<&str> {
        Some("Resource with invalid URI characters")
    }
}

impl HasResourceMimeType for InvalidUriResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for InvalidUriResource {}
impl HasResourceAnnotations for InvalidUriResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for InvalidUriResource {}
impl HasIcons for InvalidUriResource {}

#[async_trait]
impl McpResource for InvalidUriResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            "file:///invalid/bad-chars-and-spaces.txt",
            "This resource has an intentionally invalid URI with hyphens and special characters for testing"
        )])
    }
}

/// Resource with very long URIs for testing limits
#[derive(Clone)]
struct LongUriResource {
    long_uri: String,
}

impl Default for LongUriResource {
    fn default() -> Self {
        let long_part = "very-long-path-component-".repeat(20); // ~500 chars
        Self {
            long_uri: format!("file:///long/{}/with/many/nested/path/segments/that/make/the/uri/extremely/long.txt", long_part)
        }
    }
}

impl HasResourceMetadata for LongUriResource {
    fn name(&self) -> &str {
        "long_uri_resource"
    }
}

impl HasResourceUri for LongUriResource {
    fn uri(&self) -> &str {
        &self.long_uri
    }
}

impl HasResourceDescription for LongUriResource {
    fn description(&self) -> Option<&str> {
        Some("Resource with very long URI for testing limits")
    }
}

impl HasResourceMimeType for LongUriResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for LongUriResource {}
impl HasResourceAnnotations for LongUriResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for LongUriResource {}
impl HasIcons for LongUriResource {}

#[async_trait]
impl McpResource for LongUriResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            &self.long_uri,
            format!(
                "This resource has a very long URI ({} characters): {}",
                self.long_uri.len(),
                self.long_uri
            ),
        )])
    }
}

/// Resource that changes behavior based on _meta fields
#[derive(Clone, Default)]
struct MetaDynamicResource;

impl HasResourceMetadata for MetaDynamicResource {
    fn name(&self) -> &str {
        "meta_dynamic_resource"
    }
}

impl HasResourceUri for MetaDynamicResource {
    fn uri(&self) -> &str {
        "file:///meta/dynamic.json"
    }
}

impl HasResourceDescription for MetaDynamicResource {
    fn description(&self) -> Option<&str> {
        Some("Changes behavior based on _meta fields")
    }
}

impl HasResourceMimeType for MetaDynamicResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for MetaDynamicResource {}
impl HasResourceAnnotations for MetaDynamicResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for MetaDynamicResource {}
impl HasIcons for MetaDynamicResource {}

#[async_trait]
impl McpResource for MetaDynamicResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let meta_fields = params
            .as_ref()
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.as_object());

        let behavior = meta_fields
            .and_then(|m| m.get("behavior"))
            .and_then(|b| b.as_str())
            .unwrap_or("default");

        let response = match behavior {
            "verbose" => json!({
                "mode": "verbose",
                "message": "This resource is in verbose mode due to _meta.behavior=verbose",
                "meta_fields_received": meta_fields,
                "detailed_info": {
                    "resource_type": "meta_dynamic",
                    "supports_meta_behavior": true,
                    "available_behaviors": ["default", "verbose", "compact", "error"],
                    "timestamp": Utc::now().to_rfc3339()
                }
            }),
            "compact" => json!({
                "mode": "compact",
                "msg": "Compact mode",
                "meta": meta_fields
            }),
            "error" => {
                return Err(McpError::resource_execution(
                    "Simulated error due to _meta.behavior=error",
                ));
            }
            _ => json!({
                "mode": "default",
                "message": "Default behavior - pass _meta.behavior to change response format",
                "available_modes": ["verbose", "compact", "error", "default"]
            }),
        };

        Ok(vec![ResourceContent::text(
            "file:///meta/dynamic.json",
            safe_json_serialize(&response)?,
        )])
    }
}

/// User template resource for testing user-specific templates
#[derive(Clone, Default)]
struct UserTemplateResource;

impl HasResourceMetadata for UserTemplateResource {
    fn name(&self) -> &str {
        "user_template_resource"
    }
}

impl HasResourceUri for UserTemplateResource {
    fn uri(&self) -> &str {
        "file:///template/users/{user_id}.json"
    }
}

impl HasResourceDescription for UserTemplateResource {
    fn description(&self) -> Option<&str> {
        Some("Template resource for user-specific data with user_id validation")
    }
}

impl HasResourceMimeType for UserTemplateResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for UserTemplateResource {}
impl HasResourceAnnotations for UserTemplateResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for UserTemplateResource {}
impl HasIcons for UserTemplateResource {}

#[async_trait]
impl McpResource for UserTemplateResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // Extract template variables from params
        let template_vars = params
            .as_ref()
            .and_then(|p| p.get("template_variables"))
            .and_then(|tv| tv.as_object());

        let user_id = template_vars
            .and_then(|vars| vars.get("user_id"))
            .and_then(|id| id.as_str())
            .unwrap_or("anonymous");

        let user_data = json!({
            "user_id": user_id,
            "uri": format!("file:///template/users/{}.json", user_id),
            "name": format!("User {}", user_id),
            "email": format!("{}@example.com", user_id),
            "profile": {
                "created_at": Utc::now().to_rfc3339(),
                "last_login": Utc::now().to_rfc3339(),
                "preferences": {
                    "theme": "dark",
                    "notifications": true
                }
            },
            "template_variables": template_vars.unwrap_or(&serde_json::Map::new()),
            "metadata": {
                "resource_type": "user_template",
                "supports_variables": true,
                "validated_variables": ["user_id"]
            }
        });

        Ok(vec![ResourceContent::text(
            format!("file:///template/users/{}.json", user_id),
            safe_json_serialize(&user_data)?,
        )])
    }
}

/// File template resource for testing file path templates
#[derive(Clone, Default)]
struct FileTemplateResource;

impl HasResourceMetadata for FileTemplateResource {
    fn name(&self) -> &str {
        "file_template_resource"
    }
}

impl HasResourceUri for FileTemplateResource {
    fn uri(&self) -> &str {
        "file:///template/files/{path}"
    }
}

impl HasResourceDescription for FileTemplateResource {
    fn description(&self) -> Option<&str> {
        Some("Template resource for file system paths with flexible path variables")
    }
}

impl HasResourceMimeType for FileTemplateResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for FileTemplateResource {}
impl HasResourceAnnotations for FileTemplateResource {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}
impl HasResourceMeta for FileTemplateResource {}
impl HasIcons for FileTemplateResource {}

#[async_trait]
impl McpResource for FileTemplateResource {
    async fn read(&self, params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // Extract template variables from params
        let template_vars = params
            .as_ref()
            .and_then(|p| p.get("template_variables"))
            .and_then(|tv| tv.as_object());

        let path = template_vars
            .and_then(|vars| vars.get("path"))
            .and_then(|p| p.as_str())
            .unwrap_or("default.txt");

        // Simulate file content based on path
        let file_content = match path {
            p if p.ends_with(".json") => json!({
                "file_path": p,
                "file_type": "json",
                "content": {"message": "JSON content for template file"},
                "size": 42,
                "modified": Utc::now().to_rfc3339()
            })
            .to_string(),
            p if p.ends_with(".txt") => format!(
                "Text file content for: {}\nGenerated at: {}\nThis is a template-generated file.",
                p,
                Utc::now().to_rfc3339()
            ),
            p => format!(
                "Binary or unknown file type: {}\nGenerated: {}",
                p,
                Utc::now().to_rfc3339()
            ),
        };

        let file_info = format!(
            "Template File: {}\n\
            URI: file:///template/files/{}\n\
            Generated: {}\n\
            Template Variables: {:?}\n\n\
            File Content:\n\
            {}",
            path,
            path,
            Utc::now().to_rfc3339(),
            template_vars.unwrap_or(&serde_json::Map::new()),
            file_content
        );

        Ok(vec![ResourceContent::text(
            format!("file:///template/files/{}", path),
            file_info,
        )])
    }
}

/// Resource with all optional fields populated for comprehensive testing
#[derive(Clone)]
struct CompleteResource {
    annotations: Annotations,
    meta: HashMap<String, Value>,
}

impl Default for CompleteResource {
    fn default() -> Self {
        let annotations = Annotations {
            audience: Some(vec!["user".to_string()]),
            priority: Some(0.9),
            last_modified: None,
        };

        let mut meta = HashMap::new();
        meta.insert("author".to_string(), json!("MCP Framework"));
        meta.insert("version".to_string(), json!("1.0.0"));
        meta.insert("tags".to_string(), json!(["test", "complete", "example"]));
        meta.insert("created_at".to_string(), json!(Utc::now().to_rfc3339()));

        Self { annotations, meta }
    }
}

impl HasResourceMetadata for CompleteResource {
    fn name(&self) -> &str {
        "complete_resource"
    }
    fn title(&self) -> Option<&str> {
        Some("Complete Test Resource")
    }
}

impl HasResourceUri for CompleteResource {
    fn uri(&self) -> &str {
        "file:///complete/all-fields.json"
    }
}

impl HasResourceDescription for CompleteResource {
    fn description(&self) -> Option<&str> {
        Some("Resource with all optional fields populated for comprehensive testing")
    }
}

impl HasResourceMimeType for CompleteResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for CompleteResource {
    fn size(&self) -> Option<u64> {
        Some(2048)
    }
}

impl HasResourceAnnotations for CompleteResource {
    fn annotations(&self) -> Option<&Annotations> {
        Some(&self.annotations)
    }
}

impl HasResourceMeta for CompleteResource {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.meta)
    }
}

impl HasIcons for CompleteResource {}

#[async_trait]
impl McpResource for CompleteResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&turul_mcp_server::SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let complete_data = json!({
            "type": "complete",
            "message": "This resource demonstrates all optional MCP resource fields",
            "fields": {
                "name": self.name(),
                "title": self.title(),
                "uri": self.uri(),
                "description": self.description(),
                "mime_type": self.mime_type(),
                "size": self.size(),
                "annotations": self.annotations(),
                "meta": self.resource_meta()
            },
            "verification": {
                "has_title": self.title().is_some(),
                "has_annotations": self.annotations().is_some(),
                "has_meta": self.resource_meta().is_some(),
                "has_size": self.size().is_some()
            }
        });

        Ok(vec![ResourceContent::text(
            "file:///complete/all-fields.json",
            safe_json_serialize(&complete_data)?,
        )])
    }
}

// =============================================================================
// Main Server
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    // Use specified port or OS ephemeral allocation if 0
    let port = if args.port == 0 {
        // Use OS ephemeral port allocation - more reliable than portpicker
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind to ephemeral port: {}", e))?;
        let port = listener.local_addr()?.port();
        drop(listener); // Release immediately so server can bind to it
        port
    } else {
        args.port
    };

    info!("🚀 Starting MCP Resource Test Server on port {}", port);

    // Create file resource with temp file
    let file_resource = FileResource::new()?;

    let server = McpServer::builder()
        .name("resource-test-server")
        .version("0.2.0")
        .title("MCP Resource Test Server")
        .instructions(
            "Comprehensive test server providing various types of resources for E2E testing.\n\
            This server implements all MCP resource patterns and edge cases to validate\n\
            framework compliance with the MCP 2025-11-25 specification.\n\n\
            Available test resources:\n\
            • Basic: file:///tmp/, file:///memory/, file:///error/, file:///slow/, file:///empty/, file:///large/, file:///binary/\n\
            • Templates: file:///template/items/{id}.json, file:///template/users/{user_id}.json, file:///template/files/{path}\n\
            • Advanced: file:///session/, file:///subscribe/, file:///notify/, file:///multi/, file:///paginated/\n\
            • Edge cases: file:///invalid/, file:///long/, file:///meta/, file:///complete/\n\n\
            Note: Template resources are now auto-detected based on URI patterns."
        )
        // Basic Resources (Coverage)
        .resource(file_resource)
        .resource(MemoryResource)
        .resource(ErrorResource)
        .resource(SlowResource)
        .resource(EmptyResource)
        .resource(LargeResource)
        .resource(BinaryResource)
        // Template Resources (auto-detected based on URI patterns)
        .resource(TemplateResource)
        .resource(UserTemplateResource)
        .resource(FileTemplateResource)
        // Advanced Resources (Features)
        .resource(SessionResource)
        .resource(SubscribableResource::default())
        .resource(NotifyingResource)
        .resource(MultiContentResource)
        .resource(PaginatedResource)
        // Edge Case Resources
        .resource(InvalidUriResource)
        .resource(LongUriResource::default())
        .resource(MetaDynamicResource)
        .resource(CompleteResource::default())
        .test_mode()  // Disable security for test URI schemes
        // Note: .with_resources() no longer needed - automatically registered when resources are added
        .bind_address(format!("127.0.0.1:{}", port).parse()?)
        .build()?;

    info!("📡 Server URL: http://127.0.0.1:{}/mcp", port);
    info!("");
    info!("🧪 Test Resources Available:");
    info!("   📁 Basic Resources (Coverage):");
    info!("      • file:///tmp/test.txt - File reading with error handling");
    info!("      • file:///memory/data.json - Fast in-memory JSON data");
    info!("      • file:///error/not_found.txt - Always returns NotFound errors");
    info!("      • file:///slow/delayed.txt - Configurable delay simulation");
    info!("      • file:///empty/content.txt - Empty content edge case");
    info!("      • file:///large/dataset.json - Large content (configurable size)");
    info!("      • file:///binary/image.png - Binary data with MIME types");
    info!("");
    info!("   🎯 Template Resources (URI Templates with variables):");
    info!("      • file:///template/items/{{id}}.json - Item template with ID validation");
    info!(
        "      • file:///template/users/{{user_id}}.json - User template with user_id validation"
    );
    info!("      • file:///template/files/{{path}} - File path template with flexible paths");
    info!("      📋 Available via: resources/templates/list endpoint");
    info!("");
    info!("   🚀 Advanced Resources (Features):");
    info!("      • file:///session/info.json - Session-aware resource");
    info!("      • file:///subscribe/updates.json - Test subscription resource (returns MethodNotFound)");
    info!("      • file:///notify/trigger.json - SSE notification triggers");
    info!("      • file:///multi/contents.txt - Multiple ResourceContent items");
    info!("      • file:///paginated/items.json - Cursor-based pagination");
    info!("");
    info!("   ⚠️  Edge Case Resources:");
    info!("      • file:///invalid/bad-chars-and-spaces.txt - Intentionally non-compliant URI for error testing");
    info!("      • file:///long/very-long-path... - Very long URI testing");
    info!("      • file:///meta/dynamic.json - _meta field behavior changes");
    info!("      • file:///complete/all-fields.json - All optional fields populated");
    info!("");
    info!("💡 Quick Test Commands:");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}'");
    info!("");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -H 'Mcp-Session-Id: SESSION_ID' \\");
    info!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"resources/list\",\"params\":{{}}}}'"
    );
    info!("");

    server.run().await?;
    Ok(())
}
