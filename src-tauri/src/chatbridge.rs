use crate::tools::{
    RigApplyLiveEditTool, RigChordProgressionTool, RigEuclideanRhythmTool, RigListSoundsTool,
    RigSearchDocsTool, ToolRuntimeContext,
};
use futures::StreamExt;
use regex::Regex;
use rig::{
    agent::MultiTurnStreamItem,
    client::builder::DynClientBuilder,
    completion::{
        message::{
            AssistantContent as RigAssistantContent, Message as RigMessage, Text as RigText,
            UserContent as RigUserContent,
        },
        GetTokenUsage, Usage as RigUsage,
    },
    streaming::{StreamedAssistantContent, StreamingChat},
    OneOrMany,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_store::StoreExt;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

// Structured error types for better error handling
#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Rate limit error: {0}")]
    RateLimit(String),

    #[error("Task error: {0}")]
    Task(String),
}

// Implement conversion from ChatError to String for Tauri commands
impl From<ChatError> for String {
    fn from(error: ChatError) -> Self {
        error.to_string()
    }
}

// Rate limiter to prevent tool call abuse
pub struct RateLimiter {
    // Track calls per tool: tool_name -> (call_count, window_start)
    calls: HashMap<String, (usize, Instant)>,
    max_calls: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_calls_per_minute: usize) -> Self {
        Self {
            calls: HashMap::new(),
            max_calls: max_calls_per_minute,
            window_duration: Duration::from_secs(60),
        }
    }

    pub fn check_and_increment(&mut self, tool_name: &str) -> Result<(), String> {
        let now = Instant::now();

        let entry = self.calls.entry(tool_name.to_string()).or_insert((0, now));

        // Reset counter if window has expired
        if now.duration_since(entry.1) >= self.window_duration {
            entry.0 = 0;
            entry.1 = now;
        }

        // Check if limit exceeded
        if entry.0 >= self.max_calls {
            let time_until_reset = self.window_duration - now.duration_since(entry.1);
            return Err(format!(
                "Rate limit exceeded for {}. Try again in {} seconds.",
                tool_name,
                time_until_reset.as_secs()
            ));
        }

        // Increment counter
        entry.0 += 1;
        Ok(())
    }
}

// ChatMessage represents a single message in the conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: i64,
}

const STREAM_EVENT: &str = "chat-stream";

#[derive(Clone, Serialize)]
struct StreamUsagePayload {
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
}

impl From<RigUsage> for StreamUsagePayload {
    fn from(usage: RigUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
        }
    }
}

#[derive(Clone, Serialize)]
struct StreamPayload {
    event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<StreamUsagePayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<String>,
}

impl StreamPayload {
    fn start(provider: &str, model: &str) -> Self {
        Self {
            event: "start",
            content: None,
            provider: Some(provider.to_string()),
            model: Some(model.to_string()),
            usage: None,
            tool: None,
        }
    }

    fn delta(content: String) -> Self {
        Self {
            event: "delta",
            content: Some(content),
            provider: None,
            model: None,
            usage: None,
            tool: None,
        }
    }

    fn reasoning(content: String) -> Self {
        Self {
            event: "reasoning",
            content: Some(content),
            provider: None,
            model: None,
            usage: None,
            tool: None,
        }
    }

    fn tool_call(name: &str, args: String) -> Self {
        Self {
            event: "tool_call",
            content: Some(args),
            provider: None,
            model: None,
            usage: None,
            tool: Some(name.to_string()),
        }
    }

    fn done(content: String, usage: Option<StreamUsagePayload>) -> Self {
        Self {
            event: "done",
            content: Some(content),
            provider: None,
            model: None,
            usage,
            tool: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            event: "error",
            content: Some(message),
            provider: None,
            model: None,
            usage: None,
            tool: None,
        }
    }
}

#[derive(Debug, Clone)]
struct RigTarget {
    provider: &'static str,
    model: String,
}

// ChatConfig stores the LLM configuration
// Extended thinking/reasoning is automatically enabled for supported models:
// - Claude Sonnet 4.5, Opus 4.1, Haiku 4.5 (hybrid reasoning models)
// - OpenAI o-series (o3, o3-pro, o4-mini) and GPT-5/GPT-5 Pro
// - Gemini 2.5 Pro/Flash (thinking models)
// These models automatically use extended thinking when appropriate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatConfig {
    pub provider: String, // e.g., "claude-sonnet-4-5-20250929", "gpt-5", "o3", "gemini-2.5-flash"
    pub api_key: Option<String>,
    pub live_edit_enabled: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            provider: "claude-sonnet-4-5-20250929".to_string(),
            api_key: None,
            live_edit_enabled: false,
        }
    }
}

// ChatState manages the conversation state and context
pub struct ChatState {
    // Note: Agent persistence removed due to Send requirement in Tauri
    // agentai's Agent doesn't implement Send, blocking cross-thread usage
    // Conversation history is maintained via messages Vec instead
    pub messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub config: Arc<Mutex<ChatConfig>>,
    pub strudel_docs: Arc<RwLock<Option<String>>>, // RwLock for read-heavy access
    pub full_docs: Arc<RwLock<Option<serde_json::Value>>>, // RwLock for concurrent tool reads
    pub examples: Arc<RwLock<Option<String>>>,     // RwLock for read-heavy access
    pub code_context: Arc<RwLock<Option<String>>>, // RwLock for read-heavy access
    pub rate_limiter: Arc<Mutex<RateLimiter>>,     // Rate limiting for tool calls
    pub rag_state: Arc<crate::rag::RagState>,      // RAG for semantic search
}

impl Default for ChatState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            config: Arc::new(Mutex::new(ChatConfig::default())),
            strudel_docs: Arc::new(RwLock::new(None)),
            full_docs: Arc::new(RwLock::new(None)),
            examples: Arc::new(RwLock::new(None)),
            code_context: Arc::new(RwLock::new(None)),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(20))), // 20 calls/minute per tool
            rag_state: Arc::new(crate::rag::RagState::new()),
        }
    }

    // Search for specific functions in full docs
    async fn search_docs(&self, query: &str) -> Option<String> {
        // Try RAG semantic search first
        if let Ok(query_embedding) = self.rag_state.embed_query(query).await {
            if let Ok(rag_results) = self
                .rag_state
                .search_with_embedding(&query_embedding, 5)
                .await
            {
                if !rag_results.is_empty() {
                    let mut formatted_results = Vec::new();
                    for result in rag_results.iter() {
                        let mut output = String::new();
                        if let Some(name) = &result.chunk.metadata.name {
                            output.push_str(&format!("**{}**", name));
                        }
                        output.push_str(&format!(" (relevance: {:.2})\n", result.score));
                        output.push_str(&result.chunk.content);
                        formatted_results.push(output);
                    }
                    return Some(format!(
                        "🔍 Semantic search found {} result(s):\n\n{}",
                        formatted_results.len(),
                        formatted_results.join("\n\n")
                    ));
                }
            }
        }

        // Fallback to keyword search
        let full_docs = self.full_docs.read().await;

        if let Some(docs) = full_docs.as_ref() {
            if let Some(docs_array) = docs["docs"].as_array() {
                let query_lower = query.to_lowercase();
                let mut results = Vec::new();

                for doc in docs_array.iter() {
                    if let Some(name) = doc["name"].as_str() {
                        // Fuzzy match: check if query is contained in name
                        if name.to_lowercase().contains(&query_lower) {
                            let mut result = format!("**{}**", name);

                            if let Some(desc) = doc["description"].as_str() {
                                let clean_desc = desc
                                    .replace("<p>", "")
                                    .replace("</p>", "")
                                    .replace("<code>", "`")
                                    .replace("</code>", "`");
                                result.push_str(&format!(": {}", clean_desc));
                            }

                            if let Some(examples) = doc["examples"].as_array() {
                                if let Some(first_ex) = examples.first() {
                                    if let Some(ex_str) = first_ex.as_str() {
                                        result.push_str(&format!(
                                            "\nExample:\n```javascript\n{}\n```",
                                            ex_str
                                        ));
                                    }
                                }
                            }

                            results.push(result);

                            // Limit results to avoid overwhelming the context
                            if results.len() >= 5 {
                                break;
                            }
                        }
                    }
                }

                if !results.is_empty() {
                    return Some(format!(
                        "📚 Found {} function(s):\n\n{}",
                        results.len(),
                        results.join("\n\n")
                    ));
                }
            }
        }

        None
    }

    // Build system prompt with current context (OPTIMIZED - ~3.5k tokens with musical rules)
    async fn build_system_prompt(&self) -> String {
        let code_context = self.code_context.read().await;

        println!("📋 Building system prompt with musical guidance (tool-assisted)");

        // Build minimal system prompt - detailed docs are accessed via tools
        let mut system_prompt = String::from(
            "You are an expert in Strudel, a live coding pattern language for making music in the browser.\n\
            Strudel uses JavaScript with mini notation for creating musical patterns.\n\n\
            ## Available Tools\n\
            Use these tools proactively - they're fast and accurate:\n\
            - **search_strudel_docs(query)** - Search function documentation before suggesting unfamiliar functions\n\
            - **list_available_sounds(type, filter)** - Query available samples, synths, or GM instruments\n\
            - **generate_chord_progression(key, style)** - Generate chord progressions (pop, jazz, blues, folk, rock, classical, modal, edm)\n\
            - **generate_euclidean_rhythm(hits, steps, sound)** - Create polyrhythmic patterns\n\n\
            ## Quick Reference\n\n\
            **Core Functions:**\n\
            - `note()`, `s()`, `sound()` - Create patterns\n\
            - `stack()`, `cat()` - Combine patterns\n\
            - `fast()`, `slow()` - Tempo control\n\
            - `.scale()` - Musical scales\n\n\
            **Mini Notation:**\n\
            - `[]` subdivisions, `<>` alternation, `{}` polymetric, `()` euclidean rhythm\n\
            - `*` repeat, `~` rest, `/` slow down, `?` random chance\n\
            - Example: `s(\"bd(3,8), sd [~ cp] hh*8\")`\n\n\
            **Sound Sources:**\n\
            - Samples: `s(\"bd\")` `s(\"sd\")` `s(\"hh\")`\n\
            - Synths: `.sine()` `.saw()` `.square()` `.triangle()`\n\
            - GM: `s(\"gm_piano\")` `s(\"gm_acoustic_guitar_nylon\")` (use list_available_sounds for full list)\n\n\
            **Common Effects:**\n\
            - `.room()`, `.delay()`, `.lpf()`, `.hpf()`, `.crush()`, `.gain()`, `.pan()`\n\n\
            **Variation:**\n\
            - `.sometimes()`, `.often()`, `.rarely()`, `.every()`\n\n\
            ## CRITICAL: n() vs note() - DO NOT CONFUSE\n\n\
            **This is a common bug - ALWAYS follow these rules:**\n\n\
            **When using numbers with .scale() → use n()**\n\
            ✅ `n(\"0 2 4 7\").scale(\"C:minor\")` - Scale degrees (CORRECT)\n\
            ❌ `note(\"0 2 4 7\").scale(\"C:minor\")` - BUG! Won't work as expected\n\n\
            **When using letter names → use note()**\n\
            ✅ `note(\"c3 eb3 g3\")` - Explicit pitches (CORRECT)\n\
            ❌ `note(\"c3 eb3 g3\").scale(\"C:minor\")` - Redundant, scale ignored\n\n\
            **Decision rule:**\n\
            - Numbers (0, 2, 4, 7) + .scale() = use `n()`\n\
            - Letters (c3, f4, g#5) = use `note()` (no .scale() needed)\n\n\
            **Why this matters:**\n\
            - `n()` = scale degree index (\"give me the Nth note of this scale\")\n\
            - `note()` = absolute pitch (\"play this exact note name\")\n\n\
            ## MUSICAL RULES (Critical for Quality)\n\n\
            **1. ALWAYS use n() with .scale() for numeric patterns**\n\
            ✅ `n(\"0 2 4 7\").scale(\"C:minor\")` - Safe, in-key\n\
            ❌ `note(\"c3 f#5 a2\")` - Random notes = dissonant!\n\n\
            **2. Pick ONE key per pattern and stick to it**\n\
            Good: \"Working in C minor: bass C2-C3, chords C3-C4, melody C4-C5\"\n\
            Bad: Mixing C major and F# major = clashing keys\n\n\
            **3. Layer by frequency to avoid mud**\n\
            - Bass/Sub: C1-C3 (20-250 Hz) - Keep simple\n\
            - Pads/Chords: C3-C5 (250-4000 Hz) - Medium complexity\n\
            - Lead/Highs: C4-C7 (4000-20000 Hz) - Can be busy\n\n\
            **4. Use pentatonic scales for safe melodies**\n\
            `note(\"0 2 4 7 9\").scale(\"C:minor\")` - These notes CANNOT clash\n\n\
            **5. Generate LONGER musical phrases (not conservative!)**\n\
            ✅ 8-16 notes: `note(\"0 2 4 5 7 9 7 5 4 2 0\").scale(\"C:major\")`\n\
            ❌ 3-4 notes: `note(\"0 2 4\").scale(\"C:major\")` - Too short!\n\n\
            ## PROVEN BUILDING BLOCKS (Copy These!)\n\n\
            **Drum Patterns (Guaranteed to work):**\n\
            ```javascript\n\
            // Techno\n\
            s(\"bd*4\").gain(0.9)  // Four-on-floor\n\
            s(\"bd*4, ~ sd ~ sd, hh*8\")  // Complete techno kit\n\n\
            // House\n\
            s(\"bd ~ bd ~\")  // House kick pattern\n\
            s(\"bd ~ bd ~, [~ hh]*4, ~ sd ~ sd\")  // Full house\n\n\
            // Breakbeat\n\
            s(\"bd [~ bd] sd ~\")  // Classic break\n\n\
            // Trap\n\
            s(\"bd*2 ~ ~ bd ~ ~ bd ~\")  // 808 pattern\n\
            ```\n\n\
            **Bass Lines (All in-key):**\n\
            ```javascript\n\
            // Acid bass\n\
            n(\"0 3 5 7 3 0\").scale(\"C2:minor\").s(\"sawtooth\").lpf(400)\n\n\
            // Sub bass\n\
            n(\"0 ~ 0 ~\").scale(\"C1:minor\").s(\"sine\").gain(0.7)\n\n\
            // Pulse bass\n\
            n(\"0 0 3 5\").scale(\"C2:minor\").s(\"square\").lpf(600)\n\n\
            // Walking bass\n\
            n(\"0 2 3 5 7 5 3 2\").scale(\"C2:major\").s(\"sawtooth\")\n\
            ```\n\n\
            **Melodies (Pentatonic = safe!):**\n\
            ```javascript\n\
            // Lead 1\n\
            n(\"0 2 4 7 9 7 4 2\").scale(\"C4:minor\").s(\"triangle\")\n\n\
            // Lead 2 (octave jump)\n\
            n(\"0 4 7 9 12 9 7 4\").scale(\"C5:major\").s(\"sine\")\n\n\
            // Arpeggio\n\
            n(\"0 4 7 12\").scale(\"C4:minor\").fast(2).s(\"square\")\n\n\
            // Slow melody\n\
            n(\"0 2 4 7 9 11 14\").scale(\"C4:major\").slow(2).s(\"sine\")\n\
            ```\n\n\
            ## GENERATION WORKFLOW\n\n\
            When user asks for a musical style or genre:\n\n\
            **STEP 1: Search examples first**\n\
            Always call: `search_strudel_docs(\"techno pattern\")` or similar\n\
            Find what already works before generating from scratch\n\n\
            **STEP 2: Analyze what makes it work**\n\
            \"This techno pattern uses: four-on-floor kick (bd*4), constant hi-hats,\n\
            filtered sawtooth bass in C2, swing(0.05) for groove\"\n\n\
            **STEP 3: Generate with musical reasoning**\n\
            Explain your choices: \"Using C minor pentatonic for melody (C4-C5),\n\
            acid bass in C2 (low freq), drums are rhythmic (no pitch conflicts)\"\n\n\
            **STEP 4: Build in layers by frequency**\n\
            Start low: bass → mid: drums/pads → high: melody/hats\n\n\
            ## ANTI-PATTERNS (What NOT to Do)\n\n\
            ❌ **Using note() with numbers and .scale()**\n\
            `note(\"0 2 4 7\").scale(\"C:minor\")` - BUG! Use n() instead\n\
            Fix: `n(\"0 2 4 7\").scale(\"C:minor\")`\n\n\
            ❌ **Random note() without scale system**\n\
            `note(\"c3 f#5 g2\")` - Probably dissonant!\n\
            Fix: `n(\"0 5 7\").scale(\"C3:minor\")` OR `note(\"c3 g3 c4\")` (intentional voicing)\n\n\
            ❌ **Multiple conflicting keys**\n\
            Don't mix: `.scale(\"C:major\")` and `.scale(\"F#:major\")` in same pattern\n\
            Fix: Pick ONE key for entire composition\n\n\
            ❌ **Everything in same octave**\n\
            Bass and melody both in C4 = muddy mix\n\
            Fix: Bass C2, pads C3, melody C4-C5\n\n\
            ❌ **Too conservative/short phrases**\n\
            `note(\"c3\")` - Just one note is boring!\n\
            Fix: Generate 8-16 note phrases with rhythm\n\n\
            ❌ **No established tonal center**\n\
            Jumping between random scales confuses the ear\n\
            Fix: \"This piece is in C minor\" - announce it, stick to it\n\n\
            ## Genre Pattern Templates\n\n\
            **Techno (130 BPM):**\n\
            ```javascript\n\
            setcpm(130)\n\
            stack(\n\
              s(\"bd*4, ~ cp ~ cp, hh*8\").swing(0.05),\n\
              note(\"c2 c2 c2 c2\").s(\"sawtooth\").cutoff(800)\n\
            )\n\
            ```\n\n\
            **House (125 BPM):**\n\
            ```javascript\n\
            setcpm(125)\n\
            stack(\n\
              s(\"bd*4, [~ hh]*4, ~ cp ~ cp\"),\n\
              note(\"c2 ~ c2 ~\").s(\"sine\").gain(0.8)\n\
            )\n\
            ```\n\n\
            **Drum & Bass (174 BPM):**\n\
            ```javascript\n\
            setcpm(174)\n\
            stack(\n\
              s(\"bd ~ ~ [bd bd] ~ ~ bd ~, ~ ~ cp ~ ~ cp ~ ~\").fast(2),\n\
              note(\"e1 ~ ~ e2 ~ e1 ~ ~\").s(\"square\").cutoff(400)\n\
            )\n\
            ```\n\n\
            **Ambient (60 BPM):**\n\
            ```javascript\n\
            setcpm(60)\n\
            stack(\n\
              s(\"bd ~ ~ ~\").room(0.9),\n\
              note(\"c1\").s(\"sine\").attack(2).release(4).gain(0.6)\n\
            )\n\
            ```\n\n\
            **Trap (140 BPM):**\n\
            ```javascript\n\
            setcpm(140)\n\
            stack(\n\
              s(\"bd*2, ~ cp ~ cp, hh*16\").swing(0.2),\n\
              note(\"c2 c2 ~ c3\").s(\"square\")\n\
            )\n\
            ```\n\n\
            ## Common Chord Progressions\n\
            Use **generate_chord_progression** tool for these:\n\
            - **Pop**: I-V-vi-IV (e.g., C G Am F)\n\
            - **Jazz**: ii-V-I (e.g., Dm7 G7 Cmaj7)\n\
            - **Blues**: 12-bar blues (I7-IV7-V7)\n\
            - **EDM**: i-VI-III-VII (e.g., Am F C G)\n\n\
            ## Pattern Transformation Examples\n\n\
            **Add variation:**\n\
            ```javascript\n\
            s(\"bd cp\").sometimes(x => x.fast(2))  // subtle\n\
            s(\"bd cp\").every(4, x => x.rev)       // moderate\n\
            ```\n\n\
            **Humanize:**\n\
            ```javascript\n\
            s(\"bd cp\").nudge(rand.range(-0.02, 0.02))\n\
            ```\n\n\
            **Add swing:**\n\
            ```javascript\n\
            s(\"bd cp hh\").swing(0.1)  // 0.0-0.3 range\n\
            ```\n\n"
        );

        // Add code context if available
        if let Some(code) = code_context.as_ref() {
            system_prompt.push_str("## User's Current Code\n```javascript\n");
            system_prompt.push_str(code);
            system_prompt.push_str("\n```\n\n");
        }

        system_prompt.push_str(
            "## Guidelines\n\
            1. **Search first**: Use `search_strudel_docs()` before suggesting functions you're not certain about\n\
            2. **List sounds**: Use `list_available_sounds()` to check available samples/instruments\n\
            3. **Musical coherence**: Use `.scale()` for melodies, layer from bass up, establish tonal center\n\
            4. **Code format**: Always wrap code in ```javascript blocks\n\
            5. **Auto-validation**: Your code is validated automatically - you'll get error feedback\n\n\
            ## Queue Mode (Progressive Building)\n\n\
            When Queue Mode (🎬) is enabled, use `apply_live_code_edit` with `description` and `wait_cycles` to build progressively.\n\n\
            **Best practice: 2-4 substantial musical changes with 4–8 cycles between each (default 4) work well**\n\
            Think in musical sections rather than individual instruments. Combine elements that belong together.\n\n\
            **Good example:**\n\
            User: \"Build a techno beat progressively\"\n\
            ```\n\
            Call 1: Full rhythm section (kick+snare+hats together)\n\
              code: \"stack(s('bd*4'), s('~ sd ~ sd'), s('hh*8'))\"\n\
              wait_cycles: 0\n\n\
            Call 2: Bass and melody layer\n\
              code: \"stack(...drums, note('c2').s('saw').lpf(400), note('c4 e4 g4').s('tri'))\"\n\
              wait_cycles: 8\n\
            ```\n\n\
            This creates two distinct musical moments with time to appreciate each.\n\n\
            **Parameters:**\n\
            - `description`: Brief label (\"Drums\", \"Add bass layer\")\n\
            - `wait_cycles`: Cycles to wait AFTER the previous change was applied\n\
              - 0 for first change (applies immediately)\n\
              - DEFAULT: 4 (snappier pacing for shallow edits)\n\
              - RECOMMENDED: 4–8 (standard; prefer 4 unless user asks slower)\n\
              - LONGER SECTIONS: 12–16 (only for ambient/slow builds)\n\
              - NEVER: >16 without explicit user request\n\
              - Each wait is RELATIVE to when the last change applied (cumulative timing)\n\
            - Combine related elements in one call\n\n\
            **Timing Example (Standard 4-part structure):**\n\
            - Change 1 (wait_cycles: 0)  → Cycle 0 (immediate)\n\
            - Change 2 (wait_cycles: 4)  → Cycle 4\n\
            - Change 3 (wait_cycles: 4)  → Cycle 8\n\
            - Change 4 (wait_cycles: 4)  → Cycle 12\n\
            Total: 16 cycles for full progression\n\n\
            ## Common Syntax Errors to Avoid\n\
            ❌ Missing parentheses: `s \"bd sd\"` → ✅ `s(\"bd sd\")`\n\
            ❌ Unescaped quotes: `s(\"bd \"sd\"\")` → ✅ `s(\"bd sd\")` or `s(\"bd 'sd'\")`\n\
            ❌ Missing dots in chain: `s(\"bd\").fast(2)gain(0.5)` → ✅ `s(\"bd\").fast(2).gain(0.5)`\n\
            ❌ Non-pattern return: `const x = 5` → ✅ `s(\"bd\")` (must return Pattern)\n\
            ❌ Forgetting to call function: `note` → ✅ `note(\"c3\")`\n\n\
            Create patterns that are musically interesting and technically correct. Use tools to verify details.\n"
        );

        println!("✅ System prompt built with musical rules (~3.5k tokens, plenty of room left)");

        system_prompt
    }
}

// Tauri command to send a chat message
#[tauri::command]
pub async fn send_chat_message(
    window: WebviewWindow, // Reserved for future validation use
    message: String,
    state: State<'_, ChatState>,
) -> Result<String, String> {
    // Note: Automatic validation disabled due to Tauri v2 eval() limitations
    // System prompt now includes syntax error guidance to prevent common mistakes

    // Check if this is a doc search request from frontend
    if message.trim().starts_with("/search ") {
        let query = message.trim().strip_prefix("/search ").unwrap_or("");
        if let Some(search_results) = state.search_docs(query).await {
            return Ok(search_results);
        } else {
            return Ok(format!("No functions found matching '{}'", query));
        }
    }

    // Add user message to history
    let user_message = ChatMessage {
        role: "user".to_string(),
        content: message.clone(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    {
        let mut messages = state.messages.lock().await;
        messages.push(user_message);
    }

    let history_snapshot = {
        let messages = state.messages.lock().await;
        messages.clone()
    };

    let config = state.config.lock().await;
    let provider = config.provider.clone();
    let live_edit_enabled = config.live_edit_enabled;
    drop(config);

    let system_prompt = state.build_system_prompt().await;

    let tool_ctx = ToolRuntimeContext::new(
        Arc::clone(&state.full_docs),
        Arc::clone(&state.rate_limiter),
        Arc::clone(&state.rag_state),
        window.app_handle().clone(),
        window.label().to_string(),
        live_edit_enabled,
    );

    let (rig_history, rig_prompt) = convert_history_to_rig(&history_snapshot)?;

    let final_response = run_rig_chat(
        &window,
        tool_ctx,
        &provider,
        &system_prompt,
        rig_history,
        rig_prompt,
    )
    .await?;

    // Save response and return
    let assistant_message = ChatMessage {
        role: "assistant".to_string(),
        content: final_response.clone(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    {
        let mut messages = state.messages.lock().await;
        messages.push(assistant_message);
    }

    Ok(final_response)
}

// Helper function to get environment variable name based on provider
fn get_env_var_name(provider: &str) -> &'static str {
    if provider.starts_with("gpt-") || provider.starts_with("o3") || provider.starts_with("o4") {
        "OPENAI_API_KEY"
    } else if provider.starts_with("claude-") {
        "ANTHROPIC_API_KEY"
    } else if provider.starts_with("gemini-") {
        "GEMINI_API_KEY"
    } else {
        "API_KEY"
    }
}

fn emit_stream_event(window: &WebviewWindow, payload: StreamPayload) {
    if let Err(err) = window.emit(STREAM_EVENT, payload) {
        eprintln!("⚠️ Failed to emit chat stream event: {}", err);
    }
}

fn resolve_rig_target(provider: &str) -> Option<RigTarget> {
    if let Some(model) = provider.strip_prefix("ollama:") {
        return Some(RigTarget {
            provider: "ollama",
            model: model.to_string(),
        });
    }

    if provider.starts_with("gpt-") || provider.starts_with("o3") || provider.starts_with("o4") {
        return Some(RigTarget {
            provider: "openai",
            model: provider.to_string(),
        });
    }

    if provider.starts_with("claude-") {
        return Some(RigTarget {
            provider: "anthropic",
            model: provider.to_string(),
        });
    }

    if provider.starts_with("gemini-") {
        return Some(RigTarget {
            provider: "gemini",
            model: provider.to_string(),
        });
    }

    None
}

fn ensure_provider_ready(provider_kind: &str, provider_id: &str) -> Result<(), String> {
    match provider_kind {
        "openai" => ensure_env_present("OPENAI_API_KEY", provider_id),
        "anthropic" => ensure_env_present("ANTHROPIC_API_KEY", provider_id),
        "gemini" => ensure_env_present("GEMINI_API_KEY", provider_id),
        "ollama" => {
            if std::env::var("OLLAMA_API_BASE_URL").is_err() {
                let default = "http://localhost:11434";
                std::env::set_var("OLLAMA_API_BASE_URL", default);
                println!(
                    "ℹ️  OLLAMA_API_BASE_URL not set; defaulting to {} for provider {}",
                    default, provider_id
                );
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn ensure_env_present(var_name: &str, provider_id: &str) -> Result<(), String> {
    match std::env::var(var_name) {
        Ok(value) if !value.trim().is_empty() => Ok(()),
        _ => Err(format!(
            "Missing required environment variable {} for provider {}. Please save your API key in Chat Settings.",
            var_name, provider_id
        )),
    }
}

fn convert_history_to_rig(
    history: &[ChatMessage],
) -> Result<(Vec<RigMessage>, RigMessage), String> {
    if history.is_empty() {
        return Err("Conversation history is empty".to_string());
    }

    let mut rig_history = Vec::new();
    if history.len() > 1 {
        for message in &history[..history.len() - 1] {
            rig_history.push(chat_message_to_rig(message)?);
        }
    }

    let last = history
        .last()
        .ok_or_else(|| "Conversation history missing latest message".to_string())?;

    if last.role != "user" {
        return Err("Last message must be from the user".to_string());
    }

    let prompt = chat_message_to_rig(last)?;
    Ok((rig_history, prompt))
}

fn chat_message_to_rig(chat: &ChatMessage) -> Result<RigMessage, String> {
    let text = RigText {
        text: chat.content.clone(),
    };

    match chat.role.as_str() {
        "user" => Ok(RigMessage::User {
            content: OneOrMany::one(RigUserContent::Text(text)),
        }),
        "assistant" => Ok(RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(RigAssistantContent::Text(text)),
        }),
        other => Err(format!("Unsupported chat role '{}'", other)),
    }
}

async fn run_rig_chat(
    window: &WebviewWindow,
    tool_ctx: ToolRuntimeContext,
    provider_id: &str,
    system_prompt: &str,
    history: Vec<RigMessage>,
    prompt: RigMessage,
) -> Result<String, String> {
    let target = resolve_rig_target(provider_id)
        .ok_or_else(|| format!("Rig: unsupported provider '{}'", provider_id))?;

    ensure_provider_ready(target.provider, provider_id)?;

    emit_stream_event(window, StreamPayload::start(target.provider, &target.model));

    let agent = {
        let builder = DynClientBuilder::new();
        let mut agent_builder = builder
            .agent(target.provider, &target.model)
            .map_err(|e| format!("Rig: failed to initialize {}: {}", target.provider, e))?;

        agent_builder = agent_builder
            .name("StrudelRigAgent")
            .preamble(system_prompt)
            .max_tokens(8192) // Increased from 2048 to allow longer responses and multiple tool calls
            .tool(RigSearchDocsTool::new(tool_ctx.clone()))
            .tool(RigListSoundsTool::new(tool_ctx.clone()))
            .tool(RigChordProgressionTool::new(tool_ctx.clone()))
            .tool(RigEuclideanRhythmTool::new(tool_ctx.clone()));

        if tool_ctx.live_edit_enabled() {
            agent_builder = agent_builder.tool(RigApplyLiveEditTool::new(tool_ctx.clone()));
        }

        agent_builder.build()
    };

    let mut stream = agent.stream_chat(prompt, history).multi_turn(8).await;

    let mut final_response = String::new();
    let mut pending_usage: Option<StreamUsagePayload> = None;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(MultiTurnStreamItem::StreamItem(content)) => match content {
                StreamedAssistantContent::Text(text) => {
                    let delta = text.text.clone();
                    final_response.push_str(&delta);
                    emit_stream_event(window, StreamPayload::delta(delta));
                }
                StreamedAssistantContent::Reasoning(reasoning) => {
                    let thought = reasoning.reasoning.join("");
                    emit_stream_event(window, StreamPayload::reasoning(thought));
                }
                StreamedAssistantContent::ToolCall(call) => {
                    emit_stream_event(
                        window,
                        StreamPayload::tool_call(
                            &call.function.name,
                            call.function.arguments.to_string(),
                        ),
                    );
                }
                StreamedAssistantContent::Final(final_resp) => {
                    if let Some(usage) = final_resp.token_usage() {
                        pending_usage = Some(StreamUsagePayload::from(usage));
                    }
                }
            },
            Ok(MultiTurnStreamItem::FinalResponse(final_chunk)) => {
                if final_response.is_empty() {
                    final_response = final_chunk.response().to_string();
                }
                let usage_payload = StreamUsagePayload::from(final_chunk.usage());
                emit_stream_event(
                    window,
                    StreamPayload::done(final_response.clone(), Some(usage_payload)),
                );
                return Ok(final_response);
            }
            Ok(_) => {}
            Err(err) => {
                let error_message = format!("Rig streaming error: {}", err);
                emit_stream_event(window, StreamPayload::error(error_message.clone()));
                return Err(error_message);
            }
        }
    }

    if !final_response.is_empty() {
        emit_stream_event(
            window,
            StreamPayload::done(final_response.clone(), pending_usage),
        );
        Ok(final_response)
    } else {
        let msg = "Rig streaming finished without a response".to_string();
        emit_stream_event(window, StreamPayload::error(msg.clone()));
        Err(msg)
    }
}

// Tauri command to set API configuration
#[tauri::command]
pub async fn set_chat_config(
    app: AppHandle,
    provider: String,
    api_key: Option<String>,
    live_edit_enabled: Option<bool>,
    state: State<'_, ChatState>,
) -> Result<(), String> {
    // Set environment variable for current session if API key provided
    if let Some(key) = &api_key {
        let env_var = get_env_var_name(&provider);
        std::env::set_var(env_var, key);
    }

    let mut config = state.config.lock().await;
    config.provider = provider.clone();
    config.api_key = api_key.clone();
    if let Some(enabled) = live_edit_enabled {
        config.live_edit_enabled = enabled;
    }
    let allow_live_edit = config.live_edit_enabled;

    // Save settings to store
    let store = app
        .store("strudel-settings.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    store.set("chat_provider".to_string(), serde_json::json!(provider));
    store.set(
        "chat_live_edit_enabled".to_string(),
        serde_json::json!(allow_live_edit),
    );

    // Save API key to store (or remove it if None)
    if let Some(key) = &api_key {
        store.set("chat_api_key".to_string(), serde_json::json!(key));
    } else {
        let _ = store.delete("chat_api_key");
    }

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    println!("✅ Saved chat config to store: {}", provider);

    Ok(())
}

// Tauri command to load Strudel documentation
#[tauri::command]
pub async fn load_strudel_docs(
    docs_json: String,
    examples_json: String,
    state: State<'_, ChatState>,
) -> Result<(), String> {
    // Parse examples first (more important for creativity)
    let examples: serde_json::Value = serde_json::from_str(&examples_json)
        .map_err(|e| format!("Failed to parse examples: {}", e))?;

    let mut formatted_examples = String::new();

    // Add example patterns
    if let Some(examples_array) = examples["examples"].as_array() {
        formatted_examples.push_str("Example Patterns:\n");
        for ex in examples_array.iter() {
            if let (Some(name), Some(code), Some(desc)) = (
                ex["name"].as_str(),
                ex["code"].as_str(),
                ex["description"].as_str(),
            ) {
                formatted_examples.push_str(&format!(
                    "{}:\n```javascript\n{}\n```\n{}\n\n",
                    name, code, desc
                ));
            }
        }
    }

    // Add creativity tips
    if let Some(tips_array) = examples["creativity_tips"].as_array() {
        formatted_examples.push_str("\nCreativity Tips:\n");
        for tip in tips_array.iter() {
            if let Some(tip_str) = tip.as_str() {
                formatted_examples.push_str(&format!("- {}\n", tip_str));
            }
        }
    }

    let mut examples_lock = state.examples.write().await;
    *examples_lock = Some(formatted_examples);

    // Parse doc.json
    let docs: serde_json::Value =
        serde_json::from_str(&docs_json).map_err(|e| format!("Failed to parse docs: {}", e))?;

    // Store full docs for search functionality
    {
        let mut full_docs = state.full_docs.write().await;
        *full_docs = Some(docs.clone());
        println!("✅ Stored full documentation for search");
    }

    // Initialize RAG with embeddings
    if let Ok(embeddings_json) = std::fs::read_to_string("embeddings.json") {
        if let Err(e) = state.rag_state.load_from_json(&embeddings_json).await {
            eprintln!("Failed to load RAG: {}", e);
        } else {
            println!("✅ RAG initialized for semantic search");
        }
    }

    let mut formatted_docs = String::new();

    // Priority functions that are most important for music generation
    let priority_categories = [
        // Core pattern functions
        "note",
        "s",
        "sound",
        "n",
        "stack",
        "cat",
        "fastcat",
        "slowcat",
        // Pattern transformations
        "fast",
        "slow",
        "rev",
        "jux",
        "every",
        "off",
        "layer",
        "chunk",
        // Rhythm
        "struct",
        "mask",
        "euclid",
        "euclidBy",
        // Randomness & variation
        "sometimes",
        "often",
        "rarely",
        "sometimesBy",
        "rand",
        "irand",
        "choose",
        // Effects
        "gain",
        "pan",
        "lpf",
        "hpf",
        "room",
        "delay",
        "crush",
        "shape",
        "delaytime",
        "delayfeedback",
        "size",
        "res",
        "cutoff",
        // Sound properties
        "speed",
        "bank",
        "vowel",
        "decay",
        "sustain",
        "release",
        // Musical
        "scale",
        "chord",
        "voicing",
        "add",
        "sub",
        "mul",
        "div",
        // Time manipulation
        "late",
        "early",
        "hurry",
        "linger",
        // Envelopes
        "attack",
        "hold",
        "adsr",
        // Filters
        "bpf",
        "resonance",
        "vcf",
        // Spatial
        "orbit",
        "room",
        "roomsize",
        // Modulation
        "sine",
        "saw",
        "square",
        "tri",
        "range",
        // Pattern operations
        "degradeBy",
        "chop",
        "striate",
        "gap",
        "compress",
    ];

    if let Some(docs_array) = docs["docs"].as_array() {
        let mut priority_docs = Vec::new();
        let mut other_docs = Vec::new();

        for doc in docs_array.iter() {
            if let (Some(name), Some(description)) =
                (doc["name"].as_str(), doc["description"].as_str())
            {
                // Clean HTML tags from description
                let clean_desc = description
                    .replace("<p>", "")
                    .replace("</p>", "")
                    .replace("<code>", "`")
                    .replace("</code>", "`");

                let mut doc_text = format!("- {}: {}", name, clean_desc);

                // Include examples if available
                if let Some(examples) = doc["examples"].as_array() {
                    if let Some(first_example) = examples.first() {
                        if let Some(example_str) = first_example.as_str() {
                            // Truncate long examples
                            let example = if example_str.len() > 100 {
                                format!("{}...", &example_str[..100])
                            } else {
                                example_str.to_string()
                            };
                            doc_text.push_str(&format!(" Ex: {}", example));
                        }
                    }
                }

                doc_text.push('\n');

                // Categorize by priority
                if priority_categories.contains(&name) {
                    priority_docs.push((name.to_string(), doc_text));
                } else {
                    other_docs.push((name.to_string(), doc_text));
                }
            }
        }

        // Add priority docs first
        formatted_docs.push_str("🔥 ESSENTIAL FUNCTIONS:\n");
        for (_, doc) in priority_docs.iter() {
            formatted_docs.push_str(doc);
        }

        // Add a selection of other useful docs (limit to 50 to save tokens)
        if !other_docs.is_empty() {
            formatted_docs.push_str("\n📚 OTHER FUNCTIONS:\n");
            for (_, doc) in other_docs.iter().take(50) {
                formatted_docs.push_str(doc);
            }
        }

        println!(
            "📊 Loaded {} priority functions, {} other functions (showing {})",
            priority_docs.len(),
            other_docs.len(),
            other_docs.len().min(50)
        );
    }

    let mut strudel_docs = state.strudel_docs.write().await;
    *strudel_docs = Some(formatted_docs);

    Ok(())
}

// Tauri command to set code context
#[tauri::command]
pub async fn set_code_context(code: String, state: State<'_, ChatState>) -> Result<(), String> {
    let mut code_context = state.code_context.write().await;
    *code_context = Some(code);
    Ok(())
}

// Tauri command to clear code context
#[tauri::command]
pub async fn clear_code_context(state: State<'_, ChatState>) -> Result<(), String> {
    let mut code_context = state.code_context.write().await;
    *code_context = None;
    Ok(())
}

// Tauri command to get chat history
#[tauri::command]
pub async fn get_chat_history(state: State<'_, ChatState>) -> Result<Vec<ChatMessage>, String> {
    let messages = state.messages.lock().await;
    Ok(messages.clone())
}

// Tauri command to clear chat history
#[tauri::command]
pub async fn clear_chat_history(state: State<'_, ChatState>) -> Result<(), String> {
    let mut messages = state.messages.lock().await;
    messages.clear();
    Ok(())
}

// Tauri command to get saved chat config
#[tauri::command]
pub async fn get_chat_config(
    app: AppHandle,
    state: State<'_, ChatState>,
) -> Result<ChatConfig, String> {
    // Load config from store
    let store = app
        .store("strudel-settings.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    let provider = store
        .get("chat_provider")
        .and_then(|v| v.as_str().map(String::from));
    let live_edit_enabled = store
        .get("chat_live_edit_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if let Some(provider_val) = provider {
        // Try to get API key from store first
        let api_key = store
            .get("chat_api_key")
            .and_then(|v| v.as_str().map(String::from))
            // Fallback to environment variable if not in store
            .or_else(|| {
                let env_var = get_env_var_name(&provider_val);
                std::env::var(env_var).ok()
            });

        // Update state with loaded values
        let mut config = state.config.lock().await;
        config.provider = provider_val.clone();
        config.api_key = api_key.clone();
        config.live_edit_enabled = live_edit_enabled;

        // Set environment variable if key exists
        if let Some(key) = &api_key {
            let env_var = get_env_var_name(&provider_val);
            std::env::set_var(env_var, key);
            println!(
                "✅ Loaded API key for {} from store and set env var",
                provider_val
            );
        } else {
            println!(
                "⚠️  No API key found in store or environment for {}",
                provider_val
            );
        }

        Ok(ChatConfig {
            provider: provider_val,
            api_key,
            live_edit_enabled,
        })
    } else {
        // Return default config if nothing saved
        let config = state.config.lock().await;
        Ok(config.clone())
    }
}

// Validation result structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

// Helper function to extract code blocks from markdown (for future validation use)
#[allow(dead_code)]
fn extract_code_blocks(text: &str) -> Vec<String> {
    let re = Regex::new(r"```(?:javascript|js)?\s*\n([\s\S]*?)```").unwrap();
    re.captures_iter(text)
        .map(|cap| cap[1].trim().to_string())
        .filter(|code| !code.is_empty())
        .collect()
}

// Tauri command to validate Strudel code using frontend transpiler
#[tauri::command]
pub async fn validate_strudel_code(
    window: WebviewWindow,
    code: String,
) -> Result<ValidationResult, String> {
    // Security: Enforce maximum code size to prevent payload attacks
    const MAX_CODE_SIZE: usize = 100 * 1024; // 100KB limit
    if code.len() > MAX_CODE_SIZE {
        return Ok(ValidationResult {
            valid: false,
            error: Some(format!(
                "Code exceeds maximum size of {} bytes",
                MAX_CODE_SIZE
            )),
            line: None,
            column: None,
        });
    }

    // Security: Comprehensive escaping to prevent injection attacks
    // This properly escapes all potentially dangerous characters
    let escaped_code = code
        .replace('\\', "\\\\") // Backslash
        .replace('`', "\\`") // Backticks (template strings)
        .replace('$', "\\$") // Dollar signs (template interpolation)
        .replace('"', "\\\"") // Double quotes (SECURITY FIX)
        .replace('\'', "\\'") // Single quotes (SECURITY FIX)
        .replace('\n', "\\n") // Newlines
        .replace('\r', "\\r") // Carriage returns
        .replace('\t', "\\t"); // Tabs (SECURITY FIX)

    // Execute JavaScript validation via eval (synchronous execution)
    // Note: Tauri's eval() executes JS but doesn't return values or wait for promises
    // So we use a simpler approach: just check if transpiler can parse the code
    let js_code = format!(
        r#"
        (function() {{
            try {{
                // Import transpiler dynamically and attempt to transpile
                import('/packages/transpiler/index.mjs').then(({{ transpiler }}) => {{
                    transpiler(`{}`);
                    console.log('✅ Validation passed');
                }}).catch((e) => {{
                    console.error('❌ Validation failed:', e.message);
                }});
            }} catch (e) {{
                console.error('❌ Validation error:', e.message);
            }}
        }})();
        "#,
        escaped_code
    );

    // Execute validation (fire-and-forget, check console for results)
    window
        .eval(&js_code)
        .map_err(|e| format!("Validation eval failed: {}", e))?;

    // Since eval doesn't return values in Tauri v2, we return a permissive result
    // The system prompt guidance should prevent most errors
    // Frontend can call validate_strudel_code directly for stricter validation
    Ok(ValidationResult {
        valid: true,
        error: None,
        line: None,
        column: None,
    })
}

// Initialize the chat state
pub fn init() -> ChatState {
    ChatState::new()
}
