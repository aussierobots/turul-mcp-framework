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

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::tools::ToolSchema;
use turul_mcp_protocol::{McpError, McpResult, schema::JsonSchema};
use turul_mcp_server::{McpServer, SessionContext};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};
use uuid::Uuid;

/// Shared platform instance accessible by tools via OnceLock
static PLATFORM: OnceLock<CustomerOnboardingPlatform> = OnceLock::new();

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

fn get_platform() -> McpResult<&'static CustomerOnboardingPlatform> {
    PLATFORM
        .get()
        .ok_or_else(|| McpError::tool_execution("Platform not initialized"))
}

/// Tool for starting customer onboarding workflows
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "start_onboarding_workflow",
    description = "Start a customer onboarding workflow (personal or business account creation)"
)]
pub struct StartOnboardingWorkflowTool {
    #[param(description = "Type of account onboarding workflow")]
    pub workflow_type: String,

    #[param(description = "Step index to start from (default: 0)", optional)]
    pub step_index: Option<u64>,
}

impl StartOnboardingWorkflowTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let platform = get_platform()?;
        let step_index = self.step_index.unwrap_or(0) as usize;

        if let Some(workflow) = platform
            .onboarding_config
            .customer_onboarding_workflows
            .get(&self.workflow_type)
        {
            if step_index >= workflow.steps.len() {
                return Err(McpError::param_out_of_range(
                    "step_index",
                    &step_index.to_string(),
                    &format!("0-{}", workflow.steps.len() - 1),
                ));
            }

            let current_step = &workflow.steps[step_index];
            let schema = platform.build_form_schema(&current_step.fields);
            println!(
                "Generated schema for step '{}' ({}): {} fields",
                current_step.title,
                current_step.description,
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            // Simplified elicitation demonstration (complex API migration in progress)
            let progress_token = format!("onboarding_{}_{}", self.workflow_type, Uuid::new_v4());

            let result = json!({
                "workflow_id": workflow.workflow_id,
                "workflow_type": self.workflow_type,
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
                "completion_actions": workflow.completion_actions,
                "summary": format!(
                    "CUSTOMER ONBOARDING WORKFLOW STARTED\n\
                    Workflow: {} ({})\n\
                    Current Step: {} of {} - {}\n\
                    Progress Token: {}\n\
                    {} total fields, {} required fields\n\
                    Step ID: {}\n\
                    Fields: {}\n\
                    Next: {}",
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
                            "{} ({}): {} {}",
                            f.name,
                            f.field_type,
                            if f.required { "Required" } else { "Optional" },
                            f.help_text.as_deref().unwrap_or("")
                        ))
                        .collect::<Vec<_>>()
                        .join("; "),
                    if step_index + 1 < workflow.steps.len() {
                        format!(
                            "Continue to step {}: {}",
                            step_index + 2,
                            workflow.steps[step_index + 1].title
                        )
                    } else {
                        "Complete workflow and trigger completion actions".to_string()
                    }
                )
            });

            Ok(result)
        } else {
            Err(McpError::invalid_param_type(
                "workflow_type",
                "personal_account|business_account",
                &self.workflow_type,
            ))
        }
    }
}

/// Tool for handling compliance forms (GDPR, CCPA, etc.)
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "compliance_form",
    description = "Handle compliance forms for GDPR data requests, CCPA opt-outs, and other regulatory requirements"
)]
pub struct ComplianceFormTool {
    #[param(description = "Type of compliance form to generate")]
    pub form_type: String,
}

impl ComplianceFormTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let platform = get_platform()?;

        if let Some(compliance_form) = platform
            .onboarding_config
            .compliance_forms
            .get(&self.form_type)
        {
            let schema = platform.build_form_schema(&compliance_form.fields);
            println!(
                "Generated compliance form schema: {} fields",
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            let compliance_info = match self.form_type.as_str() {
                "gdpr_data_request" => {
                    "GDPR DATA SUBJECT REQUEST\n\
                    This form enables EU residents to exercise their rights under GDPR:\n\
                    Article 15: Right of access to personal data\n\
                    Article 16: Right to rectification\n\
                    Article 17: Right to erasure (\"right to be forgotten\")\n\
                    Article 18: Right to restriction of processing\n\
                    Article 20: Right to data portability\n\
                    Article 21: Right to object to processing\n\
                    Legal Requirements:\n\
                    Identity verification required\n\
                    Response within 30 days (extendable to 60 days)\n\
                    Must be free of charge (with exceptions)\n\
                    Audit trail maintained for compliance"
                }
                "ccpa_opt_out" => {
                    "CCPA DO NOT SELL REQUEST\n\
                    This form enables California residents to opt out under CCPA:\n\
                    Right to know what personal information is collected\n\
                    Right to delete personal information\n\
                    Right to opt out of sale of personal information\n\
                    Right to equal service and price\n\
                    California Legal Requirements:\n\
                    Must process within 15 business days\n\
                    Cannot discriminate against users who opt out\n\
                    Must maintain \"Do Not Sell My Personal Information\" link\n\
                    Audit trail for regulatory compliance"
                }
                _ => "Compliance form processing",
            };

            let result = json!({
                "form_type": self.form_type,
                "form_name": compliance_form.name,
                "request_id": Uuid::new_v4().to_string(),
                "elicitation_request": {
                    "title": "Demo Elicitation",
                    "prompt": "Simplified demonstration",
                    "schema": "Form schema demo"
                },
                "regulatory_framework": match self.form_type.as_str() {
                    "gdpr_data_request" => "GDPR (General Data Protection Regulation)",
                    "ccpa_opt_out" => "CCPA (California Consumer Privacy Act)",
                    _ => "Data Protection Regulation"
                },
                "processing_time": match self.form_type.as_str() {
                    "gdpr_data_request" => "30 days (extendable to 60 days)",
                    "ccpa_opt_out" => "15 business days",
                    _ => "Varies by regulation"
                },
                "compliance_info": compliance_info,
                "fields": compliance_form
                    .fields
                    .iter()
                    .map(|f| json!({
                        "name": f.name,
                        "type": f.field_type,
                        "help": f.help_text.as_deref().unwrap_or(&f.label)
                    }))
                    .collect::<Vec<_>>()
            });

            Ok(result)
        } else {
            Err(McpError::invalid_param_type(
                "form_type",
                "gdpr_data_request|ccpa_opt_out",
                &self.form_type,
            ))
        }
    }
}

/// Tool for collecting user preferences and notification settings
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "collect_user_preferences",
    description = "Collect user preferences for notifications, accessibility, and personalization settings"
)]
pub struct PreferenceCollectionTool {
    #[param(description = "Type of preferences to collect")]
    pub preference_type: String,
}

impl PreferenceCollectionTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let platform = get_platform()?;

        if let Some(preference_collection) = platform
            .onboarding_config
            .preference_collection
            .get(&self.preference_type)
        {
            let schema = if !preference_collection.fields.is_empty() {
                // Simple field-based preferences
                platform.build_form_schema(&preference_collection.fields)
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
                "Generated preference schema for '{}': {} properties",
                self.preference_type,
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            let result = json!({
                "preference_type": self.preference_type,
                "preference_name": preference_collection.name,
                "description": preference_collection.description,
                "categories": preference_collection.categories.len(),
                "total_settings": preference_collection.categories.iter().map(|c| c.settings.len()).sum::<usize>(),
                "request_id": Uuid::new_v4().to_string(),
                "elicitation_request": {
                    "title": "Demo Elicitation",
                    "prompt": "Simplified demonstration",
                    "schema": "Form schema demo"
                },
                "category_details": preference_collection.categories.iter().map(|cat| {
                    json!({
                        "category": cat.category,
                        "description": cat.description,
                        "settings": cat.settings.iter().map(|s| {
                            json!({
                                "label": s.label,
                                "channels": s.channels
                            })
                        }).collect::<Vec<_>>()
                    })
                }).collect::<Vec<_>>()
            });

            Ok(result)
        } else {
            Err(McpError::invalid_param_type(
                "preference_type",
                "notification_preferences|accessibility_preferences",
                &self.preference_type,
            ))
        }
    }
}

/// Tool for conducting customer satisfaction surveys
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "customer_satisfaction_survey",
    description = "Conduct customer satisfaction surveys and feedback collection"
)]
pub struct CustomerSurveyTool {
    #[param(description = "Type of survey to conduct")]
    pub survey_type: String,

    #[param(description = "Customer segment for targeted survey", optional)]
    pub customer_segment: Option<String>,
}

impl CustomerSurveyTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let platform = get_platform()?;
        let customer_segment = self.customer_segment.as_deref().unwrap_or("existing_customer");

        if let Some(survey_template) = platform
            .onboarding_config
            .survey_templates
            .get(&self.survey_type)
        {
            let schema = platform.build_form_schema(&survey_template.fields);
            println!(
                "Generated survey schema for '{}': {} fields",
                survey_template.name,
                schema.properties.as_ref().map_or(0, |p| p.len())
            );

            let survey_id = format!("survey_{}_{}", self.survey_type, Uuid::new_v4());

            let result = json!({
                "survey_id": survey_id,
                "survey_type": self.survey_type,
                "customer_segment": customer_segment,
                "survey_name": survey_template.name,
                "description": survey_template.description,
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
                },
                "fields": survey_template
                    .fields
                    .iter()
                    .map(|f| json!({
                        "name": f.name,
                        "type": f.field_type,
                        "help": f.help_text.as_deref().unwrap_or(&f.label)
                    }))
                    .collect::<Vec<_>>()
            });

            Ok(result)
        } else {
            Err(McpError::invalid_param_type(
                "survey_type",
                "customer_satisfaction",
                &self.survey_type,
            ))
        }
    }
}

/// Tool for demonstrating data validation and business rules
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "data_validation_demo",
    description = "Demonstrate data validation rules, business logic, and compliance checks"
)]
pub struct DataValidationTool {
    #[param(description = "Category of validation to demonstrate")]
    pub validation_category: String,
}

impl DataValidationTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let platform = get_platform()?;

        let validation_demo = match self.validation_category.as_str() {
            "field_validation" => {
                "FIELD VALIDATION RULES:\n\
                String Validation: Length constraints, character set restrictions, pattern matching, whitespace normalization\n\
                Email Validation: RFC 5322 format, domain validation, MX record checking, disposable email detection\n\
                Phone Validation: International format, country code validation, number portability check\n\
                Password Validation: Minimum 12 chars, mixed case, numbers and symbols, entropy calculation (50+ bits)\n\
                Date Validation: ISO 8601 format, range validation, business day calculations, timezone handling"
                    .to_string()
            }
            "business_rules" => {
                let age_rules = &platform
                    .validation_config
                    .validation_rules
                    .business_rules
                    .age_verification;
                format!(
                    "BUSINESS RULES VALIDATION:\n\
                    Age Verification: min={}, max={}, method={}\n\
                    KYC Requirements: Government ID, proof of address, identity score threshold\n\
                    Data Quality: Duplicate detection, address standardization, name normalization\n\
                    Transaction Limits: Daily limits, monthly caps, velocity checks",
                    age_rules.minimum_age, age_rules.maximum_age, age_rules.age_calculation
                )
            }
            "security_policies" => {
                "SECURITY POLICY VALIDATION:\n\
                Authentication: Password expiry 90 days, lockout after 5 attempts, session timeout 4 hours\n\
                Two-Factor: Required for admin, TOTP/SMS/Email methods, backup codes\n\
                Encryption: AES-256 at rest, TLS 1.3 in transit, annual key rotation\n\
                Access Controls: RBAC, ABAC, least privilege, regular reviews"
                    .to_string()
            }
            "compliance_checks" => {
                "COMPLIANCE VALIDATION:\n\
                GDPR: Consent management, data subject rights, retention limits, breach notification (72h)\n\
                CCPA: Consumer rights, opt-out mechanisms, non-discrimination\n\
                PCI DSS: Cardholder data protection, secure transmission, vulnerability management\n\
                HIPAA: PHI protection, minimum necessary standard, safeguards\n\
                Standards: ISO 27001, SOC 2 Type II, NIST CSF, CIS Controls"
                    .to_string()
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "validation_category",
                    "field_validation|business_rules|security_policies|compliance_checks",
                    &self.validation_category,
                ));
            }
        };

        let result = json!({
            "validation_category": self.validation_category,
            "demonstration_id": Uuid::new_v4().to_string(),
            "validation_rules_loaded": !platform.validation_config.validation_rules.field_types.is_empty(),
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
            ],
            "validation_details": validation_demo
        });

        Ok(result)
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
        "Starting Customer Onboarding and Data Collection Platform on port {}",
        port
    );
    info!("Server URL: http://127.0.0.1:{}/mcp", port);

    // Initialize the platform with external configuration
    let platform = CustomerOnboardingPlatform::new()?;
    PLATFORM.set(platform).ok();

    let server = McpServer::builder()
        .name("customer-onboarding-platform")
        .version("2.0.0")
        .title("Customer Onboarding and Data Collection Platform")
        .instructions("This platform provides comprehensive customer onboarding workflows, compliance forms, preference collection, and survey capabilities using MCP elicitation. All workflows are driven by external configuration files and demonstrate real-world data collection patterns.")
        .tool(StartOnboardingWorkflowTool::default())
        .tool(ComplianceFormTool::default())
        .tool(PreferenceCollectionTool::default())
        .tool(CustomerSurveyTool::default())
        .tool(DataValidationTool::default())
        .with_elicitation() // Enable elicitation support
        .bind_address(format!("127.0.0.1:{}", port).parse()?)
        .build()?;

    info!(
        "Customer Onboarding Platform running at: http://127.0.0.1:{}/mcp",
        port
    );
    info!("");
    info!("Real-world Use Cases:");
    info!("  Personal account onboarding with KYC verification");
    info!("  Business account onboarding with compliance checks");
    info!("  GDPR/CCPA compliance forms and data subject requests");
    info!("  User preference and notification settings management");
    info!("  Customer satisfaction surveys and feedback collection");
    info!("  Comprehensive data validation and business rules");
    info!("");
    info!("Available tools:");
    info!("  start_onboarding_workflow - Multi-step customer onboarding");
    info!("  compliance_form - GDPR/CCPA regulatory compliance");
    info!("  collect_user_preferences - Notification and accessibility settings");
    info!("  customer_satisfaction_survey - Feedback and NPS collection");
    info!("  data_validation_demo - Validation rules and business logic");
    info!("");
    info!("External Configuration:");
    info!("  data/onboarding_workflows.json - Workflow definitions and forms");
    info!("  data/validation_rules.yaml - Business rules and validation logic");
    info!("  data/reference_data.md - Geographic and industry reference data");
    info!("");
    info!("Example usage:");
    info!("  curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("    -H 'Content-Type: application/json' \\");
    info!(
        "    -d '{{\"method\": \"tools/call\", \"params\": {{\"name\": \"start_onboarding_workflow\", \"arguments\": {{\"workflow_type\": \"personal_account\"}}}}}}'"
    );

    server.run().await?;
    Ok(())
}
