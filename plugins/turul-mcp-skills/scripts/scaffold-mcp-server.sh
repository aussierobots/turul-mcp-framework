#!/usr/bin/env bash
# scaffold-mcp-server.sh — Generate a new Turul MCP server project
#
# Usage: bash scripts/scaffold-mcp-server.sh <project-name> [--storage <backend>] [directory]
#   project-name: Name for the new server (e.g., "my-mcp-server")
#   --storage:    Storage backend (inmemory|sqlite|postgres|dynamodb). Default: inmemory
#   directory:    Parent directory (defaults to current directory)

set -euo pipefail

# --- Parse arguments ---
PROJECT_NAME=""
STORAGE="inmemory"
PARENT_DIR="."

while [ $# -gt 0 ]; do
    case "$1" in
        --storage)
            if [ $# -lt 2 ]; then
                echo "Error: --storage requires a value (inmemory|sqlite|postgres|dynamodb)"
                exit 1
            fi
            STORAGE="$2"
            shift 2
            ;;
        -*)
            echo "Error: Unknown flag $1"
            echo "Usage: scaffold-mcp-server.sh <project-name> [--storage <backend>] [directory]"
            exit 1
            ;;
        *)
            if [ -z "$PROJECT_NAME" ]; then
                PROJECT_NAME="$1"
            else
                PARENT_DIR="$1"
            fi
            shift
            ;;
    esac
done

if [ -z "$PROJECT_NAME" ]; then
    echo "Usage: scaffold-mcp-server.sh <project-name> [--storage <backend>] [directory]"
    echo "  project-name: Name for the new server (e.g., my-mcp-server)"
    echo "  --storage:    Storage backend (inmemory|sqlite|postgres|dynamodb). Default: inmemory"
    echo "  directory:    Parent directory (defaults to current directory)"
    exit 1
fi

# Validate storage backend
case "$STORAGE" in
    inmemory|sqlite|postgres|dynamodb) ;;
    *)
        echo "Error: Invalid storage backend '$STORAGE'. Must be one of: inmemory, sqlite, postgres, dynamodb"
        exit 1
        ;;
esac

PROJECT_DIR="$PARENT_DIR/$PROJECT_NAME"

# Validate project name (must be valid Rust crate name)
if ! echo "$PROJECT_NAME" | grep -qE '^[a-z][a-z0-9_-]*$'; then
    echo "Error: Project name must be lowercase, start with a letter, and contain only [a-z0-9_-]"
    exit 1
fi

# Convert hyphens to underscores for Rust identifiers
CRATE_IDENT=$(echo "$PROJECT_NAME" | tr '-' '_')

if [ -d "$PROJECT_DIR" ]; then
    echo "Error: Directory $PROJECT_DIR already exists"
    exit 1
fi

echo "=== Scaffolding Turul MCP Server: $PROJECT_NAME ==="
echo "Directory: $PROJECT_DIR"
echo "Storage:   $STORAGE"
echo ""

mkdir -p "$PROJECT_DIR/src"

# --- Generate Cargo.toml ---

# Base dependencies (all backends)
cat > "$PROJECT_DIR/Cargo.toml" << CARGO_EOF
[package]
name = "$PROJECT_NAME"
version = "0.1.0"
edition = "2021"

[dependencies]
CARGO_EOF

# Server dependency with correct features
case "$STORAGE" in
    inmemory)
        cat >> "$PROJECT_DIR/Cargo.toml" << 'CARGO_EOF'
turul-mcp-server = "0.3"
CARGO_EOF
        ;;
    sqlite)
        cat >> "$PROJECT_DIR/Cargo.toml" << 'CARGO_EOF'
turul-mcp-server = { version = "0.3", features = ["http", "sse", "sqlite"] }
turul-mcp-session-storage = { version = "0.3", features = ["sqlite"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls"] }
CARGO_EOF
        ;;
    postgres)
        cat >> "$PROJECT_DIR/Cargo.toml" << 'CARGO_EOF'
turul-mcp-server = { version = "0.3", features = ["http", "sse", "postgres"] }
turul-mcp-session-storage = { version = "0.3", features = ["postgres"] }
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls"] }
CARGO_EOF
        ;;
    dynamodb)
        cat >> "$PROJECT_DIR/Cargo.toml" << 'CARGO_EOF'
turul-mcp-server = { version = "0.3", features = ["http", "sse", "dynamodb"] }
turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
aws-sdk-dynamodb = "1"
aws-config = "1"
CARGO_EOF
        ;;
esac

# Common dependencies (all backends)
cat >> "$PROJECT_DIR/Cargo.toml" << 'CARGO_EOF'
turul-mcp-derive = "0.3"
turul-mcp-protocol = "0.3"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
CARGO_EOF

# --- Generate main.rs ---

case "$STORAGE" in
    inmemory)
        cat > "$PROJECT_DIR/src/main.rs" << 'MAIN_EOF'
// turul-mcp-server v0.3.0
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpServer};

/// A simple tool — replace this with your own implementation.
#[mcp_tool(
    name = "hello",
    description = "Say hello to someone"
)]
async fn hello(
    #[param(description = "Name to greet")] name: String,
) -> McpResult<String> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let server = McpServer::builder()
        .name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .tool_fn(hello)
        .build()?;

    tracing::info!("Starting MCP server...");
    server.run().await?;
    Ok(())
}
MAIN_EOF
        ;;
    sqlite)
        cat > "$PROJECT_DIR/src/main.rs" << 'MAIN_EOF'
// turul-mcp-server v0.3.0 — SQLite session storage
use std::path::PathBuf;
use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpServer};
use turul_mcp_session_storage::{SqliteConfig, SqliteSessionStorage};

/// A simple tool — replace this with your own implementation.
#[mcp_tool(
    name = "hello",
    description = "Say hello to someone"
)]
async fn hello(
    #[param(description = "Name to greet")] name: String,
) -> McpResult<String> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let sqlite_config = SqliteConfig {
        database_path: std::env::var("SQLITE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./sessions.db")),
        create_tables_if_missing: true,
        create_database_if_missing: true,
        ..Default::default()
    };

    let storage = Arc::new(SqliteSessionStorage::with_config(sqlite_config).await?);

    let server = McpServer::builder()
        .name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .with_session_storage(storage)
        .tool_fn(hello)
        .build()?;

    tracing::info!("Starting MCP server with SQLite session storage...");
    server.run().await?;
    Ok(())
}
MAIN_EOF
        ;;
    postgres)
        cat > "$PROJECT_DIR/src/main.rs" << 'MAIN_EOF'
// turul-mcp-server v0.3.0 — PostgreSQL session storage
use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpServer};
use turul_mcp_session_storage::{PostgresConfig, PostgresSessionStorage};

/// A simple tool — replace this with your own implementation.
#[mcp_tool(
    name = "hello",
    description = "Say hello to someone"
)]
async fn hello(
    #[param(description = "Name to greet")] name: String,
) -> McpResult<String> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let postgres_config = PostgresConfig {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://mcp:mcp_pass@localhost:5432/mcp_sessions".to_string()),
        create_tables_if_missing: true,
        ..Default::default()
    };

    let storage = Arc::new(PostgresSessionStorage::with_config(postgres_config).await?);

    let server = McpServer::builder()
        .name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .with_session_storage(storage)
        .tool_fn(hello)
        .build()?;

    tracing::info!("Starting MCP server with PostgreSQL session storage...");
    server.run().await?;
    Ok(())
}
MAIN_EOF
        ;;
    dynamodb)
        cat > "$PROJECT_DIR/src/main.rs" << 'MAIN_EOF'
// turul-mcp-server v0.3.0 — DynamoDB session storage
use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpServer};
use turul_mcp_session_storage::{DynamoDbConfig, DynamoDbSessionStorage};

/// A simple tool — replace this with your own implementation.
#[mcp_tool(
    name = "hello",
    description = "Say hello to someone"
)]
async fn hello(
    #[param(description = "Name to greet")] name: String,
) -> McpResult<String> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let dynamodb_config = DynamoDbConfig {
        table_name: std::env::var("MCP_SESSION_TABLE")
            .unwrap_or_else(|_| "mcp-sessions".to_string()),
        create_tables_if_missing: true,
        ..Default::default()
    };

    let storage = Arc::new(DynamoDbSessionStorage::with_config(dynamodb_config).await?);

    let server = McpServer::builder()
        .name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .with_session_storage(storage)
        .tool_fn(hello)
        .build()?;

    tracing::info!("Starting MCP server with DynamoDB session storage...");
    server.run().await?;
    Ok(())
}
MAIN_EOF
        ;;
esac

# --- Generate .env template for non-inmemory backends ---
case "$STORAGE" in
    sqlite)
        cat > "$PROJECT_DIR/.env.example" << 'ENV_EOF'
# SQLite session storage
SQLITE_PATH=./sessions.db
ENV_EOF
        echo "Created $PROJECT_DIR/.env.example"
        ;;
    postgres)
        cat > "$PROJECT_DIR/.env.example" << 'ENV_EOF'
# PostgreSQL session storage
DATABASE_URL=postgres://mcp:mcp_pass@localhost:5432/mcp_sessions
ENV_EOF
        echo "Created $PROJECT_DIR/.env.example"
        ;;
    dynamodb)
        cat > "$PROJECT_DIR/.env.example" << 'ENV_EOF'
# DynamoDB session storage
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=us-east-1
MCP_SESSION_TABLE=mcp-sessions
ENV_EOF
        echo "Created $PROJECT_DIR/.env.example"
        ;;
esac

echo "Created $PROJECT_DIR/Cargo.toml"
echo "Created $PROJECT_DIR/src/main.rs"
echo ""
echo "=== Next Steps ==="
echo "  cd $PROJECT_DIR"
echo "  cargo check          # Verify it compiles"
echo "  cargo run             # Start the server"
echo ""
echo "To add more tools, see the tool-creation-patterns skill."
echo "For output schemas, see the output-schemas skill."
if [ "$STORAGE" != "inmemory" ]; then
    echo "For storage backend details, see the storage-backend-matrix reference."
fi
