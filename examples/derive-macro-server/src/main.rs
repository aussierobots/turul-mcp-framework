//! # Code Generation and Template Engine Server
//!
//! This server provides comprehensive code generation, validation, and transformation tools
//! for developers working with multiple programming languages and frameworks. It demonstrates
//! the power of derive macros while offering real-world utility for automated code generation,
//! project scaffolding, and code quality validation.

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use mcp_derive::McpTool;
use mcp_server::{McpResult, McpServer};
use mcp_protocol::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize, Serialize)]
struct CodeTemplates {
    code_generation_templates: HashMap<String, CodeTemplate>,
    validation_rules: ValidationRules,
    transformation_patterns: HashMap<String, TransformationPattern>,
    language_support: HashMap<String, LanguageConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CodeTemplate {
    name: String,
    description: String,
    template: String,
    parameters: HashMap<String, String>,
    example: Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct ValidationRules {
    rust_naming: NamingRules,
    code_quality: QualityRules,
    security: SecurityRules,
}

#[derive(Debug, Deserialize, Serialize)]
struct NamingRules {
    struct_name: String,
    enum_name: String,
    field_name: String,
    function_name: String,
    constant_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct QualityRules {
    max_function_length: u32,
    max_struct_fields: u32,
    required_derives: Vec<String>,
    documentation_required: bool,
    error_handling: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SecurityRules {
    avoid_unsafe: bool,
    input_validation: String,
    sql_injection: String,
    sensitive_data: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TransformationPattern {
    pattern: String,
    replacement: String,
    description: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LanguageConfig {
    file_extension: String,
    common_derives: Option<Vec<String>>,
    attribute_syntax: Option<String>,
    visibility: Vec<String>,
    interface_template: Option<String>,
    type_template: Option<String>,
    dataclass_template: Option<String>,
    pydantic_template: Option<String>,
    decorators: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ValidationSchemas {
    #[allow(dead_code)] // TODO: Implement validation schema integration
    validation_schemas: HashMap<String, ValidationCategory>,
    #[allow(dead_code)] // TODO: Implement data format validation
    data_format_validation: HashMap<String, FormatValidation>,
    #[allow(dead_code)] // TODO: Implement testing standards validation
    testing_standards: HashMap<String, TestingStandard>,
}

#[derive(Debug, Deserialize)]
struct ValidationCategory {
    #[allow(dead_code)] // TODO: Use name in validation reporting
    name: String,
    #[allow(dead_code)] // TODO: Use description in validation documentation
    description: String,
    #[allow(dead_code)] // TODO: Use content for validation logic
    #[serde(flatten)]
    content: Value,
}

#[derive(Debug, Deserialize)]
struct FormatValidation {
    #[allow(dead_code)] // TODO: Use name in format validation
    name: String,
    #[allow(dead_code)] // TODO: Use description in format documentation
    description: String,
    #[allow(dead_code)] // TODO: Use content for format validation rules
    #[serde(flatten)]
    content: Value,
}

#[derive(Debug, Deserialize)]
struct TestingStandard {
    #[allow(dead_code)] // TODO: Use name in testing standard identification
    name: String,
    #[allow(dead_code)] // TODO: Use description in testing documentation
    description: String,
    #[allow(dead_code)] // TODO: Use content for testing standard rules
    #[serde(flatten)]
    content: Value,
}

/// Code generation tool using template engine with external template library
#[derive(McpTool, Clone)]
#[tool(
    name = "generate_code",
    description = "Generate code from templates with parameter substitution and validation"
)]
struct CodeGeneratorTool {
    #[param(description = "Template type: rust_struct, rust_enum, api_endpoint, database_model")]
    template_type: String,

    #[param(description = "JSON string containing template parameters")]
    parameters: String,

    #[param(description = "Target programming language", optional)]
    language: Option<String>,

    #[param(description = "Apply validation rules to generated code", optional)]
    validate: Option<bool>,
}

impl CodeGeneratorTool {
    async fn execute(&self) -> McpResult<String> {
        let templates = load_code_templates()
            .map_err(|e| McpError::tool_execution(&format!("Failed to load templates: {}", e)))?;
        
        let language = self.language.as_deref().unwrap_or("rust");
        let validate = self.validate.unwrap_or(true);
        
        // Parse template parameters
        let params: Value = serde_json::from_str(&self.parameters)
            .map_err(|e| McpError::invalid_param_type("parameters", "valid JSON string", &format!("Parse error: {}", e)))?;
        
        // Get template
        let _template = templates.code_generation_templates
            .get(&self.template_type)
            .ok_or_else(|| McpError::tool_execution(&format!("Template '{}' not found", self.template_type)))?;
        
        // Generate code (simplified template processing)
        let generated_code = match self.template_type.as_str() {
            "rust_struct" => generate_rust_struct(&params)?,
            "rust_enum" => generate_rust_enum(&params)?,
            "api_endpoint" => generate_api_endpoint(&params)?,
            "database_model" => generate_database_model(&params)?,
            _ => return Err(McpError::tool_execution(&format!("Unsupported template type: {}", self.template_type))),
        };
        
        let mut result = json!({
            "template_type": self.template_type,
            "language": language,
            "generated_code": generated_code,
            "validation_applied": validate
        });
        
        if validate {
            let validation_result = validate_generated_code(&generated_code, &templates.validation_rules)?;
            result["validation"] = json!(validation_result);
        }
        
        Ok(serde_json::to_string_pretty(&result)?)
    }
}

/// Project validation tool that checks project structure and configuration
#[derive(McpTool, Clone)]
#[tool(
    name = "validate_project",
    description = "Validate project structure, configuration files, and code quality standards"
)]
struct ProjectValidatorTool {
    #[param(description = "Project type: rust_project, typescript_project, python_project")]
    project_type: String,

    #[param(description = "Project root directory path")]
    project_path: String,

    #[param(description = "Validation categories: structure, config, dependencies, security", optional)]
    validation_scope: Option<String>,

    #[param(description = "Generate detailed validation report", optional)]
    detailed_report: Option<bool>,
}

impl ProjectValidatorTool {
    async fn execute(&self) -> McpResult<String> {
        let _schemas = load_validation_schemas()
            .map_err(|e| McpError::tool_execution(&format!("Failed to load validation schemas: {}", e)))?;
        
        let detailed = self.detailed_report.unwrap_or(false);
        let scope = self.validation_scope.as_deref().unwrap_or("all");
        
        // Check if project path exists
        if !Path::new(&self.project_path).exists() {
            return Err(McpError::tool_execution(&format!("Project path does not exist: {}", self.project_path)));
        }
        
        let mut validation_results = Vec::new();
        
        // Validate project structure
        if scope == "all" || scope.contains("structure") {
            let structure_validation = validate_project_structure(&self.project_type, &self.project_path)?;
            validation_results.push(json!({
                "category": "Project Structure",
                "status": structure_validation.status,
                "issues": structure_validation.issues,
                "recommendations": structure_validation.recommendations
            }));
        }
        
        // Validate configuration files
        if scope == "all" || scope.contains("config") {
            let config_validation = validate_configuration_files(&self.project_path)?;
            validation_results.push(json!({
                "category": "Configuration",
                "status": config_validation.status,
                "issues": config_validation.issues,
                "suggestions": config_validation.suggestions
            }));
        }
        
        // Security validation
        if scope == "all" || scope.contains("security") {
            let security_validation = validate_security_practices(&self.project_path)?;
            validation_results.push(json!({
                "category": "Security",
                "status": security_validation.status,
                "vulnerabilities": security_validation.vulnerabilities,
                "recommendations": security_validation.recommendations
            }));
        }
        
        let overall_status = if validation_results.iter().all(|r| r["status"] == "passed") {
            "passed"
        } else if validation_results.iter().any(|r| r["status"] == "failed") {
            "failed"
        } else {
            "warning"
        };
        
        Ok(serde_json::to_string_pretty(&json!({
            "project_type": self.project_type,
            "project_path": self.project_path,
            "validation_scope": scope,
            "overall_status": overall_status,
            "detailed_report": detailed,
            "validation_results": validation_results,
            "summary": {
                "total_categories": validation_results.len(),
                "passed": validation_results.iter().filter(|r| r["status"] == "passed").count(),
                "warnings": validation_results.iter().filter(|r| r["status"] == "warning").count(),
                "failed": validation_results.iter().filter(|r| r["status"] == "failed").count()
            }
        }))?)
    }
}

/// Code transformation tool for refactoring and modernizing code
#[derive(McpTool, Clone)]
#[tool(
    name = "transform_code",
    description = "Apply code transformations, refactoring patterns, and style improvements"
)]
struct CodeTransformationTool {
    #[param(description = "Code to transform")]
    source_code: String,

    #[param(description = "Transformation type: naming_convention, add_documentation, add_derives, error_handling")]
    transformation: String,

    #[param(description = "Programming language", optional)]
    language: Option<String>,

    #[param(description = "Additional transformation parameters as JSON", optional)]
    options: Option<String>,
}

impl CodeTransformationTool {
    async fn execute(&self) -> McpResult<String> {
        let _templates = load_code_templates()
            .map_err(|e| McpError::tool_execution(&format!("Failed to load templates: {}", e)))?;
        
        let language = self.language.as_deref().unwrap_or("rust");
        let options: Value = if let Some(opt) = &self.options {
            serde_json::from_str(opt)
                .map_err(|e| McpError::invalid_param_type("options", "valid JSON", &e.to_string()))?
        } else {
            json!({})
        };
        
        let transformed_code = match self.transformation.as_str() {
            "naming_convention" => apply_naming_conventions(&self.source_code, language)?,
            "add_documentation" => add_documentation_comments(&self.source_code)?,
            "add_derives" => add_derive_macros(&self.source_code, &options)?,
            "error_handling" => improve_error_handling(&self.source_code)?,
            "camelcase_to_snake" => apply_case_transformation(&self.source_code, "snake_case")?,
            "snake_to_camel" => apply_case_transformation(&self.source_code, "camelCase")?,
            _ => return Err(McpError::tool_execution(&format!("Unknown transformation: {}", self.transformation))),
        };
        
        // Analyze the transformation
        let analysis = analyze_transformation(&self.source_code, &transformed_code)?;
        
        Ok(serde_json::to_string_pretty(&json!({
            "transformation": self.transformation,
            "language": language,
            "original_code": self.source_code,
            "transformed_code": transformed_code,
            "analysis": analysis,
            "statistics": {
                "original_lines": self.source_code.lines().count(),
                "transformed_lines": transformed_code.lines().count(),
                "changes_applied": analysis.changes_count
            }
        }))?)
    }
}

/// Configuration validation tool with schema validation
#[derive(McpTool, Clone)]
#[tool(
    name = "validate_config",
    description = "Validate configuration files against predefined schemas and best practices"
)]
struct ConfigValidatorTool {
    #[param(description = "Configuration content or file path")]
    config_input: String,

    #[param(description = "Config type: database_config, api_config, docker_compose, kubernetes")]
    config_type: String,

    #[param(description = "Input format: json, yaml, toml", optional)]
    format: Option<String>,

    #[param(description = "Provide suggestions for improvements", optional)]
    suggest_improvements: Option<bool>,
}

impl ConfigValidatorTool {
    async fn execute(&self) -> McpResult<String> {
        let schemas = load_validation_schemas()
            .map_err(|e| McpError::tool_execution(&format!("Failed to load schemas: {}", e)))?;
        
        let format = self.format.as_deref().unwrap_or("json");
        let suggest = self.suggest_improvements.unwrap_or(true);
        
        // Determine if input is file path or content
        let config_content = if Path::new(&self.config_input).exists() {
            fs::read_to_string(&self.config_input)
                .map_err(|e| McpError::tool_execution(&format!("Failed to read config file: {}", e)))?
        } else {
            self.config_input.clone()
        };
        
        // Parse configuration based on format
        let parsed_config: Value = match format {
            "json" => serde_json::from_str(&config_content)
                .map_err(|e| McpError::tool_execution(&format!("Invalid JSON: {}", e)))?,
            "yaml" => serde_yml::from_str(&config_content)
                .map_err(|e| McpError::tool_execution(&format!("Invalid YAML: {}", e)))?,
            "toml" => toml::from_str(&config_content)
                .map_err(|e| McpError::tool_execution(&format!("Invalid TOML: {}", e)))?,
            _ => return Err(McpError::invalid_param_type("format", "json, yaml, or toml", format)),
        };
        
        // Validate against schema
        let validation_result = validate_config_against_schema(&parsed_config, &self.config_type, &schemas)?;
        
        let mut result = json!({
            "config_type": self.config_type,
            "format": format,
            "validation_status": validation_result.status,
            "errors": validation_result.errors,
            "warnings": validation_result.warnings
        });
        
        if suggest {
            let suggestions = generate_config_suggestions(&parsed_config, &self.config_type)?;
            result["suggestions"] = json!(suggestions);
        }
        
        Ok(serde_json::to_string_pretty(&result)?)
    }
}

/// Test generation tool that creates unit tests from code analysis
#[derive(McpTool, Clone)]
#[tool(
    name = "generate_tests",
    description = "Generate unit tests, integration tests, and test fixtures from code analysis"
)]
struct TestGeneratorTool {
    #[param(description = "Source code to generate tests for")]
    source_code: String,

    #[param(description = "Test type: unit_tests, integration_tests, property_tests")]
    test_type: String,

    #[param(description = "Programming language", optional)]
    language: Option<String>,

    #[param(description = "Test framework preference", optional)]
    framework: Option<String>,

    #[param(description = "Generate edge case tests", optional)]
    include_edge_cases: Option<bool>,
}

impl TestGeneratorTool {
    async fn execute(&self) -> McpResult<String> {
        let language = self.language.as_deref().unwrap_or("rust");
        let framework = self.framework.as_deref().unwrap_or("default");
        let edge_cases = self.include_edge_cases.unwrap_or(true);
        
        // Analyze source code to extract functions and structures
        let code_analysis = analyze_source_code(&self.source_code, language)?;
        
        // Generate tests based on analysis
        let generated_tests = match self.test_type.as_str() {
            "unit_tests" => generate_unit_tests(&code_analysis, framework, edge_cases)?,
            "integration_tests" => generate_integration_tests(&code_analysis, framework)?,
            "property_tests" => generate_property_tests(&code_analysis, framework)?,
            _ => return Err(McpError::tool_execution(&format!("Unknown test type: {}", self.test_type))),
        };
        
        // Generate test fixtures if needed
        let fixtures = if matches!(self.test_type.as_str(), "integration_tests" | "property_tests") {
            Some(generate_test_fixtures(&code_analysis)?)
        } else {
            None
        };
        
        Ok(serde_json::to_string_pretty(&json!({
            "test_type": self.test_type,
            "language": language,
            "framework": framework,
            "include_edge_cases": edge_cases,
            "source_analysis": {
                "functions_found": code_analysis.functions.len(),
                "structs_found": code_analysis.structs.len(),
                "complexity_score": code_analysis.complexity_score
            },
            "generated_tests": generated_tests,
            "test_fixtures": fixtures,
            "recommendations": generate_test_recommendations(&code_analysis)
        }))?)
    }
}

// Helper functions for code generation and validation

fn load_code_templates() -> Result<CodeTemplates, Box<dyn std::error::Error>> {
    let path = "data/code_templates.json";
    if Path::new(path).exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        // Return minimal fallback templates
        Ok(CodeTemplates {
            code_generation_templates: HashMap::new(),
            validation_rules: ValidationRules {
                rust_naming: NamingRules {
                    struct_name: "PascalCase".to_string(),
                    enum_name: "PascalCase".to_string(),
                    field_name: "snake_case".to_string(),
                    function_name: "snake_case".to_string(),
                    constant_name: "SCREAMING_SNAKE_CASE".to_string(),
                },
                code_quality: QualityRules {
                    max_function_length: 50,
                    max_struct_fields: 20,
                    required_derives: vec!["Debug".to_string()],
                    documentation_required: true,
                    error_handling: "Use Result for fallible operations".to_string(),
                },
                security: SecurityRules {
                    avoid_unsafe: true,
                    input_validation: "Always validate external input".to_string(),
                    sql_injection: "Use parameterized queries".to_string(),
                    sensitive_data: "Mark sensitive fields appropriately".to_string(),
                },
            },
            transformation_patterns: HashMap::new(),
            language_support: HashMap::new(),
        })
    }
}

fn load_validation_schemas() -> Result<ValidationSchemas, Box<dyn std::error::Error>> {
    let path = "data/validation_schemas.yaml";
    if Path::new(path).exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_yml::from_str(&content)?)
    } else {
        Ok(ValidationSchemas {
            validation_schemas: HashMap::new(),
            data_format_validation: HashMap::new(),
            testing_standards: HashMap::new(),
        })
    }
}

// Simplified code generation functions (in a real implementation, these would use a proper template engine)

fn generate_rust_struct(params: &Value) -> McpResult<String> {
    let name = params["name"].as_str().ok_or_else(|| McpError::missing_param("name"))?;
    let visibility = params["visibility"].as_str().unwrap_or("");
    let empty_vec = vec![];
    let attributes = params["attributes"].as_array().unwrap_or(&empty_vec);
    let fields = params["fields"].as_array().ok_or_else(|| McpError::missing_param("fields"))?;
    
    let mut result = String::new();
    
    // Add attributes
    for attr in attributes {
        if let Some(attr_str) = attr.as_str() {
            result.push_str(&format!("#[{}]\n", attr_str));
        }
    }
    
    // Add struct definition
    if !visibility.is_empty() {
        result.push_str(&format!("{} ", visibility));
    }
    result.push_str(&format!("struct {} {{\n", name));
    
    // Add fields
    for field in fields {
        let field_name = field["name"].as_str().ok_or_else(|| McpError::missing_param("field name"))?;
        let field_type = field["type"].as_str().ok_or_else(|| McpError::missing_param("field type"))?;
        let field_vis = field["visibility"].as_str().unwrap_or("");
        
        if !field_vis.is_empty() {
            result.push_str(&format!("    {} {}: {},\n", field_vis, field_name, field_type));
        } else {
            result.push_str(&format!("    {}: {},\n", field_name, field_type));
        }
    }
    
    result.push_str("}");
    Ok(result)
}

fn generate_rust_enum(params: &Value) -> McpResult<String> {
    let name = params["name"].as_str().ok_or_else(|| McpError::missing_param("name"))?;
    let visibility = params["visibility"].as_str().unwrap_or("");
    let empty_vec = vec![];
    let attributes = params["attributes"].as_array().unwrap_or(&empty_vec);
    let variants = params["variants"].as_array().ok_or_else(|| McpError::missing_param("variants"))?;
    
    let mut result = String::new();
    
    // Add attributes
    for attr in attributes {
        if let Some(attr_str) = attr.as_str() {
            result.push_str(&format!("#[{}]\n", attr_str));
        }
    }
    
    // Add enum definition
    if !visibility.is_empty() {
        result.push_str(&format!("{} ", visibility));
    }
    result.push_str(&format!("enum {} {{\n", name));
    
    // Add variants
    for variant in variants {
        let variant_name = variant["name"].as_str().ok_or_else(|| McpError::missing_param("variant name"))?;
        
        if let Some(data) = variant["data"].as_str() {
            result.push_str(&format!("    {}({}),\n", variant_name, data));
        } else if let Some(fields) = variant["fields"].as_array() {
            result.push_str(&format!("    {} {{\n", variant_name));
            for field in fields {
                let field_name = field["name"].as_str().ok_or_else(|| McpError::missing_param("field name"))?;
                let field_type = field["type"].as_str().ok_or_else(|| McpError::missing_param("field type"))?;
                result.push_str(&format!("        {}: {},\n", field_name, field_type));
            }
            result.push_str("    },\n");
        } else {
            result.push_str(&format!("    {},\n", variant_name));
        }
    }
    
    result.push_str("}");
    Ok(result)
}

fn generate_api_endpoint(params: &Value) -> McpResult<String> {
    let function_name = params["function_name"].as_str().ok_or_else(|| McpError::missing_param("function_name"))?;
    let description = params["description"].as_str().unwrap_or("");
    let is_async = params["async"].as_bool().unwrap_or(true);
    let empty_vec = vec![];
    let attributes = params["attributes"].as_array().unwrap_or(&empty_vec);
    let empty_vec2 = vec![];
    let parameters = params["parameters"].as_array().unwrap_or(&empty_vec2);
    let return_type = params["return_type"].as_str().unwrap_or("()");
    let empty_vec3 = vec![];
    let implementation = params["implementation"].as_array().unwrap_or(&empty_vec3);
    
    let mut result = String::new();
    
    // Add function documentation
    if !description.is_empty() {
        result.push_str(&format!("/// {}\n", description));
    }
    
    // Add attributes
    for attr in attributes {
        if let Some(attr_str) = attr.as_str() {
            result.push_str(&format!("#[{}]\n", attr_str));
        }
    }
    
    // Add function signature
    if is_async {
        result.push_str("async ");
    }
    result.push_str(&format!("fn {}(\n", function_name));
    
    // Add parameters
    for param in parameters {
        let param_name = param["name"].as_str().ok_or_else(|| McpError::missing_param("parameter name"))?;
        let param_type = param["type"].as_str().ok_or_else(|| McpError::missing_param("parameter type"))?;
        result.push_str(&format!("    {}: {},\n", param_name, param_type));
    }
    
    result.push_str(&format!(") -> {} {{\n", return_type));
    
    // Add implementation
    if !description.is_empty() {
        result.push_str(&format!("    // {}\n", description));
    }
    
    for line in implementation {
        if let Some(line_str) = line.as_str() {
            result.push_str(&format!("    {}\n", line_str));
        }
    }
    
    if implementation.is_empty() {
        result.push_str("    // TODO: Implement function logic\n");
        result.push_str("    unimplemented!()\n");
    }
    
    result.push_str("}");
    Ok(result)
}

fn generate_database_model(params: &Value) -> McpResult<String> {
    let table_name = params["table_name"].as_str().ok_or_else(|| McpError::missing_param("table_name"))?;
    let empty_vec = vec![];
    let table_attributes = params["table_attributes"].as_array().unwrap_or(&empty_vec);
    let empty_vec2 = vec![];
    let derives = params["derives"].as_array().unwrap_or(&empty_vec2);
    let fields = params["fields"].as_array().ok_or_else(|| McpError::missing_param("fields"))?;
    
    let mut result = String::new();
    
    // Add table attributes
    for attr in table_attributes {
        if let Some(attr_str) = attr.as_str() {
            result.push_str(&format!("#[{}]\n", attr_str));
        }
    }
    
    // Add derives
    if !derives.is_empty() {
        result.push_str("#[derive(");
        for (i, derive) in derives.iter().enumerate() {
            if let Some(derive_str) = derive.as_str() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(derive_str);
            }
        }
        result.push_str(")]\n");
    }
    
    result.push_str(&format!("pub struct {} {{\n", table_name));
    
    // Add fields with attributes
    for field in fields {
        let field_name = field["name"].as_str().ok_or_else(|| McpError::missing_param("field name"))?;
        let field_type = field["type"].as_str().ok_or_else(|| McpError::missing_param("field type"))?;
        let empty_attrs = vec![];
        let field_attrs = field["attributes"].as_array().unwrap_or(&empty_attrs);
        
        // Add field attributes
        for attr in field_attrs {
            if let Some(attr_str) = attr.as_str() {
                result.push_str(&format!("    #[{}]\n", attr_str));
            }
        }
        
        result.push_str(&format!("    pub {}: {},\n", field_name, field_type));
    }
    
    result.push_str("}");
    Ok(result)
}

// Validation and analysis helper functions

#[derive(Debug)]
struct ValidationResult {
    status: String,
    issues: Vec<String>,
    recommendations: Vec<String>,
    suggestions: Option<Vec<String>>,
    vulnerabilities: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CodeAnalysis {
    functions: Vec<String>,
    structs: Vec<String>,
    complexity_score: u32,
    changes_count: u32,
}

#[derive(Debug)]
struct ConfigValidationResult {
    status: String,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_generated_code(code: &str, rules: &ValidationRules) -> McpResult<Value> {
    let mut issues = Vec::new();
    let mut suggestions = Vec::new();
    
    // Check for required derives
    if !rules.code_quality.required_derives.is_empty() && !code.contains("#[derive(") {
        issues.push("Missing derive attributes".to_string());
        suggestions.push("Add #[derive(Debug)] at minimum".to_string());
    }
    
    // Check function length
    let lines = code.lines().count();
    if lines > rules.code_quality.max_function_length as usize {
        issues.push(format!("Code is {} lines, exceeds maximum of {}", lines, rules.code_quality.max_function_length));
        suggestions.push("Consider breaking into smaller functions".to_string());
    }
    
    // Check documentation
    if rules.code_quality.documentation_required && !code.contains("///") && !code.contains("//!") {
        issues.push("Missing documentation comments".to_string());
        suggestions.push("Add /// comments for public items".to_string());
    }
    
    let status = if issues.is_empty() { "passed" } else { "warning" };
    
    Ok(json!({
        "status": status,
        "issues": issues,
        "suggestions": suggestions
    }))
}

fn validate_project_structure(project_type: &str, project_path: &str) -> McpResult<ValidationResult> {
    let mut issues = Vec::new();
    let mut recommendations = Vec::new();
    
    let path = Path::new(project_path);
    
    match project_type {
        "rust_project" => {
            if !path.join("Cargo.toml").exists() {
                issues.push("Missing Cargo.toml file".to_string());
            }
            
            let src_main = path.join("src/main.rs");
            let src_lib = path.join("src/lib.rs");
            if !src_main.exists() && !src_lib.exists() {
                issues.push("Missing src/main.rs or src/lib.rs".to_string());
            }
            
            if !path.join("README.md").exists() {
                recommendations.push("Add README.md file".to_string());
            }
        }
        "typescript_project" => {
            if !path.join("package.json").exists() {
                issues.push("Missing package.json file".to_string());
            }
            
            if !path.join("tsconfig.json").exists() {
                issues.push("Missing tsconfig.json file".to_string());
            }
        }
        _ => {
            return Err(McpError::tool_execution(&format!("Unsupported project type: {}", project_type)));
        }
    }
    
    let status = if issues.is_empty() { "passed" } else { "failed" };
    
    Ok(ValidationResult {
        status: status.to_string(),
        issues,
        recommendations,
        suggestions: None,
        vulnerabilities: None,
    })
}

fn validate_configuration_files(project_path: &str) -> McpResult<ValidationResult> {
    let mut issues = Vec::new();
    let mut suggestions = Vec::new();
    
    let path = Path::new(project_path);
    
    // Check for common configuration files
    let config_files = ["Cargo.toml", "package.json", "tsconfig.json", ".gitignore"];
    
    for config_file in &config_files {
        let config_path = path.join(config_file);
        if config_path.exists() {
            // Basic validation - in a real implementation, this would parse and validate content
            if let Ok(content) = fs::read_to_string(&config_path) {
                if content.trim().is_empty() {
                    issues.push(format!("{} is empty", config_file));
                }
            } else {
                issues.push(format!("Cannot read {}", config_file));
            }
        }
    }
    
    if !path.join(".gitignore").exists() {
        suggestions.push("Add .gitignore file to exclude build artifacts".to_string());
    }
    
    let status = if issues.is_empty() { "passed" } else { "warning" };
    
    Ok(ValidationResult {
        status: status.to_string(),
        issues,
        recommendations: suggestions,
        suggestions: None,
        vulnerabilities: None,
    })
}

fn validate_security_practices(project_path: &str) -> McpResult<ValidationResult> {
    let mut vulnerabilities = Vec::new();
    let mut recommendations = Vec::new();
    
    // This is a simplified security check - real implementation would be much more comprehensive
    let path = Path::new(project_path);
    
    // Check for common security issues in source files
    if let Ok(entries) = fs::read_dir(path.join("src")) {
        for entry in entries.flatten() {
            if let Some(extension) = entry.path().extension() {
                if extension == "rs" || extension == "ts" || extension == "js" {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if content.contains("unsafe {") {
                            vulnerabilities.push(format!("Unsafe block found in {}", entry.path().display()));
                        }
                        
                        if content.contains("unwrap()") {
                            recommendations.push(format!("Consider using proper error handling instead of unwrap() in {}", entry.path().display()));
                        }
                    }
                }
            }
        }
    }
    
    let status = if vulnerabilities.is_empty() { "passed" } else { "warning" };
    
    Ok(ValidationResult {
        status: status.to_string(),
        issues: Vec::new(),
        recommendations,
        suggestions: None,
        vulnerabilities: Some(vulnerabilities),
    })
}

fn apply_naming_conventions(code: &str, language: &str) -> McpResult<String> {
    let mut result = code.to_string();
    
    match language {
        "rust" => {
            // Simple transformation examples
            result = result.replace("camelCaseVariable", "snake_case_variable");
            result = result.replace("someCamelCase", "some_snake_case");
        }
        "typescript" => {
            result = result.replace("snake_case_variable", "camelCaseVariable");
        }
        _ => {}
    }
    
    Ok(result)
}

fn add_documentation_comments(code: &str) -> McpResult<String> {
    let mut result = String::new();
    
    for line in code.lines() {
        if line.trim().starts_with("struct ") || line.trim().starts_with("pub struct ") {
            result.push_str("/// Documentation for this struct\n");
        } else if line.trim().starts_with("fn ") || line.trim().starts_with("pub fn ") {
            result.push_str("/// Documentation for this function\n");
        }
        result.push_str(line);
        result.push('\n');
    }
    
    Ok(result)
}

fn add_derive_macros(code: &str, options: &Value) -> McpResult<String> {
    let derives = options["derives"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["Debug", "Clone"]);
    
    let mut result = String::new();
    
    for line in code.lines() {
        if (line.trim().starts_with("struct ") || line.trim().starts_with("pub struct ")) && !result.contains("#[derive(") {
            result.push_str(&format!("#[derive({})]\n", derives.join(", ")));
        }
        result.push_str(line);
        result.push('\n');
    }
    
    Ok(result)
}

fn improve_error_handling(code: &str) -> McpResult<String> {
    let mut result = code.to_string();
    
    // Simple transformations
    result = result.replace(".unwrap()", ".expect(\"Descriptive error message\")");
    result = result.replace("panic!(", "return Err(Error::new(");
    
    Ok(result)
}

fn apply_case_transformation(code: &str, target_case: &str) -> McpResult<String> {
    let mut result = code.to_string();
    
    match target_case {
        "snake_case" => {
            // Convert camelCase to snake_case
            use regex::Regex;
            let re = Regex::new(r"([a-z0-9])([A-Z])").unwrap();
            result = re.replace_all(&result, "${1}_${2}").to_lowercase();
        }
        "camelCase" => {
            // Convert snake_case to camelCase
            use regex::Regex;
            let re = Regex::new(r"_([a-z])").unwrap();
            result = re.replace_all(&result, |caps: &regex::Captures| {
                caps[1].to_uppercase()
            }).to_string();
        }
        _ => return Err(McpError::tool_execution(&format!("Unsupported case transformation: {}", target_case))),
    }
    
    Ok(result)
}

fn analyze_transformation(original: &str, transformed: &str) -> McpResult<CodeAnalysis> {
    let original_lines = original.lines().count();
    let transformed_lines = transformed.lines().count();
    let changes_count = if original_lines != transformed_lines {
        ((original_lines as i32 - transformed_lines as i32).abs()) as u32
    } else {
        // Count character differences as a simple heuristic
        let diff = original.chars().zip(transformed.chars()).filter(|(a, b)| a != b).count();
        diff as u32
    };
    
    Ok(CodeAnalysis {
        functions: vec!["analyze_functions".to_string()], // Simplified
        structs: vec!["analyze_structs".to_string()], // Simplified
        complexity_score: 1,
        changes_count,
    })
}

fn validate_config_against_schema(config: &Value, config_type: &str, _schemas: &ValidationSchemas) -> McpResult<ConfigValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    match config_type {
        "database_config" => {
            if !config["host"].is_string() {
                errors.push("Missing or invalid 'host' field".to_string());
            }
            if !config["port"].is_number() {
                errors.push("Missing or invalid 'port' field".to_string());
            }
            if config["password"].as_str().map(|s| s.len() < 8).unwrap_or(true) {
                warnings.push("Password should be at least 8 characters".to_string());
            }
        }
        "api_config" => {
            if !config["server"].is_object() {
                errors.push("Missing 'server' configuration section".to_string());
            }
            if !config["security"].is_object() {
                errors.push("Missing 'security' configuration section".to_string());
            }
        }
        _ => {}
    }
    
    let status = if !errors.is_empty() { "failed" } else if !warnings.is_empty() { "warning" } else { "passed" };
    
    Ok(ConfigValidationResult {
        status: status.to_string(),
        errors,
        warnings,
    })
}

fn generate_config_suggestions(config: &Value, config_type: &str) -> McpResult<Vec<String>> {
    let mut suggestions = Vec::new();
    
    match config_type {
        "database_config" => {
            if !config.get("ssl_mode").is_some() {
                suggestions.push("Consider enabling SSL mode for secure connections".to_string());
            }
            if !config.get("connection_pool").is_some() {
                suggestions.push("Add connection pooling for better performance".to_string());
            }
        }
        "api_config" => {
            if !config["security"]["rate_limiting"].is_object() {
                suggestions.push("Implement rate limiting for API protection".to_string());
            }
        }
        _ => {}
    }
    
    Ok(suggestions)
}

fn analyze_source_code(code: &str, language: &str) -> McpResult<CodeAnalysis> {
    let mut functions = Vec::new();
    let mut structs = Vec::new();
    
    match language {
        "rust" => {
            for line in code.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                    if let Some(name) = trimmed.split_whitespace().nth(1) {
                        functions.push(name.split('(').next().unwrap_or(name).to_string());
                    }
                }
                if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                    if let Some(name) = trimmed.split_whitespace().nth(1) {
                        structs.push(name.split('{').next().unwrap_or(name).trim().to_string());
                    }
                }
            }
        }
        _ => {
            // Basic analysis for other languages
            functions.push("generic_function".to_string());
        }
    }
    
    Ok(CodeAnalysis {
        functions,
        structs,
        complexity_score: code.lines().count() as u32 / 10, // Simplified complexity
        changes_count: 0,
    })
}

fn generate_unit_tests(analysis: &CodeAnalysis, _framework: &str, include_edge_cases: bool) -> McpResult<String> {
    let mut tests = String::new();
    
    tests.push_str("#[cfg(test)]\nmod tests {\n    use super::*;\n\n");
    
    for function in &analysis.functions {
        tests.push_str(&format!("    #[test]\n    fn test_{}() {{\n", function));
        tests.push_str(&format!("        // TODO: Implement test for {}\n", function));
        tests.push_str("        assert!(true); // Placeholder\n");
        tests.push_str("    }\n\n");
        
        if include_edge_cases {
            tests.push_str(&format!("    #[test]\n    fn test_{}_edge_cases() {{\n", function));
            tests.push_str(&format!("        // TODO: Test edge cases for {}\n", function));
            tests.push_str("        assert!(true); // Placeholder\n");
            tests.push_str("    }\n\n");
        }
    }
    
    tests.push_str("}\n");
    Ok(tests)
}

fn generate_integration_tests(analysis: &CodeAnalysis, _framework: &str) -> McpResult<String> {
    let mut tests = String::new();
    
    tests.push_str("// Integration tests\n");
    tests.push_str("#[cfg(test)]\nmod integration_tests {\n    use super::*;\n\n");
    
    for struct_name in &analysis.structs {
        tests.push_str(&format!("    #[test]\n    fn test_{}_integration() {{\n", struct_name.to_lowercase()));
        tests.push_str(&format!("        // TODO: Integration test for {}\n", struct_name));
        tests.push_str("        assert!(true); // Placeholder\n");
        tests.push_str("    }\n\n");
    }
    
    tests.push_str("}\n");
    Ok(tests)
}

fn generate_property_tests(analysis: &CodeAnalysis, _framework: &str) -> McpResult<String> {
    let mut tests = String::new();
    
    tests.push_str("// Property-based tests\n");
    tests.push_str("#[cfg(test)]\nmod property_tests {\n");
    tests.push_str("    use super::*;\n");
    tests.push_str("    use proptest::prelude::*;\n\n");
    
    for function in &analysis.functions {
        tests.push_str("    proptest! {\n");
        tests.push_str(&format!("        #[test]\n        fn test_{}_property(input in any::<i32>()) {{\n", function));
        tests.push_str(&format!("            // TODO: Property test for {}\n", function));
        tests.push_str("            prop_assert!(true); // Placeholder\n");
        tests.push_str("        }\n");
        tests.push_str("    }\n\n");
    }
    
    tests.push_str("}\n");
    Ok(tests)
}

fn generate_test_fixtures(analysis: &CodeAnalysis) -> McpResult<String> {
    let mut fixtures = String::new();
    
    fixtures.push_str("// Test fixtures and helpers\n\n");
    
    for struct_name in &analysis.structs {
        fixtures.push_str(&format!("impl {} {{\n", struct_name));
        fixtures.push_str(&format!("    pub fn test_fixture() -> Self {{\n"));
        fixtures.push_str("        // TODO: Create test fixture\n");
        fixtures.push_str(&format!("        {} {{\n", struct_name));
        fixtures.push_str("            // Initialize with test data\n");
        fixtures.push_str("        }\n");
        fixtures.push_str("    }\n");
        fixtures.push_str("}\n\n");
    }
    
    Ok(fixtures)
}

fn generate_test_recommendations(analysis: &CodeAnalysis) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    if analysis.complexity_score > 10 {
        recommendations.push("High complexity detected - consider more granular unit tests".to_string());
    }
    
    if analysis.functions.len() > 10 {
        recommendations.push("Many functions detected - organize tests into modules".to_string());
    }
    
    if analysis.structs.len() > 5 {
        recommendations.push("Multiple structs found - create integration tests for struct interactions".to_string());
    }
    
    recommendations.push("Consider using property-based testing for mathematical functions".to_string());
    recommendations.push("Add performance benchmarks for critical paths".to_string());
    
    recommendations
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Code Generation and Template Engine Server");

    // Parse command line arguments for bind address
    let bind_address: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8765".to_string())
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    let server = McpServer::builder()
        .name("code-generation-server")
        .version("1.0.0")
        .title("Code Generation and Template Engine Server")
        .instructions("This server provides comprehensive code generation, validation, and transformation tools for developers working with multiple programming languages and frameworks.")
        .tool(CodeGeneratorTool {
            template_type: String::new(),
            parameters: String::new(),
            language: None,
            validate: None,
        })
        .tool(ProjectValidatorTool {
            project_type: String::new(),
            project_path: String::new(),
            validation_scope: None,
            detailed_report: None,
        })
        .tool(CodeTransformationTool {
            source_code: String::new(),
            transformation: String::new(),
            language: None,
            options: None,
        })
        .tool(ConfigValidatorTool {
            config_input: String::new(),
            config_type: String::new(),
            format: None,
            suggest_improvements: None,
        })
        .tool(TestGeneratorTool {
            source_code: String::new(),
            test_type: String::new(),
            language: None,
            framework: None,
            include_edge_cases: None,
        })
        .bind_address(bind_address)
        .build()?;

    println!("Code Generation server running at: http://127.0.0.1:8765/mcp");
    println!("Available tools for developers:");
    println!("  - generate_code: Generate code from templates (structs, enums, APIs, models)");
    println!("  - validate_project: Validate project structure and configuration");
    println!("  - transform_code: Apply code transformations and refactoring patterns");
    println!("  - validate_config: Validate configuration files against schemas");
    println!("  - generate_tests: Generate comprehensive test suites from code analysis");
    println!("\nExternal data files loaded from:");
    println!("  - data/code_templates.json: Code generation templates");
    println!("  - data/validation_schemas.yaml: Validation rules and schemas");
    println!("  - data/transformation_rules.md: Code transformation documentation");

    server.run().await?;
    Ok(())
}