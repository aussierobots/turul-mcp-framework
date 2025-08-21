# Code Transformation Rules and Patterns

## Overview
This document defines transformation rules, patterns, and best practices for automated code generation and refactoring operations.

## Naming Convention Transformations

### Case Conversions
- **snake_case** → **camelCase**: `user_name` → `userName`
- **camelCase** → **snake_case**: `userName` → `user_name`
- **snake_case** → **PascalCase**: `user_name` → `UserName`
- **kebab-case** → **snake_case**: `user-name` → `user_name`
- **SCREAMING_SNAKE_CASE**: For constants and environment variables

### Language-Specific Naming
#### Rust
- **Structs/Enums**: PascalCase (`UserAccount`, `HttpResponse`)
- **Functions/Variables**: snake_case (`get_user`, `user_count`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_CONNECTIONS`, `DEFAULT_TIMEOUT`)
- **Modules**: snake_case (`user_management`, `api_handlers`)

#### TypeScript/JavaScript
- **Classes/Interfaces**: PascalCase (`UserService`, `ApiResponse`)
- **Functions/Variables**: camelCase (`getUser`, `userCount`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_RETRIES`, `API_BASE_URL`)
- **Files**: kebab-case (`user-service.ts`, `api-client.ts`)

#### Python
- **Classes**: PascalCase (`UserService`, `DatabaseConnection`)
- **Functions/Variables**: snake_case (`get_user`, `connection_pool`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_CONNECTIONS`, `DEFAULT_TIMEOUT`)
- **Modules**: snake_case (`user_service`, `database_utils`)

## Code Structure Transformations

### Adding Documentation
```rust
// Before
struct User {
    id: u64,
    name: String,
}

// After
/// Represents a user in the system
/// 
/// # Example
/// ```rust
/// let user = User {
///     id: 1,
///     name: "Alice".to_string(),
/// };
/// ```
struct User {
    /// Unique identifier for the user
    id: u64,
    /// Display name for the user
    name: String,
}
```

### Adding Derive Macros
```rust
// Before
struct User {
    id: u64,
    name: String,
}

// After
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}
```

### Error Handling Transformation
```rust
// Before
fn divide(a: f64, b: f64) -> f64 {
    a / b  // Can panic on division by zero
}

// After
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Division by zero".to_string())
    } else {
        Ok(a / b)
    }
}
```

## Database Model Transformations

### Basic Model to Full ORM Model
```rust
// Before
struct User {
    id: i32,
    username: String,
    email: String,
}

// After
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct User {
    #[diesel(column_name = user_id)]
    pub id: i32,
    
    #[diesel(column_name = username)]
    pub username: String,
    
    #[diesel(column_name = email_address)]
    pub email: String,
    
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
```

## API Endpoint Transformations

### Basic Function to REST Endpoint
```rust
// Before
fn get_user(id: u64) -> User {
    // Implementation
}

// After
use actix_web::{web, HttpResponse, Result};

#[get("/users/{id}")]
pub async fn get_user(
    path: web::Path<u64>,
    db: web::Data<DatabaseConnection>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    
    match db.find_user_by_id(user_id).await {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(_) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        }))),
    }
}
```

## Configuration Transformations

### Environment Variables to Configuration Struct
```rust
// Generate from environment variables
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: std::env::var("DB_HOST")
                .unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidPort)?,
            username: std::env::var("DB_USERNAME")
                .map_err(|_| ConfigError::MissingUsername)?,
            password: std::env::var("DB_PASSWORD")
                .map_err(|_| ConfigError::MissingPassword)?,
            database: std::env::var("DB_NAME")
                .map_err(|_| ConfigError::MissingDatabaseName)?,
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        })
    }
}
```

## Testing Code Generation

### Generate Unit Tests from Function Signatures
```rust
// For function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Generate test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_positive_numbers() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_add_negative_numbers() {
        assert_eq!(add(-2, -3), -5);
    }

    #[test]
    fn test_add_mixed_signs() {
        assert_eq!(add(-2, 3), 1);
        assert_eq!(add(2, -3), -1);
    }

    #[test]
    fn test_add_with_zero() {
        assert_eq!(add(0, 5), 5);
        assert_eq!(add(5, 0), 5);
        assert_eq!(add(0, 0), 0);
    }
}
```

## Migration Patterns

### Legacy Code Modernization
#### From manual serialization to Serde
```rust
// Before
impl User {
    fn to_json(&self) -> String {
        format!(
            r#"{{"id":{},"name":"{}","email":"{}"}}"#,
            self.id, self.name, self.email
        )
    }
    
    fn from_json(json: &str) -> Result<Self, ParseError> {
        // Manual parsing logic
    }
}

// After
#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

// Automatic serialization/deserialization with serde_json
```

#### From manual error handling to Result types
```rust
// Before
fn process_data(data: &str) -> Option<ProcessedData> {
    if data.is_empty() {
        None
    } else {
        // Processing logic
        Some(processed)
    }
}

// After
#[derive(Debug)]
enum ProcessingError {
    EmptyInput,
    InvalidFormat(String),
    ProcessingFailed,
}

fn process_data(data: &str) -> Result<ProcessedData, ProcessingError> {
    if data.is_empty() {
        return Err(ProcessingError::EmptyInput);
    }
    
    // Processing logic with detailed error handling
    match parse_input(data) {
        Ok(parsed) => process_parsed(parsed)
            .map_err(|_| ProcessingError::ProcessingFailed),
        Err(e) => Err(ProcessingError::InvalidFormat(e.to_string())),
    }
}
```

## Code Quality Transformations

### Performance Optimizations
#### String handling optimization
```rust
// Before (inefficient)
fn build_message(parts: &[&str]) -> String {
    let mut result = String::new();
    for part in parts {
        result = result + part;  // Creates new string each time
    }
    result
}

// After (optimized)
fn build_message(parts: &[&str]) -> String {
    let capacity = parts.iter().map(|s| s.len()).sum();
    let mut result = String::with_capacity(capacity);
    for part in parts {
        result.push_str(part);  // Efficient string building
    }
    result
}
```

### Security Enhancements
#### Input validation transformation
```rust
// Before
fn create_user(username: String, email: String) -> User {
    User { username, email }  // No validation
}

// After
use regex::Regex;
use once_cell::sync::Lazy;

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex")
});

#[derive(Debug)]
enum ValidationError {
    InvalidUsername(String),
    InvalidEmail(String),
}

fn create_user(username: String, email: String) -> Result<User, ValidationError> {
    // Validate username
    if username.is_empty() || username.len() > 50 {
        return Err(ValidationError::InvalidUsername(
            "Username must be 1-50 characters".to_string()
        ));
    }
    
    // Validate email
    if !EMAIL_REGEX.is_match(&email) {
        return Err(ValidationError::InvalidEmail(
            "Invalid email format".to_string()
        ));
    }
    
    Ok(User { username, email })
}
```

## Regular Expression Patterns

### Common transformation patterns
```regex
# Remove trailing whitespace
Pattern: \s+$
Replacement: (empty)

# Convert TODO comments to GitHub issues format
Pattern: // TODO: (.+)
Replacement: // TODO(#issue): $1

# Add pub to struct fields
Pattern: ^(\s+)(\w+): (.+),$
Replacement: $1pub $2: $3,

# Convert println! debug to tracing::debug!
Pattern: println!\("Debug: (.+)"\);
Replacement: tracing::debug!($1);

# Convert unwrap() calls to proper error handling
Pattern: \.unwrap\(\)
Replacement: .expect("Descriptive error message")

# Add Clone derive to structs missing it
Pattern: ^#\[derive\(([^)]*)\)\]\nstruct
Replacement: #[derive($1, Clone)]\nstruct
```

## Automated Refactoring Rules

### Code Style Consistency
1. **Import Organization**: Group and sort imports by category
2. **Line Length**: Break long lines at logical boundaries
3. **Indentation**: Consistent 4-space indentation for Rust, 2-space for JSON/YAML
4. **Trailing Commas**: Add trailing commas in multi-line structures
5. **Blank Lines**: Consistent spacing between code blocks

### Modernization Rules
1. **Replace deprecated APIs**: Update to latest library versions
2. **Use const generics**: Replace old-style generic parameters where applicable
3. **Async/await**: Convert callback-based code to async/await
4. **Pattern matching**: Replace if-else chains with match expressions
5. **Iterator chains**: Replace manual loops with iterator methods

### Security Hardening
1. **Input sanitization**: Add validation to all external inputs
2. **SQL injection prevention**: Convert to parameterized queries
3. **XSS prevention**: Escape HTML output
4. **CSRF protection**: Add CSRF tokens to forms
5. **Secure defaults**: Use secure configuration defaults