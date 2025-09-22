//! MCP Prompts Example
//!
//! This example demonstrates MCP Prompts specification compliance
//! using derive macros and proper prompt implementation patterns.

use mcp_prompts_tests::{
    AnalyzeErrorPrompt, GenerateDocsPrompt, MultiContentPrompt, PlanProjectPrompt,
    ReviewCodePrompt, TemplateVarPrompt,
};
use tracing::info;
use turul_mcp_server::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("💬 MCP Prompts Specification Example");

    // Create prompt instances demonstrating different patterns
    let review_prompt = ReviewCodePrompt::new("rust", "fn hello() { println!(\"Hello MCP!\"); }")
        .with_focus("security")
        .with_target_level("senior");
    let docs_prompt = GenerateDocsPrompt::new(
        "api",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        "markdown",
    );
    let error_prompt = AnalyzeErrorPrompt::new("NullPointerException at line 42", "java");
    let plan_prompt = PlanProjectPrompt::new("Build MCP Server", "rust", "detailed");

    info!("📋 Testing Prompt Definition Traits:");
    info!(
        "  - Review Prompt: {} ({})",
        review_prompt.name(),
        review_prompt.description().unwrap_or("No description")
    );
    info!(
        "  - Docs Prompt: {} ({})",
        docs_prompt.name(),
        docs_prompt.description().unwrap_or("No description")
    );
    info!(
        "  - Error Prompt: {} ({})",
        error_prompt.name(),
        error_prompt.description().unwrap_or("No description")
    );
    info!(
        "  - Plan Prompt: {} ({})",
        plan_prompt.name(),
        plan_prompt.description().unwrap_or("No description")
    );

    // Test prompt argument schemas
    info!("📝 Testing Prompt Arguments:");
    info!(
        "  - Review prompt args: {:?}",
        review_prompt.arguments().map(|args| args.len())
    );
    info!(
        "  - Docs prompt args: {:?}",
        docs_prompt.arguments().map(|args| args.len())
    );
    info!(
        "  - Error prompt args: {:?}",
        error_prompt.arguments().map(|args| args.len())
    );
    info!(
        "  - Plan prompt args: {:?}",
        plan_prompt.arguments().map(|args| args.len())
    );

    // Test prompt message rendering
    info!("🎨 Testing Prompt Message Rendering:");

    let review_messages = review_prompt.render(Some(HashMap::new())).await?;
    info!(
        "✅ Review prompt rendered: {} messages",
        review_messages.len()
    );

    let docs_messages = docs_prompt.render(Some(HashMap::new())).await?;
    info!("✅ Docs prompt rendered: {} messages", docs_messages.len());

    let error_messages = error_prompt.render(Some(HashMap::new())).await?;
    info!(
        "✅ Error prompt rendered: {} messages",
        error_messages.len()
    );

    let plan_messages = plan_prompt.render(Some(HashMap::new())).await?;
    info!("✅ Plan prompt rendered: {} messages", plan_messages.len());

    // Test custom argument handling
    info!("🔧 Testing Custom Arguments:");
    let mut custom_args = HashMap::new();
    custom_args.insert("language".to_string(), json!("python"));
    custom_args.insert(
        "code".to_string(),
        json!("def hello(): print('Hello MCP!')"),
    );
    custom_args.insert("focus_area".to_string(), json!("performance"));

    let custom_messages = review_prompt.render(Some(custom_args)).await?;
    info!(
        "✅ Review prompt with custom args: {} messages",
        custom_messages.len()
    );

    info!("🚀 Testing New ContentBlock Prompts:");

    // Test MultiContentPrompt with all ContentBlock variants
    let mut multi_prompt = MultiContentPrompt::new("financial");
    multi_prompt.include_chart = Some("true".to_string());

    let multi_messages = multi_prompt.generate_multi_content_messages().await?;
    info!(
        "✅ MultiContentPrompt: {} messages with all ContentBlock types",
        multi_messages.len()
    );

    // Demonstrate ContentBlock types
    for (i, message) in multi_messages.iter().enumerate() {
        use turul_mcp_protocol::prompts::ContentBlock;
        let content_type = match &message.content {
            ContentBlock::Text { .. } => "Text",
            ContentBlock::Image { .. } => "Image",
            ContentBlock::ResourceLink { .. } => "ResourceLink",
            ContentBlock::Resource { .. } => "Resource (embedded)",
        };
        info!(
            "   Message {}: {} content - Role: {:?}",
            i + 1,
            content_type,
            message.role
        );
    }

    // Test TemplateVarPrompt with variable substitution
    let template_prompt = TemplateVarPrompt::new("Alice", "data_analysis");

    let template_messages = template_prompt.generate_template_messages().await?;
    info!(
        "✅ TemplateVarPrompt: {} messages with variable substitution",
        template_messages.len()
    );

    // Test with argument override
    let mut template_args = HashMap::new();
    template_args.insert("user_name".to_string(), json!("Bob"));
    template_args.insert("task_type".to_string(), json!("code_review"));
    template_args.insert("priority".to_string(), json!("high"));

    let override_messages = template_prompt.render(Some(template_args)).await?;
    info!(
        "✅ TemplateVarPrompt with args override: {} messages",
        override_messages.len()
    );

    info!("📊 Demonstrating ContentBlock Types:");

    // Show Text ContentBlock
    if let turul_mcp_protocol::prompts::ContentBlock::Text { text } = &multi_messages[0].content {
        info!("📄 Text ContentBlock example:");
        info!("   Text length: {} characters", text.len());
        info!("   Sample: {:?}", &text[..text.len().min(50)]);
    }

    // Show ResourceLink ContentBlock
    if multi_messages.len() > 1 {
        if let turul_mcp_protocol::prompts::ContentBlock::ResourceLink { resource } =
            &multi_messages[1].content
        {
            info!("🔗 ResourceLink ContentBlock example:");
            info!("   URI: {}", resource.uri);
            info!("   Name: {}", resource.name);
            info!("   MIME Type: {:?}", resource.mime_type);
        }
    }

    // Show Image ContentBlock (if chart is included)
    if multi_messages.len() > 2 {
        if let turul_mcp_protocol::prompts::ContentBlock::Image { data, mime_type } =
            &multi_messages[2].content
        {
            info!("🖼️ Image ContentBlock example:");
            info!("   MIME Type: {}", mime_type);
            info!("   Base64 length: {} characters", data.len());
        }
    }

    // Show embedded Resource ContentBlock
    if let Some(resource_msg) = multi_messages.iter().find(|msg| {
        matches!(
            msg.content,
            turul_mcp_protocol::prompts::ContentBlock::Resource { .. }
        )
    }) {
        if let turul_mcp_protocol::prompts::ContentBlock::Resource {
            resource,
            annotations,
            meta,
        } = &resource_msg.content
        {
            info!("📦 Embedded Resource ContentBlock example:");
            match resource {
                turul_mcp_protocol::prompts::ResourceContents::Text { uri, text, .. } => {
                    info!("   Resource URI: {}", uri);
                    info!("   Text length: {} characters", text.len());
                }
                turul_mcp_protocol::prompts::ResourceContents::Blob { uri, blob, .. } => {
                    info!("   Resource URI: {}", uri);
                    info!("   Blob length: {} characters", blob.len());
                }
            }
            info!("   Has annotations: {}", annotations.is_some());
            info!("   Has meta: {}", meta.is_some());
        }
    }

    // Build MCP server with all prompts
    let _server = McpServer::builder()
        .name("MCP Prompts Test Server")
        .version("1.0.0")
        .prompt(review_prompt)
        .prompt(docs_prompt)
        .prompt(error_prompt)
        .prompt(plan_prompt)
        .prompt(multi_prompt)
        .prompt(template_prompt)
        .build()?;

    info!("🎉 MCP Prompts Example completed successfully!");
    info!("✨ All prompt patterns working: Derive Macros ✅ | Message Rendering ✅ | ContentBlock Types ✅ | Variable Substitution ✅");

    Ok(())
}
