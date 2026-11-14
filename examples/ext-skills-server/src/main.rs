//! # Skills Extension Server (`io.modelcontextprotocol/skills`, SEP-2640)
//!
//! Serves this repository's own plugin skills
//! (`plugins/turul-mcp-skills/skills/*`) over MCP: `skills/list`,
//! `skills/get`, and every skill file through `resources/read` with the digest
//! and size its manifest declares (ADR-033).
//!
//! ```bash
//! cargo run -p ext-skills-server            # http://127.0.0.1:8650/mcp
//! cargo run -p ext-skills-server -- 9000    # another port
//! ```

use std::path::Path;

use turul_mcp_ext_skills::{SkillEntry, SkillName, SkillPath};
use turul_mcp_server::ext_skills::SkillCatalog;
use turul_mcp_server::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let port: u16 = std::env::args()
        .nth(1)
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(8650);

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/turul-mcp-skills/skills");
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
    println!(
        "serving {} skills on http://127.0.0.1:{port}/mcp",
        dirs.len()
    );

    let server = McpServer::builder()
        .name("ext-skills-server")
        .version(env!("CARGO_PKG_VERSION"))
        .with_ext_skills(catalog)
        .bind_address(format!("127.0.0.1:{port}").parse()?)
        .build()?;
    server.run().await?;
    Ok(())
}
