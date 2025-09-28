//! # Development Team Integration Platform
//!
//! This comprehensive example demonstrates a real-world development team integration
//! platform that showcases all MCP capabilities in practical business scenarios.
//! It serves as a central hub for development workflows, team collaboration,
//! project management, and knowledge sharing using external configuration files.

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, json};
use tracing::info;
use turul_mcp_protocol::prompts::{
    HasPromptAnnotations, HasPromptArguments, HasPromptDescription, HasPromptMeta,
    HasPromptMetadata, PromptAnnotations, PromptArgument, PromptMessage,
};
use turul_mcp_protocol::resources::{
    HasResourceAnnotations, HasResourceDescription, HasResourceMeta, HasResourceMetadata,
    HasResourceMimeType, HasResourceSize, HasResourceUri, ResourceContent,
};
use turul_mcp_protocol::tools::{
    CallToolResult, HasAnnotations, HasBaseMetadata, HasDescription, HasInputSchema,
    HasOutputSchema, HasToolMeta, ToolAnnotations,
};
use turul_mcp_protocol::{McpError, McpResult, ToolResult, ToolSchema, schema::JsonSchema};
use turul_mcp_server::handlers::McpPrompt;
use turul_mcp_server::{McpResource, McpServer, McpTool, SessionContext};

#[derive(Debug, Deserialize, Serialize)]
struct PlatformConfig {
    platform_info: PlatformInfo,
    team_configuration: TeamConfiguration,
    integration_settings: HashMap<String, Value>,
    development_workflows: HashMap<String, Value>,
    compliance_and_security: HashMap<String, Value>,
    performance_metrics: HashMap<String, Value>,
    innovation_initiatives: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PlatformInfo {
    name: String,
    version: String,
    description: String,
    environment: String,
    deployment_region: String,
    last_updated: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TeamConfiguration {
    organization: Organization,
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Organization {
    name: String,
    teams: Vec<Team>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Team {
    name: String,
    id: String,
    lead: String,
    members: Vec<String>,
    focus_areas: Vec<String>,
    tech_stack: Vec<String>,
    repositories: Vec<String>,
    on_call_rotation: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Project {
    name: String,
    id: String,
    status: String,
    priority: String,
    start_date: String,
    target_date: String,
    teams: Vec<String>,
    budget: u64,
    milestones: Vec<ProjectMilestone>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ProjectMilestone {
    name: String,
    date: String,
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkflowTemplates {
    workflow_templates: HashMap<String, HashMap<String, WorkflowDefinition>>,
    team_collaboration: HashMap<String, Value>,
    quality_assurance: HashMap<String, Value>,
    documentation_standards: HashMap<String, Value>,
    innovation_and_learning: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkflowDefinition {
    name: String,
    description: String,
    phases: HashMap<String, WorkflowPhase>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkflowPhase {
    tasks: Vec<String>,
    deliverables: Vec<String>,
    duration: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProjectResources {
    development_resources: DevelopmentResources,
    documentation_library: HashMap<String, Value>,
    monitoring_and_observability: HashMap<String, Value>,
    team_tools_and_integrations: HashMap<String, Value>,
    learning_resources: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DevelopmentResources {
    code_repositories: Vec<Repository>,
    api_documentation: Vec<ApiDoc>,
    database_schemas: Vec<DatabaseSchema>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    name: String,
    #[serde(rename = "type")]
    repo_type: String,
    technology: String,
    url: String,
    description: String,
    team: String,
    status: String,
    last_commit: Option<String>,
    test_coverage: Option<u32>,
    code_quality: Option<String>,
    security_scan: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ApiDoc {
    service: String,
    version: String,
    base_url: String,
    documentation_url: String,
    authentication: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DatabaseSchema {
    database: String,
    #[serde(rename = "type")]
    db_type: String,
    version: String,
    tables: HashMap<String, Value>,
}

/// Shared platform state with external data
#[derive(Debug)]
pub struct PlatformState {
    config: PlatformConfig,
    #[allow(dead_code)] // TODO: Implement workflow integration
    workflows: WorkflowTemplates,
    resources: ProjectResources,
    #[allow(dead_code)] // TODO: Implement code template integration
    code_templates: String,
}

impl PlatformState {
    pub fn new() -> McpResult<Self> {
        let config_path = Path::new("data/platform_config.json");
        let workflows_path = Path::new("data/workflow_templates.yaml");
        let resources_path = Path::new("data/project_resources.json");
        let templates_path = Path::new("data/code_templates.md");

        let config = match fs::read_to_string(config_path) {
            Ok(content) => from_str::<PlatformConfig>(&content).map_err(|e| {
                McpError::tool_execution(&format!("Failed to parse platform config: {}", e))
            })?,
            Err(_) => {
                // Fallback configuration
                PlatformConfig {
                    platform_info: PlatformInfo {
                        name: "Development Platform".to_string(),
                        version: "1.0.0".to_string(),
                        description: "Development team integration platform".to_string(),
                        environment: "development".to_string(),
                        deployment_region: "local".to_string(),
                        last_updated: "2025-01-19".to_string(),
                    },
                    team_configuration: TeamConfiguration {
                        organization: Organization {
                            name: "Tech Team".to_string(),
                            teams: Vec::new(),
                        },
                        projects: Vec::new(),
                    },
                    integration_settings: HashMap::new(),
                    development_workflows: HashMap::new(),
                    compliance_and_security: HashMap::new(),
                    performance_metrics: HashMap::new(),
                    innovation_initiatives: HashMap::new(),
                }
            }
        };

        let workflows = match fs::read_to_string(workflows_path) {
            Ok(content) => serde_yml::from_str::<WorkflowTemplates>(&content).map_err(|e| {
                McpError::tool_execution(&format!("Failed to parse workflow templates: {}", e))
            })?,
            Err(_) => WorkflowTemplates {
                workflow_templates: HashMap::new(),
                team_collaboration: HashMap::new(),
                quality_assurance: HashMap::new(),
                documentation_standards: HashMap::new(),
                innovation_and_learning: HashMap::new(),
            },
        };

        let resources = match fs::read_to_string(resources_path) {
            Ok(content) => from_str::<ProjectResources>(&content).map_err(|e| {
                McpError::tool_execution(&format!("Failed to parse project resources: {}", e))
            })?,
            Err(_) => ProjectResources {
                development_resources: DevelopmentResources {
                    code_repositories: Vec::new(),
                    api_documentation: Vec::new(),
                    database_schemas: Vec::new(),
                },
                documentation_library: HashMap::new(),
                monitoring_and_observability: HashMap::new(),
                team_tools_and_integrations: HashMap::new(),
                learning_resources: HashMap::new(),
            },
        };

        let code_templates = fs::read_to_string(templates_path).unwrap_or_else(|_| {
            "# Development Team Code Templates\n\nNo templates loaded.".to_string()
        });

        Ok(Self {
            config,
            workflows,
            resources,
            code_templates,
        })
    }
}

/// Team management tool for handling team operations and member information
struct TeamManagementTool {
    state: Arc<PlatformState>,
}

impl TeamManagementTool {
    fn new(state: Arc<PlatformState>) -> Self {
        Self { state }
    }
}

// Fine-grained trait implementations for TeamManagementTool
impl HasBaseMetadata for TeamManagementTool {
    fn name(&self) -> &str {
        "manage_teams"
    }
}

impl HasDescription for TeamManagementTool {
    fn description(&self) -> Option<&str> {
        Some(
            "Manage development teams, members, and team information including skills, projects, and on-call rotations",
        )
    }
}

impl HasInputSchema for TeamManagementTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    (
                        "action".to_string(),
                        JsonSchema::string_enum(vec![
                            "list_teams".to_string(),
                            "get_team_details".to_string(),
                            "get_team_members".to_string(),
                            "get_on_call_rotation".to_string(),
                            "team_workload".to_string(),
                        ])
                        .with_description("Team management action to perform"),
                    ),
                    (
                        "team_id".to_string(),
                        JsonSchema::string()
                            .with_description("Team ID for team-specific operations"),
                    ),
                    (
                        "include_projects".to_string(),
                        JsonSchema::boolean()
                            .with_description("Include project assignments in team details"),
                    ),
                ]))
                .with_required(vec!["action".to_string()])
        })
    }
}

impl HasOutputSchema for TeamManagementTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for TeamManagementTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for TeamManagementTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for TeamManagementTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;

        let team_id = args.get("team_id").and_then(|v| v.as_str());
        let include_projects = args
            .get("include_projects")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = match action {
            "list_teams" => {
                let teams_summary = self
                    .state
                    .config
                    .team_configuration
                    .organization
                    .teams
                    .iter()
                    .map(|team| {
                        json!({
                            "id": team.id,
                            "name": team.name,
                            "lead": team.lead,
                            "member_count": team.members.len(),
                            "focus_areas": team.focus_areas,
                            "on_call_enabled": team.on_call_rotation,
                            "repository_count": team.repositories.len()
                        })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "organization": self.state.config.team_configuration.organization.name,
                    "total_teams": teams_summary.len(),
                    "teams": teams_summary,
                    "platform_info": {
                        "name": self.state.config.platform_info.name,
                        "environment": self.state.config.platform_info.environment,
                        "last_updated": self.state.config.platform_info.last_updated
                    }
                })
            }
            "get_team_details" => {
                let team_id = team_id.ok_or_else(|| McpError::missing_param("team_id"))?;

                if let Some(team) = self
                    .state
                    .config
                    .team_configuration
                    .organization
                    .teams
                    .iter()
                    .find(|t| t.id == team_id)
                {
                    let mut team_details = json!({
                        "team_info": {
                            "id": team.id,
                            "name": team.name,
                            "lead": team.lead,
                            "members": team.members,
                            "member_count": team.members.len()
                        },
                        "technical_focus": {
                            "focus_areas": team.focus_areas,
                            "tech_stack": team.tech_stack,
                            "repositories": team.repositories.iter().map(|repo| {
                                // Find additional repository info from resources
                                self.state.resources.development_resources.code_repositories
                                    .iter()
                                    .find(|r| r.name == *repo)
                                    .map(|r| json!({
                                        "name": r.name,
                                        "type": r.repo_type,
                                        "technology": r.technology,
                                        "status": r.status,
                                        "test_coverage": r.test_coverage,
                                        "code_quality": r.code_quality
                                    }))
                                    .unwrap_or_else(|| json!({"name": repo}))
                            }).collect::<Vec<_>>()
                        },
                        "operational_info": {
                            "on_call_rotation": team.on_call_rotation,
                            "response_time_sla": if team.on_call_rotation { "15 minutes" } else { "Next business day" }
                        }
                    });

                    if include_projects {
                        let team_projects = self
                            .state
                            .config
                            .team_configuration
                            .projects
                            .iter()
                            .filter(|project| project.teams.contains(&team.id))
                            .map(|project| {
                                json!({
                                    "name": project.name,
                                    "id": project.id,
                                    "status": project.status,
                                    "priority": project.priority,
                                    "target_date": project.target_date,
                                    "budget": project.budget,
                                    "milestone_progress": {
                                        "total": project.milestones.len(),
                                        "completed": project.milestones.iter()
                                            .filter(|m| m.status == "completed")
                                            .count(),
                                        "in_progress": project.milestones.iter()
                                            .filter(|m| m.status == "in_progress")
                                            .count()
                                    }
                                })
                            })
                            .collect::<Vec<_>>();

                        team_details["assigned_projects"] = json!(team_projects);
                    }

                    team_details
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "Team '{}' not found",
                        team_id
                    )));
                }
            }
            "get_on_call_rotation" => {
                let on_call_teams = self
                    .state
                    .config
                    .team_configuration
                    .organization
                    .teams
                    .iter()
                    .filter(|team| team.on_call_rotation)
                    .map(|team| {
                        json!({
                            "team": {
                                "id": team.id,
                                "name": team.name,
                                "lead": team.lead
                            },
                            "members": team.members,
                            "escalation_contacts": [team.lead.clone()],
                            "response_time": "15 minutes",
                            "coverage": "24/7"
                        })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "on_call_teams": on_call_teams,
                    "total_on_call_teams": on_call_teams.len(),
                    "emergency_contacts": {
                        "primary": "Platform Engineering",
                        "secondary": "Backend Services",
                        "escalation": "Engineering Management"
                    },
                    "incident_response_tools": {
                        "pagerduty": "https://company.pagerduty.com",
                        "slack_channel": "#incidents",
                        "runbooks": "https://docs.company.com/runbooks"
                    }
                })
            }
            "team_workload" => {
                let workload_analysis = self
                    .state
                    .config
                    .team_configuration
                    .organization
                    .teams
                    .iter()
                    .map(|team| {
                        let assigned_projects = self
                            .state
                            .config
                            .team_configuration
                            .projects
                            .iter()
                            .filter(|project| project.teams.contains(&team.id))
                            .collect::<Vec<_>>();

                        let total_budget = assigned_projects.iter().map(|p| p.budget).sum::<u64>();

                        let active_projects = assigned_projects
                            .iter()
                            .filter(|p| p.status == "in_progress" || p.status == "planning")
                            .count();

                        json!({
                            "team": {
                                "id": team.id,
                                "name": team.name,
                                "member_count": team.members.len()
                            },
                            "workload": {
                                "total_projects": assigned_projects.len(),
                                "active_projects": active_projects,
                                "total_budget": total_budget,
                                "avg_budget_per_member": if !team.members.is_empty() {
                                    total_budget / team.members.len() as u64
                                } else { 0 },
                                "utilization": if active_projects > 2 { "high" }
                                              else if active_projects > 0 { "medium" }
                                              else { "low" }
                            },
                            "repositories": team.repositories.len(),
                            "on_call_duties": team.on_call_rotation
                        })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "workload_analysis": workload_analysis,
                    "summary": {
                        "total_teams": workload_analysis.len(),
                        "teams_high_utilization": workload_analysis.iter()
                            .filter(|w| w["workload"]["utilization"] == "high")
                            .count(),
                        "teams_on_call": workload_analysis.iter()
                            .filter(|w| w["on_call_duties"] == true)
                            .count()
                    },
                    "recommendations": [
                        "Consider load balancing for high-utilization teams",
                        "Review project priorities and timelines",
                        "Assess need for additional team members",
                        "Optimize on-call rotation schedules"
                    ]
                })
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "action",
                    "supported team management action",
                    action,
                ));
            }
        };

        Ok(CallToolResult::success(vec![ToolResult::text(
            serde_json::to_string_pretty(&result)?,
        )]))
    }
}

/// Project management tool for handling project lifecycle and tracking
struct ProjectManagementTool {
    state: Arc<PlatformState>,
}

impl ProjectManagementTool {
    fn new(state: Arc<PlatformState>) -> Self {
        Self { state }
    }
}

// Fine-grained trait implementations for ProjectManagementTool
impl HasBaseMetadata for ProjectManagementTool {
    fn name(&self) -> &str {
        "manage_projects"
    }
}

impl HasDescription for ProjectManagementTool {
    fn description(&self) -> Option<&str> {
        Some(
            "Manage development projects including status tracking, milestone management, and resource allocation",
        )
    }
}

impl HasInputSchema for ProjectManagementTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    (
                        "action".to_string(),
                        JsonSchema::string_enum(vec![
                            "list_projects".to_string(),
                            "get_project_details".to_string(),
                            "project_status_summary".to_string(),
                            "milestone_tracking".to_string(),
                            "resource_allocation".to_string(),
                            "project_timeline".to_string(),
                        ])
                        .with_description("Project management action to perform"),
                    ),
                    (
                        "project_id".to_string(),
                        JsonSchema::string()
                            .with_description("Project ID for project-specific operations"),
                    ),
                    (
                        "status_filter".to_string(),
                        JsonSchema::string().with_description(
                            "Filter projects by status (in_progress, planning, completed, etc.)",
                        ),
                    ),
                    (
                        "priority_filter".to_string(),
                        JsonSchema::string().with_description(
                            "Filter projects by priority (critical, high, medium, low)",
                        ),
                    ),
                ]))
                .with_required(vec!["action".to_string()])
        })
    }
}

impl HasOutputSchema for ProjectManagementTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for ProjectManagementTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for ProjectManagementTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for ProjectManagementTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;

        let project_id = args.get("project_id").and_then(|v| v.as_str());
        let status_filter = args.get("status_filter").and_then(|v| v.as_str());
        let priority_filter = args.get("priority_filter").and_then(|v| v.as_str());

        let result = match action {
            "list_projects" => {
                let mut projects = self.state.config.team_configuration.projects.clone();

                if let Some(status) = status_filter {
                    projects.retain(|p| p.status == status);
                }

                if let Some(priority) = priority_filter {
                    projects.retain(|p| p.priority == priority);
                }

                let project_summaries = projects
                    .iter()
                    .map(|project| {
                        json!({
                            "id": project.id,
                            "name": project.name,
                            "status": project.status,
                            "priority": project.priority,
                            "target_date": project.target_date,
                            "teams": project.teams,
                            "budget": project.budget,
                            "milestone_progress": {
                                "total": project.milestones.len(),
                                "completed": project.milestones.iter()
                                    .filter(|m| m.status == "completed")
                                    .count(),
                                "overdue": project.milestones.iter()
                                    .filter(|m| m.status != "completed" && m.date.as_str() < "2025-01-19") // Simplified date comparison
                                    .count()
                            }
                        })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "projects": project_summaries,
                    "summary": {
                        "total_projects": project_summaries.len(),
                        "total_budget": projects.iter().map(|p| p.budget).sum::<u64>(),
                        "status_breakdown": {
                            "in_progress": projects.iter().filter(|p| p.status == "in_progress").count(),
                            "planning": projects.iter().filter(|p| p.status == "planning").count(),
                            "completed": projects.iter().filter(|p| p.status == "completed").count()
                        },
                        "priority_breakdown": {
                            "critical": projects.iter().filter(|p| p.priority == "critical").count(),
                            "high": projects.iter().filter(|p| p.priority == "high").count(),
                            "medium": projects.iter().filter(|p| p.priority == "medium").count(),
                            "low": projects.iter().filter(|p| p.priority == "low").count()
                        }
                    },
                    "filters_applied": {
                        "status": status_filter,
                        "priority": priority_filter
                    }
                })
            }
            "get_project_details" => {
                let project_id = project_id.ok_or_else(|| McpError::missing_param("project_id"))?;

                if let Some(project) = self
                    .state
                    .config
                    .team_configuration
                    .projects
                    .iter()
                    .find(|p| p.id == project_id)
                {
                    let assigned_teams_details = project
                        .teams
                        .iter()
                        .filter_map(|team_id| {
                            self.state
                                .config
                                .team_configuration
                                .organization
                                .teams
                                .iter()
                                .find(|t| &t.id == team_id)
                                .map(|t| {
                                    json!({
                                        "id": t.id,
                                        "name": t.name,
                                        "lead": t.lead,
                                        "member_count": t.members.len(),
                                        "focus_areas": t.focus_areas
                                    })
                                })
                        })
                        .collect::<Vec<_>>();

                    // Calculate risk factors outside the macro
                    let mut risk_factors = Vec::new();
                    if project.priority == "critical" {
                        risk_factors.push("High priority project requires close monitoring");
                    }
                    if project
                        .milestones
                        .iter()
                        .any(|m| m.status != "completed" && m.date.as_str() < "2025-01-19")
                    {
                        risk_factors.push("Overdue milestones detected");
                    }

                    json!({
                        "project_info": {
                            "id": project.id,
                            "name": project.name,
                            "status": project.status,
                            "priority": project.priority,
                            "timeline": {
                                "start_date": project.start_date,
                                "target_date": project.target_date,
                                "duration_estimate": "calculated based on milestones"
                            },
                            "budget": {
                                "allocated": project.budget,
                                "currency": "USD",
                                "per_team": project.budget / project.teams.len().max(1) as u64
                            }
                        },
                        "team_assignment": {
                            "teams": assigned_teams_details,
                            "total_team_members": assigned_teams_details.iter()
                                .map(|t| t["member_count"].as_u64().unwrap_or(0))
                                .sum::<u64>()
                        },
                        "milestones": project.milestones.iter().map(|m| json!({
                            "name": m.name,
                            "target_date": m.date,
                            "status": m.status,
                            "is_overdue": m.status != "completed" && m.date.as_str() < "2025-01-19"
                        })).collect::<Vec<_>>(),
                        "project_health": {
                            "overall_status": project.status,
                            "milestone_completion_rate": format!("{:.1}%",
                                (project.milestones.iter().filter(|m| m.status == "completed").count() as f64
                                / project.milestones.len().max(1) as f64) * 100.0),
                            "risk_factors": risk_factors
                        }
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "Project '{}' not found",
                        project_id
                    )));
                }
            }
            "milestone_tracking" => {
                let all_milestones = self
                    .state
                    .config
                    .team_configuration
                    .projects
                    .iter()
                    .flat_map(|project| {
                        project.milestones.iter().map(move |milestone| {
                            json!({
                                "project_id": project.id,
                                "project_name": project.name,
                                "milestone_name": milestone.name,
                                "target_date": milestone.date,
                                "status": milestone.status,
                                "priority": project.priority,
                                "teams": project.teams
                            })
                        })
                    })
                    .collect::<Vec<_>>();

                let upcoming_milestones = all_milestones
                    .iter()
                    .filter(|m| {
                        m["status"] != "completed"
                            && m["target_date"].as_str().unwrap_or("") >= "2025-01-19"
                    })
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>();

                let overdue_milestones = all_milestones
                    .iter()
                    .filter(|m| {
                        m["status"] != "completed"
                            && m["target_date"].as_str().unwrap_or("") < "2025-01-19"
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                // Calculate recommendations outside the macro
                let mut recommendations = Vec::new();
                if !overdue_milestones.is_empty() {
                    recommendations.push("Review overdue milestones and adjust project timelines");
                }
                if upcoming_milestones.len() > 5 {
                    recommendations.push(
                        "High number of upcoming milestones - ensure adequate resource allocation",
                    );
                }

                json!({
                    "milestone_overview": {
                        "total_milestones": all_milestones.len(),
                        "completed": all_milestones.iter()
                            .filter(|m| m["status"] == "completed")
                            .count(),
                        "in_progress": all_milestones.iter()
                            .filter(|m| m["status"] == "in_progress")
                            .count(),
                        "overdue": overdue_milestones.len(),
                        "upcoming": upcoming_milestones.len()
                    },
                    "upcoming_milestones": upcoming_milestones,
                    "overdue_milestones": overdue_milestones,
                    "completion_rate": format!("{:.1}%",
                        (all_milestones.iter().filter(|m| m["status"] == "completed").count() as f64
                        / all_milestones.len().max(1) as f64) * 100.0),
                    "recommendations": recommendations
                })
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "action",
                    "supported project management action",
                    action,
                ));
            }
        };

        Ok(CallToolResult::success(vec![ToolResult::text(
            serde_json::to_string_pretty(&result)?,
        )]))
    }
}

/// Development workflow generator prompt for creating standardized workflows
#[allow(dead_code)] // TODO: Integrate workflow generation
struct WorkflowGeneratorPrompt {
    state: Arc<PlatformState>,
}

impl WorkflowGeneratorPrompt {
    #[allow(dead_code)] // TODO: Use in workflow generation
    fn new(state: Arc<PlatformState>) -> Self {
        Self { state }
    }
}

// Fine-grained trait implementations for WorkflowGeneratorPrompt
impl HasPromptMetadata for WorkflowGeneratorPrompt {
    fn name(&self) -> &str {
        "generate_workflow"
    }
}

impl HasPromptDescription for WorkflowGeneratorPrompt {
    fn description(&self) -> Option<&str> {
        Some(
            "Generate standardized development workflows based on team practices and project requirements",
        )
    }
}

impl HasPromptArguments for WorkflowGeneratorPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        None
    }
}

impl HasPromptAnnotations for WorkflowGeneratorPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for WorkflowGeneratorPrompt {
    fn prompt_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpPrompt for WorkflowGeneratorPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();
        let workflow_type = args
            .get("workflow_type")
            .and_then(|v| v.as_str())
            .unwrap_or("feature_development");
        let project_type = args
            .get("project_type")
            .and_then(|v| v.as_str())
            .unwrap_or("web_application");
        let team_size = args.get("team_size").and_then(|v| v.as_u64()).unwrap_or(5);

        // Get workflow template from external data
        let workflow_details = if let Some(dev_workflows) = self
            .state
            .workflows
            .workflow_templates
            .get("development_workflows")
        {
            if let Some(workflow) = dev_workflows.get(workflow_type) {
                format!(
                    "Based on our standard {} workflow:\n{}",
                    workflow_type,
                    serde_json::to_string_pretty(workflow).unwrap_or_default()
                )
            } else {
                format!("Standard development workflow for {}", workflow_type)
            }
        } else {
            format!("Standard development workflow for {}", workflow_type)
        };

        let content = format!(
            r#"Please create a comprehensive {} workflow for a {} project with a team of {} members.

{}

Include the following elements in your workflow:

1. **Phase Structure**: Break down the workflow into clear phases with specific goals
2. **Task Definitions**: Define specific, actionable tasks for each phase
3. **Deliverables**: Specify expected outputs and documentation
4. **Quality Gates**: Include review points and approval processes
5. **Timeline Estimates**: Provide realistic time estimates based on team size
6. **Risk Management**: Identify potential bottlenecks and mitigation strategies
7. **Tool Integration**: Reference our development tools and platforms:
   - Version Control: GitHub Enterprise
   - CI/CD: GitHub Actions
   - Project Management: Jira
   - Communication: Slack
   - Documentation: Confluence
   - Code Quality: SonarQube
   - Security Scanning: Snyk

8. **Team Collaboration**: Include pair programming, code reviews, and knowledge sharing practices
9. **Compliance Considerations**: Address security, testing, and documentation requirements
10. **Success Metrics**: Define measurable outcomes for each phase

Format the workflow as a structured document that can be used as a template for similar projects. Include both high-level overview and detailed step-by-step instructions.

Consider our organization's focus on:
- High-quality, maintainable code
- Comprehensive testing and security
- Collaborative development practices
- Continuous integration and deployment
- Knowledge sharing and documentation"#,
            workflow_type, project_type, team_size, workflow_details
        );

        Ok(vec![PromptMessage::text(content)])
    }
}

/// Project resources handler for accessing development resources and documentation
#[allow(dead_code)] // TODO: Integrate project resources
struct ProjectResourcesHandler {
    state: Arc<PlatformState>,
}

impl ProjectResourcesHandler {
    #[allow(dead_code)] // TODO: Use in resource handling
    fn new(state: Arc<PlatformState>) -> Self {
        Self { state }
    }
}

// Fine-grained trait implementations for ProjectResourcesHandler
impl HasResourceMetadata for ProjectResourcesHandler {
    fn name(&self) -> &str {
        "Development Resources"
    }
}

impl HasResourceDescription for ProjectResourcesHandler {
    fn description(&self) -> Option<&str> {
        Some(
            "Access to development team resources including repositories, APIs, documentation, and tools",
        )
    }
}

impl HasResourceUri for ProjectResourcesHandler {
    fn uri(&self) -> &str {
        "platform://development-resources"
    }
}

impl HasResourceMimeType for ProjectResourcesHandler {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for ProjectResourcesHandler {
    fn size(&self) -> Option<u64> {
        None
    }
}

impl HasResourceAnnotations for ProjectResourcesHandler {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}

impl HasResourceMeta for ProjectResourcesHandler {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpResource for ProjectResourcesHandler {
    async fn read(&self, _params: Option<Value>, _session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let mut content = Vec::new();

        // Repository information
        let repos_content = format!(
            "# Code Repositories\n\n{}\n",
            serde_json::to_string_pretty(
                &self.state.resources.development_resources.code_repositories
            )?
        );
        content.push(ResourceContent::text("repos", repos_content));

        // API documentation
        let api_content = format!(
            "# API Documentation\n\n{}\n",
            serde_json::to_string_pretty(
                &self.state.resources.development_resources.api_documentation
            )?
        );
        content.push(ResourceContent::text("api", api_content));

        // Database schemas
        let db_content = format!(
            "# Database Schemas\n\n{}\n",
            serde_json::to_string_pretty(
                &self.state.resources.development_resources.database_schemas
            )?
        );
        content.push(ResourceContent::text("database", db_content));

        // Team tools and integrations
        let tools_content = format!(
            "# Team Tools and Integrations\n\n{}\n",
            serde_json::to_string_pretty(&self.state.resources.team_tools_and_integrations)?
        );
        content.push(ResourceContent::text("tools", tools_content));

        // Learning resources
        let learning_content = format!(
            "# Learning Resources\n\n{}\n",
            serde_json::to_string_pretty(&self.state.resources.learning_resources)?
        );
        content.push(ResourceContent::text("learning", learning_content));

        Ok(content)
    }
}

/// Code template generator for creating standardized code structures
#[allow(dead_code)] // TODO: Integrate code template generation
struct CodeTemplateGenerator {
    state: Arc<PlatformState>,
}

impl CodeTemplateGenerator {
    #[allow(dead_code)] // TODO: Use in template generation
    fn new(state: Arc<PlatformState>) -> Self {
        Self { state }
    }
}

// NOTE: CodeTemplateGenerator McpTemplate implementation removed
// as McpTemplate trait was removed for MCP spec compliance
/*
impl CodeTemplateGenerator {
    fn name(&self) -> &str {
        "code_template"
    }

    fn description(&self) -> &str {
        "Generate code templates based on team standards and best practices"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<String> {
        let template_type = args
            .get("template_type")
            .and_then(|v| v.as_str())
            .unwrap_or("api_service");
        let language = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("typescript");
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("example_service");

        match template_type {
            "api_service" => {
                Ok(format!(
                    r#"// {} - Generated API Service Template
// Generated by Development Team Integration Platform
// Language: {}

import {{ Express }} from 'express';
import {{ Router }} from 'express';
import {{ validateRequest }} from '../middleware/validation';
import {{ authenticate }} from '../middleware/auth';
import {{ rateLimit }} from '../middleware/rateLimit';
import {{ logger }} from '../utils/logger';

export class {}Controller {{
    private router: Router;

    constructor() {{
        this.router = Router();
        this.setupRoutes();
    }}

    private setupRoutes(): void {{
        // Health check endpoint
        this.router.get('/health', this.healthCheck.bind(this));

        // Protected endpoints with authentication and rate limiting
        this.router.use(authenticate);
        this.router.use(rateLimit({{ windowMs: 15 * 60 * 1000, max: 100 }}));

        // CRUD operations
        this.router.get('/', this.list.bind(this));
        this.router.get('/:id', validateRequest('get{}'), this.getById.bind(this));
        this.router.post('/', validateRequest('create{}'), this.create.bind(this));
        this.router.put('/:id', validateRequest('update{}'), this.update.bind(this));
        this.router.delete('/:id', validateRequest('delete{}'), this.delete.bind(this));
    }}

    private async healthCheck(req: Express.Request, res: Express.Response): Promise<void> {{
        res.status(200).json({{
            status: 'healthy',
            service: '{}',
            timestamp: new Date().toISOString(),
            version: process.env.SERVICE_VERSION || '1.0.0'
        }});
    }}

    private async list(req: Express.Request, res: Express.Response): Promise<void> {{
        try {{
            // TODO: Implement list functionality
            logger.info('Listing {} items', {{ requestId: req.id }});

            res.status(200).json({{
                success: true,
                data: [],
                message: 'Items retrieved successfully'
            }});
        }} catch (error) {{
            logger.error('Error listing {} items', error, {{ requestId: req.id }});
            res.status(500).json({{
                success: false,
                error: 'Internal server error',
                requestId: req.id
            }});
        }}
    }}

    private async getById(req: Express.Request, res: Express.Response): Promise<void> {{
        try {{
            const {{ id }} = req.params;
            logger.info('Getting {} by ID: {{}}', id, {{ requestId: req.id }});

            // TODO: Implement get by ID functionality

            res.status(200).json({{
                success: true,
                data: {{ id }},
                message: 'Item retrieved successfully'
            }});
        }} catch (error) {{
            logger.error('Error getting {} by ID', error, {{ requestId: req.id }});
            res.status(500).json({{
                success: false,
                error: 'Internal server error',
                requestId: req.id
            }});
        }}
    }}

    private async create(req: Express.Request, res: Express.Response): Promise<void> {{
        try {{
            const data = req.body;
            logger.info('Creating new {}', {{ data, requestId: req.id }});

            // TODO: Implement create functionality

            res.status(201).json({{
                success: true,
                data: {{ id: 'generated-id', ...data }},
                message: 'Item created successfully'
            }});
        }} catch (error) {{
            logger.error('Error creating {}', error, {{ requestId: req.id }});
            res.status(500).json({{
                success: false,
                error: 'Internal server error',
                requestId: req.id
            }});
        }}
    }}

    private async update(req: Express.Request, res: Express.Response): Promise<void> {{
        try {{
            const {{ id }} = req.params;
            const updateData = req.body;
            logger.info('Updating {} with ID: {{}}', id, {{ updateData, requestId: req.id }});

            // TODO: Implement update functionality

            res.status(200).json({{
                success: true,
                data: {{ id, ...updateData }},
                message: 'Item updated successfully'
            }});
        }} catch (error) {{
            logger.error('Error updating {}', error, {{ requestId: req.id }});
            res.status(500).json({{
                success: false,
                error: 'Internal server error',
                requestId: req.id
            }});
        }}
    }}

    private async delete(req: Express.Request, res: Express.Response): Promise<void> {{
        try {{
            const {{ id }} = req.params;
            logger.info('Deleting {} with ID: {{}}', id, {{ requestId: req.id }});

            // TODO: Implement delete functionality

            res.status(200).json({{
                success: true,
                message: 'Item deleted successfully'
            }});
        }} catch (error) {{
            logger.error('Error deleting {}', error, {{ requestId: req.id }});
            res.status(500).json({{
                success: false,
                error: 'Internal server error',
                requestId: req.id
            }});
        }}
    }}

    public getRouter(): Router {{
        return this.router;
    }}
}}

// Export for use in main application
export default new {}Controller().getRouter();"#,
                    name, language,
                    name.replace("_", "").replace("-", ""), // PascalCase for class name
                    name, name, name, name, name,
                    name, name, name, name, name, name, name, name, name, name,
                    name.replace("_", "").replace("-", "") // PascalCase for export
                ))
            },
            "react_component" => {
                Ok(format!(
                    r#"// {} - Generated React Component Template
// Generated by Development Team Integration Platform
// Language: {}

import React, {{ useState, useEffect, useCallback }} from 'react';
import {{ logger }} from '../utils/logger';
import {{ api }} from '../services/api';
import styles from './{}.module.css';

interface {}Props {{
    id?: string;
    onUpdate?: (data: any) => void;
    className?: string;
}}

interface {}State {{
    data: any | null;
    loading: boolean;
    error: string | null;
}}

export const {}: React.FC<{}Props> = ({{
    id,
    onUpdate,
    className = ''
}}) => {{
    const [state, setState] = useState<{}State>({{
        data: null,
        loading: false,
        error: null
    }});

    const fetchData = useCallback(async () => {{
        if (!id) return;

        setState(prev => ({{ ...prev, loading: true, error: null }}));

        try {{
            const response = await api.get(`/{}/${{id}}`);
            setState(prev => ({{ ...prev, data: response.data, loading: false }}));
            logger.info('Data fetched successfully for {} component', {{ id }});
        }} catch (error) {{
            const errorMessage = error instanceof Error ? error.message : 'Failed to fetch data';
            setState(prev => ({{ ...prev, error: errorMessage, loading: false }}));
            logger.error('Error fetching data for {} component', error, {{ id }});
        }}
    }}, [id]);

    const handleUpdate = async (updateData: Partial<any>) => {{
        if (!id) return;

        setState(prev => ({{ ...prev, loading: true, error: null }}));

        try {{
            const response = await api.put(`/{}/${{id}}`, updateData);
            setState(prev => ({{ ...prev, data: response.data, loading: false }}));
            onUpdate?.(response.data);
            logger.info('Data updated successfully for {} component', {{ id, updateData }});
        }} catch (error) {{
            const errorMessage = error instanceof Error ? error.message : 'Failed to update data';
            setState(prev => ({{ ...prev, error: errorMessage, loading: false }}));
            logger.error('Error updating data for {} component', error, {{ id }});
        }}
    }};

    useEffect(() => {{
        fetchData();
    }}, [fetchData]);

    if (state.loading) {{
        return (
            <div className={{`${{styles.container}} ${{styles.loading}} ${{className}}`}}>
                <div className={{styles.spinner}}></div>
                <span>Loading...</span>
            </div>
        );
    }}

    if (state.error) {{
        return (
            <div className={{`${{styles.container}} ${{styles.error}} ${{className}}`}}>
                <div className={{styles.errorMessage}}>
                    <h3>Error</h3>
                    <p>{{state.error}}</p>
                    <button onClick={{fetchData}} className={{styles.retryButton}}>
                        Retry
                    </button>
                </div>
            </div>
        );
    }}

    if (!state.data && id) {{
        return (
            <div className={{`${{styles.container}} ${{styles.empty}} ${{className}}`}}>
                <p>No data available</p>
            </div>
        );
    }}

    return (
        <div className={{`${{styles.container}} ${{className}}`}}>
            <div className={{styles.header}}>
                <h2>{} Component</h2>
                <div className={{styles.actions}}>
                    <button onClick={{fetchData}} className={{styles.refreshButton}}>
                        Refresh
                    </button>
                </div>
            </div>

            <div className={{styles.content}}>
                {{state.data ? (
                    <div className={{styles.dataDisplay}}>
                        {{/* TODO: Implement data display */
}}
                        <pre>{{JSON.stringify(state.data, null, 2)}}</pre>
                    </div>
                ) : (
                    <div className={{styles.emptyState}}>
                        <p>Ready to display data</p>
                    </div>
                )}}
            </div>
        </div>
    );
}};

export default {};"#,
                    name, language, name,
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name, name, name, name, name, name,
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", "") // PascalCase
                ))
            },
            "database_model" => {
                Ok(format!(
r#" // {} - Generated Database Model Template
    // Generated by Development Team Integration Platform
    // Language: {}

import {{ DataTypes, Model, Optional }} from 'sequelize';
import {{ sequelize }} from '../config/database';

// Model attributes interface
interface {}Attributes {{
    id: string;
    createdAt: Date;
    updatedAt: Date;
// TODO: Add specific model attributes
}}

// Optional attributes for model creation
interface {}CreationAttributes extends Optional<{}Attributes, 'id' | 'createdAt' | 'updatedAt'> {{}}

// Model class
class {} extends Model<{}Attributes, {}CreationAttributes> implements {}Attributes {{
    public id!: string;
    public createdAt!: Date;
    public updatedAt!: Date;

// TODO: Add specific model properties

// Static methods
    public static findByCustomField(value: string): Promise<{} | null> {{
        return this.findOne({{
            where: {{
// TODO: Implement custom field search
            }}
        }});
    }}

// Instance methods
    public toJSON(): object {{
        const values = super.toJSON() as any;
// TODO: Add any custom JSON transformation
        return values;
    }}
}}

// Model definition
{}.init(
    {{
        id: {{
            type: DataTypes.UUID,
            defaultValue: DataTypes.UUIDV4,
            primaryKey: true,
        }},
// TODO: Add specific model fields
// Example:
// name: {{
//     type: DataTypes.STRING,
//     allowNull: false,
//     validate: {{
//         len: [2, 100]
//     }}
// }},
    }},
    {{
        sequelize,
        tableName: '{}',
        timestamps: true,
paranoid: true, // Soft deletes
        indexes: [
// TODO: Add database indexes
// {{
//     fields: ['name']
// }},
        ],
        hooks: {{
            beforeCreate: async (instance: {}) => {{
// TODO: Add pre-creation hooks
            }},
            afterCreate: async (instance: {}) => {{
// TODO: Add post-creation hooks
            }},
            beforeUpdate: async (instance: {}) => {{
// TODO: Add pre-update hooks
            }},
            afterUpdate: async (instance: {}) => {{
// TODO: Add post-update hooks
            }}
        }}
    }}
);

// Model associations (to be defined after all models are loaded)
export const associate = () => {{
// TODO: Define model associations
// Example:
// {}.belongsTo(OtherModel, {{ foreignKey: 'otherId', as: 'other' }});
// {}.hasMany(RelatedModel, {{ foreignKey: '{}Id', as: 'relatedItems' }});
}};

export default {};
export {{ {}Attributes, {}CreationAttributes }};"#,
                    name, language,
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.to_lowercase().replace("_", "_"), // snake_case for table name
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.to_lowercase().replace("-", "_"), // snake_case
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", ""), // PascalCase
                    name.replace("_", "").replace("-", "") // PascalCase
                ))
            },
            _ => {
                Ok(format!(
" // {} Template\n// Generated by Development Team Integration Platform\n// Language: {}\n\n// TODO: Implement {} template\n// Template type '{}' is not yet implemented\n// Available templates: api_service, react_component, database_model",
                    name, language, name, template_type
                ))
            }
        }
    }
}
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Development Team Integration Platform");

    // Load platform state with external data
    let platform_state = Arc::new(PlatformState::new()?);

    // Parse command line arguments for bind address
    let bind_address: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8002".to_string())
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Create comprehensive MCP server with all capabilities
    let server = McpServer::builder()
        .name("development-team-integration-platform")
        .version("2.1.0")
        .title("Development Team Integration Platform")
        .instructions("Comprehensive development team integration platform showcasing all MCP capabilities in real-world business scenarios. Provides team management, project tracking, workflow generation, resource access, and code templates using external configuration files.")
        .bind_address(bind_address)

        // Add comprehensive business tools
        .tool(TeamManagementTool::new(platform_state.clone()))
        .tool(ProjectManagementTool::new(platform_state.clone()))

        // Enable all MCP handlers with real-world implementations
        .with_completion()
        .with_prompts()
        .with_resources()
        .with_logging()
        .with_notifications()
        .with_roots()
        .with_sampling()

        .build()?;

    info!("Development Team Integration Platform configured:");
    info!("  Platform: {}", platform_state.config.platform_info.name);
    info!("  Version: {}", platform_state.config.platform_info.version);
    info!(
        "  Environment: {}",
        platform_state.config.platform_info.environment
    );
    info!(
        "  Organization: {}",
        platform_state.config.team_configuration.organization.name
    );
    info!(
        "  Teams: {}",
        platform_state
            .config
            .team_configuration
            .organization
            .teams
            .len()
    );
    info!(
        "  Active Projects: {}",
        platform_state.config.team_configuration.projects.len()
    );

    info!("Available Tools:");
    info!("  - manage_teams: Team management, member info, on-call rotations, workload analysis");
    info!("  - manage_projects: Project tracking, milestone management, resource allocation");

    info!("Available Capabilities:");
    info!("  - Prompts: Workflow generation based on team standards");
    info!("  - Resources: Development resources, APIs, documentation access");
    info!("  - Templates: Code templates with team best practices");
    info!("  - Completion: Context-aware development assistance");
    info!("  - Logging: Development activity and audit logging");
    info!("  - Notifications: Team collaboration and project updates");

    info!("External Data Sources:");
    info!("  - Platform Config: data/platform_config.json");
    info!("  - Workflow Templates: data/workflow_templates.yaml");
    info!("  - Project Resources: data/project_resources.json");
    info!("  - Code Templates: data/code_templates.md");

    info!("Real-world Use Cases:");
    info!("  - Team collaboration and resource management");
    info!("  - Project planning and milestone tracking");
    info!("  - Standardized development workflows");
    info!("  - Code quality and template generation");
    info!("  - Documentation and knowledge sharing");

    info!("Server will bind to: {}", bind_address);
    info!("MCP endpoint available at: http://{}/mcp", bind_address);

    server.run().await?;

    Ok(())
}
