//! Skills host driver for the upstream conformance harness's client scenarios
//! (`sep-2640-client-*`), and a usable example of loading skills with
//! verification (ADR-033 §6).
//!
//! The harness sets `MCP_CONFORMANCE_SCENARIO` and passes the server URL as the
//! last argument. `no-prefetch` lists and loads nothing; every other scenario
//! loads each listed skill: `SKILL.md` first, its supporting files only if that
//! verifies.
//!
//! ```bash
//! cargo run -p ext-skills-server --bin ext-skills-client -- http://127.0.0.1:8650/mcp
//! ```

use turul_mcp_client::McpClient;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_ext_skills::SkillResources;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args()
        .next_back()
        .filter(|a| a.starts_with("http"))
        .unwrap_or_else(|| "http://127.0.0.1:8650/mcp".to_string());
    let scenario = std::env::var("MCP_CONFORMANCE_SCENARIO").unwrap_or_default();

    let client = McpClient::new(Box::new(HttpTransport::new(&url)?), Default::default());
    client.connect().await?;

    let mut skills = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = client.skills_list(cursor.as_deref()).await?;
        skills.extend(page.skills);
        match page.next_cursor {
            Some(c) => cursor = Some(c.0),
            None => break,
        }
    }
    println!("listed {} skills", skills.len());
    if scenario.ends_with("no-prefetch") {
        return Ok(());
    }

    for skill in &skills {
        match client.skill_fetch(skill, &skill.uri).await {
            Err(e) => {
                println!("rejected {}: {e}", skill.uri);
                continue;
            }
            Ok(_) => println!("loaded {}", skill.uri),
        }
        if let SkillResources::Manifest(files) = &skill.resources {
            for f in files.iter().filter(|f| f.uri() != skill.uri) {
                match client.skill_fetch(skill, f.uri()).await {
                    Ok(bytes) => println!("  {} ({} bytes, verified)", f.uri(), bytes.len()),
                    Err(e) => println!("  rejected {}: {e}", f.uri()),
                }
            }
        }
    }
    Ok(())
}
