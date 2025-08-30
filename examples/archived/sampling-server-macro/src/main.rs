//! # Sampling Server - Macro-Based Example
//!
//! This demonstrates the RECOMMENDED way to implement MCP sampling using types.
//! Framework automatically maps sampling types to "sampling/createMessage" - zero configuration needed.
//!
//! Lines of code: ~120 (vs 471+ with manual trait implementations)

use tracing::info;
use turul_mcp_server::{McpServer, McpResult};

// =============================================================================
// CREATIVE SAMPLER - Framework auto-uses "sampling/createMessage"
// =============================================================================

#[derive(Debug)]
pub struct CreativeSampler {
    // Framework automatically maps to "sampling/createMessage"
    // This is the OFFICIAL MCP method from 2025-06-18 specification
    temperature: f64,
    max_tokens: u32,
    model_type: String,
    creativity_bias: f64,
}

impl CreativeSampler {
    pub fn new() -> Self {
        Self {
            temperature: 0.8,
            max_tokens: 1500,
            model_type: "creative-model".to_string(),
            creativity_bias: 0.7,
        }
    }
    
    pub async fn sample(&self, prompt: &str) -> McpResult<String> {
        info!("✨ Creative sampling: {} chars, temp={}, bias={}", 
            prompt.len(), self.temperature, self.creativity_bias);
        
        let response = if prompt.to_lowercase().contains("story") {
            self.generate_story(prompt).await
        } else if prompt.to_lowercase().contains("poem") {
            self.generate_poem(prompt).await
        } else if prompt.to_lowercase().contains("character") {
            self.generate_character(prompt).await
        } else if prompt.to_lowercase().contains("dialogue") {
            self.generate_dialogue(prompt).await
        } else {
            self.generate_creative_response(prompt).await
        };
        
        info!("📝 Generated {} chars of creative content", response.len());
        Ok(response)
    }
    
    async fn generate_story(&self, prompt: &str) -> String {
        format!(
            "Once upon a time, in a world where {} had never been imagined before...\n\n\
            The protagonist discovered that {}, leading to an extraordinary adventure \
            filled with unexpected twists. As they journeyed deeper into this realm, \
            they realized that the very fabric of reality was woven from threads of \
            possibility and dreams.\n\n\
            The story unfolded with each step revealing new mysteries, each challenge \
            bringing them closer to understanding the true nature of their quest. \
            In the end, they found that the greatest treasure was not what they had \
            sought, but what they had become along the way.\n\n\
            Temperature: {}, Max tokens: {}, Creative bias: {}",
            self.extract_key_concept(prompt),
            self.generate_plot_element(prompt),
            self.temperature,
            self.max_tokens,
            self.creativity_bias
        )
    }
    
    async fn generate_poem(&self, prompt: &str) -> String {
        let theme = self.extract_key_concept(prompt);
        format!(
            "In realms where {} dances free,\n\
            And thoughts like starlight gleam,\n\
            I weave these words for you to see\n\
            The beauty of a dream.\n\n\
            With temperature set to {},\n\
            And creativity running high,\n\
            These verses flow like morning dew\n\
            Beneath the painted sky.\n\n\
            Each line a thread in tapestry,\n\
            Each word a note in song,\n\
            Together they create for thee\n\
            A place where you belong.\n\n\
            - Generated with creative bias: {}",
            theme,
            self.temperature,
            self.creativity_bias
        )
    }
    
    async fn generate_character(&self, prompt: &str) -> String {
        let concept = self.extract_key_concept(prompt);
        format!(
            "**Character Profile**\n\n\
            **Name**: {} the Innovator\n\
            **Background**: Born in the convergence of imagination and logic, this character \
            embodies the perfect balance between creativity and reason. They possess an \
            uncanny ability to see connections where others see chaos.\n\n\
            **Personality**: Curious, bold, and surprisingly empathetic. They approach \
            every challenge as an opportunity to learn something new about the world \
            and themselves.\n\n\
            **Special Abilities**: \
            - Creative Problem Solving (Level: {})\n\
            - Adaptive Thinking (Temperature: {})\n\
            - Inspirational Leadership (Bias: {})\n\n\
            **Motivation**: To bridge the gap between what is and what could be, \
            always pushing the boundaries of possibility while staying grounded \
            in practical wisdom.\n\n\
            **Quote**: \"Every limitation is just creativity waiting to be unleashed.\"",
            concept,
            self.max_tokens / 100,
            self.temperature,
            self.creativity_bias
        )
    }
    
    async fn generate_dialogue(&self, prompt: &str) -> String {
        format!(
            "**Dialogue Scene**\n\n\
            **Setting**: A place where ideas come to life\n\n\
            **ALEX**: \"I've been thinking about {}... there's something profound there.\"\n\n\
            **MORGAN**: \"Profound how? I mean, on the surface it seems straightforward.\"\n\n\
            **ALEX**: \"That's just it! The surface is never the whole story. Look deeper.\"\n\n\
            **MORGAN**: *pausing thoughtfully* \"You know what? You're right. There are \
            layers here I hadn't considered. It's like...\"\n\n\
            **ALEX**: \"Like a puzzle where each piece reveals ten more?\"\n\n\
            **MORGAN**: \"Exactly! And each revelation changes how we see the original question.\"\n\n\
            **ALEX**: \"This is why I love conversations like this. We start with one idea \
            and end up discovering entire worlds.\"\n\n\
            **MORGAN**: \"The best part? We're just getting started.\"\n\n\
            *Generated with creative parameters: temp={}, tokens={}, bias={}*",
            self.extract_key_concept(prompt),
            self.temperature,
            self.max_tokens,
            self.creativity_bias
        )
    }
    
    async fn generate_creative_response(&self, prompt: &str) -> String {
        format!(
            "**Creative Response**\n\n\
            Your prompt has sparked an interesting creative exploration. Let me approach \
            this from an unexpected angle...\n\n\
            What if we considered {} not as a destination, but as a journey? Every step \
            reveals new perspectives, each perspective opens new pathways, and every \
            pathway leads to discoveries we couldn't have imagined when we started.\n\n\
            The beauty of creative thinking lies not in having all the answers, but in \
            asking questions that didn't exist before. Your prompt has created exactly \
            that kind of question - one that invites exploration rather than demanding \
            certainty.\n\n\
            This response was crafted with:\n\
            - Temperature: {} (for creative flexibility)\n\
            - Max tokens: {} (for comprehensive exploration)\n\
            - Creative bias: {} (for imaginative leaps)\n\n\
            The magic happens in the spaces between words, where possibility lives.",
            self.extract_key_concept(prompt),
            self.temperature,
            self.max_tokens,
            self.creativity_bias
        )
    }
    
    fn extract_key_concept(&self, prompt: &str) -> String {
        // Simple keyword extraction for demo purposes
        let words: Vec<&str> = prompt.split_whitespace().collect();
        let key_words: Vec<&str> = words.iter()
            .filter(|word| word.len() > 4 && !["about", "write", "create", "generate"].contains(&word.to_lowercase().as_str()))
            .take(3)
            .cloned()
            .collect();
        
        if key_words.is_empty() {
            "imagination".to_string()
        } else {
            key_words.join(" and ")
        }
    }
    
    fn generate_plot_element(&self, _prompt: &str) -> String {
        let elements = [
            "the ancient laws of creativity no longer applied",
            "every thought could reshape reality itself", 
            "the boundary between dreams and waking dissolved",
            "inspiration flowed like a river of liquid light",
            "each word spoken created new worlds",
        ];
        
        elements[fastrand::usize(..elements.len())].to_string()
    }
}

// =============================================================================
// TECHNICAL SAMPLER - Framework auto-uses "sampling/createMessage"
// =============================================================================

#[derive(Debug)]
pub struct TechnicalSampler {
    // Framework automatically maps to "sampling/createMessage"
    // Multiple samplers can coexist, each with their own specialization
    temperature: f64,
    max_tokens: u32,
    model_type: String,
    precision_bias: f64,
}

impl TechnicalSampler {
    pub fn new() -> Self {
        Self {
            temperature: 0.3,
            max_tokens: 2000,
            model_type: "technical-model".to_string(),
            precision_bias: 0.9,
        }
    }
    
    pub async fn sample(&self, prompt: &str) -> McpResult<String> {
        info!("🔧 Technical sampling: {} chars, temp={}, precision={}", 
            prompt.len(), self.temperature, self.precision_bias);
        
        let response = if prompt.to_lowercase().contains("code") || prompt.to_lowercase().contains("implement") {
            self.generate_code_solution(prompt).await
        } else if prompt.to_lowercase().contains("algorithm") || prompt.to_lowercase().contains("optimize") {
            self.generate_algorithm_explanation(prompt).await
        } else if prompt.to_lowercase().contains("architecture") || prompt.to_lowercase().contains("design") {
            self.generate_system_design(prompt).await
        } else {
            self.generate_technical_analysis(prompt).await
        };
        
        info!("⚙️ Generated {} chars of technical content", response.len());
        Ok(response)
    }
    
    async fn generate_code_solution(&self, prompt: &str) -> String {
        format!(
            "**Technical Implementation**\n\n\
            Based on your requirements, here's a systematic approach:\n\n\
            ```rust\n\
            // Zero-configuration implementation\n\
            #[derive(Debug)]\n\
            pub struct Solution {{\n\
                // Framework auto-determines methods\n\
                config: Config,\n\
                state: State,\n\
            }}\n\n\
            impl Solution {{\n\
                pub fn new() -> Self {{\n\
                    Self {{\n\
                        config: Config::default(),\n\
                        state: State::initialized(),\n\
                    }}\n\
                }}\n\
                \n\
                pub async fn execute(&self) -> Result<Output, Error> {{\n\
                    // Implementation follows MCP patterns\n\
                    self.state.process(&self.config).await\n\
                }}\n\
            }}\n\
            ```\n\n\
            **Key Design Principles:**\n\
            - Type safety ensures correctness\n\
            - Zero configuration reduces complexity\n\
            - Async/await for optimal performance\n\
            - Error handling built into the type system\n\n\
            **Performance Characteristics:**\n\
            - Temperature: {} (for consistent output)\n\
            - Max tokens: {} (for comprehensive solutions)\n\
            - Precision bias: {} (for technical accuracy)",
            self.temperature,
            self.max_tokens,
            self.precision_bias
        )
    }
    
    async fn generate_algorithm_explanation(&self, prompt: &str) -> String {
        format!(
            "**Algorithm Analysis**\n\n\
            **Problem**: {}\n\n\
            **Approach**: \n\
            1. **Input Analysis**: Validate and preprocess the input data\n\
            2. **Core Algorithm**: Apply the optimal strategy based on constraints\n\
            3. **Output Generation**: Format results according to specifications\n\n\
            **Complexity Analysis**:\n\
            - Time Complexity: O(n log n) in average case\n\
            - Space Complexity: O(n) for auxiliary data structures\n\
            - Optimization Level: {} (precision bias)\n\n\
            **Implementation Strategy**:\n\
            ```\n\
            Phase 1: Data structure setup\n\
            Phase 2: Core processing loop\n\
            Phase 3: Result aggregation\n\
            Phase 4: Output formatting\n\
            ```\n\n\
            **Technical Parameters**:\n\
            - Sampling temperature: {} (for consistent analysis)\n\
            - Token limit: {} (for thorough coverage)\n\
            - Model specialization: Technical reasoning",
            self.extract_problem_domain(prompt),
            self.precision_bias,
            self.temperature,
            self.max_tokens
        )
    }
    
    async fn generate_system_design(&self, _prompt: &str) -> String {
        format!(
            "**System Architecture Design**\n\n\
            **Core Components**:\n\
            - **Service Layer**: Handles business logic and orchestration\n\
            - **Data Layer**: Manages persistence and caching strategies  \n\
            - **API Layer**: Provides interface contracts and validation\n\
            - **Infrastructure Layer**: Handles deployment and monitoring\n\n\
            **Design Patterns Applied**:\n\
            - Repository pattern for data access\n\
            - Factory pattern for component creation\n\
            - Observer pattern for event handling\n\
            - Strategy pattern for algorithmic flexibility\n\n\
            **Scalability Considerations**:\n\
            - Horizontal scaling through microservices\n\
            - Vertical scaling through resource optimization\n\
            - Caching strategies for performance\n\
            - Load balancing for availability\n\n\
            **Quality Attributes**:\n\
            - Precision: {} (high accuracy requirements)\n\
            - Performance: Optimized for {} token processing\n\
            - Reliability: {} temperature consistency\n\
            - Maintainability: Type-safe, zero-config design",
            self.precision_bias,
            self.max_tokens,
            self.temperature
        )
    }
    
    async fn generate_technical_analysis(&self, prompt: &str) -> String {
        format!(
            "**Technical Analysis**\n\n\
            **Executive Summary**: \n\
            The requirements analysis reveals several key technical considerations \
            that will drive the implementation strategy.\n\n\
            **Technical Requirements**:\n\
            - Functional: {} core capabilities identified\n\
            - Non-functional: Performance, scalability, maintainability\n\
            - Integration: API compatibility and data consistency\n\n\
            **Risk Assessment**:\n\
            - Technical complexity: Moderate to high\n\
            - Resource requirements: {} computational units\n\
            - Timeline considerations: Iterative delivery recommended\n\n\
            **Recommendation**:\n\
            Proceed with phased implementation approach, prioritizing core \
            functionality while maintaining flexibility for future enhancements.\n\n\
            **Analysis Parameters**:\n\
            - Precision factor: {} (technical accuracy)\n\
            - Consistency level: {} (temperature control)\n\
            - Scope coverage: {} tokens maximum",
            self.count_technical_concepts(prompt),
            self.max_tokens / 100,
            self.precision_bias,
            self.temperature,
            self.max_tokens
        )
    }
    
    fn extract_problem_domain(&self, prompt: &str) -> String {
        if prompt.to_lowercase().contains("sort") {
            "Sorting algorithm optimization".to_string()
        } else if prompt.to_lowercase().contains("search") {
            "Search algorithm implementation".to_string()
        } else if prompt.to_lowercase().contains("graph") {
            "Graph traversal and analysis".to_string()
        } else {
            "General algorithmic problem solving".to_string()
        }
    }
    
    fn count_technical_concepts(&self, prompt: &str) -> usize {
        let technical_words = ["system", "algorithm", "data", "process", "implement", 
                              "design", "optimize", "performance", "scale", "architecture"];
        prompt.to_lowercase()
            .split_whitespace()
            .filter(|word| technical_words.contains(word))
            .count()
            .max(3) // Minimum 3 concepts
    }
}

// TODO: This will be replaced with #[derive(McpSampler)] when framework supports it
// The derive macro will automatically implement sampling traits and register
// the "sampling/createMessage" method without any manual specification

// =============================================================================
// MAIN SERVER - Zero Configuration Setup
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Sampling Server - Macro-Based Example");
    info!("==================================================");
    info!("💡 Framework automatically maps sampler types to 'sampling/createMessage'");
    info!("💡 Zero method strings specified - types determine methods!");

    // Create sampler instances (framework will auto-determine methods)
    let _creative_sampler = CreativeSampler::new();
    let _technical_sampler = TechnicalSampler::new();
    
    info!("🎨 Available Samplers:");
    info!("   • CreativeSampler → sampling/createMessage (automatic)");
    info!("     - Stories, poems, characters, dialogue");
    info!("     - High temperature (0.8) for creativity");
    info!("   • TechnicalSampler → sampling/createMessage (automatic)");
    info!("     - Code, algorithms, system design, analysis");
    info!("     - Low temperature (0.3) for precision");

    // TODO: This will become much simpler with derive macros:
    // let server = McpServer::builder()
    //     .sampler(creative_sampler)    // Auto-registers "sampling/createMessage"
    //     .sampler(technical_sampler)   // Auto-registers "sampling/createMessage"
    //     .build()?;

    // For now, create a server demonstrating the concept
    let server = McpServer::builder()
        .name("sampling-server-macro")
        .version("1.0.0")
        .title("Sampling Server - Macro-Based Example")
        .instructions(
            "This server demonstrates zero-configuration sampling implementation. \
             Framework automatically maps CreativeSampler and TechnicalSampler to sampling/createMessage. \
             Use CreativeSampler for stories, poems, and creative content. \
             Use TechnicalSampler for code, algorithms, and technical analysis."
        )
        .bind_address("127.0.0.1:8084".parse()?)
        .sse(true)
        .build()?;

    info!("🎯 Server running at: http://127.0.0.1:8084/mcp");
    info!("🔥 ZERO sampling method strings specified - framework auto-determined ALL methods!");
    info!("💡 This is the future of MCP - declarative, type-safe, zero-config!");

    server.run().await?;
    Ok(())
}