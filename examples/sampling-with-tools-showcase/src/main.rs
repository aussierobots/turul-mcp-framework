//! MCP 2025-11-25 Sampling with Tools Showcase
//!
//! Demonstrates the new `tools` field on CreateMessageParams, which allows
//! servers to suggest tools the LLM can use during a sampling request.
//! This enables agentic workflows where the sampled LLM can call tools.

use std::collections::HashMap;
use turul_mcp_protocol::{
    Icon, Tool, ToolSchema,
    sampling::{CreateMessageParams, CreateMessageRequest, ModelPreferences, SamplingMessage},
    schema::JsonSchema,
};

fn main() {
    println!("=== MCP 2025-11-25 Sampling with Tools Showcase ===\n");

    // --- Define tools that the LLM can use during sampling ---

    let web_search = Tool::new(
        "web_search",
        ToolSchema::object()
            .with_properties(HashMap::from([("query".to_string(), JsonSchema::string())]))
            .with_required(vec!["query".to_string()]),
    )
    .with_title("Web Search")
    .with_description("Search the web for current information")
    .with_icons(vec![Icon::new("https://example.com/icons/search.png")]);

    let calculator = Tool::new(
        "calculator",
        ToolSchema::object()
            .with_properties(HashMap::from([(
                "expression".to_string(),
                JsonSchema::string(),
            )]))
            .with_required(vec!["expression".to_string()]),
    )
    .with_title("Calculator")
    .with_description("Evaluate a mathematical expression");

    let file_read = Tool::new(
        "read_file",
        ToolSchema::object()
            .with_properties(HashMap::from([("path".to_string(), JsonSchema::string())]))
            .with_required(vec!["path".to_string()]),
    )
    .with_description("Read the contents of a file");

    // --- CreateMessageParams without tools (traditional sampling) ---
    let basic_params = CreateMessageParams::new(
        vec![SamplingMessage::user_text("What is the capital of France?")],
        1024,
    )
    .with_system_prompt("You are a helpful assistant.");

    println!("1. Basic sampling request (no tools):");
    println!("{}\n", serde_json::to_string_pretty(&basic_params).unwrap());

    // --- CreateMessageParams WITH tools (new in 2025-11-25) ---
    let agentic_params = CreateMessageParams::new(
        vec![SamplingMessage::user_text(
            "Research the current weather in Tokyo and calculate the temperature in Fahrenheit.",
        )],
        2048,
    )
    .with_system_prompt("You are a research assistant. Use the provided tools to find information.")
    .with_tools(vec![web_search.clone(), calculator.clone()])
    .with_temperature(0.7);

    println!("2. Sampling request WITH tools (new in 2025-11-25):");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&agentic_params).unwrap()
    );

    // --- Full CreateMessageRequest with tools and model preferences ---
    let full_request = CreateMessageRequest::new(
        vec![SamplingMessage::user_text(
            "Read the config file and summarize its contents.",
        )],
        4096,
    )
    .with_tools(vec![file_read, web_search, calculator])
    .with_system_prompt(
        "You are a code analysis assistant with access to file reading and search tools.",
    )
    .with_model_preferences(
        ModelPreferences::new()
            .with_intelligence_priority(0.8)
            .with_speed_priority(0.5)
            .with_cost_priority(0.3),
    )
    .with_temperature(0.5);

    println!("3. Full CreateMessageRequest with tools and model preferences:");
    println!("{}\n", serde_json::to_string_pretty(&full_request).unwrap());

    // --- Show that tools field is omitted when None ---
    let no_tools_request =
        CreateMessageRequest::new(vec![SamplingMessage::user_text("Hello!")], 512);

    let json = serde_json::to_string_pretty(&no_tools_request).unwrap();
    let has_tools_field = json.contains("\"tools\"");
    println!(
        "4. Request without tools (tools field omitted: {}):",
        !has_tools_field
    );
    println!("{}\n", json);

    println!("=== The tools field enables agentic sampling workflows ===");
    println!("Servers can suggest tools for the LLM to use during sampling,");
    println!("enabling multi-step reasoning with tool calls.");
}
