//! # Real MCP Prompts Server
//!
//! This example demonstrates ACTUAL MCP protocol prompt implementation using
//! the McpPrompt trait with real prompt rendering and template substitution.
//! This replaces the previous fake tool-based approach with proper MCP protocol features.

use std::collections::HashMap;

use async_trait::async_trait;
use mcp_server::{McpServer, McpResult};
use mcp_protocol::{
    McpError,
    prompts::{HasPromptMetadata, HasPromptArguments, HasPromptMessages, HasPromptAnnotations, PromptArgument, PromptMessage},
};
use mcp_server::prompt::{McpPrompt};
use serde_json::Value;
use tracing::info;

/// Code generation prompt handler
/// Implements actual MCP prompt protocol for code generation templates
pub struct CodeGenerationPrompt {
    name: String,
    description: String,
    arguments: Vec<PromptArgument>,
    template: String,
}

impl CodeGenerationPrompt {
    pub fn new() -> Self {
        Self {
            name: "generate_code".to_string(),
            description: "Generate code based on requirements and language".to_string(),
            arguments: vec![
                PromptArgument::new("language")
                    .with_description("Programming language (rust, python, javascript, etc.)")
                    .required(),
                PromptArgument::new("requirements")
                    .with_description("Description of what the code should do")
                    .required(),
                PromptArgument::new("style")
                    .with_description("Code style preference (functional, oop, minimal, verbose)")
                    .optional(),
                PromptArgument::new("framework")
                    .with_description("Framework or library to use (optional)")
                    .optional(),
            ],
            template: r#"You are an expert {language} developer. Please generate clean, production-ready code that meets these requirements:

## Requirements
{requirements}

## Language & Style
- **Language**: {language}
- **Style**: {style|functional}
- **Framework**: {framework|standard library}

## Guidelines
- Write clean, readable code with proper error handling
- Include appropriate comments and documentation
- Follow {language} best practices and idioms
- Ensure code is production-ready and well-tested
- Use the specified framework/libraries when applicable

Please provide:
1. Complete, working code implementation
2. Brief explanation of the approach
3. Usage examples if applicable
4. Any important considerations or limitations

Generate the {language} code now:"#.to_string(),
        }
    }

    fn render_template(&self, args: &HashMap<String, Value>) -> String {
        let mut rendered = self.template.clone();
        
        // Replace required arguments
        for arg in &self.arguments {
            if let Some(value) = args.get(&arg.name) {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                let placeholder = format!("{{{}}}", arg.name);
                rendered = rendered.replace(&placeholder, &value_str);
            }
        }
        
        // Handle optional arguments with defaults
        rendered = rendered.replace("{style|functional}", 
            args.get("style")
                .and_then(|v| v.as_str())
                .unwrap_or("functional"));
                
        rendered = rendered.replace("{framework|standard library}", 
            args.get("framework")
                .and_then(|v| v.as_str())
                .unwrap_or("standard library"));
        
        rendered
    }
}

// Implement fine-grained traits for MCP compliance
impl HasPromptMetadata for CodeGenerationPrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
}

impl HasPromptArguments for CodeGenerationPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptMessages for CodeGenerationPrompt {
    fn render_messages(&self, args: Option<&HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> {
        let empty_map = HashMap::new();
        let args = args.unwrap_or(&empty_map);
        
        // Validate required arguments
        for arg in &self.arguments {
            if arg.required.unwrap_or(false) && !args.contains_key(&arg.name) {
                return Err(format!("Missing required argument: {}", arg.name));
            }
        }
        
        let rendered_content = self.render_template(args);
        Ok(vec![PromptMessage::text(rendered_content)])
    }
}

impl HasPromptAnnotations for CodeGenerationPrompt {
    fn annotations(&self) -> Option<&Value> {
        None
    }
}

// PromptDefinition automatically implemented via blanket impl

#[async_trait]
impl McpPrompt for CodeGenerationPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        match self.render_messages(args.as_ref()) {
            Ok(messages) => Ok(messages),
            Err(msg) => Err(McpError::validation(&msg)),
        }
    }
    
    async fn validate_args(&self, args: &HashMap<String, Value>) -> McpResult<()> {
        // Check required arguments
        for arg in &self.arguments {
            if arg.required.unwrap_or(false) && !args.contains_key(&arg.name) {
                return Err(McpError::validation(&format!("Missing required argument: {}", arg.name)));
            }
        }
        
        // Validate language argument
        if let Some(language) = args.get("language").and_then(|v| v.as_str()) {
            let supported_languages = &[
                "rust", "python", "javascript", "typescript", "java", "go", "c", "cpp",
                "csharp", "php", "ruby", "swift", "kotlin", "scala", "haskell", "sql"
            ];
            if !supported_languages.contains(&language.to_lowercase().as_str()) {
                return Err(McpError::validation(&format!("Unsupported language: {}", language)));
            }
        }
        
        Ok(())
    }
}

/// Code review prompt handler
pub struct CodeReviewPrompt {
    name: String,
    description: String,
    arguments: Vec<PromptArgument>,
    template: String,
}

impl CodeReviewPrompt {
    pub fn new() -> Self {
        Self {
            name: "review_code".to_string(),
            description: "Perform comprehensive code review with suggestions".to_string(),
            arguments: vec![
                PromptArgument::new("code")
                    .with_description("Code to review")
                    .required(),
                PromptArgument::new("language")
                    .with_description("Programming language of the code")
                    .required(),
                PromptArgument::new("focus")
                    .with_description("Review focus (security, performance, readability, all)")
                    .optional(),
            ],
            template: r#"Please perform a comprehensive code review of the following {language} code.

## Code to Review
```{language}
{code}
```

## Review Focus
{focus|all aspects}

## Review Criteria
Please analyze the code for:

1. **Code Quality & Style**
   - Readability and maintainability
   - Naming conventions
   - Code organization and structure
   - Documentation and comments

2. **Functionality & Logic**
   - Correctness of implementation
   - Edge case handling
   - Error handling and validation
   - Algorithm efficiency

3. **Security Considerations**
   - Input validation
   - Security vulnerabilities
   - Data sanitization
   - Authentication/authorization issues

4. **Performance & Optimization**
   - Time and space complexity
   - Resource usage
   - Potential bottlenecks
   - Optimization opportunities

5. **Best Practices**
   - Language-specific idioms
   - Framework conventions
   - Testing considerations
   - Deployment readiness

## Please provide:
- Overall assessment (Good/Needs Work/Major Issues)
- Specific issues found with line numbers if applicable
- Suggested improvements and fixes
- Positive aspects worth noting
- Priority ranking of issues (High/Medium/Low)

Begin your review now:"#.to_string(),
        }
    }
    
    fn render_template(&self, args: &HashMap<String, Value>) -> String {
        let mut rendered = self.template.clone();
        
        for (key, value) in args {
            let value_str = match value {
                Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            let placeholder = format!("{{{}}}", key);
            rendered = rendered.replace(&placeholder, &value_str);
        }
        
        // Handle optional focus with default
        rendered = rendered.replace("{focus|all aspects}", 
            args.get("focus")
                .and_then(|v| v.as_str())
                .unwrap_or("all aspects"));
        
        rendered
    }
}

impl HasPromptMetadata for CodeReviewPrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
}

impl HasPromptArguments for CodeReviewPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptMessages for CodeReviewPrompt {
    fn render_messages(&self, args: Option<&HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> {
        let empty_map = HashMap::new();
        let args = args.unwrap_or(&empty_map);
        
        for arg in &self.arguments {
            if arg.required.unwrap_or(false) && !args.contains_key(&arg.name) {
                return Err(format!("Missing required argument: {}", arg.name));
            }
        }
        
        let rendered_content = self.render_template(args);
        Ok(vec![PromptMessage::text(rendered_content)])
    }
}

impl HasPromptAnnotations for CodeReviewPrompt {
    fn annotations(&self) -> Option<&Value> {
        None
    }
}

#[async_trait]
impl McpPrompt for CodeReviewPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        match self.render_messages(args.as_ref()) {
            Ok(messages) => Ok(messages),
            Err(msg) => Err(McpError::validation(&msg)),
        }
    }
    
    async fn validate_args(&self, args: &HashMap<String, Value>) -> McpResult<()> {
        for arg in &self.arguments {
            if arg.required.unwrap_or(false) && !args.contains_key(&arg.name) {
                return Err(McpError::validation(&format!("Missing required argument: {}", arg.name)));
            }
        }
        
        // Validate focus area if provided
        if let Some(focus) = args.get("focus").and_then(|v| v.as_str()) {
            let valid_focus = &["security", "performance", "readability", "all"];
            if !valid_focus.contains(&focus.to_lowercase().as_str()) {
                return Err(McpError::validation(&format!("Invalid focus area: {}", focus)));
            }
        }
        
        Ok(())
    }
}

/// Architecture guidance prompt handler
pub struct ArchitecturePrompt {
    name: String,
    description: String,
    arguments: Vec<PromptArgument>,
}

impl ArchitecturePrompt {
    pub fn new() -> Self {
        Self {
            name: "architecture_guidance".to_string(),
            description: "Get architectural guidance and design recommendations".to_string(),
            arguments: vec![
                PromptArgument::new("project_type")
                    .with_description("Type of project (web-app, api, microservice, mobile, desktop)")
                    .required(),
                PromptArgument::new("requirements")
                    .with_description("Project requirements and constraints")
                    .required(),
                PromptArgument::new("scale")
                    .with_description("Expected scale (small, medium, large, enterprise)")
                    .optional(),
                PromptArgument::new("technology_stack")
                    .with_description("Preferred or existing technology stack")
                    .optional(),
            ],
        }
    }
}

impl HasPromptMetadata for ArchitecturePrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
}

impl HasPromptArguments for ArchitecturePrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptMessages for ArchitecturePrompt {
    fn render_messages(&self, args: Option<&HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> {
        let empty_map = HashMap::new();
        let args = args.unwrap_or(&empty_map);
        
        let project_type = args.get("project_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");
            
        let requirements = args.get("requirements")
            .and_then(|v| v.as_str())
            .unwrap_or("No specific requirements provided");
            
        let scale = args.get("scale")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");
            
        let tech_stack = args.get("technology_stack")
            .and_then(|v| v.as_str())
            .unwrap_or("flexible");

        let template = format!(r#"You are a senior software architect. Please provide comprehensive architectural guidance for a {project_type} project.

## Project Overview
- **Type**: {project_type}
- **Scale**: {scale}
- **Technology Stack**: {tech_stack}

## Requirements
{requirements}

## Please provide architectural guidance covering:

### 1. High-Level Architecture
- Overall system design and architecture patterns
- Component breakdown and responsibilities
- Data flow and interaction patterns
- Scalability considerations for {scale} scale

### 2. Technology Recommendations
- Appropriate frameworks and libraries
- Database and storage solutions
- Infrastructure and deployment considerations
- Third-party services and integrations

### 3. Design Patterns & Best Practices
- Recommended architectural patterns
- Code organization strategies
- Error handling and logging approaches
- Security considerations

### 4. Implementation Strategy
- Development phases and milestones
- Team structure recommendations
- Risk assessment and mitigation
- Testing and quality assurance strategy

### 5. Future-Proofing
- Scalability roadmap
- Technology evolution considerations
- Maintenance and operational concerns
- Documentation and knowledge transfer

Please provide detailed, actionable recommendations:"#, 
            project_type = project_type,
            scale = scale, 
            tech_stack = tech_stack,
            requirements = requirements
        );

        Ok(vec![PromptMessage::text(template)])
    }
}

impl HasPromptAnnotations for ArchitecturePrompt {
    fn annotations(&self) -> Option<&Value> {
        None
    }
}

#[async_trait]
impl McpPrompt for ArchitecturePrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        match self.render_messages(args.as_ref()) {
            Ok(messages) => Ok(messages),
            Err(msg) => Err(McpError::validation(&msg)),
        }
    }
    
    async fn validate_args(&self, args: &HashMap<String, Value>) -> McpResult<()> {
        for arg in &self.arguments {
            if arg.required.unwrap_or(false) && !args.contains_key(&arg.name) {
                return Err(McpError::validation(&format!("Missing required argument: {}", arg.name)));
            }
        }
        
        // Validate project type
        if let Some(project_type) = args.get("project_type").and_then(|v| v.as_str()) {
            let valid_types = &["web-app", "api", "microservice", "mobile", "desktop", "cli", "library"];
            if !valid_types.contains(&project_type.to_lowercase().as_str()) {
                return Err(McpError::validation(&format!("Invalid project type: {}", project_type)));
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Real MCP Prompts Server");
    info!("===================================");

    // Create prompt handlers using ACTUAL MCP protocol
    let code_generation = CodeGenerationPrompt::new();
    let code_review = CodeReviewPrompt::new();
    let architecture_guidance = ArchitecturePrompt::new();

    let server = McpServer::builder()
        .name("real-prompts-server")
        .version("1.0.0")
        .title("Real MCP Prompts Server")
        .instructions(
            "This server demonstrates ACTUAL MCP prompt protocol implementation. \
             It uses McpPrompt traits to render real prompts with template substitution, \
             not fake tools that pretend to generate prompts. This is how MCP protocol \
             features should be implemented."
        )
        .prompt(code_generation)
        .prompt(code_review)
        .prompt(architecture_guidance)
        .bind_address("127.0.0.1:8006".parse()?)
        .sse(true)
        .build()?;

    info!("🚀 Real MCP prompts server running at: http://127.0.0.1:8006/mcp");
    info!("📝 This server implements ACTUAL MCP prompts:");
    info!("   • generate_code - AI-assisted code generation with language and style options");
    info!("   • review_code - Comprehensive code review with security, performance, and quality analysis");
    info!("   • architecture_guidance - Software architecture recommendations and design patterns");
    info!("💡 Unlike previous examples, this uses real McpPrompt traits");
    info!("💡 Prompts are rendered with proper template substitution and validation");
    info!("💡 This demonstrates actual MCP protocol prompt implementation");

    server.run().await?;
    Ok(())
}