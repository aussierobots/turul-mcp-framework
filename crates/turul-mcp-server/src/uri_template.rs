//! RFC 6570-inspired URI templates for dynamic MCP resources.
//!
//! Templates such as `file:///user/{user_id}.json` match URIs, extract variables
//! with strict validation, and resolve variables back into URIs.

use regex::Regex;
use std::collections::HashMap;

use crate::McpResult;
use turul_mcp_protocol::McpError;

/// Represents a compiled URI template with validation rules.
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

/// Holds the validation rules for a template variable.
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
    /// Creates a validator for user IDs (alphanumeric, underscore, hyphen).
    pub fn user_id() -> Self {
        Self {
            pattern: Regex::new(r"^[A-Za-z0-9_-]{1,128}$").unwrap(),
            description: "alphanumeric characters, underscore, and hyphen (1-128 chars)"
                .to_string(),
            max_length: 128,
        }
    }

    /// Creates a validator for image formats.
    pub fn image_format() -> Self {
        Self {
            pattern: Regex::new(r"^(png|jpg|jpeg|webp|svg)$").unwrap(),
            description: "valid image format: png, jpg, jpeg, webp, svg".to_string(),
            max_length: 8,
        }
    }

    /// Creates a validator for document formats.
    pub fn document_format() -> Self {
        Self {
            pattern: Regex::new(r"^(pdf|txt|md|json|xml|html)$").unwrap(),
            description: "valid document format: pdf, txt, md, json, xml, html".to_string(),
            max_length: 8,
        }
    }

    /// Creates a custom validator.
    ///
    /// # Errors
    ///
    /// Returns a `McpError::tool_execution` error if `pattern` is not a valid regex.
    pub fn custom(pattern: &str, description: String, max_length: usize) -> McpResult<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| McpError::tool_execution(&format!("Invalid regex pattern: {}", e)))?;

        Ok(Self {
            pattern: regex,
            description,
            max_length,
        })
    }

    /// Validates a variable value.
    ///
    /// # Errors
    ///
    /// Returns a message if the value is longer than the maximum length or does not
    /// match the validator's pattern.
    pub fn validate(&self, value: &str) -> Result<(), String> {
        if value.len() > self.max_length {
            return Err(format!(
                "Value too long: {} characters (max {})",
                value.len(),
                self.max_length
            ));
        }

        if !self.pattern.is_match(value) {
            return Err(format!("Invalid format. Expected: {}", self.description));
        }

        Ok(())
    }
}

impl UriTemplate {
    /// Creates a new URI template with automatic MIME type detection.
    ///
    /// # Errors
    ///
    /// Returns a `McpError::tool_execution` error if the pattern cannot be compiled
    /// into a regex.
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

    /// Creates a template with an explicit MIME type.
    ///
    /// # Errors
    ///
    /// Returns a `McpError::tool_execution` error if the pattern cannot be compiled
    /// into a regex.
    pub fn with_mime_type(pattern: &str, mime_type: &str) -> McpResult<Self> {
        let mut template = Self::new(pattern)?;
        template.mime_type = Some(mime_type.to_string());
        Ok(template)
    }

    /// Adds a validation rule for a variable.
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
            let expression = captures.get(1).unwrap().as_str();
            // RFC 6570 reserved expansion `{+var}`: the value may contain
            // reserved characters, `/` included, so it matches across path
            // segments. A simple `{var}` stays within one segment.
            let (var_name, capture) = match expression.strip_prefix('+') {
                Some(name) => (name, "(.+)"),
                None => (expression, "([^/]+)"),
            };
            self.variables.push(var_name.to_string());

            let escaped_var = regex::escape(&format!("{{{}}}", expression));
            regex_pattern = regex_pattern.replace(&escaped_var, capture);
        }

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

    /// Resolves the template with variables to create an actual URI.
    ///
    /// # Errors
    ///
    /// Returns `McpError::missing_param` if a template variable has no value, or
    /// `McpError::invalid_param_type` if a value fails its validator.
    pub fn resolve(&self, variables: &HashMap<String, String>) -> McpResult<String> {
        let mut result = self.pattern.clone();

        // Validate all required variables are provided
        for var_name in &self.variables {
            let value = variables
                .get(var_name)
                .ok_or_else(|| McpError::missing_param(var_name))?;

            // Apply validation if rules exist
            if let Some(validator) = self.validators.get(var_name) {
                validator.validate(value).map_err(|e| {
                    McpError::invalid_param_type(var_name, &validator.description, &e)
                })?;
            }

            // Replace variable in pattern
            result = result.replace(&format!("{{{}}}", var_name), value);
        }

        Ok(result)
    }

    /// Extracts variables from a URI that matches this template.
    ///
    /// # Errors
    ///
    /// Returns `McpError::invalid_param_type` if the URI does not match the template,
    /// a value is not valid UTF-8 after percent-decoding, a decoded value is rooted,
    /// contains NUL or has a `.` or `..` segment, or a value fails its validator.
    pub fn extract(&self, uri: &str) -> McpResult<HashMap<String, String>> {
        let captures = self
            .regex
            .captures(uri)
            .ok_or_else(|| McpError::invalid_param_type("uri", "URI matching template", uri))?;

        let mut variables = HashMap::new();

        for (i, var_name) in self.variables.iter().enumerate() {
            if let Some(value) = captures.get(i + 1) {
                // URL-decode per RFC 6570 — template variables are percent-encoded in URIs
                let value = urlencoding::decode(value.as_str())
                    .map_err(|e| {
                        McpError::invalid_param_type(
                            var_name,
                            "valid UTF-8 after percent-decoding",
                            &e.to_string(),
                        )
                    })?
                    .into_owned();

                // RFC 6570: a simple `{var}` carries `/` as `%2F`, so a decoded
                // value may contain `/`. It must stay relative and in place: not
                // rooted (a path join would discard its base), no NUL, and not
                // be, or contain, a `.` or `..` segment.
                if value.starts_with(['/', '\\'])
                    || value.contains('\0')
                    || value
                        .split(['/', '\\'])
                        .any(|seg| seg == "." || seg == "..")
                {
                    return Err(McpError::invalid_param_type(
                        var_name,
                        "a relative value without '.' or '..' segments",
                        &value,
                    ));
                }

                // Validate decoded value
                if let Some(validator) = self.validators.get(var_name) {
                    validator.validate(&value).map_err(|e| {
                        McpError::invalid_param_type(var_name, &validator.description, &e)
                    })?;
                }

                variables.insert(var_name.clone(), value);
            }
        }

        Ok(variables)
    }

    /// Checks whether a URI matches this template.
    pub fn matches(&self, uri: &str) -> bool {
        self.regex.is_match(uri)
    }

    /// Returns the MIME type for this template.
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }

    /// Returns the original pattern.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Returns the variable names in order of appearance.
    pub fn variables(&self) -> &[String] {
        &self.variables
    }
}

/// Holds registered URI templates and looks them up by URI or pattern.
#[derive(Debug, Default)]
pub struct UriTemplateRegistry {
    templates: Vec<UriTemplate>,
}

impl UriTemplateRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a URI template.
    pub fn register(&mut self, template: UriTemplate) {
        self.templates.push(template);
    }

    /// Finds the first registered template that matches the given URI.
    pub fn find_matching(&self, uri: &str) -> Option<&UriTemplate> {
        self.templates.iter().find(|t| t.matches(uri))
    }

    /// Returns all registered templates.
    pub fn templates(&self) -> &[UriTemplate] {
        &self.templates
    }

    /// Resolves a registered template pattern with variables.
    ///
    /// # Errors
    ///
    /// Returns `McpError::invalid_param_type` if no registered template has this
    /// pattern, or any error from [`UriTemplate::resolve`].
    pub fn resolve_pattern(
        &self,
        pattern: &str,
        variables: &HashMap<String, String>,
    ) -> McpResult<String> {
        let template = self
            .templates
            .iter()
            .find(|t| t.pattern() == pattern)
            .ok_or_else(|| {
                McpError::invalid_param_type("pattern", "registered template pattern", pattern)
            })?;

        template.resolve(variables)
    }
}

#[cfg(test)]
mod tests {
    /// RFC 6570 §3.2.3 reserved expansion: `{+var}` spans `/`; `{var}` does not.
    #[test]
    fn reserved_expansion_spans_path_segments() {
        let t = UriTemplate::new("skill://reports/daily/{+file}").unwrap();
        assert!(t.matches("skill://reports/daily/data/sub/today.md"));
        assert_eq!(
            t.extract("skill://reports/daily/data/sub/today.md")
                .unwrap()["file"],
            "data/sub/today.md"
        );
        assert!(!t.matches("skill://reports/daily/"));
        let simple = UriTemplate::new("skill://reports/daily/{file}").unwrap();
        assert!(!simple.matches("skill://reports/daily/data/sub/today.md"));
        assert!(simple.matches("skill://reports/daily/today.md"));
    }

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

        let vars = template
            .extract("file:///user/alice123/avatar.png")
            .unwrap();
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
        assert_eq!(
            UriTemplate::detect_mime_type("file.json"),
            Some("application/json".to_string())
        );
        assert_eq!(
            UriTemplate::detect_mime_type("file.pdf"),
            Some("application/pdf".to_string())
        );
        assert_eq!(
            UriTemplate::detect_mime_type("file.png"),
            Some("image/png".to_string())
        );
        assert_eq!(
            UriTemplate::detect_mime_type("file.txt"),
            Some("text/plain".to_string())
        );
        assert_eq!(UriTemplate::detect_mime_type("file.unknown"), None);
        assert_eq!(UriTemplate::detect_mime_type("file"), None);
    }

    #[test]
    fn test_extract_decodes_and_refuses_dot_segments() {
        let template = UriTemplate::new("test://items/{id}").unwrap();
        assert_eq!(template.extract("test://items/a%2Fb").unwrap()["id"], "a/b");
        assert_eq!(template.extract("test://items/A%23b").unwrap()["id"], "A#b");
        assert!(
            !template.matches("test://items/a/b"),
            "raw '/' in a simple var"
        );
        for uri in [
            "test://items/.",
            "test://items/%2e",
            "test://items/..",
            "test://items/a%2F.%2Fb",
            "test://items/a%2F..%2Fb",
            "test://items/a%5C..",
            "test://items/%2Fetc%2Fpasswd",
            "test://items/%5Cetc",
            "test://items/a%00b",
        ] {
            assert!(template.extract(uri).is_err(), "{uri} accepted");
        }

        let reserved = UriTemplate::new("test://files/{+path}").unwrap();
        assert_eq!(
            reserved.extract("test://files/a%2Fb").unwrap()["path"],
            "a/b"
        );
        assert_eq!(reserved.extract("test://files/a/b").unwrap()["path"], "a/b");
        assert!(reserved.extract("test://files/a/./b").is_err());
        assert!(reserved.extract("test://files/a/../b").is_err());
        assert!(reserved.extract("test://files//etc/passwd").is_err());
        assert!(reserved.extract("test://files/%2Fetc").is_err());
    }

    #[test]
    fn test_extract_percent_encoded_hash() {
        let template = UriTemplate::new("custom://items/{item_id}").unwrap();

        // '#' → %23 in the URI
        let vars = template
            .extract("custom://items/PREFIX%23some-value")
            .unwrap();

        assert_eq!(
            vars.get("item_id"),
            Some(&"PREFIX#some-value".to_string()),
            "Percent-encoded '#' should be decoded to '#'"
        );
    }

    #[test]
    fn test_extract_unencoded_values_unchanged() {
        let template = UriTemplate::new("file:///user/{user_id}.json").unwrap();
        let vars = template.extract("file:///user/alice123.json").unwrap();
        assert_eq!(
            vars.get("user_id"),
            Some(&"alice123".to_string()),
            "Plain values should pass through unchanged"
        );
    }

    #[test]
    fn test_extract_percent_encoded_space() {
        let template = UriTemplate::new("file:///docs/{name}").unwrap();
        let vars = template.extract("file:///docs/my%20document").unwrap();
        assert_eq!(
            vars.get("name"),
            Some(&"my document".to_string()),
            "Percent-encoded space should be decoded"
        );
    }

    #[test]
    fn test_extract_percent_encoded_special_chars() {
        let template = UriTemplate::new("data://records/{record_id}").unwrap();

        // '@' → %40, '&' → %26
        let vars = template
            .extract("data://records/user%40host%26extra")
            .unwrap();

        assert_eq!(
            vars.get("record_id"),
            Some(&"user@host&extra".to_string()),
            "Multiple percent-encoded chars should all be decoded"
        );
    }
}
