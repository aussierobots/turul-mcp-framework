//! URI Template System for Dynamic Resources
//!
//! This module provides RFC 6570-inspired URI template support for dynamic MCP resources.
//! It enables patterns like `file:///user/{user_id}.json` with strict validation.

use std::collections::HashMap;
use regex::Regex;

use crate::McpResult;
use turul_mcp_protocol::McpError;

/// A compiled URI template with validation rules
#[derive(Debug, Clone)]
pub struct UriTemplate {
    /// Original template pattern
    pattern: String,
    /// Compiled regex for matching and extracting variables
    regex: Regex,
    /// Variable names in order of appearance
    variables: Vec<String>,
    /// Validation rules for each variable
    validators: HashMap<String, VariableValidator>,
    /// MIME type mapping based on file extension
    mime_type: Option<String>,
}

/// Validation rules for template variables
#[derive(Debug, Clone)]
pub struct VariableValidator {
    /// Regex pattern for valid values
    pattern: Regex,
    /// Human-readable description of valid format
    description: String,
    /// Maximum length
    max_length: usize,
}

impl VariableValidator {
    /// Create validator for user IDs (alphanumeric, underscore, hyphen)
    pub fn user_id() -> Self {
        Self {
            pattern: Regex::new(r"^[A-Za-z0-9_-]{1,128}$").unwrap(),
            description: "alphanumeric characters, underscore, and hyphen (1-128 chars)".to_string(),
            max_length: 128,
        }
    }
    
    /// Create validator for image formats
    pub fn image_format() -> Self {
        Self {
            pattern: Regex::new(r"^(png|jpg|jpeg|webp|svg)$").unwrap(),
            description: "valid image format: png, jpg, jpeg, webp, svg".to_string(),
            max_length: 8,
        }
    }
    
    /// Create validator for document formats
    pub fn document_format() -> Self {
        Self {
            pattern: Regex::new(r"^(pdf|txt|md|json|xml|html)$").unwrap(),
            description: "valid document format: pdf, txt, md, json, xml, html".to_string(),
            max_length: 8,
        }
    }
    
    /// Create custom validator
    pub fn custom(pattern: &str, description: String, max_length: usize) -> McpResult<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| McpError::tool_execution(&format!("Invalid regex pattern: {}", e)))?;
        
        Ok(Self {
            pattern: regex,
            description,
            max_length,
        })
    }
    
    /// Validate a variable value
    pub fn validate(&self, value: &str) -> Result<(), String> {
        if value.len() > self.max_length {
            return Err(format!(
                "Value too long: {} characters (max {})", 
                value.len(), 
                self.max_length
            ));
        }
        
        if !self.pattern.is_match(value) {
            return Err(format!(
                "Invalid format. Expected: {}", 
                self.description
            ));
        }
        
        Ok(())
    }
}

impl UriTemplate {
    /// Create a new URI template with automatic MIME type detection
    pub fn new(pattern: &str) -> McpResult<Self> {
        let mut template = Self {
            pattern: pattern.to_string(),
            regex: Regex::new("").unwrap(), // Placeholder
            variables: Vec::new(),
            validators: HashMap::new(),
            mime_type: Self::detect_mime_type(pattern),
        };
        
        template.compile()?;
        Ok(template)
    }
    
    /// Create template with explicit MIME type
    pub fn with_mime_type(pattern: &str, mime_type: &str) -> McpResult<Self> {
        let mut template = Self::new(pattern)?;
        template.mime_type = Some(mime_type.to_string());
        Ok(template)
    }
    
    /// Add validation rule for a variable
    pub fn with_validator(mut self, variable: &str, validator: VariableValidator) -> Self {
        self.validators.insert(variable.to_string(), validator);
        self
    }
    
    /// Compile the template pattern into a regex
    fn compile(&mut self) -> McpResult<()> {
        // Extract variables from pattern like {user_id}
        let var_regex = Regex::new(r"\{([^}]+)\}").unwrap();
        let mut regex_pattern = regex::escape(&self.pattern);
        
        for captures in var_regex.captures_iter(&self.pattern) {
            let var_name = captures.get(1).unwrap().as_str();
            self.variables.push(var_name.to_string());
            
            // Replace {var_name} with capture group
            let escaped_var = regex::escape(&format!("{{{}}}", var_name));
            regex_pattern = regex_pattern.replace(&escaped_var, "([^/]+)");
        }
        
        // Anchor the pattern
        regex_pattern = format!("^{}$", regex_pattern);
        
        self.regex = Regex::new(&regex_pattern)
            .map_err(|e| McpError::tool_execution(&format!("Failed to compile template: {}", e)))?;
        
        Ok(())
    }
    
    /// Detect MIME type from file extension in pattern
    fn detect_mime_type(pattern: &str) -> Option<String> {
        if let Some(ext_start) = pattern.rfind('.') {
            let ext = &pattern[ext_start + 1..];
            // Remove any template variables from extension
            let ext = ext.split('}').next().unwrap_or(ext);
            
            match ext {
                "json" => Some("application/json".to_string()),
                "txt" => Some("text/plain".to_string()),
                "md" => Some("text/markdown".to_string()),
                "html" => Some("text/html".to_string()),
                "xml" => Some("application/xml".to_string()),
                "pdf" => Some("application/pdf".to_string()),
                "png" => Some("image/png".to_string()),
                "jpg" | "jpeg" => Some("image/jpeg".to_string()),
                "webp" => Some("image/webp".to_string()),
                "svg" => Some("image/svg+xml".to_string()),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// Resolve template with variables to create actual URI
    pub fn resolve(&self, variables: &HashMap<String, String>) -> McpResult<String> {
        let mut result = self.pattern.clone();
        
        // Validate all required variables are provided
        for var_name in &self.variables {
            let value = variables.get(var_name)
                .ok_or_else(|| McpError::missing_param(var_name))?;
            
            // Apply validation if rules exist
            if let Some(validator) = self.validators.get(var_name) {
                validator.validate(value)
                    .map_err(|e| McpError::invalid_param_type(var_name, &validator.description, &e))?;
            }
            
            // Replace variable in pattern
            result = result.replace(&format!("{{{}}}", var_name), value);
        }
        
        Ok(result)
    }
    
    /// Extract variables from a URI that matches this template
    pub fn extract(&self, uri: &str) -> McpResult<HashMap<String, String>> {
        let captures = self.regex.captures(uri)
            .ok_or_else(|| McpError::invalid_param_type("uri", "URI matching template", uri))?;
        
        let mut variables = HashMap::new();
        
        for (i, var_name) in self.variables.iter().enumerate() {
            if let Some(value) = captures.get(i + 1) {
                let value = value.as_str().to_string();
                
                // Validate extracted value
                if let Some(validator) = self.validators.get(var_name) {
                    validator.validate(&value)
                        .map_err(|e| McpError::invalid_param_type(var_name, &validator.description, &e))?;
                }
                
                variables.insert(var_name.clone(), value);
            }
        }
        
        Ok(variables)
    }
    
    /// Check if a URI matches this template
    pub fn matches(&self, uri: &str) -> bool {
        self.regex.is_match(uri)
    }
    
    /// Get the MIME type for this template
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }
    
    /// Get the original pattern
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
    
    /// Get variable names
    pub fn variables(&self) -> &[String] {
        &self.variables
    }
}

/// Registry for managing URI templates
#[derive(Debug, Default)]
pub struct UriTemplateRegistry {
    templates: Vec<UriTemplate>,
}

impl UriTemplateRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a URI template
    pub fn register(&mut self, template: UriTemplate) {
        self.templates.push(template);
    }
    
    /// Find template that matches the given URI
    pub fn find_matching(&self, uri: &str) -> Option<&UriTemplate> {
        self.templates.iter().find(|t| t.matches(uri))
    }
    
    /// Get all registered templates
    pub fn templates(&self) -> &[UriTemplate] {
        &self.templates
    }
    
    /// Resolve a template pattern with variables
    pub fn resolve_pattern(&self, pattern: &str, variables: &HashMap<String, String>) -> McpResult<String> {
        let template = self.templates.iter()
            .find(|t| t.pattern() == pattern)
            .ok_or_else(|| McpError::invalid_param_type("pattern", "registered template pattern", pattern))?;
        
        template.resolve(variables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_id_validator() {
        let validator = VariableValidator::user_id();
        
        // Valid cases
        assert!(validator.validate("user123").is_ok());
        assert!(validator.validate("user_id").is_ok());
        assert!(validator.validate("user-name").is_ok());
        assert!(validator.validate("ABC123").is_ok());
        
        // Invalid cases
        assert!(validator.validate("user@example.com").is_err()); // @ not allowed
        assert!(validator.validate("user with spaces").is_err()); // spaces not allowed
        assert!(validator.validate("").is_err()); // empty not allowed
        assert!(validator.validate(&"a".repeat(129)).is_err()); // too long
    }
    
    #[test]
    fn test_image_format_validator() {
        let validator = VariableValidator::image_format();
        
        assert!(validator.validate("png").is_ok());
        assert!(validator.validate("jpg").is_ok());
        assert!(validator.validate("jpeg").is_ok());
        assert!(validator.validate("webp").is_ok());
        assert!(validator.validate("svg").is_ok());
        
        assert!(validator.validate("gif").is_err()); // not in allowlist
        assert!(validator.validate("PNG").is_err()); // case sensitive
        assert!(validator.validate("pdf").is_err()); // not an image
    }
    
    #[test]
    fn test_uri_template_creation() {
        let template = UriTemplate::new("file:///user/{user_id}.json").unwrap();
        assert_eq!(template.pattern(), "file:///user/{user_id}.json");
        assert_eq!(template.variables(), &["user_id"]);
        assert_eq!(template.mime_type(), Some("application/json"));
    }
    
    #[test]
    fn test_uri_template_resolution() {
        let template = UriTemplate::new("file:///user/{user_id}.json")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id());
        
        let mut vars = HashMap::new();
        vars.insert("user_id".to_string(), "alice123".to_string());
        
        let resolved = template.resolve(&vars).unwrap();
        assert_eq!(resolved, "file:///user/alice123.json");
    }
    
    #[test]
    fn test_uri_template_extraction() {
        let template = UriTemplate::new("file:///user/{user_id}.json")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id());
        
        let vars = template.extract("file:///user/alice123.json").unwrap();
        assert_eq!(vars.get("user_id"), Some(&"alice123".to_string()));
    }
    
    #[test]
    fn test_uri_template_validation_failure() {
        let template = UriTemplate::new("file:///user/{user_id}.json")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id());
        
        // Invalid user_id should fail extraction
        let result = template.extract("file:///user/invalid@user.json");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_multiple_variables() {
        let template = UriTemplate::new("file:///user/{user_id}/avatar.{format}")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id())
            .with_validator("format", VariableValidator::image_format());
        
        let vars = template.extract("file:///user/alice123/avatar.png").unwrap();
        assert_eq!(vars.get("user_id"), Some(&"alice123".to_string()));
        assert_eq!(vars.get("format"), Some(&"png".to_string()));
    }
    
    #[test]
    fn test_registry() {
        let mut registry = UriTemplateRegistry::new();
        
        let template1 = UriTemplate::new("file:///user/{user_id}.json").unwrap();
        let template2 = UriTemplate::new("file:///user/{user_id}/avatar.{format}").unwrap();
        
        registry.register(template1);
        registry.register(template2);
        
        let found = registry.find_matching("file:///user/alice123.json");
        assert!(found.is_some());
        assert_eq!(found.unwrap().pattern(), "file:///user/{user_id}.json");
    }
    
    #[test]
    fn test_mime_type_detection() {
        assert_eq!(UriTemplate::detect_mime_type("file.json"), Some("application/json".to_string()));
        assert_eq!(UriTemplate::detect_mime_type("file.pdf"), Some("application/pdf".to_string()));
        assert_eq!(UriTemplate::detect_mime_type("file.png"), Some("image/png".to_string()));
        assert_eq!(UriTemplate::detect_mime_type("file.txt"), Some("text/plain".to_string()));
        assert_eq!(UriTemplate::detect_mime_type("file.unknown"), None);
        assert_eq!(UriTemplate::detect_mime_type("file"), None);
    }
}