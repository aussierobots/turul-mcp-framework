//! MCP 2025-11-25 Icon Support Showcase
//!
//! Demonstrates the `icons` field available on Tool, Resource, Prompt,
//! ResourceTemplate, and Implementation types. Icons use the `Icon` struct
//! with `src`, `mimeType`, `sizes`, and `theme` fields.

use std::collections::HashMap;
use turul_mcp_protocol::{
    Icon, Implementation, Prompt, PromptArgument, Resource, Tool, ToolSchema,
    resources::ResourceTemplate,
    schema::JsonSchema,
};

fn main() {
    println!("=== MCP 2025-11-25 Icon Support Showcase ===\n");

    // --- Tool with an HTTPS icon ---
    let calculator_tool = Tool::new(
        "calculator",
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("a".to_string(), JsonSchema::number()),
                ("b".to_string(), JsonSchema::number()),
            ]))
            .with_required(vec!["a".to_string(), "b".to_string()]),
    )
    .with_title("Calculator")
    .with_description("Perform arithmetic calculations")
    .with_icons(vec![Icon::new("https://example.com/icons/calculator.png")]);

    println!("1. Tool with HTTPS icon:");
    println!("{}\n", serde_json::to_string_pretty(&calculator_tool).unwrap());

    // --- Tool with a data: URI icon (base64-encoded SVG) ---
    // This is a minimal SVG circle encoded as base64
    let svg_data = "PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCI+PGNpcmNsZSBjeD0iMTIiIGN5PSIxMiIgcj0iMTAiIGZpbGw9ImJsdWUiLz48L3N2Zz4=";
    let search_tool = Tool::new(
        "search",
        ToolSchema::object()
            .with_properties(HashMap::from([(
                "query".to_string(),
                JsonSchema::string(),
            )]))
            .with_required(vec!["query".to_string()]),
    )
    .with_title("Search")
    .with_description("Search the knowledge base")
    .with_icons(vec![Icon::data_uri("image/svg+xml", svg_data)]);

    println!("2. Tool with data: URI icon (SVG):");
    println!("{}\n", serde_json::to_string_pretty(&search_tool).unwrap());

    // --- Resource with icon ---
    let config_resource = Resource::new("file:///config/app.json", "app_config")
        .with_title("Application Configuration")
        .with_description("Main application configuration file")
        .with_mime_type("application/json")
        .with_icons(vec![Icon::new("https://example.com/icons/config.png")]);

    println!("3. Resource with icon:");
    println!("{}\n", serde_json::to_string_pretty(&config_resource).unwrap());

    // --- ResourceTemplate with icon ---
    let log_template = ResourceTemplate::new("logs", "file:///logs/{date}.log")
        .with_title("Log Files")
        .with_description("Daily log files by date")
        .with_mime_type("text/plain")
        .with_icons(vec![Icon::new("https://example.com/icons/logs.png")]);

    println!("4. ResourceTemplate with icon:");
    println!("{}\n", serde_json::to_string_pretty(&log_template).unwrap());

    // --- Prompt with icon ---
    let code_review_prompt = Prompt::new("code_review")
        .with_title("Code Review")
        .with_description("Generate a code review for a given file")
        .with_arguments(vec![
            PromptArgument {
                name: "file_path".to_string(),
                title: None,
                description: Some("Path to the file to review".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "focus".to_string(),
                title: Some("Review Focus".to_string()),
                description: Some("What to focus on: security, performance, style".to_string()),
                required: Some(false),
            },
        ])
        .with_icons(vec![Icon::new("https://example.com/icons/review.png")]);

    println!("5. Prompt with icon:");
    println!("{}\n", serde_json::to_string_pretty(&code_review_prompt).unwrap());

    // --- Implementation (server info) with icon ---
    let server_impl = Implementation::new("my-mcp-server", "1.0.0")
        .with_title("My MCP Server")
        .with_icons(vec![Icon::new("https://example.com/icons/server.png")]);

    println!("6. Implementation (server identity) with icon:");
    println!("{}\n", serde_json::to_string_pretty(&server_impl).unwrap());

    // --- Icon with full metadata ---
    let detailed_icon = Icon::new("https://example.com/icons/app.png")
        .with_mime_type("image/png")
        .with_sizes(vec!["32x32".to_string(), "64x64".to_string()])
        .with_theme(turul_mcp_protocol::IconTheme::Dark);

    println!("7. Icon with full metadata (mimeType, sizes, theme):");
    println!("{}\n", serde_json::to_string_pretty(&detailed_icon).unwrap());

    println!("=== All MCP types support optional icons via Icon struct ===");
    println!("Icons can be:");
    println!("  - HTTPS URLs: Icon::new(\"https://example.com/icon.png\")");
    println!("  - Data URIs:  Icon::data_uri(\"image/png\", \"<base64-data>\")");
    println!("  - With metadata: Icon::new(...).with_mime_type(...).with_sizes(...)");
}
