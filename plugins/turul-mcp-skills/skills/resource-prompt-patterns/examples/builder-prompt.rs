// turul-mcp-server v0.3
// Prompt Pattern 3: PromptBuilder
// For runtime-defined prompts with built-in {arg_name} template processing.

use turul_mcp_server::prelude::*;

fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    // Template-based prompt — {arg_name} placeholders auto-replaced
    let summarize = PromptBuilder::new("summarize")
        .description("Summarize text in a given style")
        .string_argument("text", "Text to summarize")
        .optional_string_argument("style", "Summary style (brief, detailed, bullet)")
        .template_user_message("Summarize the following text in {style} style:\n\n{text}")
        .build()?;

    // Dynamic prompt with custom .get() callback
    let translate = PromptBuilder::new("translate")
        .description("Translate text between languages")
        .string_argument("text", "Text to translate")
        .string_argument("target_lang", "Target language")
        .optional_string_argument("source_lang", "Source language (auto-detected if omitted)")
        .get(|args| async move {
            let text = args.get("text").cloned().unwrap_or_default();
            let target = args.get("target_lang").cloned().unwrap_or("English".into());
            let source = args.get("source_lang").cloned().unwrap_or("auto-detect".into());

            Ok(GetPromptResult::new(vec![
                PromptMessage::user_text(format!(
                    "Translate from {source} to {target}:\n\n{text}"
                )),
                PromptMessage::assistant_text(format!(
                    "I'll translate this text to {target}."
                )),
            ]))
        })
        .build()?;

    let server = McpServer::builder()
        .name("language-server")
        .prompt(summarize)
        .prompt(translate)
        .build()?;
    Ok(server)
}
