use agentai::Agent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// ChatMessage represents a single message in the conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,      // "user" or "assistant"
    pub content: String,
    pub timestamp: i64,
}

// ChatConfig stores the LLM configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatConfig {
    pub provider: String,    // e.g., "gpt-4o", "claude-3-5-sonnet-20241022", "ollama:llama3"
    pub api_key: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            provider: "gpt-4o".to_string(),
            api_key: None,
        }
    }
}

// ChatState manages the conversation state and context
pub struct ChatState {
    pub messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub config: Arc<Mutex<ChatConfig>>,
    pub strudel_docs: Arc<Mutex<Option<String>>>,
    pub code_context: Arc<Mutex<Option<String>>>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            config: Arc::new(Mutex::new(ChatConfig::default())),
            strudel_docs: Arc::new(Mutex::new(None)),
            code_context: Arc::new(Mutex::new(None)),
        }
    }

    // Build system prompt with current context
    async fn build_system_prompt(&self) -> String {
        let strudel_docs = self.strudel_docs.lock().await;
        let code_context = self.code_context.lock().await;

        // Build system prompt
        let mut system_prompt = String::from(
            "You are an expert in Strudel, a live coding pattern language for making music in the browser. \
            Strudel is based on TidalCycles and uses JavaScript with a mini notation for creating musical patterns.\n\n"
        );

        // Add documentation context if available
        if let Some(docs) = strudel_docs.as_ref() {
            system_prompt.push_str("You have access to the Strudel API documentation:\n");
            system_prompt.push_str(docs);
            system_prompt.push_str("\n\n");
        }

        // Add code context if available
        let has_code = code_context.is_some();
        if let Some(code) = code_context.as_ref() {
            system_prompt.push_str("The user is currently working on this code:\n");
            system_prompt.push_str("```javascript\n");
            system_prompt.push_str(code);
            system_prompt.push_str("\n```\n\n");
        }

        system_prompt.push_str(
            "When generating Strudel code:\n\
            - Use the mini notation for patterns (e.g., note(\"c3 e3 g3\"))\n\
            - Chain methods with dots (e.g., .s(\"piano\").slow(2))\n\
            - Explain the pattern in simple terms\n\
            - Focus on musical creativity and experimentation\n\
            - Always wrap code in ```javascript blocks\n"
        );

        if has_code {
            system_prompt.push_str(
                "\nWhen the user asks for changes or improvements:\n\
                - If it's a small modification, show just the modified version of their code\n\
                - If it's a complete rewrite or new pattern, provide the full new code\n\
                - Explain what changed and why\n\
                - Users can click 'Append' to add code to the end, or 'Replace All' to replace everything\n\n"
            );
        }

        system_prompt.push_str(
            "\nBe helpful, concise, and encourage musical exploration!"
        );

        system_prompt
    }
}

// Tauri command to send a chat message
#[tauri::command]
pub async fn send_chat_message(
    message: String,
    state: State<'_, ChatState>,
) -> Result<String, String> {
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

    // Get configuration
    let config = state.config.lock().await;
    let provider = config.provider.clone();
    drop(config);

    // Build system prompt with current context
    let system_prompt = state.build_system_prompt().await;

    // Clone message for move into spawn_blocking
    let message_clone = message.clone();
    let provider_clone = provider.clone();

    // Run agent in blocking thread pool to avoid Send requirements
    let response = tokio::task::spawn_blocking(move || {
        // Create agent for this request
        let mut agent = Agent::new(&system_prompt);

        // Use tokio runtime to run the async agent code
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        let result: Result<String, _> = runtime.block_on(async {
            agent
                .run(&provider_clone, &message_clone, None)
                .await
                .map_err(|e| format!("Agent error: {}", e))
        });

        result
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    // Add assistant message to history
    let assistant_message = ChatMessage {
        role: "assistant".to_string(),
        content: response.clone(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    {
        let mut messages = state.messages.lock().await;
        messages.push(assistant_message);
    }

    Ok(response)
}

// Tauri command to set API configuration
#[tauri::command]
pub async fn set_chat_config(
    provider: String,
    api_key: Option<String>,
    state: State<'_, ChatState>,
) -> Result<(), String> {
    // Set API key as environment variable if provided
    if let Some(key) = &api_key {
        // Determine which env var to set based on provider
        if provider.starts_with("gpt-") {
            std::env::set_var("OPENAI_API_KEY", key);
        } else if provider.starts_with("claude-") {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        } else if provider.starts_with("gemini-") {
            std::env::set_var("GEMINI_API_KEY", key);
        }
    }

    let mut config = state.config.lock().await;
    config.provider = provider;
    config.api_key = api_key;

    Ok(())
}

// Tauri command to load Strudel documentation
#[tauri::command]
pub async fn load_strudel_docs(
    docs_json: String,
    state: State<'_, ChatState>,
) -> Result<(), String> {
    // Parse the doc.json and extract relevant information
    let docs: serde_json::Value = serde_json::from_str(&docs_json)
        .map_err(|e| format!("Failed to parse docs: {}", e))?;

    let mut formatted_docs = String::new();

    if let Some(docs_array) = docs["docs"].as_array() {
        // Limit to first 100 functions to avoid context overflow
        for doc in docs_array.iter().take(100) {
            if let (Some(name), Some(description)) = (
                doc["name"].as_str(),
                doc["description"].as_str(),
            ) {
                formatted_docs.push_str(&format!("- {}: {}\n", name, description));
            }
        }
    }

    let mut strudel_docs = state.strudel_docs.lock().await;
    *strudel_docs = Some(formatted_docs);

    Ok(())
}

// Tauri command to set code context
#[tauri::command]
pub async fn set_code_context(
    code: String,
    state: State<'_, ChatState>,
) -> Result<(), String> {
    let mut code_context = state.code_context.lock().await;
    *code_context = Some(code);
    Ok(())
}

// Tauri command to clear code context
#[tauri::command]
pub async fn clear_code_context(
    state: State<'_, ChatState>,
) -> Result<(), String> {
    let mut code_context = state.code_context.lock().await;
    *code_context = None;
    Ok(())
}

// Tauri command to get chat history
#[tauri::command]
pub async fn get_chat_history(
    state: State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    let messages = state.messages.lock().await;
    Ok(messages.clone())
}

// Tauri command to clear chat history
#[tauri::command]
pub async fn clear_chat_history(
    state: State<'_, ChatState>,
) -> Result<(), String> {
    let mut messages = state.messages.lock().await;
    messages.clear();
    Ok(())
}

// Initialize the chat state
pub fn init() -> ChatState {
    ChatState::new()
}
