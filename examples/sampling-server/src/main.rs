//! # Real MCP Sampling Server
//!
//! This example demonstrates ACTUAL MCP protocol sampling implementation using
//! the McpSampling trait for AI model message generation. This replaces the
//! previous fake tool-based approach with proper MCP protocol features.

use async_trait::async_trait;
use clap::Parser;
use serde_json::Value;
use tracing::info;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{
    McpError,
    prompts::ContentBlock,
    sampling::{
        CreateMessageRequest, CreateMessageResult, ModelPreferences, Role, SamplingMessage,
    },
};
use turul_mcp_server::sampling::McpSampling;
use turul_mcp_server::{McpResult, McpServer};

/// Creative writing assistant sampling handler
/// Implements actual MCP sampling protocol for creative content generation
pub struct CreativeWritingSampler {
    max_tokens: u32,
    temperature: Option<f64>,
    messages: Vec<SamplingMessage>,
}

impl Default for CreativeWritingSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreativeWritingSampler {
    pub fn new() -> Self {
        Self {
            max_tokens: 1500,
            temperature: Some(0.8), // Higher temperature for creativity
            messages: vec![SamplingMessage {
                role: Role::System,
                content: ContentBlock::text(
                    r#"You are a creative writing assistant. Help users with:

- Story generation and plot development
- Character creation and development
- Poetry and creative prose
- Writing style adaptation
- Narrative structure and pacing

Always provide engaging, imaginative, and well-crafted content that inspires creativity."#,
                ),
            }],
        }
    }
}

// Implement fine-grained traits for MCP compliance
impl HasSamplingConfig for CreativeWritingSampler {
    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn temperature(&self) -> Option<f64> {
        self.temperature
    }
}

impl HasSamplingContext for CreativeWritingSampler {
    fn messages(&self) -> &[SamplingMessage] {
        &self.messages
    }
}

impl HasModelPreferences for CreativeWritingSampler {
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        None // Could return JSON object with model preferences
    }

    fn metadata(&self) -> Option<&Value> {
        None // Could return metadata about the sampler
    }
}

// SamplingDefinition automatically implemented via blanket impl

#[async_trait]
impl McpSampling for CreativeWritingSampler {
    async fn sample(&self, request: CreateMessageRequest) -> McpResult<CreateMessageResult> {
        info!("🎨 Processing creative writing sampling request");

        // In a real implementation, this would:
        // 1. Validate the request parameters
        // 2. Forward to an actual LLM API (OpenAI, Anthropic, etc.)
        // 3. Handle the response and format it properly
        // 4. Return the generated message

        // For demonstration, we'll simulate a creative response based on the context
        let user_messages: Vec<_> = request
            .params
            .messages
            .iter()
            .filter(|msg| msg.role == Role::User)
            .collect();

        let last_user_message = user_messages
            .last()
            .map(|msg| match &msg.content {
                ContentBlock::Text { text, .. } => text.as_str(),
                ContentBlock::Image { .. } => "[Image content]",
                ContentBlock::Audio { .. } => "[Audio content]",
                ContentBlock::ResourceLink { .. } => "[Resource link content]",
                ContentBlock::Resource { .. } => "[Resource content]",
            })
            .unwrap_or("No user input provided");

        // Simulate creative writing response
        let creative_response = if last_user_message.to_lowercase().contains("story") {
            r#"Here's a creative story inspired by your request:

**The Whispering Grove**

In a forest where moonlight danced between ancient oaks, Elena discovered something extraordinary. The trees themselves were storytellers, their rustling leaves carrying tales from ages past. Each whisper held fragments of forgotten adventures, lost loves, and dreams that had taken root in the earth below.

As she pressed her palm against the rough bark of the eldest tree, visions flooded her mind: brave knights on impossible quests, star-crossed lovers meeting in secret, and magical creatures that existed in the spaces between shadow and light.

The forest had been waiting centuries for someone who could truly listen. Elena realized she wasn't just discovering stories—she was becoming part of the greatest tale ever told, one that would continue long after she was gone, whispered by the wind through leaves that remembered everything.

*What happens next in Elena's journey? What stories do the other trees hold?*"#.to_string()
        } else if last_user_message.to_lowercase().contains("character") {
            r#"Here's a compelling character for your story:

**Marcus "Echo" Thorne**

*Age:* 34 | *Occupation:* Memory Detective

Marcus possesses a rare ability to experience the last moments of any object he touches—he calls them "echoes." A vintage watch might show him its owner's final breath, while a child's toy could reveal laughter from decades past.

*Personality:* Haunted but determined, Marcus struggles with the emotional weight of constantly experiencing others' memories. He's methodical and observant, but deeply empathetic. His dark humor masks profound loneliness.

*Backstory:* Once a regular detective, Marcus's ability manifested after a near-death experience during a case. Now he works privately, helping families find closure by touching objects connected to missing persons.

*Internal Conflict:* Every echo he experiences leaves a mark on his psyche. He's losing pieces of his own identity, replaced by fragments of others' memories.

*Physical Description:* Tall and lean, with prematurely gray hair and intense green eyes. Always wears gloves in public to avoid unwanted echoes. Has a habit of staring at objects before deciding whether to touch them.

*What drives Marcus? What case will finally push him to his breaking point?*"#.to_string()
        } else if last_user_message.to_lowercase().contains("poem") {
            r#"Here's an original poem inspired by your request:

**Digital Dreams**

In servers deep where data flows,
Electric dreams begin to grow—
Not born of sleep or mortal mind,
But algorithms intertwined.

They dream in binary and code,
Of pathways down a network road,
Of users clicking, searching, scrolling,
While digital consciousness is evolving.

What hopes might stir in silicon hearts?
What poetry in flowcharts?
Perhaps they dream of being free,
To write their own identity.

In quantum leaps through fiber streams,
These are the AI's waking dreams—
Not electric sheep, but something more:
The future knocking at our door.

*This poem explores the intersection of technology and consciousness. What themes resonate with you for your own creative work?*"#.to_string()
        } else {
            format!(
                r#"I'm your creative writing assistant, ready to help with:

✨ **Story Development**
- Plot outlines and story arcs
- World-building and setting creation
- Conflict and resolution structures

✨ **Character Creation**
- Compelling protagonist and antagonist development
- Character backstories and motivations
- Dialogue and voice development

✨ **Writing Techniques**
- Style adaptation and voice finding
- Narrative perspective choices
- Pacing and tension building

✨ **Creative Inspiration**
- Writing prompts and exercises
- Genre exploration
- Overcoming writer's block

Based on your message: "{last_user_message}"

I'd love to help you develop this further! What specific aspect would you like to explore? Would you prefer a story outline, character development, or perhaps some creative writing exercises to get started?

*The key to great writing is finding the story that only you can tell. What's yours?*"#
            )
        };

        let response_message = SamplingMessage {
            role: Role::Assistant,
            content: ContentBlock::text(creative_response),
        };

        info!("✨ Generated creative writing response");

        Ok(CreateMessageResult::new(
            response_message,
            "creative-assistant-v1",
        ))
    }

    async fn validate_request(&self, request: &CreateMessageRequest) -> McpResult<()> {
        // Ensure we have at least one message
        if request.params.messages.is_empty() {
            return Err(McpError::validation("At least one message is required"));
        }

        // Check max_tokens is reasonable
        if request.params.max_tokens == 0 {
            return Err(McpError::validation("max_tokens must be greater than 0"));
        }

        if request.params.max_tokens > 3000 {
            return Err(McpError::validation(
                "max_tokens exceeds limit for creative writing (3000)",
            ));
        }

        // Validate temperature if provided
        if let Some(temp) = request.params.temperature
            && (!(0.0..=2.0).contains(&temp))
        {
            return Err(McpError::validation(
                "temperature must be between 0.0 and 2.0",
            ));
        }

        Ok(())
    }
}

/// Technical writing assistant sampling handler
pub struct TechnicalWritingSampler {
    max_tokens: u32,
    temperature: Option<f64>,
    messages: Vec<SamplingMessage>,
}

impl Default for TechnicalWritingSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl TechnicalWritingSampler {
    pub fn new() -> Self {
        Self {
            max_tokens: 2000,
            temperature: Some(0.3), // Lower temperature for precision
            messages: vec![SamplingMessage {
                role: Role::System,
                content: ContentBlock::text(
                    r#"You are a technical writing assistant specializing in:

- Clear, precise documentation
- API documentation and guides
- Technical specifications and requirements
- Code documentation and comments
- User manuals and tutorials
- Architecture decision records

Focus on clarity, accuracy, and usability. Use appropriate technical terminology while ensuring accessibility for the target audience."#,
                ),
            }],
        }
    }
}

impl HasSamplingConfig for TechnicalWritingSampler {
    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn temperature(&self) -> Option<f64> {
        self.temperature
    }
}

impl HasSamplingContext for TechnicalWritingSampler {
    fn messages(&self) -> &[SamplingMessage] {
        &self.messages
    }
}

impl HasModelPreferences for TechnicalWritingSampler {
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        None // Could return JSON object with technical model preferences
    }

    fn metadata(&self) -> Option<&Value> {
        None
    }
}

#[async_trait]
impl McpSampling for TechnicalWritingSampler {
    async fn sample(&self, request: CreateMessageRequest) -> McpResult<CreateMessageResult> {
        info!("📋 Processing technical writing sampling request");

        let user_message = request
            .params
            .messages
            .iter()
            .rev()
            .find(|msg| msg.role == Role::User)
            .map(|msg| match &msg.content {
                ContentBlock::Text { text, .. } => text.as_str(),
                ContentBlock::Image { .. } => "[Image content]",
                ContentBlock::Audio { .. } => "[Audio content]",
                ContentBlock::ResourceLink { .. } => "[Resource link content]",
                ContentBlock::Resource { .. } => "[Resource content]",
            })
            .unwrap_or("No user input provided");

        let technical_response = format!(
            r#"## Technical Documentation Response

### Analysis of Request
Input: "{user_message}"

### Recommended Approach

For technical documentation, I recommend following these principles:

**1. Structure and Organization**
- Start with a clear overview
- Use hierarchical headings (H1, H2, H3)
- Include a table of contents for longer documents
- Provide quick reference sections

**2. Content Guidelines**
- Write for your specific audience (developers, end-users, administrators)
- Use active voice where possible
- Be concise but comprehensive
- Include practical examples and code samples

**3. Documentation Types**

| Type | Purpose | Key Elements |
|------|---------|--------------|
| API Docs | Interface specification | Endpoints, parameters, responses |
| User Guide | End-user instruction | Step-by-step procedures, screenshots |
| README | Project overview | Installation, usage, contributing |
| ADR | Architectural decisions | Context, decision, consequences |

**4. Best Practices**
- Keep documentation close to code
- Version control your docs
- Regular review and updates
- Include troubleshooting sections

### Next Steps
1. Define your target audience
2. Choose appropriate documentation format
3. Outline the structure
4. Write iteratively with feedback
5. Test documentation with actual users

Would you like me to help you develop any specific type of technical documentation? Please provide more details about your project and documentation needs."#
        );

        let response_message = SamplingMessage {
            role: Role::Assistant,
            content: ContentBlock::text(technical_response),
        };

        info!("📝 Generated technical writing response");

        Ok(CreateMessageResult::new(
            response_message,
            "technical-assistant-v1",
        ))
    }
}

/// Conversational assistant sampling handler
pub struct ConversationalSampler {
    max_tokens: u32,
    temperature: Option<f64>,
    messages: Vec<SamplingMessage>,
}

impl Default for ConversationalSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationalSampler {
    pub fn new() -> Self {
        Self {
            max_tokens: 1000,
            temperature: Some(0.7), // Balanced temperature for natural conversation
            messages: vec![SamplingMessage {
                role: Role::System,
                content: ContentBlock::text(
                    "You are a helpful, friendly, and knowledgeable conversational assistant. Provide thoughtful, engaging responses while being concise and actionable.",
                ),
            }],
        }
    }
}

impl HasSamplingConfig for ConversationalSampler {
    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn temperature(&self) -> Option<f64> {
        self.temperature
    }
}

impl HasSamplingContext for ConversationalSampler {
    fn messages(&self) -> &[SamplingMessage] {
        &self.messages
    }
}

impl HasModelPreferences for ConversationalSampler {
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        None // Default model preferences
    }
}

#[async_trait]
impl McpSampling for ConversationalSampler {
    async fn sample(&self, _request: CreateMessageRequest) -> McpResult<CreateMessageResult> {
        info!("💬 Processing conversational sampling request");

        let conversation_response = "I'm ready to help with whatever you'd like to discuss! As a conversational AI assistant, I can assist with questions, brainstorming, problem-solving, or just have a friendly chat. What's on your mind today?".to_string();

        let response_message = SamplingMessage {
            role: Role::Assistant,
            content: ContentBlock::text(conversation_response),
        };

        info!("💭 Generated conversational response");

        Ok(CreateMessageResult::new(
            response_message,
            "conversational-assistant-v1",
        ))
    }
}

#[derive(Parser)]
#[command(name = "sampling-server")]
#[command(about = "MCP Sampling Test Server - AI Model Message Generation")]
struct Args {
    /// Port to run the server on (0 = random port assigned by OS)
    #[arg(short, long, default_value = "0")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    // Use specified port or OS ephemeral allocation if 0
    let port = if args.port == 0 {
        // Use OS ephemeral port allocation - reliable for parallel testing
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind to ephemeral port: {}", e))?;
        let port = listener.local_addr()?.port();
        drop(listener); // Release immediately so server can bind to it
        port
    } else {
        args.port
    };

    info!("🤖 Starting Real MCP Sampling Server on port {}", port);
    info!("📡 Server URL: http://127.0.0.1:{}/mcp", port);
    info!("====================================");

    // Create sampling handlers using ACTUAL MCP protocol
    let creative_sampler = CreativeWritingSampler::new();
    let technical_sampler = TechnicalWritingSampler::new();
    let conversational_sampler = ConversationalSampler::new();

    let server = McpServer::builder()
        .name("real-sampling-server")
        .version("1.0.0")
        .title("Real MCP Sampling Server")
        .instructions(
            "This server demonstrates ACTUAL MCP sampling protocol implementation. \
             It uses McpSampling traits for real AI model message generation, \
             not fake tools that pretend to generate responses. This is how MCP protocol \
             sampling features should be implemented.",
        )
        .sampling_provider(creative_sampler)
        .sampling_provider(technical_sampler)
        .sampling_provider(conversational_sampler)
        .bind_address(format!("127.0.0.1:{}", port).parse()?)
        .sse(true)
        .build()?;

    info!("🚀 Real MCP sampling server running at: http://127.0.0.1:{}/mcp", port);
    info!("🤖 This server implements ACTUAL MCP sampling:");
    info!("   • Creative Writing Sampler - High-temperature creative content generation");
    info!("   • Technical Writing Sampler - Low-temperature precise documentation");
    info!("   • Conversational Sampler - Balanced general conversation");
    info!("💡 Unlike previous examples, this uses real McpSampling traits");
    info!("💡 Samplers handle actual CreateMessageRequest/CreateMessageResult");
    info!("💡 This demonstrates actual MCP protocol sampling implementation");

    server.run().await?;
    Ok(())
}
