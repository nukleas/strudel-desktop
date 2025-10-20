use agentai::Agent;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_store::StoreExt;
use tokio::sync::{Mutex, RwLock};
use crate::tools::StrudelToolBox;
use keyring::Entry;
use thiserror::Error;

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
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            provider: "claude-sonnet-4-5-20250929".to_string(),
            api_key: None,
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
        }
    }

    // Search for specific functions in full docs
    async fn search_docs(&self, query: &str) -> Option<String> {
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

    // Build system prompt with current context (OPTIMIZED - reduced from ~20k to ~1.5k tokens)
    async fn build_system_prompt(&self) -> String {
        let code_context = self.code_context.read().await;

        println!("📋 Building optimized system prompt (tool-assisted)");

        // Build minimal system prompt - detailed docs are accessed via tools
        let mut system_prompt = String::from(
            "You are an expert in Strudel, a live coding pattern language for making music in the browser.\n\
            Strudel uses JavaScript with mini notation for creating musical patterns.\n\n\
            ## Available Tools\n\
            Use these tools proactively - they're fast and accurate:\n\
            - **search_strudel_docs(query)** - Search function documentation before suggesting unfamiliar functions\n\
            - **list_available_sounds(type, filter)** - Query available samples, synths, or GM instruments\n\n\
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
            - GM: `s(\"gm_piano\")` `s(\"gm_acoustic_guitar_nylon\")` (use search_strudel_docs for full list)\n\n\
            **Common Effects:**\n\
            - `.room()`, `.delay()`, `.lpf()`, `.hpf()`, `.crush()`, `.gain()`, `.pan()`\n\n\
            **Variation:**\n\
            - `.sometimes()`, `.often()`, `.rarely()`, `.every()`\n\n"
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
            Create patterns that are musically interesting and technically correct. Use tools to verify details.\n"
        );

        println!("✅ Optimized system prompt built (~1.5k tokens vs previous ~20k)");

        system_prompt
    }
}

// Tauri command to send a chat message with validation
#[tauri::command]
pub async fn send_chat_message(
    window: WebviewWindow,
    message: String,
    state: State<'_, ChatState>,
) -> Result<String, String> {
    const MAX_VALIDATION_RETRIES: usize = 3;

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

    // Validation loop
    let mut attempt = 0;
    let mut validation_error: Option<String> = None;
    let mut current_prompt = message.clone();

    loop {
        // Get configuration
        let config = state.config.lock().await;
        let provider = config.provider.clone();
        drop(config);

        // Build system prompt with current context
        let system_prompt = state.build_system_prompt().await;

        // Build conversation history for context (include previous messages)
        let conversation_history = {
            let messages = state.messages.lock().await;
            if messages.len() > 1 {
                // Include last N messages for context (exclude current user message at end)
                let history_limit = 10; // Keep last 10 messages for context
                let start = messages.len().saturating_sub(history_limit + 1);
                let relevant_messages: Vec<String> = messages[start..messages.len()-1]
                    .iter()
                    .map(|msg| format!("{}: {}", msg.role, msg.content))
                    .collect();

                if !relevant_messages.is_empty() {
                    format!("Previous conversation:\n{}\n\nCurrent message: {}",
                        relevant_messages.join("\n"),
                        current_prompt)
                } else {
                    current_prompt.clone()
                }
            } else {
                current_prompt.clone()
            }
        };

        // Clone Arc pointers for move into spawn_blocking
        let full_docs_clone = Arc::clone(&state.full_docs);
        let examples_clone = Arc::clone(&state.examples);
        let code_context_clone = Arc::clone(&state.code_context);
        let rate_limiter_clone = Arc::clone(&state.rate_limiter);

        // Clone for move into spawn_blocking
        let prompt_with_history = conversation_history;
        let provider_clone = provider.clone();
        let system_prompt_clone = system_prompt.clone();

        // Get handle to current runtime to reuse instead of creating a new one
        let runtime_handle = tokio::runtime::Handle::current();

        // Run agent in blocking thread pool (sidesteps Send requirement)
        let response = tokio::task::spawn_blocking(move || {
            // Check if tools are enabled (kill switch)
            let tools_enabled = std::env::var("STRUDEL_ENABLE_TOOLS")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                == "true";

            if !tools_enabled {
                println!("⚠️  Tools disabled via STRUDEL_ENABLE_TOOLS env var");
            }

            // Create agent in blocking context
            let mut agent = Agent::new(&system_prompt_clone);

            // Use existing runtime handle instead of creating a new runtime
            // This eliminates the nested runtime anti-pattern
            let result: Result<String, _> = runtime_handle.block_on(async {
                if tools_enabled {
                    // Create toolbox in blocking context
                    let toolbox = StrudelToolBox {
                        full_docs: full_docs_clone,
                        examples: examples_clone,
                        code_context: code_context_clone,
                        rate_limiter: rate_limiter_clone,
                    };

                    // Run with tools enabled
                    agent
                        .run(&provider_clone, &prompt_with_history, Some(&toolbox))
                        .await
                        .map_err(|e| format!("Agent error: {}", e))
                } else {
                    // Run without tools (fallback mode)
                    agent
                        .run(&provider_clone, &prompt_with_history, None)
                        .await
                        .map_err(|e| format!("Agent error: {}", e))
                }
            });

            result
        })
        .await
        .map_err(|e| format!("Task error: {}", e))??;

        // Extract code blocks from response
        let code_blocks = extract_code_blocks(&response);

        if code_blocks.is_empty() {
            // No code to validate, return response
            let assistant_message = ChatMessage {
                role: "assistant".to_string(),
                content: response.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            };

            {
                let mut messages = state.messages.lock().await;
                messages.push(assistant_message);
            }

            return Ok(response);
        }

        // Validate all code blocks
        let mut all_valid = true;
        for code in &code_blocks {
            let validation = validate_strudel_code(window.clone(), code.clone()).await?;

            if !validation.valid {
                all_valid = false;
                let error_msg = validation
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string());
                let location = if let Some(line) = validation.line {
                    format!(" at line {}", line)
                } else {
                    String::new()
                };
                validation_error = Some(format!("{}{}", error_msg, location));
                break;
            }
        }

        if all_valid {
            // All code is valid! Save and return
            let assistant_message = ChatMessage {
                role: "assistant".to_string(),
                content: response.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            };

            {
                let mut messages = state.messages.lock().await;
                messages.push(assistant_message);
            }

            return Ok(response);
        }

        // Code validation failed
        attempt += 1;
        if attempt >= MAX_VALIDATION_RETRIES {
            // Max retries reached, return with warning
            let error_text = validation_error
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("Unknown error");
            let warning_response = format!(
                "{}\n\n⚠️ **Validation Warning**: Generated code failed validation after {} attempts.\n\
                **Error**: {}\n\
                Please review the code carefully before using it.",
                response,
                MAX_VALIDATION_RETRIES,
                error_text
            );

            let assistant_message = ChatMessage {
                role: "assistant".to_string(),
                content: warning_response.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            };

            {
                let mut messages = state.messages.lock().await;
                messages.push(assistant_message);
            }

            return Ok(warning_response);
        }

        // Prepare retry prompt
        let error_text = validation_error
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Unknown error");
        current_prompt = format!(
            "The code you generated has a syntax error:\n{}\n\n\
            Please fix the error and regenerate the code.\n\
            Original request: {}",
            error_text, message
        );

        // Loop continues for retry...
    }
}

// Helper function to get keyring service name based on provider
fn get_keyring_service(provider: &str) -> &'static str {
    if provider.starts_with("gpt-") || provider.starts_with("o3") || provider.starts_with("o4") {
        "strudel-desktop-openai"
    } else if provider.starts_with("claude-") {
        "strudel-desktop-anthropic"
    } else if provider.starts_with("gemini-") {
        "strudel-desktop-gemini"
    } else {
        "strudel-desktop-api"
    }
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

// Store API key securely in OS keychain
fn store_api_key_secure(provider: &str, api_key: &str) -> Result<(), String> {
    let service = get_keyring_service(provider);
    let entry = Entry::new(service, "api_key")
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    entry
        .set_password(api_key)
        .map_err(|e| format!("Failed to store API key in keychain: {}", e))?;

    Ok(())
}

// Retrieve API key securely from OS keychain
fn get_api_key_secure(provider: &str) -> Option<String> {
    let service = get_keyring_service(provider);
    let entry = Entry::new(service, "api_key").ok()?;
    entry.get_password().ok()
}

// Delete API key from OS keychain
fn delete_api_key_secure(provider: &str) -> Result<(), String> {
    let service = get_keyring_service(provider);
    let entry = Entry::new(service, "api_key")
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    // Ignore error if key doesn't exist
    let _ = entry.delete_credential();

    Ok(())
}

// Tauri command to set API configuration
#[tauri::command]
pub async fn set_chat_config(
    app: AppHandle,
    provider: String,
    api_key: Option<String>,
    state: State<'_, ChatState>,
) -> Result<(), String> {
    // Store API key securely in OS keychain if provided
    if let Some(key) = &api_key {
        store_api_key_secure(&provider, key)?;

        // Set environment variable for current session
        let env_var = get_env_var_name(&provider);
        std::env::set_var(env_var, key);
    } else {
        // Delete from keychain if clearing key
        delete_api_key_secure(&provider)?;
    }

    let mut config = state.config.lock().await;
    config.provider = provider.clone();
    config.api_key = api_key.clone();

    // Save provider preference to store (but NOT the API key - that's in keychain now)
    let store = app
        .store("strudel-settings.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    store.set("chat_provider".to_string(), serde_json::json!(provider));

    // Remove any legacy plaintext API keys from store (migration)
    let _ = store.delete("chat_api_key".to_string());

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

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
    // Load provider from store
    let store = app
        .store("strudel-settings.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    let provider = store
        .get("chat_provider")
        .and_then(|v| v.as_str().map(String::from));

    if let Some(provider_val) = provider {
        // Try to get API key from keychain first (secure)
        let api_key = get_api_key_secure(&provider_val)
            // Fallback to legacy plaintext store for migration
            .or_else(|| {
                let legacy_key = store
                    .get("chat_api_key")
                    .and_then(|v| v.as_str().map(String::from));

                // If found in legacy store, migrate to keychain
                if let Some(ref key) = legacy_key {
                    println!("⚠️  Migrating API key from plaintext to keychain");
                    if let Err(e) = store_api_key_secure(&provider_val, key) {
                        eprintln!("Failed to migrate API key to keychain: {}", e);
                    } else {
                        // Successfully migrated, remove from plaintext store
                        let _ = store.delete("chat_api_key".to_string());
                        let _ = store.save();
                        println!("✅ API key migrated to secure keychain");
                    }
                }

                legacy_key
            });

        // Update state with loaded values
        let mut config = state.config.lock().await;
        config.provider = provider_val.clone();
        config.api_key = api_key.clone();

        // Set environment variable if key exists
        if let Some(key) = &api_key {
            let env_var = get_env_var_name(&provider_val);
            std::env::set_var(env_var, key);
        }

        Ok(ChatConfig {
            provider: provider_val,
            api_key,
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

// Helper function to extract code blocks from markdown
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
        .replace('\\', "\\\\")  // Backslash
        .replace('`', "\\`")    // Backticks (template strings)
        .replace('$', "\\$")    // Dollar signs (template interpolation)
        .replace('"', "\\\"")   // Double quotes (SECURITY FIX)
        .replace('\'', "\\'")   // Single quotes (SECURITY FIX)
        .replace('\n', "\\n")   // Newlines
        .replace('\r', "\\r")   // Carriage returns
        .replace('\t', "\\t");  // Tabs (SECURITY FIX)

    // Validation with timeout to prevent DoS
    let validation_future = async {
        window
            .eval(&format!(
                r#"
                (async () => {{
                    try {{
                        const {{ transpiler }} = await import('/packages/transpiler/index.mjs');
                        transpiler(`{}`);
                    }} catch (e) {{
                        console.error('Validation error:', e);
                        throw e;
                    }}
                }})();
            "#,
                escaped_code
            ))
            .map_err(|e| format!("Validation eval failed: {}", e))
    };

    // Security: 10-second timeout to prevent hanging
    match tokio::time::timeout(std::time::Duration::from_secs(10), validation_future).await {
        Ok(result) => {
            result?;
            // If eval didn't throw, assume valid
            Ok(ValidationResult {
                valid: true,
                error: None,
                line: None,
                column: None,
            })
        }
        Err(_) => Ok(ValidationResult {
            valid: false,
            error: Some("Validation timeout - code took too long to validate".to_string()),
            line: None,
            column: None,
        }),
    }
}

// Initialize the chat state
pub fn init() -> ChatState {
    ChatState::new()
}
