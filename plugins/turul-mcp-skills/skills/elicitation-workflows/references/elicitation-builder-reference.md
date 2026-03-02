# Elicitation Builder Reference

Complete API reference for `ElicitationBuilder`, `ElicitResultBuilder`, `DynamicElicitation`, and convenience constructors.

## ElicitationBuilder

Fluent builder for creating `ElicitCreateRequest` objects.

### Construction

```rust
use turul_mcp_builders::ElicitationBuilder;

let builder = ElicitationBuilder::new("Enter your details");
```

### Metadata

```rust
builder
    .title("User Information")           // Optional dialog title
    .meta_value("request_id", json!("req-123"))  // Add metadata key-value
    .meta(HashMap::from([("key".into(), json!("value"))]))  // Set all metadata
```

### Field Methods

#### String Fields

```rust
// Basic string
.string_field("name", "Your full name")

// With length constraints
.string_field_with_length("username", "Username", Some(3), Some(20))

// With format (Email, Uri, Date, DateTime)
.string_field_with_format("email", "Email address", StringFormat::Email)
.string_field_with_format("website", "Website URL", StringFormat::Uri)
.string_field_with_format("birthday", "Date of birth", StringFormat::Date)
.string_field_with_format("timestamp", "Event time", StringFormat::DateTime)
```

#### Number Fields

```rust
// Float number
.number_field("price", "Item price")

// Float with range
.number_field_with_range("score", "Score (0-100)", Some(0.0), Some(100.0))

// Integer
.integer_field("quantity", "Number of items")

// Integer with range
.integer_field_with_range("age", "Age", Some(0.0), Some(120.0))
```

#### Boolean Fields

```rust
// Basic boolean
.boolean_field("active", "Is active")

// With default value
.boolean_field_with_default("newsletter", "Subscribe to newsletter", false)
```

#### Enum Fields

```rust
// Basic enum (string values)
.enum_field("color", "Favorite color", vec!["red".into(), "green".into(), "blue".into()])

// Enum with display names (user-friendly labels)
.enum_field_with_names(
    "priority",
    "Task priority",
    vec!["p0".into(), "p1".into(), "p2".into()],
    vec!["Critical".into(), "High".into(), "Normal".into()],
)
```

### Required Fields

```rust
// Mark a single field as required
.require_field("name")

// Mark multiple fields as required
.require_fields(vec!["name".into(), "email".into()])
```

### Building

```rust
// Build an ElicitCreateRequest (for sending via the protocol)
let request: ElicitCreateRequest = builder.build();

// Build a DynamicElicitation (with validation traits)
let dynamic: DynamicElicitation = builder.build_dynamic();
```

## Convenience Constructors

One-liner shortcuts that create a builder with a single required field.

| Constructor | Field Type | Required |
|---|---|---|
| `text_input(message, field, desc)` | String | Yes |
| `number_input(message, field, desc, min, max)` | Number (float) | Yes |
| `integer_input(message, field, desc, min, max)` | Number (integer) | Yes |
| `confirm(message)` | Boolean ("confirmed") | Yes |
| `confirm_with_field(message, field, desc)` | Boolean (custom name) | Yes |
| `choice(message, field, desc, choices)` | Enum | Yes |
| `email_input(message, field, desc)` | String (email format) | Yes |
| `url_input(message, field, desc)` | String (URI format) | Yes |
| `form(message)` | (none — chain fields) | — |

### Examples

```rust
use turul_mcp_builders::ElicitationBuilder;

let req = ElicitationBuilder::text_input("Name?", "name", "Full name").build();
let req = ElicitationBuilder::number_input("Score?", "score", "0-100", Some(0.0), Some(100.0)).build();
let req = ElicitationBuilder::confirm("Proceed?").build();
let req = ElicitationBuilder::choice("Color?", "color", "Pick one", vec!["red".into(), "blue".into()]).build();
let req = ElicitationBuilder::email_input("Email?", "email", "Contact").build();
let req = ElicitationBuilder::url_input("Site?", "url", "Website").build();

// Form with multiple fields
let req = ElicitationBuilder::form("Fill in details")
    .string_field("first_name", "First name")
    .string_field("last_name", "Last name")
    .integer_field("age", "Age")
    .require_fields(vec!["first_name".into(), "last_name".into()])
    .build();
```

## ElicitResultBuilder

Static methods for constructing `ElicitResult` objects (useful for testing and mock providers).

```rust
use turul_mcp_builders::ElicitResultBuilder;
use serde_json::json;

// Accept with single field
let result = ElicitResultBuilder::accept_single("name", json!("Alice"));

// Accept with multiple fields
let result = ElicitResultBuilder::accept_fields(vec![
    ("name".into(), json!("Alice")),
    ("age".into(), json!(30)),
]);

// Accept with full content map
let mut content = HashMap::new();
content.insert("name".into(), json!("Alice"));
let result = ElicitResultBuilder::accept(content);

// Decline (user refused)
let result = ElicitResultBuilder::decline();

// Cancel (user cancelled)
let result = ElicitResultBuilder::cancel();
```

## DynamicElicitation

Created by `ElicitationBuilder::build_dynamic()`. Implements `HasElicitationMetadata`, `HasElicitationSchema`, `HasElicitationHandling`, and `ElicitationDefinition` (blanket impl).

### Validation

```rust
let dynamic = ElicitationBuilder::new("Test")
    .string_field_with_length("name", "Name", Some(2), Some(50))
    .number_field_with_range("age", "Age", Some(0.0), Some(120.0))
    .require_fields(vec!["name".into()])
    .build_dynamic();

// Validate content against schema (checks types, required fields, enum values)
let mut content = HashMap::new();
content.insert("name".into(), json!("Alice"));
content.insert("age".into(), json!(25));
assert!(dynamic.validate_content(&content).is_ok());

// Process content (validates + enforces length/range constraints)
let processed = dynamic.process_content(content).unwrap();
```

### Trait Methods

| Trait | Method | Purpose |
|---|---|---|
| `HasElicitationMetadata` | `message()` | Get the user-facing message |
| `HasElicitationMetadata` | `title()` | Get optional dialog title |
| `HasElicitationSchema` | `requested_schema()` | Get the `ElicitationSchema` |
| `HasElicitationSchema` | `validate_schema()` | Validate schema is primitive-only (always Ok) |
| `HasElicitationHandling` | `validate_content(content)` | Check required, types, enum values |
| `HasElicitationHandling` | `process_content(content)` | Validate + normalize (length, ranges) |
| `ElicitationDefinition` | `to_create_request()` | Convert to `ElicitCreateRequest` |

## PrimitiveSchemaDefinition Types

The four primitive types allowed in elicitation schemas:

### StringSchema

| Field | Type | Default | Description |
|---|---|---|---|
| `schema_type` | `String` | `"string"` | Always "string" |
| `title` | `Option<String>` | `None` | Display title |
| `description` | `Option<String>` | `None` | Field description |
| `format` | `Option<StringFormat>` | `None` | Email, Uri, Date, DateTime |
| `min_length` | `Option<usize>` | `None` | Minimum length |
| `max_length` | `Option<usize>` | `None` | Maximum length |

### NumberSchema

| Field | Type | Default | Description |
|---|---|---|---|
| `schema_type` | `String` | `"number"` or `"integer"` | Type discriminator |
| `title` | `Option<String>` | `None` | Display title |
| `description` | `Option<String>` | `None` | Field description |
| `default` | `Option<f64>` | `None` | Default value |
| `minimum` | `Option<f64>` | `None` | Minimum value |
| `maximum` | `Option<f64>` | `None` | Maximum value |

### BooleanSchema

| Field | Type | Default | Description |
|---|---|---|---|
| `schema_type` | `String` | `"boolean"` | Always "boolean" |
| `title` | `Option<String>` | `None` | Display title |
| `description` | `Option<String>` | `None` | Field description |
| `default` | `Option<bool>` | `None` | Default value |

### EnumSchema

| Field | Type | Default | Description |
|---|---|---|---|
| `schema_type` | `String` | `"string"` | Always "string" |
| `title` | `Option<String>` | `None` | Display title |
| `description` | `Option<String>` | `None` | Field description |
| `enum_values` | `Vec<String>` | (required) | Allowed values |
| `enum_names` | `Option<Vec<String>>` | `None` | Display labels (parallel to enum_values) |
