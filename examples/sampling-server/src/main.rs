//! # Sampling Server Example
//!
//! This example demonstrates MCP sampling functionality for AI model sampling
//! through the MCP protocol. Sampling allows MCP servers to request AI model
//! generation from clients, enabling interactive AI-assisted workflows.

use std::collections::HashMap;
use async_trait::async_trait;
use mcp_server::{McpServer, McpTool};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpResult};
use mcp_protocol::sampling::{CreateMessageRequest, SamplingMessage, MessageContent};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;
use chrono::Utc;
use rand::Rng;

/// Tool to demonstrate basic sampling functionality
struct BasicSamplingTool;

#[async_trait]
impl McpTool for BasicSamplingTool {
    fn name(&self) -> &str {
        "basic_sampling"
    }

    fn description(&self) -> &str {
        "Demonstrate basic AI model sampling through MCP"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("prompt".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["prompt".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let user_prompt = args.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("prompt"))?;

        // Create a sampling request
        let messages = vec![
            SamplingMessage {
                role: "user".to_string(),
                content: MessageContent::Text {
                    text: user_prompt.to_string(),
                },
            }
        ];

        let request = CreateMessageRequest::new(messages, 500)
            .with_temperature(0.7);

        // In a real implementation, this would be sent to the MCP client
        // For demonstration, we'll simulate a response
        let simulated_response = format!(
            "🤖 SAMPLING REQUEST CREATED\n\
            \n\
            Request ID: {}\n\
            Timestamp: {}\n\
            User Prompt: \"{}\"\n\
            Temperature: {:?}\n\
            Max Tokens: {:?}\n\
            \n\
            📝 Note: In a real MCP implementation, this request would be sent to the client\n\
            for processing by their AI model, and the generated response would be returned\n\
            to the server for further processing.",
            Uuid::new_v4(),
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            user_prompt,
            request.params.temperature,
            request.params.max_tokens
        );

        let results = vec![ToolResult::text(simulated_response)];
        Ok(results)
    }
}

/// Tool to demonstrate conversational sampling with history
struct ConversationalSamplingTool;

#[async_trait]
impl McpTool for ConversationalSamplingTool {
    fn name(&self) -> &str {
        "conversational_sampling"
    }

    fn description(&self) -> &str {
        "Demonstrate conversational AI sampling with message history and context"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("user_message".to_string(), JsonSchema::string());
        properties.insert("conversation_context".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["user_message".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let user_message = args.get("user_message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("user_message"))?;
            
        let context = args.get("conversation_context")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        // Build conversation history
        let mut messages = vec![];
        
        // Add context-based system message
        let system_message = match context {
            "technical" => "You are a technical expert. Provide detailed, accurate technical explanations.",
            "creative" => "You are a creative writing assistant. Help with storytelling and creative content.",
            "educational" => "You are an educational tutor. Explain concepts clearly for learning.",
            _ => "You are a helpful AI assistant. Provide balanced, informative responses.",
        };

        // Add conversation history (simulated)
        messages.extend(vec![
            SamplingMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text {
                    text: "Hello! I'm ready to help. What would you like to discuss?".to_string(),
                },
            },
            SamplingMessage {
                role: "user".to_string(),
                content: MessageContent::Text {
                    text: user_message.to_string(),
                },
            },
        ]);

        let request = CreateMessageRequest::new(messages, 750)
            .with_temperature(0.8);

        let simulated_response = format!(
            "💬 CONVERSATIONAL SAMPLING REQUEST\n\
            \n\
            Context: {} mode\n\
            System Context: {}\n\
            User Message: \"{}\"\n\
            Temperature: {:?} (more creative)\n\
            Max Tokens: {:?}\n\
            \n\
            📚 CONVERSATION FLOW:\n\
            1. Assistant: \"Hello! I'm ready to help...\"\n\
            2. User: \"{}\"\n\
            3. [AI Response would be generated here]\n\
            \n\
            🔄 The AI model would consider the full conversation context when generating its response.",
            context,
            system_message,
            user_message,
            request.params.temperature,
            request.params.max_tokens,
            user_message
        );

        let results = vec![ToolResult::text(simulated_response)];
        Ok(results)
    }
}

/// Tool to demonstrate code generation sampling
struct CodeGenerationSamplingTool;

#[async_trait]
impl McpTool for CodeGenerationSamplingTool {
    fn name(&self) -> &str {
        "code_generation_sampling"
    }

    fn description(&self) -> &str {
        "Demonstrate AI model sampling for code generation with specific constraints"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("task_description".to_string(), JsonSchema::string());
        properties.insert("programming_language".to_string(), JsonSchema::string());
        properties.insert("complexity_level".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["task_description".to_string(), "programming_language".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let task_description = args.get("task_description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("task_description"))?;
            
        let language = args.get("programming_language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("programming_language"))?;
            
        let complexity = args.get("complexity_level")
            .and_then(|v| v.as_str())
            .unwrap_or("intermediate");

        let _system_prompt = format!(
            "You are an expert {} programmer. Generate clean, well-documented, and efficient code. \
             Follow best practices for {} development. Complexity level: {}. \
             Include comments explaining the logic and any important implementation details.",
            language, language, complexity
        );

        let code_prompt = format!(
            "Please write {} code for the following task:\n\n{}\n\n\
             Requirements:\n\
             - Follow {} best practices and conventions\n\
             - Include appropriate error handling\n\
             - Add clear comments explaining the logic\n\
             - Optimize for readability and maintainability\n\
             - Complexity level: {}",
            language, task_description, language, complexity
        );

        let messages = vec![
            SamplingMessage {
                role: "user".to_string(),
                content: MessageContent::Text {
                    text: code_prompt,
                },
            }
        ];

        let request = CreateMessageRequest::new(messages, 1500)
            .with_temperature(0.3); // Lower temperature for more deterministic code

        let simulated_response = format!(
            "💻 CODE GENERATION SAMPLING REQUEST\n\
            \n\
            Language: {}\n\
            Task: \"{}\"\n\
            Complexity: {}\n\
            Temperature: {:?} (precise, deterministic)\n\
            Max Tokens: {:?}\n\
            \n\
            🎯 SPECIALIZED PROMPTING:\n\
            System Context: \"You are an expert {} programmer...\"\n\
            \n\
            📋 CODE REQUIREMENTS:\n\
            ✅ Follow {} conventions\n\
            ✅ Include error handling\n\
            ✅ Add explanatory comments\n\
            ✅ Optimize for readability\n\
            ✅ {} complexity level\n\
            \n\
            🔧 The AI model would generate production-ready code following these specifications.",
            language,
            task_description,
            complexity,
            request.params.temperature,
            request.params.max_tokens,
            language,
            language,
            complexity
        );

        let results = vec![ToolResult::text(simulated_response)];
        Ok(results)
    }
}

/// Tool to demonstrate creative writing sampling
struct CreativeWritingSamplingTool;

#[async_trait]
impl McpTool for CreativeWritingSamplingTool {
    fn name(&self) -> &str {
        "creative_writing_sampling"
    }

    fn description(&self) -> &str {
        "Demonstrate AI model sampling for creative writing with style and genre controls"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("writing_prompt".to_string(), JsonSchema::string());
        properties.insert("genre".to_string(), JsonSchema::string());
        properties.insert("style".to_string(), JsonSchema::string());
        properties.insert("length".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["writing_prompt".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let writing_prompt = args.get("writing_prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("writing_prompt"))?;
            
        let genre = args.get("genre")
            .and_then(|v| v.as_str())
            .unwrap_or("general fiction");
            
        let style = args.get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("descriptive");
            
        let length = args.get("length")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        let word_count = match length {
            "short" => (100, 300),
            "medium" => (300, 600),
            "long" => (600, 1000),
            _ => (300, 600),
        };

        let _system_prompt = format!(
            "You are a creative writing assistant specializing in {}. \
             Write in a {} style with rich imagery and engaging narrative. \
             Target length: {}-{} words. Focus on compelling storytelling, \
             character development, and atmospheric description.",
            genre, style, word_count.0, word_count.1
        );

        let enhanced_prompt = format!(
            "Creative Writing Request:\n\n\
             Prompt: {}\n\
             Genre: {}\n\
             Style: {}\n\
             Target Length: {} ({}-{} words)\n\n\
             Please create an engaging piece that brings this prompt to life with \
             vivid descriptions, compelling characters, and an immersive atmosphere.",
            writing_prompt, genre, style, length, word_count.0, word_count.1
        );

        let messages = vec![
            SamplingMessage {
                role: "user".to_string(),
                content: MessageContent::Text {
                    text: enhanced_prompt,
                },
            }
        ];

        let temperature = match style {
            "experimental" => 0.9,
            "creative" => 0.8,
            "descriptive" => 0.7,
            "formal" => 0.5,
            _ => 0.75,
        };

        let request = CreateMessageRequest::new(messages, 1200)
            .with_temperature(temperature);

        let simulated_response = format!(
            "✨ CREATIVE WRITING SAMPLING REQUEST\n\
            \n\
            📖 Writing Parameters:\n\
            Prompt: \"{}\"\n\
            Genre: {}\n\
            Style: {}\n\
            Length: {} ({}-{} words)\n\
            Temperature: {:?} (creativity level)\n\
            \n\
            🎨 CREATIVE CONFIGURATION:\n\
            System Context: \"You are a creative writing assistant...\"\n\
            Max Tokens: {:?}\n\
            \n\
            📝 WRITING GUIDELINES:\n\
            ✅ Rich imagery and descriptions\n\
            ✅ Compelling character development\n\
            ✅ Atmospheric storytelling\n\
            ✅ Genre-appropriate themes\n\
            ✅ {} narrative style\n\
            \n\
            🌟 The AI model would generate original creative content following these parameters.",
            writing_prompt,
            genre,
            style,
            length,
            word_count.0,
            word_count.1,
            request.params.temperature,
            request.params.max_tokens,
            style
        );

        let results = vec![ToolResult::text(simulated_response)];
        Ok(results)
    }
}

/// Tool to demonstrate advanced sampling with multiple model preferences
struct AdvancedSamplingTool;

#[async_trait]
impl McpTool for AdvancedSamplingTool {
    fn name(&self) -> &str {
        "advanced_sampling_demo"
    }

    fn description(&self) -> &str {
        "Demonstrate advanced MCP sampling features including model preferences, constraints, and metadata"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("task_type".to_string(), JsonSchema::string());
        properties.insert("complexity_level".to_string(), JsonSchema::string());
        properties.insert("output_format".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["task_type".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let task_type = args.get("task_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("task_type"))?;
            
        let complexity = args.get("complexity_level")
            .and_then(|v| v.as_str())
            .unwrap_or("standard");
            
        let output_format = args.get("output_format")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        let mut rng = rand::rng();
        let session_id = Uuid::new_v4();
        let request_id = format!("req_{}", rng.random::<u32>());

        // Configure sampling parameters based on task type
        let (system_prompt, hints, temperature, max_tokens) = match task_type {
            "analysis" => (
                "You are an analytical expert. Provide structured, evidence-based analysis with clear reasoning.".to_string(),
                vec!["analytical".to_string(), "structured".to_string(), "evidence_based".to_string()],
                0.4,
                1000
            ),
            "brainstorming" => (
                "You are a creative brainstorming assistant. Generate diverse, innovative ideas with explanations.".to_string(),
                vec!["creative".to_string(), "innovative".to_string(), "divergent_thinking".to_string()],
                0.8,
                800
            ),
            "problem_solving" => (
                "You are a problem-solving expert. Provide step-by-step solutions with clear methodology.".to_string(),
                vec!["systematic".to_string(), "methodical".to_string(), "solution_oriented".to_string()],
                0.5,
                1200
            ),
            "explanation" => (
                "You are an educational expert. Provide clear, comprehensive explanations with examples.".to_string(),
                vec!["educational".to_string(), "clear".to_string(), "comprehensive".to_string()],
                0.6,
                900
            ),
            _ => (
                "You are a helpful AI assistant. Provide balanced, informative responses.".to_string(),
                vec!["helpful".to_string(), "balanced".to_string(), "informative".to_string()],
                0.7,
                750
            ),
        };

        let enhanced_prompt = format!(
            "Task Type: {}\n\
            Complexity Level: {}\n\
            Output Format: {}\n\
            \n\
            Please complete this {} task with {} complexity, \
            formatted as {}. Ensure your response is well-structured \
            and addresses all aspects of the request.",
            task_type, complexity, output_format,
            task_type, complexity, output_format
        );

        let messages = vec![
            SamplingMessage {
                role: "user".to_string(),
                content: MessageContent::Text {
                    text: enhanced_prompt,
                },
            }
        ];

        let mut model_hints = hints.clone();
        model_hints.push(complexity.to_string());
        model_hints.push(output_format.replace(" ", "_"));

        let request = CreateMessageRequest::new(messages, max_tokens)
            .with_temperature(temperature);

        let simulated_response = format!(
            "🚀 ADVANCED SAMPLING CONFIGURATION\n\
            \n\
            📊 REQUEST PARAMETERS:\n\
            Request ID: {}\n\
            Session ID: {}\n\
            Task Type: {}\n\
            Complexity: {}\n\
            Output Format: {}\n\
            Temperature: {:?} (precision vs creativity balance)\n\
            Max Tokens: {:?}\n\
            \n\
            🎯 MODEL CONFIGURATION:\n\
            System Context: \"{}\"\n\
            Task Hints: {:?}\n\
            \n\
            📈 PERFORMANCE TARGETS:\n\
            ✅ Optimal model selection\n\
            ✅ Parameter tuning\n\
            ✅ Quality optimization\n\
            ✅ Format compliance: {}\n\
            \n\
            🔄 The MCP client would process this request with optimal model selection and parameter tuning.",
            request_id,
            session_id,
            task_type,
            complexity,
            output_format,
            request.params.temperature,
            request.params.max_tokens,
            system_prompt,
            hints,
            output_format
        );

        let results = vec![ToolResult::text(simulated_response)];
        Ok(results)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🤖 Starting MCP Sampling Server Example");

    let server = McpServer::builder()
        .name("sampling-server")
        .version("1.0.0")
        .title("MCP Sampling Server Example")
        .instructions("This server demonstrates MCP sampling functionality for AI model integration. Sampling allows servers to request AI model generation from clients, enabling intelligent workflows and AI-assisted processing.")
        .tool(BasicSamplingTool)
        .tool(ConversationalSamplingTool)
        .tool(CodeGenerationSamplingTool)
        .tool(CreativeWritingSamplingTool)
        .tool(AdvancedSamplingTool)
        .with_sampling() // Add sampling support
        .bind_address("127.0.0.1:8051".parse()?)
        .build()?;

    info!("🚀 Sampling server running at: http://127.0.0.1:8051/mcp");
    info!("");
    info!("📋 Features demonstrated:");
    info!("  🤖 AI model sampling via sampling/createMessage endpoint");
    info!("  💬 Conversational AI with context and history");
    info!("  💻 Code generation with language-specific prompting");
    info!("  ✨ Creative writing with style and genre controls");
    info!("  ⚙️ Advanced sampling with model preferences and constraints");
    info!("");
    info!("🔧 Available tools:");
    info!("  🎯 basic_sampling - Basic AI model sampling demonstration");
    info!("  💬 conversational_sampling - Context-aware conversation sampling");
    info!("  💻 code_generation_sampling - Specialized code generation sampling");
    info!("  ✨ creative_writing_sampling - Creative content generation sampling");
    info!("  🚀 advanced_sampling_demo - Advanced features and model preferences");
    info!("");
    info!("🧪 Test sampling endpoint:");
    info!("  curl -X POST http://127.0.0.1:8051/mcp \\\\");
    info!("    -H 'Content-Type: application/json' \\\\");
    info!("    -d '{{\\\"method\\\": \\\"sampling/createMessage\\\", \\\"params\\\": {{\\\"messages\\\": [{{\\\"role\\\": \\\"user\\\", \\\"content\\\": {{\\\"type\\\": \\\"text\\\", \\\"text\\\": \\\"Hello AI!\\\"}}}}}}]}}}}'");

    server.run().await?;
    Ok(())
}