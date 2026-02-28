#!/usr/bin/env bash
# scaffold-mcp-server.sh — Generate a new Turul MCP server project
#
# Usage: bash scripts/scaffold-mcp-server.sh <project-name> [directory]
#   project-name: Name for the new server (e.g., "my-mcp-server")
#   directory: Parent directory (defaults to current directory)

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: scaffold-mcp-server.sh <project-name> [directory]"
    echo "  project-name: Name for the new server (e.g., my-mcp-server)"
    echo "  directory: Parent directory (defaults to current directory)"
    exit 1
fi

PROJECT_NAME="$1"
PARENT_DIR="${2:-.}"
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
echo ""

mkdir -p "$PROJECT_DIR/src"

# Generate Cargo.toml
cat > "$PROJECT_DIR/Cargo.toml" << CARGO_EOF
[package]
name = "$PROJECT_NAME"
version = "0.1.0"
edition = "2021"

[dependencies]
turul-mcp-server = "0.3"
turul-mcp-derive = "0.3"
turul-mcp-protocol = "0.3"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
CARGO_EOF

# Generate main.rs with function macro pattern (Level 1 — simplest)
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
