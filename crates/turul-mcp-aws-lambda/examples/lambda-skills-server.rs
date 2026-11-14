//! Skills over MCP on AWS Lambda (ADR-033, ADR-035).
//!
//! Serves the repository's plugin skills through `LambdaMcpServerBuilder`,
//! which delegates to `McpServerBuilder::with_ext_skills`, so the Lambda
//! function serves exactly what the HTTP `ext-skills-server` example does. The
//! catalog is read and hashed once per container, at cold start.
//!
//! Locally: `cargo lambda watch -p turul-mcp-aws-lambda --bin lambda-skills-server --features ext-skills`.
//! Deployed: bundle the skill directories with the function and point
//! `SKILLS_DIR` at them.

use std::path::PathBuf;

use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_ext_skills::{SkillEntry, SkillName, SkillPath};
use turul_mcp_server::ext_skills::SkillCatalog;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    let root = std::env::var_os("SKILLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/turul-mcp-skills/skills")
        });
    let mut dirs: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("SKILL.md").is_file())
        .collect();
    dirs.sort();

    let mut catalog = SkillCatalog::new();
    for dir in &dirs {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("non-UTF-8 skill directory")?;
        let path = SkillPath::new([], SkillName::parse(name)?)?;
        catalog.add(SkillEntry::from_dir(path, dir)?)?;
    }
    tracing::info!("serving {} skills from {}", dirs.len(), root.display());

    let server = LambdaMcpServerBuilder::new()
        .name("lambda-skills-server")
        .version(env!("CARGO_PKG_VERSION"))
        .with_ext_skills(catalog)
        .sse(false)
        .build()
        .await?;
    let handler = server.handler().await?;
    turul_mcp_aws_lambda::run_streaming(handler).await
}
