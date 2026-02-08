//! Builders Showcase - Comprehensive demonstration of all 9 MCP builders
//!
//! This example showcases the power of the turul-mcp-builders crate by demonstrating
//! how to use all 9 runtime builders to create sophisticated MCP components.
//!
//! Since this is a standalone demonstration, it focuses on showing the builder
//! patterns and generated components rather than running a full server.

use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::logging::LoggingLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt().init();

    println!("🚀 MCP Builders Showcase - Demonstrating All 9 Runtime Builders");
    println!("═══════════════════════════════════════════════════════════════");

    // Demonstrate each builder type
    demonstrate_all_builders().await?;

    println!("\n✅ All 9 MCP builders successfully demonstrated!");
    println!("   This showcase proves the mcp-builders crate provides complete");
    println!("   runtime flexibility for building sophisticated MCP servers.");

    Ok(())
}

/// Demonstrates all 9 MCP builders with comprehensive examples
async fn demonstrate_all_builders() -> Result<(), Box<dyn std::error::Error>> {
    // ===========================================
    // 1. TOOL BUILDERS - Development Tools
    // ===========================================
    println!("\n🔧 1. ToolBuilder - Development Tools");
    println!("───────────────────────────────────────");

    // Create project tool
    let create_project_tool = ToolBuilder::new("create_project")
        .description("Create a new development project with scaffolding")
        .string_param("name", "Project name")
        .string_param("language", "Programming language")
        .boolean_param("git_init", "Initialize git repository")
        .execute(|args| async move {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            let language = args
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("rust");
            let git_init = args
                .get("git_init")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            Ok(json!({
                "project_created": true,
                "name": name,
                "language": language,
                "git_initialized": git_init,
                "location": format!("/workspace/{}", name)
            }))
        })
        .build()?;

    // Test the tool
    let result = create_project_tool
        .execute(json!({
            "name": "my-awesome-project",
            "language": "rust",
            "git_init": true
        }))
        .await?;

    println!("   ✅ Created project tool: {}", create_project_tool.name());
    println!(
        "   📋 Tool description: {}",
        create_project_tool
            .description()
            .unwrap_or("No description")
    );
    println!("   🧪 Test result: {}", result);

    // Build tool with async execution
    let build_tool = ToolBuilder::new("build_project")
        .description("Build project with progress simulation")
        .string_param("target", "Build target")
        .execute(|args| async move {
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("debug");

            // Simulate build progress
            println!("      🔨 Building {} target...", target);
            sleep(Duration::from_millis(100)).await;

            Ok(json!({
                "build_successful": true,
                "target": target,
                "duration_ms": 100,
                "artifacts": [format!("target/{}/myapp", target)]
            }))
        })
        .build()?;

    let build_result = build_tool.execute(json!({"target": "release"})).await?;
    println!("   ✅ Created build tool: {}", build_tool.name());
    println!("   🧪 Build result: {}", build_result);

    // ===========================================
    // 2. RESOURCE BUILDERS - Configuration & Templates
    // ===========================================
    println!("\n📄 2. ResourceBuilder - Configuration & Templates");
    println!("──────────────────────────────────────────────────");

    // Project configuration resource
    let project_config = ResourceBuilder::new("file:///workspace/config.json")
        .name("project_config")
        .description("Main project configuration file")
        .json_content(json!({
            "name": "demo-project",
            "version": "1.0.0",
            "build": {
                "target": "release",
                "features": ["cli", "server"]
            },
            "dependencies": {
                "serde": "1.0",
                "tokio": "1.0"
            }
        }))
        .build()?;

    // Read the resource
    let config_content = project_config.read().await?;
    println!("   ✅ Created config resource: file:///workspace/config.json");
    println!("   📋 Resource name: project_config");
    println!("   📄 Content available: true"); // Successfully read if we get here

    // Template resource with text content
    let template_resource = ResourceBuilder::new("file:///templates/component.rs")
        .name("rust_component_template")
        .description("Rust component template for code generation")
        .text_content(
            r#"
// {{component_name}} Component
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{component_name}} {
    id: String,
    name: String,
}

impl {{component_name}} {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
        }
    }
}
"#,
        )
        .build()?;

    let template_content = template_resource.read().await?;
    println!("   ✅ Created template resource: file:///templates/component.rs");
    println!(
        "   📝 Template content type: {}",
        match &template_content {
            turul_mcp_protocol::resources::ResourceContent::Text { .. } => "Text template",
            turul_mcp_protocol::resources::ResourceContent::Blob { .. } => "Binary template",
        }
    );

    // ===========================================
    // 3. PROMPT BUILDERS - Code Generation & AI Assistance
    // ===========================================
    println!("\n💬 3. PromptBuilder - Interactive AI Prompts");
    println!("─────────────────────────────────────────────");

    // Code generation prompt
    let code_gen_prompt = PromptBuilder::new("generate_code")
        .description("Generate code based on specifications")
        .string_argument("component_type", "Type of component to generate")
        .string_argument("language", "Programming language")
        .system_message("You are an expert software developer.")
        .user_message("Generate a {{component_type}} component in {{language}}.")
        .assistant_message("I'll create a high-quality {{component_type}} for you.")
        .build()?;

    // Test the prompt with arguments
    let mut prompt_args = HashMap::new();
    prompt_args.insert("component_type".to_string(), "database handler".to_string());
    prompt_args.insert("language".to_string(), "Rust".to_string());

    let prompt_result = code_gen_prompt.get(prompt_args).await?;
    println!(
        "   ✅ Created code generation prompt: {}",
        code_gen_prompt.name()
    );
    println!(
        "   📋 Prompt description: {}",
        code_gen_prompt.description().unwrap_or("No description")
    );
    println!(
        "   💬 Generated messages count: {}",
        prompt_result.messages.len()
    );

    // Analysis prompt
    let analysis_prompt = PromptBuilder::new("analyze_code")
        .description("Analyze code for improvements")
        .string_argument("code", "Code to analyze")
        .string_argument("focus", "Analysis focus area")
        .system_message("You are a senior software architect.")
        .user_message("Analyze this {{focus}} in the code: {{code}}")
        .build()?;

    println!("   ✅ Created analysis prompt: {}", analysis_prompt.name());
    println!(
        "   🔍 Argument count: {}",
        analysis_prompt.arguments().unwrap_or(&vec![]).len()
    );

    // ===========================================
    // 4. MESSAGE BUILDERS - AI Message Configuration
    // ===========================================
    println!("\n🤖 4. MessageBuilder - AI Message Configuration");
    println!("───────────────────────────────────────────────");

    // Code review message setup
    let code_review_message = MessageBuilder::new()
        .max_tokens(1500)
        .temperature(0.2) // Low for consistency
        // .top_p(0.9) // top_p not available in MessageBuilder
        .system_prompt("You are a senior software engineer conducting code reviews.")
        .user_text("Please review this code for quality and security.")
        .build_request();

    println!("   ✅ Created code review message configuration");
    println!(
        "   🎯 Max tokens: {}",
        code_review_message.params.max_tokens
    );
    println!(
        "   🌡️  Temperature: {}",
        code_review_message.params.temperature.unwrap_or(0.0)
    );
    println!(
        "   💬 Message count: {}",
        code_review_message.params.messages.len()
    );

    // Creative brainstorming message
    let brainstorm_message = MessageBuilder::new()
        .max_tokens(2000)
        .temperature(0.8) // Higher for creativity
        .with_model_preferences(|prefs| {
            prefs
                .cost_priority(0.3)
                .speed_priority(0.2)
                .intelligence_priority(0.9)
        })
        .system_prompt("You are a creative software architect.")
        .build_request();

    println!("   ✅ Created brainstorming message configuration");
    println!(
        "   🎨 Temperature: {}",
        brainstorm_message.params.temperature.unwrap_or(0.0)
    );
    // Model preferences don't have a len() method
    println!(
        "   🤖 Has model preferences: {}",
        brainstorm_message.params.model_preferences.is_some()
    );

    // ===========================================
    // 5. COMPLETION BUILDERS - Autocomplete Support
    // ===========================================
    println!("\n📝 5. CompletionBuilder - Autocomplete Support");
    println!("──────────────────────────────────────────────");

    // Prompt argument completion
    let prompt_completion = CompletionBuilder::prompt_argument("generate_code", "component_type")
        .context_argument("project_type", "web-service")
        .context_argument("language", "rust")
        .build();

    println!("   ✅ Created prompt argument completion");
    println!("   🎯 Reference: prompt 'generate_code', argument 'component_type'");
    // CompletionContext doesn't have len() method
    println!(
        "   🔧 Has context: {}",
        prompt_completion.params.context.is_some()
    );

    // Resource field completion
    let resource_completion = CompletionBuilder::for_resource("project_config")
        .argument_name("build.target")
        .context_argument("current_env", "development")
        .build();

    println!("   ✅ Created resource field completion");
    println!("   📄 Reference: resource 'project_config', field 'build.target'");

    // ===========================================
    // 6. ROOT BUILDERS - Directory Access Management
    // ===========================================
    println!("\n📁 6. RootBuilder - Directory Access Management");
    println!("───────────────────────────────────────────────");

    // Source code root (read-write)
    let source_root = RootBuilder::source_code_root("/workspace/src")
        .name("Project Source")
        .description("Main source code with full access")
        .read_write()
        .max_depth(8)
        .tag("source-code")
        .build()?;

    println!("   ✅ Created source root");
    println!("   📂 Name: {}", source_root.name().unwrap_or("N/A"));
    println!("   🔓 Write access: enabled");
    println!("   📊 Max depth: 8");

    // Configuration root (read-only)
    let config_root = RootBuilder::config_root("/etc/myapp")
        .name("App Configuration")
        .description("Configuration files (read-only for safety)")
        .tag("configuration")
        .build()?;

    println!("   ✅ Created config root");
    println!("   📂 Name: {}", config_root.name().unwrap_or("N/A"));
    println!("   🔒 Write access: disabled (read-only)");

    // Temporary workspace
    let temp_root = RootBuilder::workspace_root("/tmp/workspace")
        .name("Temp Workspace")
        .build()?;

    println!("   ✅ Created temp workspace");
    println!("   📂 Name: {}", temp_root.name().unwrap_or("N/A"));
    println!("   💾 Full access: enabled");

    // ===========================================
    // 7. ELICITATION BUILDERS - User Input Collection
    // ===========================================
    println!("\n📋 7. ElicitationBuilder - User Input Collection");
    println!("─────────────────────────────────────────────────");

    // Project setup form
    let project_form = ElicitationBuilder::new("Set up your new project!")
        .title("Project Setup Wizard")
        .string_field("project_name", "Project name")
        .enum_field(
            "language",
            "Programming language",
            vec![
                "rust".to_string(),
                "python".to_string(),
                "typescript".to_string(),
            ],
        )
        .boolean_field("git_init", "Initialize Git repository")
        .integer_field_with_range("team_size", "Team size", Some(1.0), Some(20.0))
        .require_fields(vec!["project_name".to_string(), "language".to_string()])
        .build();

    println!("   ✅ Created project setup form");
    println!("   📝 Title: Project Setup Wizard");
    println!("   📋 Has requested schema: true"); // Schema is always present in ElicitationBuilder
    // ElicitCreateParams uses requested_schema, not schema
    println!("   📝 Message: {}", project_form.params.message);

    // Preferences form
    let prefs_form = ElicitationBuilder::new("Configure your preferences")
        .string_field("editor", "Preferred code editor")
        .boolean_field("auto_format", "Enable auto-formatting")
        .enum_field(
            "theme",
            "Color theme",
            vec!["dark".to_string(), "light".to_string(), "auto".to_string()],
        )
        .build();

    println!("   ✅ Created preferences form");
    println!("   📄 Message: Configure your preferences");

    // ===========================================
    // 8. NOTIFICATION BUILDERS - Real-time Updates
    // ===========================================
    println!("\n📡 8. NotificationBuilder - Real-time Notifications");
    println!("────────────────────────────────────────────────");

    // Progress notification
    let progress_notification = NotificationBuilder::progress("build-task-123", 45.0)
        .total(100.0)
        .message("Compiling source code...")
        .meta_value("task_type", json!("build"))
        .meta_value("eta_seconds", json!(30))
        .build();

    println!("   ✅ Created progress notification");
    println!("   📊 Progress: 45/100");
    println!("   💬 Message: Compiling source code...");
    println!("   📋 Method: notifications/progress");

    // Resource updated notification
    let resource_notification = NotificationBuilder::resource_updated("file:///config.json")
        .meta_value("change_type", json!("modified"))
        .meta_value("timestamp", json!("2025-01-01T12:00:00Z"))
        .build();

    println!("   ✅ Created resource update notification");
    println!("   📄 Resource: file:///config.json");
    println!("   🔄 Change type: modified");

    // Zero-configuration logging message notification (framework determines method)
    let alert_notification = NotificationBuilder::logging_message(
        LoggingLevel::Warning,
        json!({
            "component": "build_system",
            "message": "High memory usage detected",
            "memory_usage_mb": 2048,
            "threshold_mb": 1536
        }),
    );

    println!("   ✅ Created logging message notification");
    println!("   🔧 Framework auto-determined method: notifications/message");
    println!("   ⚠️  Level: warning - High memory usage detected");

    // ===========================================
    // 9. LOGGING BUILDERS - Activity & Performance Tracking
    // ===========================================
    println!("\n📊 9. LoggingBuilder - Activity & Performance Tracking");
    println!("──────────────────────────────────────────────────────");

    // Development activity log
    let activity_log = LoggingBuilder::info(json!({
        "activity": "project_created",
        "project_name": "demo-project",
        "language": "rust",
        "components": ["cli", "server", "tests"]
    }))
    .logger("dev-activities")
    .meta_value("session_id", json!(Uuid::now_v7()))
    .meta_value("user", json!("developer@company.com"))
    .build();

    // activity_log is already a LoggingMessageNotification
    println!("   ✅ Created development activity log");
    println!("   📋 Method: {} (MCP spec-compliant)", activity_log.method);
    println!("   📊 Structured data: project setup completed, 3 components processed");
    println!("   ℹ️  Level: Info, Logger: dev-activities");

    // Performance metrics log (using spec-compliant structured logging)
    let mut perf_data = HashMap::new();
    perf_data.insert("operation".to_string(), json!("compilation_time"));
    perf_data.insert("duration_ms".to_string(), json!(1250.0));
    perf_data.insert("unit".to_string(), json!("ms"));

    let perf_log = LoggingBuilder::structured(LoggingLevel::Info, perf_data)
        .logger("performance-monitor")
        .meta_value("target", json!("release"))
        .meta_value("optimization_level", json!(3))
        .build();

    println!("   ✅ Created performance metrics log");
    println!("   📋 Method: notifications/message (MCP spec-compliant)");
    println!("   📊 Structured metrics: compilation_time=1.25s, memory_peak=512MB");
    println!("   ⏱️  Performance data logged for monitoring dashboard");

    // Error tracking log
    let error_log = LoggingBuilder::error(json!({
        "error_type": "compilation_error",
        "file": "src/main.rs",
        "line": 42,
        "message": "undefined variable 'foo'"
    }))
    .logger("error-tracker")
    .meta_value("build_id", json!("build-456"))
    .meta_value("commit_hash", json!("abc123def"))
    .build();

    println!("   ✅ Created error tracking log");
    println!("   📄 Sample error data: compilation_error in src/main.rs:42 (demo data)");

    // Security audit log (using spec-compliant structured logging)
    let mut audit_data = HashMap::new();
    audit_data.insert("user".to_string(), json!("security-auditor"));
    audit_data.insert("action".to_string(), json!("dependency_scan"));
    audit_data.insert("outcome".to_string(), json!("passed"));
    audit_data.insert("scanner".to_string(), json!("cargo-audit"));
    audit_data.insert("vulnerabilities_found".to_string(), json!(0));
    audit_data.insert("dependencies_checked".to_string(), json!(23));
    audit_data.insert("scan_duration_ms".to_string(), json!(450));

    let security_log = LoggingBuilder::structured(LoggingLevel::Notice, audit_data)
        .logger("security-audit")
        .meta_value("scan_id", json!("scan-789"))
        .build();

    println!("   ✅ Created security audit log");
    println!("   🔒 Scanner: cargo-audit");
    println!("   ✅ Vulnerabilities found: 0");

    // Simple text logging
    let simple_log = LoggingBuilder::text(
        LoggingLevel::Warning,
        "System initialization completed successfully",
    )
    .logger("system")
    .build();

    println!("   ✅ Created simple text log");
    println!("   📄 Text: System initialization completed successfully");
    println!("   ⚠️  Level: Warning");

    // ===========================================
    // 10. BUILDER OUTPUT DEMONSTRATION - Show builders create real, usable data
    // ===========================================
    println!("\n🔧 10. Builder Output Usage - Demonstrating Real Data Structures");
    println!("────────────────────────────────────────────────────────────────");

    // Demonstrate tool metadata access
    println!("   🔧 Tool Metadata:");
    println!("     Name: {}", create_project_tool.name());
    println!(
        "     Description: {}",
        create_project_tool.description().unwrap_or("N/A")
    );

    // Demonstrate resource access
    println!("\n   📄 Resource Information:");
    println!("     Config URI: {}", project_config.uri());
    println!(
        "     Config content type: {}",
        match &config_content {
            turul_mcp_protocol::resources::ResourceContent::Text { .. } => "Text",
            turul_mcp_protocol::resources::ResourceContent::Blob { .. } => "Binary",
        }
    );
    println!(
        "     Template content type: {}",
        match &template_content {
            turul_mcp_protocol::resources::ResourceContent::Text { .. } => "Text template",
            turul_mcp_protocol::resources::ResourceContent::Blob { .. } => "Binary template",
        }
    );

    // Demonstrate prompt information
    println!("\n   💡 Prompt Details:");
    println!("     Name: {}", code_gen_prompt.name());
    println!(
        "     Description: {}",
        code_gen_prompt.description().unwrap_or("N/A")
    );
    println!(
        "     Arguments: {} defined",
        code_gen_prompt.arguments().map_or(0, |args| args.len())
    );

    // Show that notifications are MCP protocol compliant
    println!("\n   📡 Notification Methods (MCP Spec Compliant):");
    println!("     Progress: {}", progress_notification.method);
    println!("     Resource Updated: {}", resource_notification.method);
    println!("     Logging Message: {}", alert_notification.method);

    // Show completion context
    println!("\n   🎯 Completion Context:");
    println!(
        "     Prompt completion reference: {:?}",
        prompt_completion.params.reference
    );
    println!(
        "     Resource completion available: {:?}",
        resource_completion.params.reference
    );

    // Show root configurations
    println!("\n   📁 Root Configurations:");
    println!("     Source root: {}", source_root.name().unwrap_or("N/A"));
    println!("     Config root: {}", config_root.name().unwrap_or("N/A"));
    println!("     Temp workspace: {}", temp_root.name().unwrap_or("N/A"));

    // Show elicitation forms
    println!("\n   📋 Elicitation Forms:");
    println!("     Project form message: {}", project_form.params.message);
    println!(
        "     Preferences form message: {}",
        prefs_form.params.message
    );

    // Show logging capabilities
    println!("\n   📊 Logging Methods (All MCP Compliant):");
    println!("     Activity log: {}", activity_log.method);
    println!("     Performance log: {}", perf_log.method);
    println!("     Error log: {}", error_log.method);
    println!("     Security log: {}", security_log.method);
    println!("     Simple log: {}", simple_log.method);

    println!("\n✨ All 9 builders create production-ready MCP protocol components!");
    println!("🔧 Ready for server integration, transport, and client consumption.");

    Ok(())
}
