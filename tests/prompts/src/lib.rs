//! Prompts module demonstrating MCP prompt integration
//!
//! This module shows how to properly implement MCP prompts using the derive macro
//! with correct argument handling and message generation.

use turul_mcp_derive::McpPrompt;
use turul_mcp_server::prelude::*;

/// Code review prompt
#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(
    name = "review_code",
    description = "Review code for quality, security, and best practices"
)]
pub struct ReviewCodePrompt {
    #[argument(description = "Programming language (rust, python, javascript, etc.)")]
    pub language: String,

    #[argument(description = "Code to review")]
    pub code: String,

    #[argument(description = "Review focus: security, performance, style, or comprehensive")]
    pub focus: Option<String>,

    #[argument(description = "Experience level: beginner, intermediate, or expert")]
    pub target_level: Option<String>,
}

impl ReviewCodePrompt {
    pub fn new(language: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            code: code.into(),
            focus: None,
            target_level: None,
        }
    }

    pub fn with_focus(mut self, focus: impl Into<String>) -> Self {
        self.focus = Some(focus.into());
        self
    }

    pub fn with_target_level(mut self, level: impl Into<String>) -> Self {
        self.target_level = Some(level.into());
        self
    }
}

#[async_trait]
impl McpPrompt for ReviewCodePrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let focus = self.focus.as_deref().unwrap_or("comprehensive");
        let level = self.target_level.as_deref().unwrap_or("intermediate");

        let combined_prompt = format!(
            "You are an experienced {} developer and code reviewer. \
             Provide {} code review feedback tailored for {} developers. \
             Focus on actionable improvements and explain the reasoning behind suggestions.\n\n\
             Please review this {} code:\n\n```{}\n{}\n```\n\n\
             Provide feedback on:\n\
             1. Code quality and readability\n\
             2. Potential bugs or issues\n\
             3. Performance considerations\n\
             4. Security concerns\n\
             5. Best practices and style\n\
             6. Specific improvement suggestions",
            self.language, focus, level, self.language, self.language, self.code
        );

        Ok(vec![PromptMessage::user_text(&combined_prompt)])
    }
}

/// Documentation generation prompt
#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(
    name = "generate_docs",
    description = "Generate comprehensive documentation for code or APIs"
)]
pub struct GenerateDocsPrompt {
    #[argument(description = "Type of documentation: api, function, class, or module")]
    pub doc_type: String,

    #[argument(description = "Code or API specification to document")]
    pub content: String,

    #[argument(description = "Documentation format: markdown, rst, or html")]
    pub format: String,

    #[argument(description = "Target audience: developer, user, or admin")]
    pub audience: Option<String>,
}

impl GenerateDocsPrompt {
    pub fn new(
        doc_type: impl Into<String>,
        content: impl Into<String>,
        format: impl Into<String>,
    ) -> Self {
        Self {
            doc_type: doc_type.into(),
            content: content.into(),
            format: format.into(),
            audience: None,
        }
    }
}

#[async_trait]
impl McpPrompt for GenerateDocsPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let audience = self.audience.as_deref().unwrap_or("developer");

        let combined_prompt = format!(
            "You are a technical documentation expert specializing in {} documentation. \
             Create clear, comprehensive {} documentation targeted at {}s. \
             Follow documentation best practices and include examples where appropriate.\n\n\
             Generate {} documentation for this {}:\n\n{}\n\n\
             Include:\n\
             1. Clear description and purpose\n\
             2. Usage examples\n\
             3. Parameter/argument details\n\
             4. Return values or responses\n\
             5. Error handling\n\
             6. Notes and best practices\n\n\
             Format as {} for {} audience.",
            self.format,
            self.doc_type,
            audience,
            self.doc_type,
            self.doc_type,
            self.content,
            self.format,
            audience
        );

        Ok(vec![PromptMessage::user_text(&combined_prompt)])
    }
}

/// Error analysis prompt
#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(
    name = "analyze_error",
    description = "Analyze error messages and logs to provide debugging guidance"
)]
pub struct AnalyzeErrorPrompt {
    #[argument(description = "Error message or stack trace")]
    pub error_message: String,

    #[argument(description = "Programming language or system context")]
    pub context: String,

    #[argument(description = "Additional context: code snippets, configuration, etc.")]
    pub additional_context: Option<String>,

    #[argument(description = "Urgency level: low, medium, high, or critical")]
    pub urgency: Option<String>,
}

impl AnalyzeErrorPrompt {
    pub fn new(error_message: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            error_message: error_message.into(),
            context: context.into(),
            additional_context: None,
            urgency: None,
        }
    }
}

#[async_trait]
impl McpPrompt for AnalyzeErrorPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let urgency = self.urgency.as_deref().unwrap_or("medium");
        let additional = self.additional_context.as_deref().unwrap_or("");

        let mut combined_prompt = format!(
            "You are an expert debugging assistant with deep knowledge of {} systems. \
             Provide clear, actionable debugging guidance. Prioritize {} urgency issues \
             with step-by-step troubleshooting approaches.\n\n\
             Help me debug this {} error:\n\n{}\n\n",
            self.context, urgency, self.context, self.error_message
        );

        if !additional.is_empty() {
            combined_prompt.push_str(&format!("Additional context:\n{}\n\n", additional));
        }

        combined_prompt.push_str(
            "Please provide:\n\
             1. Error explanation and root cause analysis\n\
             2. Step-by-step debugging approach\n\
             3. Potential solutions with code examples\n\
             4. Prevention strategies\n\
             5. Related issues to check for",
        );

        Ok(vec![PromptMessage::user_text(&combined_prompt)])
    }
}

/// Project planning prompt
#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(
    name = "plan_project",
    description = "Create project plans, task breakdowns, and implementation strategies"
)]
pub struct PlanProjectPrompt {
    #[argument(description = "Project description and goals")]
    pub project_description: String,

    #[argument(description = "Technology stack or domain")]
    pub technology: String,

    #[argument(description = "Project timeline: short, medium, or long")]
    pub timeline: String,

    #[argument(description = "Team size: solo, small, medium, or large")]
    pub team_size: Option<String>,

    #[argument(description = "Special constraints or requirements")]
    pub constraints: Option<String>,
}

impl PlanProjectPrompt {
    pub fn new(
        project_description: impl Into<String>,
        technology: impl Into<String>,
        timeline: impl Into<String>,
    ) -> Self {
        Self {
            project_description: project_description.into(),
            technology: technology.into(),
            timeline: timeline.into(),
            team_size: None,
            constraints: None,
        }
    }
}

#[async_trait]
impl McpPrompt for PlanProjectPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let team_size = self.team_size.as_deref().unwrap_or("small");
        let constraints = self
            .constraints
            .as_deref()
            .unwrap_or("standard project constraints");

        let combined_prompt = format!(
            "You are an experienced project manager and {} architect. \
             Create detailed, realistic project plans for {} teams with {} timeline. \
             Consider {} and provide actionable implementation strategies.\n\n\
             Create a project plan for:\n\n{}\n\n\
             Technology: {}\n\
             Timeline: {}\n\
             Team size: {}\n\
             Constraints: {}\n\n\
             Provide:\n\
             1. Project overview and scope\n\
             2. Phase breakdown with milestones\n\
             3. Task prioritization and dependencies\n\
             4. Resource allocation recommendations\n\
             5. Risk assessment and mitigation\n\
             6. Success metrics and deliverables",
            self.technology,
            team_size,
            self.timeline,
            constraints,
            self.project_description,
            self.technology,
            self.timeline,
            team_size,
            constraints
        );

        Ok(vec![PromptMessage::user_text(&combined_prompt)])
    }
}

/// Multi-content prompt demonstrating all ContentBlock variants
#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(
    name = "multi_content",
    description = "Demonstrates all ContentBlock types: Text, Image, ResourceLink, and embedded Resource"
)]
pub struct MultiContentPrompt {
    #[argument(
        name = "analysis_type",
        description = "Type of analysis to perform",
        required = true
    )]
    pub analysis_type: String,

    #[argument(
        name = "include_chart",
        description = "Whether to include chart visualization"
    )]
    pub include_chart: Option<String>,
}

impl MultiContentPrompt {
    pub fn new(analysis_type: impl Into<String>) -> Self {
        Self {
            analysis_type: analysis_type.into(),
            include_chart: None,
        }
    }

    pub async fn generate_multi_content_messages(&self) -> McpResult<Vec<PromptMessage>> {
        use serde_json::json;
        use std::collections::HashMap;
        use turul_mcp_protocol::prompts::{ContentBlock, ResourceContents, ResourceReference};

        let mut messages = vec![];

        // 1. Text ContentBlock
        messages.push(PromptMessage::user_text(format!(
            "Please perform a {} analysis using the provided elements.",
            self.analysis_type
        )));

        // 2. ResourceLink ContentBlock
        let data_resource = ResourceReference {
            uri: "file:///analysis/dataset.csv".to_string(),
            name: "analysis_dataset".to_string(),
            title: Some("Analysis Dataset".to_string()),
            description: Some("CSV dataset for analysis".to_string()),
            mime_type: Some("text/csv".to_string()),
            annotations: None,
            meta: None,
        };

        let resource_link_message = PromptMessage {
            role: turul_mcp_protocol::prompts::Role::User,
            content: ContentBlock::resource_link(data_resource),
        };
        messages.push(resource_link_message);

        // 3. Image ContentBlock
        if self.include_chart.as_deref() == Some("true") {
            let chart_image = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEklEQVR42mNk+M9QwwAGIxFYAA6gAhHlqe7fAAAAAElFTkSuQmCC";

            let image_message = PromptMessage {
                role: turul_mcp_protocol::prompts::Role::User,
                content: ContentBlock::image(chart_image, "image/png"),
            };
            messages.push(image_message);
        }

        // 4. Embedded Resource ContentBlock
        let config_resource = ResourceContents::text_with_mime(
            "file:///analysis/config.json",
            json!({
                "analysis_type": self.analysis_type,
                "parameters": {"confidence_level": 0.95}
            })
            .to_string(),
            "application/json",
        );

        let mut meta = HashMap::new();
        meta.insert("config_version".to_string(), json!("2.1"));

        let embedded_resource_message = PromptMessage {
            role: turul_mcp_protocol::prompts::Role::User,
            content: ContentBlock::Resource {
                resource: config_resource,
                annotations: Some(Annotations::new().with_title("analysis_configuration")),
                meta: Some(meta),
            },
        };
        messages.push(embedded_resource_message);

        Ok(messages)
    }
}

#[async_trait]
impl McpPrompt for MultiContentPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        self.generate_multi_content_messages().await
    }
}

/// Template prompt with variable substitution
#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(
    name = "template_vars",
    description = "Demonstrates template variable substitution in prompt messages"
)]
pub struct TemplateVarPrompt {
    #[argument(name = "user_name", description = "Name of the user", required = true)]
    pub user_name: String,

    #[argument(
        name = "task_type",
        description = "Type of task to perform",
        required = true
    )]
    pub task_type: String,

    #[argument(name = "priority", description = "Task priority level")]
    pub priority: Option<String>,
}

impl TemplateVarPrompt {
    pub fn new(user_name: impl Into<String>, task_type: impl Into<String>) -> Self {
        Self {
            user_name: user_name.into(),
            task_type: task_type.into(),
            priority: None,
        }
    }

    pub async fn generate_template_messages(&self) -> McpResult<Vec<PromptMessage>> {
        let priority_text = self.priority.as_deref().unwrap_or("normal");

        let user_message = format!(
            "Hello {}! You have been assigned a {} task with {} priority.",
            self.user_name, self.task_type, priority_text
        );

        Ok(vec![
            PromptMessage::user_text(&user_message),
            PromptMessage::assistant_text("I understand the task assignment."),
        ])
    }
}

#[async_trait]
impl McpPrompt for TemplateVarPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        self.generate_template_messages().await
    }
}
