//! # AI-Assisted Development Prompts Server
//!
//! This example demonstrates a real-world MCP prompts server that provides AI-assisted 
//! code generation, review, and architecture guidance for development teams. The server 
//! loads prompt templates and best practices from external files, demonstrating how 
//! teams can maintain and share AI prompts for consistent code quality and practices.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use async_trait::async_trait;
use mcp_server::{McpServer, McpResult};
use mcp_server::handlers::McpPrompt;
use mcp_protocol::{prompts::PromptMessage, McpError};
use serde_json::{Value, from_str};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
struct CodeTemplates {
    templates: HashMap<String, HashMap<String, TemplateDefinition>>,
    best_practices: HashMap<String, BestPractices>,
    code_review_checklist: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TemplateDefinition {
    template: String,
    #[serde(default)]
    test_template: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BestPractices {
    naming: HashMap<String, String>,
    documentation: HashMap<String, String>,
    #[serde(default)]
    testing: Option<HashMap<String, String>>,
    #[serde(default)]
    error_handling: Option<HashMap<String, String>>,
}

/// AI-assisted code generation prompt that loads templates from external files
/// Real-world use case: Team-consistent code generation with best practices
struct CodeGenerationPrompt {
    templates: CodeTemplates,
}

impl CodeGenerationPrompt {
    fn new() -> McpResult<Self> {
        let templates_path = Path::new("data/code_templates.json");
        
        let templates = match fs::read_to_string(templates_path) {
            Ok(content) => {
                from_str::<CodeTemplates>(&content)
                    .map_err(|e| McpError::tool_execution(&format!("Failed to parse templates: {}", e)))?
            },
            Err(_) => {
                // Fallback templates for basic functionality
                CodeTemplates {
                    templates: HashMap::new(),
                    best_practices: HashMap::new(),
                    code_review_checklist: HashMap::new(),
                }
            }
        };
        
        Ok(Self { templates })
    }
    
    fn get_template(&self, language: &str, template_type: &str) -> Option<&TemplateDefinition> {
        self.templates.templates
            .get(language)
            .and_then(|lang_templates| lang_templates.get(template_type))
    }
    
    fn get_best_practices(&self, language: &str) -> Option<&BestPractices> {
        self.templates.best_practices.get(language)
    }
}

#[async_trait]
impl McpPrompt for CodeGenerationPrompt {
    fn name(&self) -> &str {
        "code-generation"
    }

    fn description(&self) -> &str {
        "Generate production-ready code using team templates and best practices loaded from external files"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        let function_name = args.get("function_name")
            .and_then(|v| v.as_str())
            .unwrap_or("example_function");
        
        let language = args.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("rust");
        
        let description = args.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("a well-designed function that follows team standards");
        
        let template_type = args.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("function");
        
        let include_tests = args.get("include_tests")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut prompt_content = format!(
            "You are a senior {} developer creating production-ready code for a development team. \
             Generate code that follows the team's established patterns and best practices.\n\n",
            language
        );

        // Add best practices if available
        if let Some(best_practices) = self.get_best_practices(language) {
            prompt_content.push_str("## Team Best Practices\n\n");
            
            if !best_practices.naming.is_empty() {
                prompt_content.push_str("**Naming Conventions:**\n");
                for (item, convention) in &best_practices.naming {
                    prompt_content.push_str(&format!("- {}: {}\n", item, convention));
                }
                prompt_content.push('\n');
            }
            
            if !best_practices.documentation.is_empty() {
                prompt_content.push_str("**Documentation Standards:**\n");
                for (aspect, standard) in &best_practices.documentation {
                    prompt_content.push_str(&format!("- {}: {}\n", aspect, standard));
                }
                prompt_content.push('\n');
            }
            
            if let Some(error_handling) = &best_practices.error_handling {
                prompt_content.push_str("**Error Handling:**\n");
                for (practice, description) in error_handling {
                    prompt_content.push_str(&format!("- {}: {}\n", practice, description));
                }
                prompt_content.push('\n');
            }
        }

        // Add template-specific instructions
        if let Some(template_def) = self.get_template(language, template_type) {
            prompt_content.push_str(&format!(
                "## Code Generation Task\n\n\
                 Create a {} {} named '{}' that {}.\n\n\
                 Use this template structure as a guide:\n\
                 ```{}```\n\n",
                language, template_type, function_name, description, template_def.template
            ));
            
            if include_tests {
                if let Some(test_template) = &template_def.test_template {
                    prompt_content.push_str(&format!(
                        "Also generate comprehensive tests using this template:\n\
                         ```{}```\n\n",
                        test_template
                    ));
                }
            }
        } else {
            prompt_content.push_str(&format!(
                "## Code Generation Task\n\n\
                 Create a {} {} named '{}' that {}.\n\n",
                language, template_type, function_name, description
            ));
        }

        prompt_content.push_str(
            "## Requirements\n\
             - Follow the team's coding standards and best practices above\n\
             - Include comprehensive documentation\n\
             - Implement proper error handling\n\
             - Make the code production-ready and maintainable\n\
             - Include usage examples in documentation\n"
        );
        
        if include_tests {
            prompt_content.push_str("- Generate comprehensive unit tests with edge cases\n");
        }

        prompt_content.push_str("\n**Generate clean, well-structured code that the team can confidently deploy to production.**");

        Ok(vec![PromptMessage::text(prompt_content)])
    }
}

/// Documentation prompt for explaining code or concepts
struct DocumentationPrompt;

#[async_trait]
impl McpPrompt for DocumentationPrompt {
    fn name(&self) -> &str {
        "documentation"
    }

    fn description(&self) -> &str {
        "Generate comprehensive documentation for code, APIs, or technical concepts"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        let subject = args.get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("subject"))?;
        
        let doc_type = args.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("api");
        
        let audience = args.get("audience")
            .and_then(|v| v.as_str())
            .unwrap_or("developers");
        
        let format = args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        let prompt_content = format!(
            "You are a technical documentation expert. Create clear, comprehensive documentation \
             in {} format for a {} audience. Focus on practical examples and clear explanations.\n\n\
             Please create {} documentation for: {}\n\n\
             Include:\n\
             - Clear overview and purpose\n\
             - Detailed usage examples\n\
             - Parameter/configuration details\n\
             - Common use cases and patterns\n\
             - Troubleshooting section\n\
             - Best practices and tips\n\n\
             Target audience: {}\n\
             Format: {}",
            format, audience, doc_type, subject, audience, format
        );

        Ok(vec![PromptMessage::text(prompt_content)])
    }
}

/// AI-assisted code review prompt that loads guidelines from external files
/// Real-world use case: Consistent code reviews using team standards
struct CodeReviewPrompt {
    templates: CodeTemplates,
    review_guidelines: String,
}

impl CodeReviewPrompt {
    fn new() -> McpResult<Self> {
        let templates_path = Path::new("data/code_templates.json");
        let guidelines_path = Path::new("data/review_guidelines.md");
        
        let templates = match fs::read_to_string(templates_path) {
            Ok(content) => {
                from_str::<CodeTemplates>(&content)
                    .map_err(|e| McpError::tool_execution(&format!("Failed to parse templates: {}", e)))?
            },
            Err(_) => {
                CodeTemplates {
                    templates: HashMap::new(),
                    best_practices: HashMap::new(),
                    code_review_checklist: HashMap::new(),
                }
            }
        };
        
        let review_guidelines = fs::read_to_string(guidelines_path)
            .unwrap_or_else(|_| "# Code Review Guidelines\n\nUse general best practices for code review.".to_string());
        
        Ok(Self { templates, review_guidelines })
    }
}

#[async_trait]
impl McpPrompt for CodeReviewPrompt {
    fn name(&self) -> &str {
        "code-review"
    }

    fn description(&self) -> &str {
        "Perform comprehensive code review using team guidelines and checklists loaded from external files"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        let code = args.get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("code"))?;
        
        let language = args.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("auto-detect");
        
        let focus = args.get("focus")
            .and_then(|v| v.as_str())
            .unwrap_or("comprehensive");
        
        let severity = args.get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("all"); // all, critical, suggestions

        let mut prompt_content = format!(
            "You are a senior software engineer conducting a comprehensive code review for a development team. \
             Use the team's established review guidelines and focus on providing actionable, constructive feedback.\n\n"
        );

        // Add team review guidelines
        prompt_content.push_str("## Team Review Guidelines\n\n");
        prompt_content.push_str(&self.review_guidelines);
        prompt_content.push_str("\n\n");

        // Add language-specific checklist if available
        if let Some(checklist) = self.templates.code_review_checklist.get(language) {
            prompt_content.push_str(&format!("## {} Specific Checklist\n\n", language));
            for item in checklist {
                prompt_content.push_str(&format!("- {}\n", item));
            }
            prompt_content.push_str("\n");
        }

        // Add general checklist
        if let Some(general_checklist) = self.templates.code_review_checklist.get("general") {
            prompt_content.push_str("## General Review Checklist\n\n");
            for item in general_checklist {
                prompt_content.push_str(&format!("- {}\n", item));
            }
            prompt_content.push_str("\n");
        }

        prompt_content.push_str(&format!(
            "## Code to Review\n\n\
             **Language**: {}\n\
             **Review Focus**: {}\n\
             **Severity Filter**: {}\n\n\
             ```{}\n{}\n```\n\n",
            language, focus, severity, language, code
        ));

        prompt_content.push_str(
            "## Review Instructions\n\n\
             Please conduct a thorough code review following the team guidelines above. \
             Provide feedback in these categories:\n\n\
             ### 🔴 Critical Issues (Must Fix)\n\
             - Security vulnerabilities\n\
             - Functional bugs\n\
             - Performance issues\n\
             - Breaking changes\n\n\
             ### 🟡 Important Improvements (Should Fix)\n\
             - Code quality issues\n\
             - Maintainability concerns\n\
             - Missing error handling\n\
             - Documentation gaps\n\n\
             ### 🔵 Suggestions (Consider)\n\
             - Style improvements\n\
             - Code optimizations\n\
             - Better abstractions\n\
             - Alternative approaches\n\n\
             ### ✅ Positive Feedback\n\
             - Well-implemented patterns\n\
             - Good practices followed\n\
             - Clean, readable code\n\n"
        );

        match severity {
            "critical" => prompt_content.push_str("**Focus only on critical issues that must be fixed before merge.**\n"),
            "suggestions" => prompt_content.push_str("**Focus on suggestions and improvements, assume the code is functionally correct.**\n"),
            _ => prompt_content.push_str("**Provide comprehensive feedback across all categories.**\n"),
        }

        prompt_content.push_str(
            "\n**For each issue, provide:**\n\
             - Clear explanation of the problem\n\
             - Specific line references if applicable\n\
             - Suggested improvement with code examples\n\
             - Reasoning behind the suggestion\n\n\
             **Be constructive, specific, and focus on helping the author improve the code quality.**"
        );

        Ok(vec![PromptMessage::text(prompt_content)])
    }
}

/// Debugging prompt for troubleshooting issues
struct DebuggingPrompt;

#[async_trait]
impl McpPrompt for DebuggingPrompt {
    fn name(&self) -> &str {
        "debugging"
    }

    fn description(&self) -> &str {
        "Help debug issues with step-by-step troubleshooting guidance"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        let problem = args.get("problem")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("problem"))?;
        
        let context = args.get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("general software development");
        
        let error_message = args.get("error_message")
            .and_then(|v| v.as_str());

        let mut prompt_content = format!(
            "You are an expert debugging specialist. Provide systematic, step-by-step \
             troubleshooting guidance. Focus on methodical problem-solving approaches \
             and practical solutions.\n\n\
             I'm experiencing this problem: {}\n\
             Context: {}\n\n",
            problem, context
        );

        if let Some(error) = error_message {
            prompt_content.push_str(&format!("Error message: {}\n\n", error));
        }

        prompt_content.push_str(
            "Please provide:\n\
             1. Immediate steps to gather more information\n\
             2. Common causes and how to check for them\n\
             3. Step-by-step debugging process\n\
             4. Potential solutions ranked by likelihood\n\
             5. Prevention strategies for the future\n\
             6. Additional tools or techniques that might help\n\n\
             Be specific and provide concrete examples where possible."
        );

        Ok(vec![PromptMessage::text(prompt_content)])
    }
}

/// Architecture design prompt that loads patterns from external YAML file
/// Real-world use case: Consistent architecture guidance using proven patterns
struct ArchitecturePrompt {
    architecture_patterns: String,
}

impl ArchitecturePrompt {
    fn new() -> McpResult<Self> {
        let patterns_path = Path::new("data/architecture_patterns.yaml");
        
        let architecture_patterns = fs::read_to_string(patterns_path)
            .unwrap_or_else(|_| {
                "# Architecture Patterns\n\nUse general architecture best practices.".to_string()
            });
        
        Ok(Self { architecture_patterns })
    }
}

#[async_trait]
impl McpPrompt for ArchitecturePrompt {
    fn name(&self) -> &str {
        "architecture-design"
    }

    fn description(&self) -> &str {
        "Design software architecture using proven patterns and best practices loaded from external files"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        let system_type = args.get("system_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("system_type"))?;
        
        let requirements = args.get("requirements")
            .and_then(|v| v.as_str())
            .unwrap_or("general system requirements");
        
        let scale = args.get("scale")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");
        
        let constraints = args.get("constraints")
            .and_then(|v| v.as_str())
            .unwrap_or("standard technical and business constraints");
        
        let focus_areas = args.get("focus_areas")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "scalability, maintainability, security".to_string());

        let mut prompt_content = format!(
            "You are a senior software architect with extensive experience designing production systems. \
             Use the team's established architecture patterns and design principles to create a comprehensive \
             system design that follows proven practices.\n\n"
        );

        // Add architecture patterns and guidelines
        prompt_content.push_str("## Architecture Patterns and Guidelines\n\n");
        prompt_content.push_str(&self.architecture_patterns);
        prompt_content.push_str("\n\n");

        prompt_content.push_str(&format!(
            "## Architecture Design Request\n\n\
             **System Type**: {}\n\
             **Scale**: {}\n\
             **Focus Areas**: {}\n\n\
             **Requirements**: {}\n\n\
             **Constraints**: {}\n\n",
            system_type, scale, focus_areas, requirements, constraints
        ));

        prompt_content.push_str(
            "## Design Tasks\n\n\
             Based on the patterns and principles above, please provide a comprehensive architecture design:\n\n\
             ### 1. 🏗️ Architecture Pattern Selection\n\
             - Recommend the most suitable architecture pattern(s) from the available options\n\
             - Justify the selection based on requirements and constraints\n\
             - Explain how the chosen pattern addresses the specific needs\n\n\
             ### 2. 🧩 System Components and Responsibilities\n\
             - Break down the system into logical components\n\
             - Define clear responsibilities for each component\n\
             - Identify interfaces and communication patterns\n\n\
             ### 3. 📊 Data Architecture\n\
             - Data flow and storage strategy\n\
             - Database selection and partitioning\n\
             - Caching and data consistency approaches\n\n\
             ### 4. 🔧 Technology Stack Recommendations\n\
             - Specific technologies for each layer\n\
             - Justification based on requirements and team capabilities\n\
             - Consider maintenance and operational aspects\n\n\
             ### 5. ⚡ Scalability and Performance\n\
             - Horizontal and vertical scaling strategies\n\
             - Performance bottleneck identification and mitigation\n\
             - Load balancing and resource optimization\n\n\
             ### 6. 🔒 Security Architecture\n\
             - Authentication and authorization strategy\n\
             - Data protection and encryption\n\
             - Network security and API protection\n\n\
             ### 7. 🚀 Deployment and Operations\n\
             - Deployment strategy and CI/CD pipeline\n\
             - Monitoring, logging, and observability\n\
             - Disaster recovery and backup strategies\n\n\
             ### 8. 🧪 Testing Strategy\n\
             - Unit, integration, and end-to-end testing approaches\n\
             - Performance and security testing\n\
             - Chaos engineering and resilience testing\n\n\
             ### 9. ⚠️ Risk Assessment and Mitigation\n\
             - Identify potential risks and failure points\n\
             - Mitigation strategies and contingency plans\n\
             - Trade-offs and architectural decisions\n\n\
             ### 10. 🗺️ Implementation Roadmap\n\
             - Phase-based implementation approach\n\
             - Dependencies and critical path\n\
             - Success metrics and validation criteria\n\n"
        );

        prompt_content.push_str(
            "## Output Requirements\n\n\
             - **Use proven patterns** from the guidelines above\n\
             - **Include ASCII diagrams** for system architecture and data flow\n\
             - **Provide specific examples** and configuration details where applicable\n\
             - **Consider operational aspects** like monitoring, debugging, and maintenance\n\
             - **Address the stated focus areas** in detail\n\
             - **Explain design decisions** and trade-offs clearly\n\n\
             **Create a production-ready architecture that the development team can implement with confidence.**"
        );

        Ok(vec![PromptMessage::text(prompt_content)])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting AI-Assisted Development Prompts Server");

    let code_gen_prompt = CodeGenerationPrompt::new()?;
    let doc_prompt = DocumentationPrompt;
    let review_prompt = CodeReviewPrompt::new()?;
    let debug_prompt = DebuggingPrompt;
    let arch_prompt = ArchitecturePrompt::new()?;

    let server = McpServer::builder()
        .name("ai-development-prompts")
        .version("1.0.0")
        .title("AI-Assisted Development Prompts Server")
        .instructions("Real-world AI prompts server for development teams. Provides AI-assisted code generation, review, and architecture guidance using team templates and best practices loaded from external files.")
        .prompt(code_gen_prompt)
        .prompt(doc_prompt)
        .prompt(review_prompt)
        .prompt(debug_prompt)
        .prompt(arch_prompt)
        .with_prompts()
        .bind_address("127.0.0.1:8040".parse()?)
        .build()?;
    
    info!("AI development prompts server running at: http://127.0.0.1:8040/mcp");
    info!("Real-world AI-assisted development prompts:");
    info!("  - code-generation: Generate production-ready code using team templates and best practices");
    info!("  - documentation: Create comprehensive technical documentation");
    info!("  - code-review: Perform consistent code reviews using team guidelines and checklists");
    info!("  - debugging: Step-by-step troubleshooting with systematic approaches");
    info!("  - architecture-design: Design systems using proven architecture patterns and principles");
    info!("External data files: data/code_templates.json, data/review_guidelines.md, data/architecture_patterns.yaml");

    server.run().await?;
    Ok(())
}