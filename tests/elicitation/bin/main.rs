//! # Customer Onboarding and Data Collection Platform
//!
//! This server provides a comprehensive customer onboarding system using MCP elicitation
//! to collect structured customer data, handle compliance forms, manage user preferences,
//! and conduct surveys. It demonstrates real-world patterns for data collection workflows,
//! regulatory compliance, and user experience optimization.
//!
//! Features:
//! - Multi-step customer onboarding flows (personal & business)
//! - GDPR/CCPA compliance forms and data subject requests
//! - User preference collection and management
//! - Customer satisfaction surveys and feedback collection
//! - Comprehensive validation with external reference data
//! - Accessibility compliance and internationalization support

use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations};
use turul_mcp_protocol::{McpError, McpResult, ToolResult, ToolSchema, schema::JsonSchema};
use turul_mcp_server::{McpServer, McpTool, SessionContext};
// ElicitationBuilder import removed - using simplified demonstrations
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};
use uuid::Uuid;

/// Configuration for onboarding workflows loaded from external JSON
#[derive(Debug, Deserialize, Clone)]
struct OnboardingConfig {
    customer_onboarding_workflows: HashMap<String, OnboardingWorkflow>,
    compliance_forms: HashMap<String, ComplianceForm>,
    preference_collection: HashMap<String, PreferenceCollection>,
    survey_templates: HashMap<String, SurveyTemplate>,
}

#[derive(Debug, Deserialize, Clone)]
struct OnboardingWorkflow {
    name: String,
    #[allow(dead_code)] // TODO: Use description in workflow presentation
    description: String,
    workflow_id: String,
    steps: Vec<WorkflowStep>,
    #[serde(default)]
    completion_actions: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct WorkflowStep {
    step_id: String,
    title: String,
    description: String,
    fields: Vec<FormField>,
}

#[derive(Debug, Deserialize, Clone)]
struct FormField {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    label: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    choices: Value,
    #[serde(default)]
    help_text: Option<String>,
    #[allow(dead_code)] // TODO: Implement field validation
    #[serde(default)]
    validation: HashMap<String, Value>,
    #[allow(dead_code)] // TODO: Implement default values
    #[serde(default)]
    default: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct ComplianceForm {
    name: String,
    description: String,
    fields: Vec<FormField>,
}

#[derive(Debug, Deserialize, Clone)]
struct PreferenceCollection {
    name: String,
    description: String,
    #[serde(default)]
    categories: Vec<PreferenceCategory>,
    #[serde(default)]
    fields: Vec<FormField>,
}

#[derive(Debug, Deserialize, Clone)]
struct PreferenceCategory {
    category: String,
    description: String,
    settings: Vec<PreferenceSetting>,
}

#[derive(Debug, Deserialize, Clone)]
struct PreferenceSetting {
    name: String,
    label: String,
    channels: Vec<String>,
    #[allow(dead_code)] // TODO: Implement default channel selection
    #[serde(default)]
    default_channels: Vec<String>,
    #[serde(default)]
    required: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct SurveyTemplate {
    name: String,
    description: String,
    fields: Vec<FormField>,
}

/// Validation rules configuration loaded from external YAML
#[derive(Debug, Deserialize, Clone)]
struct ValidationConfig {
    validation_rules: ValidationRules,
}

#[derive(Debug, Deserialize, Clone)]
struct ValidationRules {
    field_types: HashMap<String, FieldTypeValidation>,
    business_rules: BusinessRules,
    #[allow(dead_code)] // TODO: Implement security policy validation
    security_policies: SecurityPolicies,
    #[allow(dead_code)] // TODO: Implement error handling configuration
    #[serde(default)]
    error_handling: ErrorHandling,
}

#[derive(Debug, Deserialize, Clone)]
struct FieldTypeValidation {
    #[allow(dead_code)] // TODO: Use rules for field type validation
    #[serde(flatten)]
    rules: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone)]
struct BusinessRules {
    age_verification: AgeVerificationRules,
    #[allow(dead_code)] // TODO: Implement KYC validation
    kyc_requirements: KycRequirements,
    #[allow(dead_code)] // TODO: Implement additional business rules
    #[serde(flatten)]
    other_rules: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone)]
struct AgeVerificationRules {
    minimum_age: u8,
    maximum_age: u8,
    age_calculation: String,
}

#[derive(Debug, Deserialize, Clone)]
struct KycRequirements {
    #[allow(dead_code)] // TODO: Implement individual KYC processing
    individual: KycLevel,
    #[allow(dead_code)] // TODO: Implement business KYC processing
    business: KycLevel,
}

#[derive(Debug, Deserialize, Clone)]
struct KycLevel {
    #[allow(dead_code)] // TODO: Use required documents in KYC validation
    required_documents: Vec<String>,
    #[allow(dead_code)] // TODO: Use requirements for KYC compliance checks
    #[serde(flatten)]
    requirements: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone)]
struct SecurityPolicies {
    #[allow(dead_code)] // TODO: Implement security policy enforcement
    #[serde(flatten)]
    policies: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct ErrorHandling {
    #[allow(dead_code)] // TODO: Implement error handling rules
    #[serde(flatten)]
    rules: HashMap<String, Value>,
}

/// Customer onboarding platform that manages all data collection workflows
#[derive(Clone)]
struct CustomerOnboardingPlatform {
    onboarding_config: OnboardingConfig,
    validation_config: ValidationConfig,
    reference_data: HashMap<String, Vec<String>>,
}

impl CustomerOnboardingPlatform {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to find data directory in multiple locations for testing compatibility
        let data_paths = [
            "data",
            "examples/elicitation-server/data",
            "../elicitation-server/data",
        ];

        let data_dir = data_paths
            .iter()
            .find(|path| Path::new(path).join("onboarding_workflows.json").exists())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                warn!("Could not find data directory in any expected location, using fallback");
                "data".to_string()
            });

        // Load onboarding workflows configuration
        let onboarding_config = if Path::new(&format!("{}/onboarding_workflows.json", data_dir))
            .exists()
        {
            let content = fs::read_to_string(format!("{}/onboarding_workflows.json", data_dir))?;
            serde_json::from_str(&content)?
        } else {
            warn!("onboarding_workflows.json not found, using minimal fallback configuration");
            OnboardingConfig {
                customer_onboarding_workflows: HashMap::new(),
                compliance_forms: HashMap::new(),
                preference_collection: HashMap::new(),
                survey_templates: HashMap::new(),
            }
        };

        // Load validation rules configuration
        let validation_config =
            if Path::new(&format!("{}/validation_rules.yaml", data_dir)).exists() {
                let content = fs::read_to_string(format!("{}/validation_rules.yaml", data_dir))?;
                serde_yml::from_str(&content)?
            } else {
                warn!("validation_rules.yaml not found, using minimal fallback configuration");
                ValidationConfig {
                    validation_rules: ValidationRules {
                        field_types: HashMap::new(),
                        business_rules: BusinessRules {
                            age_verification: AgeVerificationRules {
                                minimum_age: 18,
                                maximum_age: 150,
                                age_calculation: "from_birth_date".to_string(),
                            },
                            kyc_requirements: KycRequirements {
                                individual: KycLevel {
                                    required_documents: vec!["government_id".to_string()],
                                    requirements: HashMap::new(),
                                },
                                business: KycLevel {
                                    required_documents: vec!["business_registration".to_string()],
                                    requirements: HashMap::new(),
                                },
                            },
                            other_rules: HashMap::new(),
                        },
                        security_policies: SecurityPolicies {
                            policies: HashMap::new(),
                        },
                        error_handling: ErrorHandling {
                            rules: HashMap::new(),
                        },
                    },
                }
            };

        // Load reference data (parse from markdown file)
        let reference_data = Self::load_reference_data();

        Ok(Self {
            onboarding_config,
            validation_config,
            reference_data,
        })
    }

    fn load_reference_data() -> HashMap<String, Vec<String>> {
        let mut reference_data = HashMap::new();

        // In a real implementation, this would parse the markdown file
        // For now, provide fallback data
        reference_data.insert(
            "us_states_and_provinces".to_string(),
            vec![
                "Alabama".to_string(),
                "Alaska".to_string(),
                "Arizona".to_string(),
                "Arkansas".to_string(),
                "California".to_string(),
                "Colorado".to_string(),
                "Connecticut".to_string(),
                "Delaware".to_string(),
                "Florida".to_string(),
                "Georgia".to_string(),
                "Hawaii".to_string(),
                "Idaho".to_string(),
                // Abbreviated for space - in real implementation would load full list
            ],
        );

        reference_data.insert(
            "supported_countries".to_string(),
            vec![
                "United States".to_string(),
                "Canada".to_string(),
                "United Kingdom".to_string(),
                "Australia".to_string(),
                "Germany".to_string(),
                "France".to_string(),
                "Italy".to_string(),
                "Spain".to_string(),
                "Netherlands".to_string(),
                "Belgium".to_string(),
            ],
        );

        reference_data.insert(
            "world_timezones".to_string(),
            vec![
                "Pacific/Honolulu (UTC-10) - Hawaii".to_string(),
                "America/Los_Angeles (UTC-8) - Pacific Time".to_string(),
                "America/Denver (UTC-7) - Mountain Time".to_string(),
                "America/Chicago (UTC-6) - Central Time".to_string(),
                "America/New_York (UTC-5) - Eastern Time".to_string(),
                "Europe/London (UTC+0/+1) - UK Time".to_string(),
                "Europe/Paris (UTC+1) - Central European Time".to_string(),
                "Asia/Tokyo (UTC+9) - Japan Time".to_string(),
            ],
        );

        reference_data.insert(
            "naics_industries".to_string(),
            vec![
                "Software Publishers".to_string(),
                "Computer Systems Design and Related Services".to_string(),
                "Data Processing, Hosting, and Related Services".to_string(),
                "Internet Publishing and Broadcasting".to_string(),
                "Telecommunications".to_string(),
                "Finance and Insurance".to_string(),
                "Health Care and Social Assistance".to_string(),
                "Manufacturing".to_string(),
                "Retail Trade".to_string(),
                "Professional, Scientific, and Technical Services".to_string(),
            ],
        );

        reference_data
    }

    fn build_form_schema(&self, fields: &[FormField]) -> ToolSchema {
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        for field in fields {
            let field_schema = match field.field_type.as_str() {
                "string" | "text" => JsonSchema::string().with_description("Text input"),
                "email" => JsonSchema::string().with_description("Valid email address"),
                "phone" => JsonSchema::string()
                    .with_description("Phone number with international format (+1-555-123-4567)"),
                "date" => JsonSchema::string().with_description("Date in YYYY-MM-DD format"),
                "number" => JsonSchema::number().with_description("Numeric input"),
                "boolean" => JsonSchema::boolean(),
                "choice" => {
                    if let Some(choices) = field.choices.as_array() {
                        let choice_strings: Vec<String> = choices
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        JsonSchema::string_enum(choice_strings)
                    } else if let Some(choice_ref) = field.choices.as_str() {
                        // Reference to external choice data
                        if let Some(choices) = self.reference_data.get(choice_ref) {
                            JsonSchema::string_enum(choices.clone())
                        } else {
                            JsonSchema::string()
                        }
                    } else {
                        JsonSchema::string()
                    }
                }
                "multi_choice" => {
                    if let Some(choices) = field.choices.as_array() {
                        let choice_strings: Vec<String> = choices
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        JsonSchema::array(JsonSchema::string_enum(choice_strings))
                    } else {
                        JsonSchema::array(JsonSchema::string())
                    }
                }
                "password" => JsonSchema::string()
                    .with_description("Strong password meeting security requirements"),
                "text_area" => JsonSchema::string().with_description("Multi-line text input"),
                "file_upload" => {
                    JsonSchema::string().with_description("File upload path or reference")
                }
                _ => JsonSchema::string(),
            };

            let final_schema = if let Some(description) = &field.help_text {
                field_schema.with_description(description)
            } else {
                field_schema.with_description(&field.label)
            };

            properties.insert(field.name.clone(), final_schema);

            if field.required {
                required.push(field.name.clone());
            }
        }

        ToolSchema::object()
            .with_properties(properties)
            .with_required(required)
    }

    #[allow(dead_code)] // TODO: Use in workflow status reporting
    fn get_workflow_summary(&self, workflow_id: &str) -> Option<String> {
        self.onboarding_config
            .customer_onboarding_workflows
            .get(workflow_id)
            .map(|workflow| {
                format!(
                    "Workflow: {} ({})\nSteps: {}\nDescription: {}",
                    workflow.name,
                    workflow.workflow_id,
                    workflow.steps.len(),
                    workflow.description
                )
            })
    }
}

/// Tool for starting customer onboarding workflows
struct StartOnboardingWorkflowTool {
    platform: CustomerOnboardingPlatform,
}

// Implement fine-grained traits
impl HasBaseMetadata for StartOnboardingWorkflowTool {
    fn name(&self) -> &str {
        "start_onboarding_workflow"
    }

    fn title(&self) -> Option<&str> {
        Some("Start Onboarding Workflow")
    }
}

impl HasDescription for StartOnboardingWorkflowTool {
    fn description(&self) -> Option<&str> {
        Some("Start a customer onboarding workflow (personal or business account creation)")
    }
}

impl HasInputSchema for StartOnboardingWorkflowTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            properties.insert(
                "workflow_type".to_string(),
                JsonSchema::string_enum(vec![
                    "personal_account".to_string(),
                    "business_account".to_string(),
                ])
                .with_description("Type of account onboarding workflow"),
            );
            properties.insert(
                "step_index".to_string(),
                JsonSchema::number()
                    .with_minimum(0.0)
                    .with_description("Step index to start from (default: 0)"),
            );

            ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["workflow_type".to_string()])
        })
    }
}

impl HasOutputSchema for StartOnboardingWorkflowTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for StartOnboardingWorkflowTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for StartOnboardingWorkflowTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for StartOnboardingWorkflowTool {}
impl HasExecution for StartOnboardingWorkflowTool {}

// ToolDefinition automatically implemented via blanket impl!

#[async_trait]
impl McpTool for StartOnboardingWorkflowTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let workflow_type = args
            .get("workflow_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("workflow_type"))?;

        let step_index = args.get("step_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        if let Some(workflow) = self
            .platform
            .onboarding_config
            .customer_onboarding_workflows
            .get(workflow_type)
        {
            if step_index >= workflow.steps.len() {
                return Err(McpError::param_out_of_range(
                    "step_index",
                    &step_index.to_string(),
                    &format!("0-{}", workflow.steps.len() - 1),
                ));
            }

            let current_step = &workflow.steps[step_index];
            let schema = self.platform.build_form_schema(&current_step.fields);
            println!(
                "📋 Generated schema for step '{}' ({}): {} fields",
                current_step.title,
                current_step.description,
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            // Simplified elicitation demonstration (complex API migration in progress)
            let progress_token = format!("onboarding_{}_{}", workflow_type, Uuid::new_v4());

            let result = json!({
                "workflow_id": workflow.workflow_id,
                "workflow_type": workflow_type,
                "current_step": {
                    "index": step_index,
                    "id": current_step.step_id,
                    "title": current_step.title,
                    "description": current_step.description,
                    "total_steps": workflow.steps.len()
                },
                "progress_token": progress_token,
                "elicitation_request": {
                    "title": "Demo Elicitation",
                    "prompt": "Simplified demonstration",
                    "schema": "Form schema demo"
                },
                "field_count": current_step.fields.len(),
                "required_fields": current_step.fields.iter().filter(|f| f.required).count(),
                "next_step_available": step_index + 1 < workflow.steps.len(),
                "completion_actions": workflow.completion_actions
            });

            let summary = format!(
                "🚀 CUSTOMER ONBOARDING WORKFLOW STARTED\n\
                \n\
                Workflow: {} ({})\n\
                Current Step: {} of {} - {}\n\
                Progress Token: {}\n\
                \n\
                📋 Current Step Details:\n\
                • {} total fields\n\
                • {} required fields\n\
                • Step ID: {}\n\
                \n\
                🎯 Fields in this step:\n\
                {}\n\
                \n\
                ⚡ Real-world Implementation:\n\
                • Client renders form based on schema\n\
                • Validates input against business rules\n\
                • Supports file uploads and document verification\n\
                • Progress tracking across multi-step flow\n\
                • Accessibility compliance (WCAG 2.1 AA)\n\
                • GDPR/CCPA compliant data handling\n\
                \n\
                🔄 Next Steps:\n\
                {}\n\
                \n\
                📊 This demonstrates real-world customer onboarding patterns!",
                workflow.name,
                workflow.workflow_id,
                step_index + 1,
                workflow.steps.len(),
                current_step.title,
                progress_token,
                current_step.fields.len(),
                current_step.fields.iter().filter(|f| f.required).count(),
                current_step.step_id,
                current_step
                    .fields
                    .iter()
                    .map(|f| format!(
                        "  • {} ({}): {} {}",
                        f.name,
                        f.field_type,
                        if f.required { "Required" } else { "Optional" },
                        f.help_text.as_deref().unwrap_or("")
                    ))
                    .collect::<Vec<_>>()
                    .join("\n"),
                if step_index + 1 < workflow.steps.len() {
                    format!(
                        "Continue to step {}: {}",
                        step_index + 2,
                        workflow.steps[step_index + 1].title
                    )
                } else {
                    "Complete workflow and trigger completion actions".to_string()
                }
            );

            Ok(CallToolResult::success(vec![
                ToolResult::text(summary),
                ToolResult::text(format!(
                    "Workflow Data:\n{}",
                    serde_json::to_string_pretty(&result)?
                )),
            ]))
        } else {
            Err(McpError::invalid_param_type(
                "workflow_type",
                "personal_account|business_account",
                workflow_type,
            ))
        }
    }
}

/// Tool for handling compliance forms (GDPR, CCPA, etc.)
struct ComplianceFormTool {
    platform: CustomerOnboardingPlatform,
}

// Implement fine-grained traits
impl HasBaseMetadata for ComplianceFormTool {
    fn name(&self) -> &str {
        "compliance_form"
    }

    fn title(&self) -> Option<&str> {
        Some("Compliance Form Handler")
    }
}

impl HasDescription for ComplianceFormTool {
    fn description(&self) -> Option<&str> {
        Some(
            "Handle compliance forms for GDPR data requests, CCPA opt-outs, and other regulatory requirements",
        )
    }
}

impl HasInputSchema for ComplianceFormTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            properties.insert(
                "form_type".to_string(),
                JsonSchema::string_enum(vec![
                    "gdpr_data_request".to_string(),
                    "ccpa_opt_out".to_string(),
                ])
                .with_description("Type of compliance form to generate"),
            );

            ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["form_type".to_string()])
        })
    }
}

impl HasOutputSchema for ComplianceFormTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for ComplianceFormTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for ComplianceFormTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for ComplianceFormTool {}
impl HasExecution for ComplianceFormTool {}

// ToolDefinition automatically implemented via blanket impl!

#[async_trait]
impl McpTool for ComplianceFormTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let form_type = args
            .get("form_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("form_type"))?;

        if let Some(compliance_form) = self
            .platform
            .onboarding_config
            .compliance_forms
            .get(form_type)
        {
            let schema = self.platform.build_form_schema(&compliance_form.fields);
            println!(
                "📋 Generated compliance form schema: {} fields",
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            // Simplified compliance form demonstration
            let _form_demo = format!(
                "Compliance form: {} - {}",
                compliance_form.name, compliance_form.description
            );

            let compliance_info = match form_type {
                "gdpr_data_request" => {
                    "🇪🇺 GDPR DATA SUBJECT REQUEST\n\
                    \n\
                    This form enables EU residents to exercise their rights under GDPR:\n\
                    • Article 15: Right of access to personal data\n\
                    • Article 16: Right to rectification\n\
                    • Article 17: Right to erasure (\"right to be forgotten\")\n\
                    • Article 18: Right to restriction of processing\n\
                    • Article 20: Right to data portability\n\
                    • Article 21: Right to object to processing\n\
                    \n\
                    Legal Requirements:\n\
                    • Identity verification required\n\
                    • Response within 30 days (extendable to 60 days)\n\
                    • Must be free of charge (with exceptions)\n\
                    • Audit trail maintained for compliance"
                }
                "ccpa_opt_out" => {
                    "🇺🇸 CCPA DO NOT SELL REQUEST\n\
                    \n\
                    This form enables California residents to opt out under CCPA:\n\
                    • Right to know what personal information is collected\n\
                    • Right to delete personal information\n\
                    • Right to opt out of sale of personal information\n\
                    • Right to equal service and price\n\
                    \n\
                    California Legal Requirements:\n\
                    • Must process within 15 business days\n\
                    • Cannot discriminate against users who opt out\n\
                    • Must maintain \"Do Not Sell My Personal Information\" link\n\
                    • Audit trail for regulatory compliance"
                }
                _ => "Compliance form processing",
            };

            let result = json!({
                "form_type": form_type,
                "form_name": compliance_form.name,
                "request_id": Uuid::new_v4(),
                "elicitation_request": {
                    "title": "Demo Elicitation",
                    "prompt": "Simplified demonstration",
                    "schema": "Form schema demo"
                },
                "regulatory_framework": match form_type {
                    "gdpr_data_request" => "GDPR (General Data Protection Regulation)",
                    "ccpa_opt_out" => "CCPA (California Consumer Privacy Act)",
                    _ => "Data Protection Regulation"
                },
                "processing_time": match form_type {
                    "gdpr_data_request" => "30 days (extendable to 60 days)",
                    "ccpa_opt_out" => "15 business days",
                    _ => "Varies by regulation"
                }
            });

            let summary = format!(
                "📋 COMPLIANCE FORM: {}\n\
                \n\
                Request ID: {}\n\
                \n\
                {}\n\
                \n\
                🔧 Form Fields:\n\
                {}\n\
                \n\
                ⚖️ Legal Processing:\n\
                • Identity verification required\n\
                • Secure document handling\n\
                • Audit trail maintained\n\
                • Automated response workflows\n\
                • Integration with legal team\n\
                \n\
                🛡️ Privacy Protection:\n\
                • Data minimization principles\n\
                • Purpose limitation\n\
                • Storage limitation\n\
                • Transparency and accountability\n\
                \n\
                📊 This demonstrates regulatory compliance workflows!",
                compliance_form.name,
                Uuid::new_v4(),
                compliance_info,
                compliance_form
                    .fields
                    .iter()
                    .map(|f| format!(
                        "  • {} ({}): {}",
                        f.name,
                        f.field_type,
                        f.help_text.as_deref().unwrap_or(&f.label)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            Ok(CallToolResult::success(vec![
                ToolResult::text(summary),
                ToolResult::text(format!(
                    "Compliance Data:\n{}",
                    serde_json::to_string_pretty(&result)?
                )),
            ]))
        } else {
            Err(McpError::invalid_param_type(
                "form_type",
                "gdpr_data_request|ccpa_opt_out",
                form_type,
            ))
        }
    }
}

/// Tool for collecting user preferences and notification settings
struct PreferenceCollectionTool {
    platform: CustomerOnboardingPlatform,
}

impl HasBaseMetadata for PreferenceCollectionTool {
    fn name(&self) -> &str {
        "collect_user_preferences"
    }

    fn title(&self) -> Option<&str> {
        Some("Collect User Preferences")
    }
}

impl HasDescription for PreferenceCollectionTool {
    fn description(&self) -> Option<&str> {
        Some(
            "Collect user preferences for notifications, accessibility, and personalization settings",
        )
    }
}

impl HasInputSchema for PreferenceCollectionTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            properties.insert(
                "preference_type".to_string(),
                JsonSchema::string_enum(vec![
                    "notification_preferences".to_string(),
                    "accessibility_preferences".to_string(),
                ])
                .with_description("Type of preferences to collect"),
            );

            ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["preference_type".to_string()])
        })
    }
}

impl HasOutputSchema for PreferenceCollectionTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for PreferenceCollectionTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for PreferenceCollectionTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for PreferenceCollectionTool {}
impl HasExecution for PreferenceCollectionTool {}

#[async_trait]
impl McpTool for PreferenceCollectionTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let preference_type = args
            .get("preference_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("preference_type"))?;

        if let Some(preference_collection) = self
            .platform
            .onboarding_config
            .preference_collection
            .get(preference_type)
        {
            let schema = if !preference_collection.fields.is_empty() {
                // Simple field-based preferences
                self.platform
                    .build_form_schema(&preference_collection.fields)
            } else {
                // Category-based preferences (like notification preferences)
                let mut properties = HashMap::new();
                let mut required = Vec::new();

                for category in &preference_collection.categories {
                    for setting in &category.settings {
                        let field_name = format!(
                            "{}_{}",
                            category.category.to_lowercase().replace(" ", "_"),
                            setting.name
                        );
                        let channels_schema =
                            JsonSchema::array(JsonSchema::string_enum(setting.channels.clone()));
                        properties.insert(
                            field_name.clone(),
                            channels_schema.with_description(&setting.label),
                        );

                        if setting.required {
                            required.push(field_name);
                        }
                    }
                }

                ToolSchema::object()
                    .with_properties(properties)
                    .with_required(required)
            };
            println!(
                "📊 Generated preference schema for '{}': {} properties",
                preference_type,
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            // Simplified preference collection demonstration
            let _preference_demo = format!(
                "Preference collection: {} - {}",
                preference_collection.name, preference_collection.description
            );

            let preference_details = if preference_type == "notification_preferences" {
                let categories = &preference_collection.categories;
                format!(
                    "📱 NOTIFICATION PREFERENCE CATEGORIES:\n\
                    \n\
                    {}\n\
                    \n\
                    📢 Available Channels:\n\
                    • Email: Rich formatting, attachments, archive\n\
                    • SMS: High open rates, immediate delivery\n\
                    • Push: Real-time, interactive, contextual\n\
                    • In-app: Context-aware, no external dependencies\n\
                    \n\
                    ⚖️ Compliance Notes:\n\
                    • Marketing requires explicit consent\n\
                    • Security notifications cannot be disabled\n\
                    • Easy unsubscribe mechanisms provided\n\
                    • Granular control over frequency and content",
                    categories
                        .iter()
                        .map(|cat| format!(
                            "  {} - {}\n    {}",
                            cat.category,
                            cat.description,
                            cat.settings
                                .iter()
                                .map(|s| format!("    • {}: {}", s.label, s.channels.join(", ")))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ))
                        .collect::<Vec<_>>()
                        .join("\n\n")
                )
            } else {
                "🌐 ACCESSIBILITY PREFERENCES:\n\
                \n\
                This form helps customize the interface for accessibility:\n\
                • Visual: High contrast, large text, reduced motion\n\
                • Auditory: Screen reader compatibility\n\
                • Motor: Enhanced keyboard navigation\n\
                • Cognitive: Simplified interfaces, extended timeouts\n\
                \n\
                🛡️ WCAG 2.1 Compliance:\n\
                • Level AA conformance\n\
                • Assistive technology support\n\
                • Universal design principles\n\
                • Regular accessibility audits"
                    .to_string()
            };

            let result = json!({
                "preference_type": preference_type,
                "preference_name": preference_collection.name,
                "categories": preference_collection.categories.len(),
                "total_settings": preference_collection.categories.iter().map(|c| c.settings.len()).sum::<usize>(),
                "elicitation_request": {
                    "title": "Demo Elicitation",
                    "prompt": "Simplified demonstration",
                    "schema": "Form schema demo"
                }
            });

            let summary = format!(
                "⚙️ USER PREFERENCE COLLECTION: {}\n\
                \n\
                Request ID: {}\n\
                \n\
                {}\n\
                \n\
                🎛️ Preference Management Features:\n\
                • Granular control over settings\n\
                • Real-time preview of changes\n\
                • Bulk enable/disable options\n\
                • Import/export preference profiles\n\
                • History of preference changes\n\
                \n\
                📊 Analytics Integration:\n\
                • User engagement tracking\n\
                • A/B testing for optimal defaults\n\
                • Preference trend analysis\n\
                • Churn prediction based on settings\n\
                \n\
                🔄 This demonstrates comprehensive preference management!",
                preference_collection.name,
                Uuid::new_v4(),
                preference_details
            );

            Ok(CallToolResult::success(vec![
                ToolResult::text(summary),
                ToolResult::text(format!(
                    "Preference Data:\n{}",
                    serde_json::to_string_pretty(&result)?
                )),
            ]))
        } else {
            Err(McpError::invalid_param_type(
                "preference_type",
                "notification_preferences|accessibility_preferences",
                preference_type,
            ))
        }
    }
}

/// Tool for conducting customer satisfaction surveys
struct CustomerSurveyTool {
    platform: CustomerOnboardingPlatform,
}

impl HasBaseMetadata for CustomerSurveyTool {
    fn name(&self) -> &str {
        "customer_satisfaction_survey"
    }

    fn title(&self) -> Option<&str> {
        Some("Customer Satisfaction Survey")
    }
}

impl HasDescription for CustomerSurveyTool {
    fn description(&self) -> Option<&str> {
        Some("Conduct customer satisfaction surveys and feedback collection")
    }
}

impl HasInputSchema for CustomerSurveyTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            properties.insert(
                "survey_type".to_string(),
                JsonSchema::string_enum(vec!["customer_satisfaction".to_string()])
                    .with_description("Type of survey to conduct"),
            );
            properties.insert(
                "customer_segment".to_string(),
                JsonSchema::string_enum(vec![
                    "new_customer".to_string(),
                    "existing_customer".to_string(),
                    "premium_customer".to_string(),
                    "at_risk_customer".to_string(),
                ])
                .with_description("Customer segment for targeted survey"),
            );

            ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["survey_type".to_string()])
        })
    }
}

impl HasOutputSchema for CustomerSurveyTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for CustomerSurveyTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for CustomerSurveyTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for CustomerSurveyTool {}
impl HasExecution for CustomerSurveyTool {}

#[async_trait]
impl McpTool for CustomerSurveyTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let survey_type = args
            .get("survey_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("survey_type"))?;

        let customer_segment = args
            .get("customer_segment")
            .and_then(|v| v.as_str())
            .unwrap_or("existing_customer");

        if let Some(survey_template) = self
            .platform
            .onboarding_config
            .survey_templates
            .get(survey_type)
        {
            let schema = self.platform.build_form_schema(&survey_template.fields);
            println!(
                "📋 Generated survey schema for '{}': {} fields",
                survey_template.name,
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            let survey_title = format!(
                "{} - {}",
                survey_template.name,
                customer_segment.replace("_", " ")
            );
            // Simplified survey demonstration
            let _survey_demo =
                format!("Survey: {} - {}", survey_title, survey_template.description);

            let survey_id = format!("survey_{}_{}", survey_type, Uuid::new_v4());

            let result = json!({
                "survey_id": survey_id,
                "survey_type": survey_type,
                "customer_segment": customer_segment,
                "survey_name": survey_template.name,
                "expected_completion_time": "3-5 minutes",
                "incentive": match customer_segment {
                    "premium_customer" => "10% discount on next purchase",
                    "new_customer" => "$5 account credit",
                    "at_risk_customer" => "Priority support upgrade",
                    _ => "Entry into monthly prize drawing"
                },
                "elicitation_request": {
                    "title": "Demo Elicitation",
                    "prompt": "Simplified demonstration",
                    "schema": "Form schema demo"
                },
                "analytics_tracking": {
                    "nps_calculation": true,
                    "sentiment_analysis": true,
                    "trend_tracking": true,
                    "actionable_insights": true
                }
            });

            let survey_methodology = "📊 SURVEY METHODOLOGY:\n\
                \n\
                Rating Scales:\n\
                • Satisfaction: 1-5 Likert scale\n\
                • NPS: 0-10 likelihood to recommend\n\
                • Matrix Rating: Multiple aspects on same scale\n\
                \n\
                Data Collection Standards:\n\
                • GDPR compliant data processing\n\
                • Anonymous response options\n\
                • Secure data transmission\n\
                • Retention policy compliance\n\
                \n\
                Analytics Integration:\n\
                • Real-time dashboard updates\n\
                • Sentiment analysis processing\n\
                • Trend analysis and benchmarking\n\
                • Automated alert triggers\n\
                \n\
                Action Planning:\n\
                • Automatic ticket creation for issues\n\
                • Follow-up workflow triggers\n\
                • Executive summary generation\n\
                • Department-specific insights";

            let summary = format!(
                "📋 CUSTOMER SATISFACTION SURVEY\n\
                \n\
                Survey ID: {}\n\
                Type: {}\n\
                Segment: {}\n\
                Expected Duration: 3-5 minutes\n\
                \n\
                🎯 Survey Fields:\n\
                {}\n\
                \n\
                🎁 Incentive: {}\n\
                \n\
                {}\n\
                \n\
                📈 Business Impact:\n\
                • Customer retention insights\n\
                • Product improvement priorities\n\
                • Service quality metrics\n\
                • Competitive analysis data\n\
                \n\
                🔄 This demonstrates comprehensive feedback collection!",
                survey_id,
                survey_template.name,
                customer_segment.replace("_", " "),
                survey_template
                    .fields
                    .iter()
                    .map(|f| format!(
                        "  • {} ({}): {}",
                        f.name,
                        f.field_type,
                        f.help_text.as_deref().unwrap_or(&f.label)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n"),
                result["incentive"].as_str().unwrap_or("None"),
                survey_methodology
            );

            Ok(CallToolResult::success(vec![
                ToolResult::text(summary),
                ToolResult::text(format!(
                    "Survey Data:\n{}",
                    serde_json::to_string_pretty(&result)?
                )),
            ]))
        } else {
            Err(McpError::invalid_param_type(
                "survey_type",
                "customer_satisfaction",
                survey_type,
            ))
        }
    }
}

/// Tool for demonstrating data validation and business rules
struct DataValidationTool {
    platform: CustomerOnboardingPlatform,
}

impl HasBaseMetadata for DataValidationTool {
    fn name(&self) -> &str {
        "data_validation_demo"
    }

    fn title(&self) -> Option<&str> {
        Some("Data Validation Demo")
    }
}

impl HasDescription for DataValidationTool {
    fn description(&self) -> Option<&str> {
        Some("Demonstrate data validation rules, business logic, and compliance checks")
    }
}

impl HasInputSchema for DataValidationTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            properties.insert(
                "validation_category".to_string(),
                JsonSchema::string_enum(vec![
                    "field_validation".to_string(),
                    "business_rules".to_string(),
                    "security_policies".to_string(),
                    "compliance_checks".to_string(),
                ])
                .with_description("Category of validation to demonstrate"),
            );

            ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["validation_category".to_string()])
        })
    }
}

impl HasOutputSchema for DataValidationTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for DataValidationTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for DataValidationTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for DataValidationTool {}
impl HasExecution for DataValidationTool {}

#[async_trait]
impl McpTool for DataValidationTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let validation_category = args
            .get("validation_category")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("validation_category"))?;

        let validation_demo = match validation_category {
            "field_validation" => {
                "🔍 FIELD VALIDATION RULES:\n\
                \n\
                String Validation:\n\
                • Length constraints (min/max)\n\
                • Character set restrictions\n\
                • Pattern matching (regex)\n\
                • Whitespace normalization\n\
                \n\
                Email Validation:\n\
                • RFC 5322 format compliance\n\
                • Domain validation\n\
                • MX record checking\n\
                • Disposable email detection\n\
                • Deliverability testing\n\
                \n\
                Phone Validation:\n\
                • International format (+1-555-123-4567)\n\
                • Country code validation\n\
                • Number portability check\n\
                • Carrier identification\n\
                \n\
                Password Validation:\n\
                • Minimum 12 characters\n\
                • Mixed case requirements\n\
                • Number and symbol requirements\n\
                • Dictionary word detection\n\
                • Personal info detection\n\
                • Entropy calculation (50+ bits)\n\
                \n\
                Date Validation:\n\
                • Format standardization (ISO 8601)\n\
                • Range validation (1900-current)\n\
                • Business day calculations\n\
                • Timezone handling"
            }
            "business_rules" => {
                let age_rules = &self
                    .platform
                    .validation_config
                    .validation_rules
                    .business_rules
                    .age_verification;
                format!(
                    "⚖️ BUSINESS RULES VALIDATION:\n\
                    \n\
                    Age Verification:\n\
                    • Minimum age: {} years\n\
                    • Maximum age: {} years\n\
                    • Calculation method: {}\n\
                    • Document verification required\n\
                    \n\
                    KYC Requirements:\n\
                    Individual Accounts:\n\
                    • Government-issued photo ID\n\
                    • Proof of address (< 90 days)\n\
                    • Identity score threshold: 0.8+\n\
                    \n\
                    Business Accounts:\n\
                    • Business registration documents\n\
                    • Tax identification number\n\
                    • Authorized representative ID\n\
                    • KYB score threshold: 0.85+\n\
                    \n\
                    Data Quality Rules:\n\
                    • Duplicate detection algorithms\n\
                    • Address standardization\n\
                    • Name normalization\n\
                    • Data completeness scoring\n\
                    \n\
                    Transaction Limits:\n\
                    • Daily transaction limits\n\
                    • Monthly volume caps\n\
                    • Velocity checks\n\
                    • Risk-based adjustments",
                    age_rules.minimum_age, age_rules.maximum_age, age_rules.age_calculation
                )
                .leak()
            }
            "security_policies" => {
                "🔒 SECURITY POLICY VALIDATION:\n\
                \n\
                Authentication Policies:\n\
                • Password expiry: 90 days\n\
                • Failed login lockout: 5 attempts\n\
                • Lockout duration: 30 minutes\n\
                • Session timeout: 4 hours\n\
                • Concurrent session limits\n\
                \n\
                Two-Factor Authentication:\n\
                • Required for admin accounts\n\
                • Required for high-value transactions\n\
                • Supported methods: TOTP, SMS, Email\n\
                • Backup code generation\n\
                • Recovery procedures\n\
                \n\
                Data Encryption:\n\
                • At rest: AES-256 encryption\n\
                • In transit: TLS 1.3 minimum\n\
                • Key rotation: Annual schedule\n\
                • Hardware security modules\n\
                \n\
                Access Controls:\n\
                • Role-based access control (RBAC)\n\
                • Attribute-based access control (ABAC)\n\
                • Principle of least privilege\n\
                • Regular access reviews\n\
                • Privileged access management\n\
                \n\
                Audit and Monitoring:\n\
                • Comprehensive audit logging\n\
                • Real-time security monitoring\n\
                • Anomaly detection\n\
                • Incident response procedures"
            }
            "compliance_checks" => {
                "📋 COMPLIANCE VALIDATION:\n\
                \n\
                GDPR Compliance (EU):\n\
                • Lawful basis identification\n\
                • Consent management\n\
                • Data subject rights handling\n\
                • Data retention limits\n\
                • Cross-border transfer controls\n\
                • Breach notification (72 hours)\n\
                \n\
                CCPA Compliance (California):\n\
                • Consumer rights notifications\n\
                • Opt-out mechanisms\n\
                • Do not sell disclosures\n\
                • Non-discrimination policies\n\
                • Authorized agent procedures\n\
                \n\
                PCI DSS Compliance:\n\
                • Cardholder data protection\n\
                • Secure network transmission\n\
                • Vulnerability management\n\
                • Access control measures\n\
                • Network monitoring\n\
                • Security testing procedures\n\
                \n\
                HIPAA Compliance (Healthcare):\n\
                • Protected health information (PHI)\n\
                • Minimum necessary standard\n\
                • Administrative safeguards\n\
                • Physical safeguards\n\
                • Technical safeguards\n\
                • Business associate agreements\n\
                \n\
                Industry Standards:\n\
                • ISO 27001 information security\n\
                • SOC 2 Type II controls\n\
                • NIST Cybersecurity Framework\n\
                • CIS Critical Security Controls"
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "validation_category",
                    "field_validation|business_rules|security_policies|compliance_checks",
                    validation_category,
                ));
            }
        };

        let result = json!({
            "validation_category": validation_category,
            "demonstration_id": Uuid::new_v4(),
            "validation_rules_loaded": !self.platform.validation_config.validation_rules.field_types.is_empty(),
            "business_rules_active": true,
            "compliance_frameworks": [
                "GDPR", "CCPA", "PCI DSS", "HIPAA", "SOX", "FERPA"
            ],
            "validation_services": [
                "email_validator_api",
                "phone_validator_api",
                "address_validation_api",
                "identity_verification_api",
                "document_verification_api"
            ]
        });

        let summary = format!(
            "🛡️ DATA VALIDATION DEMONSTRATION\n\
            \n\
            Category: {}\n\
            Demonstration ID: {}\n\
            \n\
            {}\n\
            \n\
            🔧 Implementation Features:\n\
            • Real-time validation feedback\n\
            • Progressive enhancement\n\
            • Graceful degradation\n\
            • Internationalization support\n\
            • Accessibility compliance\n\
            \n\
            📊 Validation Pipeline:\n\
            1. Client-side validation (immediate feedback)\n\
            2. Server-side validation (security/business rules)\n\
            3. Third-party API validation (email/phone/address)\n\
            4. Compliance rule checking\n\
            5. Business logic validation\n\
            6. Data quality scoring\n\
            \n\
            🚀 This demonstrates enterprise-grade validation systems!",
            validation_category.replace("_", " "),
            Uuid::new_v4(),
            validation_demo
        );

        Ok(CallToolResult::success(vec![
            ToolResult::text(summary),
            ToolResult::text(format!(
                "Validation Data:\n{}",
                serde_json::to_string_pretty(&result)?
            )),
        ]))
    }
}

#[derive(Parser)]
#[command(name = "elicitation-server")]
#[command(about = "MCP Elicitation Test Server - Customer Onboarding Platform")]
struct Args {
    /// Port to run the server on (0 = random port assigned by OS)
    #[arg(short, long, default_value = "0")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    // Use specified port or OS ephemeral allocation if 0
    let port = if args.port == 0 {
        // Use OS ephemeral port allocation - reliable for parallel testing
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind to ephemeral port: {}", e))?;
        let port = listener.local_addr()?.port();
        drop(listener); // Release immediately so server can bind to it
        port
    } else {
        args.port
    };

    info!(
        "🚀 Starting Customer Onboarding and Data Collection Platform on port {}",
        port
    );
    info!("📡 Server URL: http://127.0.0.1:{}/mcp", port);

    // Initialize the platform with external configuration
    let platform = CustomerOnboardingPlatform::new()?;

    let server = McpServer::builder()
        .name("customer-onboarding-platform")
        .version("2.0.0")
        .title("Customer Onboarding and Data Collection Platform")
        .instructions("This platform provides comprehensive customer onboarding workflows, compliance forms, preference collection, and survey capabilities using MCP elicitation. All workflows are driven by external configuration files and demonstrate real-world data collection patterns.")
        .tool(StartOnboardingWorkflowTool { platform: platform.clone() })
        .tool(ComplianceFormTool { platform: platform.clone() })
        .tool(PreferenceCollectionTool { platform: platform.clone() })
        .tool(CustomerSurveyTool { platform: platform.clone() })
        .tool(DataValidationTool { platform })
        .with_elicitation() // Enable elicitation support
        .bind_address(format!("127.0.0.1:{}", port).parse()?)
        .build()?;

    info!(
        "🌐 Customer Onboarding Platform running at: http://127.0.0.1:{}/mcp",
        port
    );
    info!("");
    info!("🏢 Real-world Use Cases:");
    info!("  👤 Personal account onboarding with KYC verification");
    info!("  🏛️  Business account onboarding with compliance checks");
    info!("  📋 GDPR/CCPA compliance forms and data subject requests");
    info!("  ⚙️  User preference and notification settings management");
    info!("  📊 Customer satisfaction surveys and feedback collection");
    info!("  🛡️  Comprehensive data validation and business rules");
    info!("");
    info!("🔧 Available tools:");
    info!("  🚀 start_onboarding_workflow - Multi-step customer onboarding");
    info!("  📋 compliance_form - GDPR/CCPA regulatory compliance");
    info!("  ⚙️  collect_user_preferences - Notification and accessibility settings");
    info!("  📊 customer_satisfaction_survey - Feedback and NPS collection");
    info!("  🛡️  data_validation_demo - Validation rules and business logic");
    info!("");
    info!("📂 External Configuration:");
    info!("  📄 data/onboarding_workflows.json - Workflow definitions and forms");
    info!("  ⚙️  data/validation_rules.yaml - Business rules and validation logic");
    info!("  📚 data/reference_data.md - Geographic and industry reference data");
    info!("");
    info!("🌟 Key Features:");
    info!("  ✨ Schema-driven form generation from external config");
    info!("  🔒 Multi-layered validation (client/server/API/compliance)");
    info!("  🌍 Internationalization and accessibility support");
    info!("  📊 Progress tracking for complex multi-step workflows");
    info!("  ⚖️  Regulatory compliance (GDPR, CCPA, PCI DSS, HIPAA)");
    info!("  🎯 Customer segmentation and personalized experiences");
    info!("");
    info!("📖 Example usage:");
    info!("  curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("    -H 'Content-Type: application/json' \\");
    info!(
        "    -d '{{\"method\": \"tools/call\", \"params\": {{\"name\": \"start_onboarding_workflow\", \"arguments\": {{\"workflow_type\": \"personal_account\"}}}}}}'"
    );

    server.run().await?;
    Ok(())
}
