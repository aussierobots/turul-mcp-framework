# ADR-007: Auto-Detection Resource Security Configuration

**Status**: ACTIVE
**Date**: 2025-09-15
**Decision Makers**: Framework Team
**Supersedes**: Manual security configuration patterns

## Context

The MCP framework's default security middleware was blocking custom URI schemes (like `file:///asx/`) used by domain-specific MCP servers. Users were forced to either:

1. Use `.test_mode()` which completely disables ALL security (dangerous in production)
2. Manually configure complex `SecurityMiddleware` settings (violates zero-configuration principle)
3. Work around security restrictions with workarounds

This created a conflict between security and the framework's zero-configuration design principle.

## Problem

- **Default security too restrictive**: Only allowed `file:///[path].(json|txt|md|html)` - blocking `.csv`, custom schemes, etc.
- **test_mode() too broad**: Disables ALL security, not just resource URI restrictions
- **Manual configuration complex**: Users must understand regex patterns, MIME types, security concepts
- **Violates ADR-003**: Zero-configuration principle requires framework to auto-configure correctly

## Decision

**Implement Auto-Detection Resource Security**: Automatically generate security patterns from registered resources.

### Core Design

1. **Auto-Pattern Generation**: When `with_resources()` is called, analyze all registered resources (static + template) and generate appropriate security patterns
2. **Safe Defaults**: Maintain security while allowing exactly what was explicitly registered
3. **Template Support**: Convert URI templates to regex patterns that match valid template instantiations
4. **Extension Detection**: Auto-detect MIME types from file extensions in registered resources

## Implementation

### Builder Integration

```rust
// In with_resources() method
let mut read_handler = if self.test_mode {
    ResourcesReadHandler::new().without_security()
} else if has_resources {
    // Auto-generate security configuration from registered resources
    let security_middleware = self.build_resource_security();
    ResourcesReadHandler::new().with_security(Arc::new(security_middleware))
} else {
    ResourcesReadHandler::new()
};
```

### Auto-Detection Algorithm

```rust
fn build_resource_security(&self) -> SecurityMiddleware {
    let mut allowed_patterns = Vec::new();
    let mut allowed_extensions = HashSet::new();

    // Extract patterns from static resources
    for uri in self.resources.keys() {
        if let Some(extension) = Self::extract_extension(uri) {
            allowed_extensions.insert(extension);
        }
        if let Some(base_pattern) = Self::uri_to_base_pattern(uri) {
            allowed_patterns.push(base_pattern);
        }
    }

    // Extract patterns from template resources
    for (template, _) in &self.template_resources {
        if let Some(pattern) = Self::template_to_regex_pattern(template.pattern()) {
            allowed_patterns.push(pattern);
        }
        if let Some(extension) = Self::extract_extension(template.pattern()) {
            allowed_extensions.insert(extension);
        }
    }

    // Build security middleware with auto-detected patterns
    SecurityMiddleware::new()
        .with_resource_access_control(ResourceAccessControl {
            access_level: AccessLevel::Public, // No session required for registered resources
            allowed_patterns: convert_to_regex(allowed_patterns),
            blocked_patterns: security_defaults(), // Still block directory traversal, etc.
            allowed_mime_types: extensions_to_mime_types(&allowed_extensions),
            ..Default::default()
        })
}
```

### Pattern Generation Examples

| Resource URI | Generated Pattern | Allows |
|-------------|------------------|---------|
| `file:///asx/data.csv` | `^file:///asx/[^/]+$` | Files in `/asx/` directory |
| `file:///api/{id}.json` | `^file:///api/[a-zA-Z0-9_.-]+\.json$` | Template instantiations |

## Benefits

1. **Zero Configuration**: Works automatically without user intervention
2. **Security Maintained**: Only allows explicitly registered resource patterns
3. **Template Support**: Handles both static and template resources correctly
4. **Safe Defaults**: Still blocks directory traversal, system files, executables
5. **Production Ready**: No dangerous "test mode" needed
6. **MCP Compliance**: Supports all standard and custom URI schemes

## Security Properties

### ✅ Still Protected Against
- Directory traversal (`../`, `..\\`)
- System file access (`/etc/`, `/proc/`)
- Executable files (`.exe`, etc.)
- Unauthorized file extensions
- Content size limits (50MB default)

### ✅ Now Allows
- Custom URI schemes registered by resources
- All file extensions used by registered resources (`.csv`, `.json`, etc.)
- Template resource patterns with variable substitution
- Domain-specific resource URIs

## Migration

### Before (Manual Configuration Required)
```rust
// Old approach - manual security configuration
let custom_security = SecurityMiddleware::new()
    .with_resource_access_control(ResourceAccessControl {
        allowed_patterns: vec![
            Regex::new(r"^file:///asx/.*\.(json|csv)$").unwrap()
        ],
        ..Default::default()
    });

let server = McpServer::builder()
    .resource(MyResource::default())
    .with_resources() // Manual call required
    .with_custom_resource_security(custom_security) // Manual security
    .build()?;
```

### After (Zero Configuration)
```rust
// New approach - automatic configuration
let server = McpServer::builder()
    .resource(MyResource::default()) // Auto-detects file:///asx/ scheme
    // .with_resources() called automatically
    // Security auto-configured from registered resources
    .build()?;
```

## Consequences

### Positive
- **Zero Configuration**: Eliminates need for manual security configuration
- **Secure by Default**: Only allows explicitly registered resource patterns
- **Framework Consistency**: Follows ADR-003 zero-configuration principle
- **Production Safe**: No dangerous test_mode bypasses needed

### Neutral
- **Pattern Generation**: Framework must correctly implement regex pattern generation
- **Template Complexity**: URI template to regex conversion requires careful implementation

### Risks
- **Pattern Bugs**: Incorrect regex generation could allow/block unintended resources
- **Performance**: Regex matching overhead (mitigated by compilation caching)

## Validation

### Test Coverage
- Static resource pattern generation
- Template resource pattern generation
- MIME type detection from extensions
- Security bypass prevention
- Production server compatibility

### Success Metrics
- ASX MCP Server works without `.test_mode()`
- All registered resources accessible
- Security restrictions still effective
- Zero manual configuration required

## Notes

This ADR addresses the specific issue where domain-specific MCP servers (like ASX) needed custom URI schemes but were blocked by overly restrictive default security. The auto-detection approach maintains security while enabling the zero-configuration experience.