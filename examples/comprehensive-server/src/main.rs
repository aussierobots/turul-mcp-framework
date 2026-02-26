//! # Development Team Integration Platform
//!
//! This comprehensive example demonstrates a real-world development team integration
//! platform that showcases all MCP capabilities in practical business scenarios.
//! It serves as a central hub for development workflows, team collaboration,
//! project management, and knowledge sharing using external configuration files.

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, json};
use tracing::info;
use turul_mcp_derive::{McpResource, McpTool};
use turul_mcp_protocol::prompts::PromptMessage;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::handlers::McpPrompt as McpPromptTrait;
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpResource as McpResourceTrait, McpServer, SessionContext};

// Module-level static for shared platform state
static PLATFORM: OnceLock<Arc<PlatformState>> = OnceLock::new();

fn get_platform() -> McpResult<&'static Arc<PlatformState>> {
    PLATFORM
        .get()
        .ok_or_else(|| McpError::tool_execution("Platform not initialized"))
}

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
        let config_path = std::path::Path::new("data/platform_config.json");
        let workflows_path = std::path::Path::new("data/workflow_templates.yaml");
        let resources_path = std::path::Path::new("data/project_resources.json");
        let templates_path = std::path::Path::new("data/code_templates.md");

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
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "manage_teams",
    description = "Manage development teams, members, and team information including skills, projects, and on-call rotations"
)]
pub struct TeamManagementTool {
    #[param(
        description = "Team management action: list_teams, get_team_details, get_team_members, get_on_call_rotation, team_workload"
    )]
    pub action: String,

    #[param(description = "Team ID for team-specific operations", optional)]
    pub team_id: Option<String>,

    #[param(description = "Include project assignments in team details", optional)]
    pub include_projects: Option<bool>,
}

impl TeamManagementTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let state = get_platform()?;
        let include_projects = self.include_projects.unwrap_or(false);

        let result = match self.action.as_str() {
            "list_teams" => {
                let teams_summary = state
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
                    "organization": state.config.team_configuration.organization.name,
                    "total_teams": teams_summary.len(),
                    "teams": teams_summary,
                    "platform_info": {
                        "name": state.config.platform_info.name,
                        "environment": state.config.platform_info.environment,
                        "last_updated": state.config.platform_info.last_updated
                    }
                })
            }
            "get_team_details" => {
                let team_id = self
                    .team_id
                    .as_deref()
                    .ok_or_else(|| McpError::missing_param("team_id"))?;

                if let Some(team) = state
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
                                state.resources.development_resources.code_repositories
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
                        let team_projects = state
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
                let on_call_teams = state
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
                let workload_analysis = state
                    .config
                    .team_configuration
                    .organization
                    .teams
                    .iter()
                    .map(|team| {
                        let assigned_projects = state
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
                    &self.action,
                ));
            }
        };

        Ok(result)
    }
}

/// Project management tool for handling project lifecycle and tracking
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "manage_projects",
    description = "Manage development projects including status tracking, milestone management, and resource allocation"
)]
pub struct ProjectManagementTool {
    #[param(
        description = "Project management action: list_projects, get_project_details, project_status_summary, milestone_tracking, resource_allocation, project_timeline"
    )]
    pub action: String,

    #[param(description = "Project ID for project-specific operations", optional)]
    pub project_id: Option<String>,

    #[param(
        description = "Filter projects by status (in_progress, planning, completed, etc.)",
        optional
    )]
    pub status_filter: Option<String>,

    #[param(
        description = "Filter projects by priority (critical, high, medium, low)",
        optional
    )]
    pub priority_filter: Option<String>,
}

impl ProjectManagementTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let state = get_platform()?;

        let result = match self.action.as_str() {
            "list_projects" => {
                let mut projects = state.config.team_configuration.projects.clone();

                if let Some(status) = &self.status_filter {
                    projects.retain(|p| p.status == *status);
                }

                if let Some(priority) = &self.priority_filter {
                    projects.retain(|p| p.priority == *priority);
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
                                    .filter(|m| m.status != "completed" && m.date.as_str() < "2025-01-19")
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
                        "status": self.status_filter,
                        "priority": self.priority_filter
                    }
                })
            }
            "get_project_details" => {
                let project_id = self
                    .project_id
                    .as_deref()
                    .ok_or_else(|| McpError::missing_param("project_id"))?;

                if let Some(project) = state
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
                            state
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
                let all_milestones = state
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
                    &self.action,
                ));
            }
        };

        Ok(result)
    }
}

/// Development workflow generator prompt for creating standardized workflows
#[derive(Clone)]
pub struct WorkflowGeneratorPrompt;

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
    fn arguments(&self) -> Option<&Vec<turul_mcp_protocol::prompts::PromptArgument>> {
        None
    }
}

impl HasPromptAnnotations for WorkflowGeneratorPrompt {
    fn annotations(&self) -> Option<&turul_mcp_protocol::prompts::PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for WorkflowGeneratorPrompt {
    fn prompt_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        None
    }
}

impl HasIcons for WorkflowGeneratorPrompt {}

#[async_trait]
impl McpPromptTrait for WorkflowGeneratorPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let state = get_platform()?;
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
        let workflow_details = if let Some(dev_workflows) = state
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
#[derive(McpResource, Clone)]
#[resource(
    uri = "platform://development-resources",
    name = "Development Resources",
    description = "Access to development team resources including repositories, APIs, documentation, and tools",
    mime_type = "text/plain"
)]
pub struct ProjectResourcesHandler;

#[async_trait]
impl McpResourceTrait for ProjectResourcesHandler {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let state = get_platform()?;
        let mut content = Vec::new();

        // Repository information
        let repos_content = format!(
            "# Code Repositories\n\n{}\n",
            serde_json::to_string_pretty(&state.resources.development_resources.code_repositories)?
        );
        content.push(ResourceContent::text("repos", repos_content));

        // API documentation
        let api_content = format!(
            "# API Documentation\n\n{}\n",
            serde_json::to_string_pretty(&state.resources.development_resources.api_documentation)?
        );
        content.push(ResourceContent::text("api", api_content));

        // Database schemas
        let db_content = format!(
            "# Database Schemas\n\n{}\n",
            serde_json::to_string_pretty(&state.resources.development_resources.database_schemas)?
        );
        content.push(ResourceContent::text("database", db_content));

        // Team tools and integrations
        let tools_content = format!(
            "# Team Tools and Integrations\n\n{}\n",
            serde_json::to_string_pretty(&state.resources.team_tools_and_integrations)?
        );
        content.push(ResourceContent::text("tools", tools_content));

        // Learning resources
        let learning_content = format!(
            "# Learning Resources\n\n{}\n",
            serde_json::to_string_pretty(&state.resources.learning_resources)?
        );
        content.push(ResourceContent::text("learning", learning_content));

        Ok(content)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Development Team Integration Platform");

    // Load platform state with external data and store in global static
    let platform_state = Arc::new(PlatformState::new()?);
    PLATFORM
        .set(platform_state.clone())
        .expect("Platform already initialized");

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
        .tool(TeamManagementTool::default())
        .tool(ProjectManagementTool::default())

        // Add prompts
        .prompt(WorkflowGeneratorPrompt)

        // Add resources
        .resource(ProjectResourcesHandler)

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
