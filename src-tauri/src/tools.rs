// Strudel AI Chat Tools
// Tools that the AI agent can use to interact with documentation and code

use crate::chatbridge::RateLimiter;
use crate::music_theory::MusicTheory;
use anyhow::{anyhow, Result as AnyResult};
use rig::{completion::ToolDefinition as RigToolDefinition, tool::Tool as RigTool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct ToolRuntimeContext {
    pub full_docs: Arc<RwLock<Option<serde_json::Value>>>,
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
    pub rag_state: Arc<crate::rag::RagState>,
    app_handle: AppHandle,
    window_label: String,
    live_edit_enabled: bool,
}

impl ToolRuntimeContext {
    pub fn new(
        full_docs: Arc<RwLock<Option<serde_json::Value>>>,
        rate_limiter: Arc<Mutex<RateLimiter>>,
        rag_state: Arc<crate::rag::RagState>,
        app_handle: tauri::AppHandle,
        window_label: String,
        live_edit_enabled: bool,
    ) -> Self {
        Self {
            full_docs,
            rate_limiter,
            rag_state,
            app_handle,
            window_label,
            live_edit_enabled,
        }
    }

    async fn check_rate_limit(&self, tool_name: &str) -> AnyResult<()> {
        let mut limiter = self.rate_limiter.lock().await;
        limiter
            .check_and_increment(tool_name)
            .map_err(|e| anyhow!(e))
    }

    pub async fn search_strudel_docs(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> AnyResult<String> {
        self.check_rate_limit("search_strudel_docs").await?;

        if query.len() > 500 {
            return Err(anyhow!("Query too long (max 500 characters)"));
        }

        let limit = limit.unwrap_or(5).min(10);

        if let Ok(query_embedding) = self.rag_state.embed_query(&query).await {
            if let Ok(rag_results) = self
                .rag_state
                .search_with_embedding(&query_embedding, limit)
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
                    return Ok(format!(
                        "Semantic search found {} result(s):\n\n{}",
                        formatted_results.len(),
                        formatted_results.join("\n\n")
                    ));
                }
            }
        }

        let full_docs = self.full_docs.read().await;

        if let Some(docs) = full_docs.as_ref() {
            if let Some(docs_array) = docs["docs"].as_array() {
                let query_lower = query.to_lowercase();
                let mut results = Vec::new();

                for doc in docs_array.iter() {
                    if let Some(name) = doc["name"].as_str() {
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

                            if results.len() >= limit {
                                break;
                            }
                        }
                    }
                }

                if !results.is_empty() {
                    return Ok(format!(
                        "Found {} function(s):\n\n{}",
                        results.len(),
                        results.join("\n\n")
                    ));
                }
            }
        }

        Ok(format!("No functions found matching '{}'", query))
    }

    pub async fn list_available_sounds(
        &self,
        sound_type: String,
        filter: Option<String>,
    ) -> AnyResult<String> {
        self.check_rate_limit("list_available_sounds").await?;

        if sound_type.len() > 50 {
            return Err(anyhow!("Sound type parameter too long"));
        }
        if let Some(ref f) = filter {
            if f.len() > 100 {
                return Err(anyhow!("Filter parameter too long"));
            }
        }

        let sounds = match sound_type.to_lowercase().as_str() {
            "samples" => vec![
                "bd", "sd", "hh", "cp", "mt", "arpy", "feel", "sn", "perc", "tabla", "tok",
                "emsoft", "dist", "crow", "metal", "pebbles", "bottle", "drum", "glitch", "bass",
                "lighter", "can", "hand", "outdoor", "coin", "birds", "wind",
            ],
            "synths" => vec!["sine", "saw", "square", "triangle", "sawtooth"],
            "gm" => vec![
                "gm_piano",
                "gm_epiano1",
                "gm_epiano2",
                "gm_harpsichord",
                "gm_acoustic_guitar_nylon",
                "gm_acoustic_guitar_steel",
                "gm_electric_guitar_jazz",
                "gm_acoustic_bass",
                "gm_electric_bass_finger",
                "gm_electric_bass_pick",
                "gm_violin",
                "gm_viola",
                "gm_cello",
                "gm_contrabass",
                "gm_trumpet",
                "gm_trombone",
                "gm_tuba",
                "gm_french_horn",
                "gm_soprano_sax",
                "gm_alto_sax",
                "gm_tenor_sax",
                "gm_baritone_sax",
                "gm_flute",
                "gm_piccolo",
                "gm_clarinet",
                "gm_oboe",
                "gm_choir_aahs",
                "gm_voice_oohs",
                "gm_synth_voice",
                "gm_lead_1_square",
                "gm_lead_2_sawtooth",
                "gm_pad_1_new_age",
                "gm_marimba",
                "gm_xylophone",
                "gm_vibraphone",
                "gm_sitar",
                "gm_banjo",
                "gm_shamisen",
                "gm_koto",
            ],
            _ => {
                return Ok(format!(
                    "Unknown sound type '{}'. Use: samples, synths, or gm",
                    sound_type
                ))
            }
        };

        let filtered: Vec<_> = if let Some(f) = filter {
            let f_lower = f.to_lowercase();
            sounds
                .into_iter()
                .filter(|s| s.to_lowercase().contains(&f_lower))
                .collect()
        } else {
            sounds
        };

        if filtered.is_empty() {
            Ok(format!("No {} sounds found matching filter", sound_type))
        } else {
            Ok(format!(
                "Available {} sounds: {}",
                sound_type,
                filtered.join(", ")
            ))
        }
    }

    pub fn live_edit_enabled(&self) -> bool {
        self.live_edit_enabled
    }

    pub async fn apply_live_edit(&self, mode: LiveEditMode, code: String) -> AnyResult<String> {
        if !self.live_edit_enabled {
            return Ok(
                "Live edits are disabled. Ask the user to enable them in Chat settings.".into(),
            );
        }

        if code.trim().is_empty() {
            return Err(anyhow!("Refusing to apply an empty code edit"));
        }

        if code.len() > 50_000 {
            return Err(anyhow!("Edit too large (max 50k characters)"));
        }

        let payload = LiveEditPayload {
            mode: mode.clone(),
            code: code.clone(),
        };

        self.app_handle
            .emit_to(&self.window_label, "chat-live-edit", payload)
            .map_err(|e| anyhow!("Failed to emit live edit event: {}", e))?;

        Ok(format!(
            "Applied {} live edit ({} chars)",
            mode,
            code.chars().count()
        ))
    }

    pub async fn queue_live_edit(
        &self,
        mode: LiveEditMode,
        code: String,
        description: String,
        wait_cycles: u32,
    ) -> AnyResult<String> {
        if !self.live_edit_enabled {
            return Ok(
                "Live edits are disabled. Ask the user to enable them in Chat settings.".into(),
            );
        }

        if code.trim().is_empty() {
            return Err(anyhow!("Refusing to queue an empty code edit"));
        }

        if code.len() > 50_000 {
            return Err(anyhow!("Edit too large (max 50k characters)"));
        }

        let payload = QueuedEditPayload {
            mode: mode.clone(),
            code: code.clone(),
            description: description.clone(),
            wait_cycles,
        };

        self.app_handle
            .emit_to(&self.window_label, "chat-queue-edit", payload)
            .map_err(|e| anyhow!("Failed to emit queue edit event: {}", e))?;

        Ok(format!(
            "Queued: {} ({} chars, wait {} cycles)",
            description,
            code.chars().count(),
            wait_cycles
        ))
    }

    pub async fn generate_chord_progression(
        &self,
        key: String,
        style: String,
    ) -> AnyResult<String> {
        self.check_rate_limit("generate_chord_progression").await?;

        if key.len() > 10 || style.len() > 50 {
            return Err(anyhow!("Parameter too long"));
        }

        let progression =
            MusicTheory::generate_chord_progression(&key, &style).map_err(|e| anyhow!("{}", e))?;

        Ok(format!(
            "{} progression in {}: {}\n\nStrudel pattern:\n```javascript\nnote(\"{}\").struct(\"1 ~ ~ ~\")\n```",
            style, key, progression, progression
        ))
    }

    pub async fn generate_euclidean_rhythm(
        &self,
        hits: usize,
        steps: usize,
        sound: Option<String>,
    ) -> AnyResult<String> {
        self.check_rate_limit("generate_euclidean_rhythm").await?;

        if hits > 32 || steps > 64 {
            return Err(anyhow!("Parameters too large (max 32 hits, 64 steps)"));
        }

        let sound_name = sound.unwrap_or_else(|| "bd".to_string());
        if sound_name.len() > 50 {
            return Err(anyhow!("Sound name too long"));
        }

        let pattern = MusicTheory::generate_euclidean_pattern(hits, steps, &sound_name)
            .map_err(|e| anyhow!("{}", e))?;

        let rhythm =
            MusicTheory::generate_euclidean_rhythm(hits, steps).map_err(|e| anyhow!("{}", e))?;

        Ok(format!(
            "Euclidean rhythm: {} hits in {} steps\nPattern: {}\n\nStrudel pattern:\n```javascript\n{}\n```",
            hits, steps, rhythm, pattern
        ))
    }
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ToolInvocationError(#[from] anyhow::Error);

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveEditMode {
    Append,
    Replace,
}

impl std::fmt::Display for LiveEditMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiveEditMode::Append => write!(f, "append"),
            LiveEditMode::Replace => write!(f, "replace"),
        }
    }
}

impl LiveEditMode {
    pub fn parse(mode: &str) -> Option<Self> {
        match mode.to_lowercase().as_str() {
            "append" => Some(LiveEditMode::Append),
            "replace" => Some(LiveEditMode::Replace),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize)]
struct LiveEditPayload {
    mode: LiveEditMode,
    code: String,
}

#[derive(Clone, Serialize)]
struct QueuedEditPayload {
    mode: LiveEditMode,
    code: String,
    description: String,
    wait_cycles: u32,
}

#[derive(Clone)]
pub struct RigSearchDocsTool {
    ctx: ToolRuntimeContext,
}

impl RigSearchDocsTool {
    pub fn new(ctx: ToolRuntimeContext) -> Self {
        Self { ctx }
    }
}

#[derive(Clone, Deserialize)]
pub struct RigSearchDocsArgs {
    query: String,
    limit: Option<usize>,
}

impl RigTool for RigSearchDocsTool {
    const NAME: &'static str = "search_strudel_docs";

    type Error = ToolInvocationError;
    type Args = RigSearchDocsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the Strudel documentation for functions, effects, and helpers."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Function or keyword to search for."
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results to return (default 5)."
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.ctx
            .search_strudel_docs(args.query, args.limit)
            .await
            .map_err(ToolInvocationError::from)
    }
}

#[derive(Clone)]
pub struct RigListSoundsTool {
    ctx: ToolRuntimeContext,
}

impl RigListSoundsTool {
    pub fn new(ctx: ToolRuntimeContext) -> Self {
        Self { ctx }
    }
}

#[derive(Clone, Deserialize)]
pub struct RigListSoundsArgs {
    sound_type: String,
    filter: Option<String>,
}

impl RigTool for RigListSoundsTool {
    const NAME: &'static str = "list_available_sounds";

    type Error = ToolInvocationError;
    type Args = RigListSoundsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: Self::NAME.to_string(),
            description: "List available Strudel samples, synths, or GM instruments.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "sound_type": {
                        "type": "string",
                        "description": "One of: samples, synths, gm."
                    },
                    "filter": {
                        "type": "string",
                        "description": "Optional filter to narrow the list."
                    }
                },
                "required": ["sound_type"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.ctx
            .list_available_sounds(args.sound_type, args.filter)
            .await
            .map_err(ToolInvocationError::from)
    }
}

#[derive(Clone)]
pub struct RigApplyLiveEditTool {
    ctx: ToolRuntimeContext,
}

impl RigApplyLiveEditTool {
    pub fn new(ctx: ToolRuntimeContext) -> Self {
        Self { ctx }
    }
}

#[derive(Clone, Deserialize)]
pub struct RigLiveEditArgs {
    mode: LiveEditMode,
    code: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    wait_cycles: Option<u32>,
}

impl RigTool for RigApplyLiveEditTool {
    const NAME: &'static str = "apply_live_code_edit";

    type Error = ToolInvocationError;
    type Args = RigLiveEditArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: Self::NAME.to_string(),
            description: "Apply code edits to the user's Strudel document. In Queue Mode (🎬), use wait_cycles to stage changes progressively. Call this tool multiple times to queue several changes."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["append", "replace"],
                        "description": "Whether to append or replace the user's code."
                    },
                    "code": {
                        "type": "string",
                        "description": "The Strudel code to write into the editor."
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description of what this change does (shown in queue UI). Example: 'Add kick drum pattern'"
                    },
                    "wait_cycles": {
                        "type": "integer",
                        "description": "Optional: Number of cycles to wait before auto-applying this change (0 = immediate, 4 = wait 4 cycles). Only used in Queue Mode."
                    }
                },
                "required": ["mode", "code"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // If description and wait_cycles are provided, queue the change
        // Otherwise apply directly (legacy behavior)
        if args.description.is_some() || args.wait_cycles.is_some() {
            self.ctx
                .queue_live_edit(
                    args.mode,
                    args.code,
                    args.description
                        .unwrap_or_else(|| "Pattern change".to_string()),
                    args.wait_cycles.unwrap_or(0),
                )
                .await
                .map_err(ToolInvocationError::from)
        } else {
            self.ctx
                .apply_live_edit(args.mode, args.code)
                .await
                .map_err(ToolInvocationError::from)
        }
    }
}

#[derive(Clone)]
pub struct RigChordProgressionTool {
    ctx: ToolRuntimeContext,
}

impl RigChordProgressionTool {
    pub fn new(ctx: ToolRuntimeContext) -> Self {
        Self { ctx }
    }
}

#[derive(Clone, Deserialize)]
pub struct RigChordProgressionArgs {
    key: String,
    style: String,
}

impl RigTool for RigChordProgressionTool {
    const NAME: &'static str = "generate_chord_progression";

    type Error = ToolInvocationError;
    type Args = RigChordProgressionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: Self::NAME.to_string(),
            description: "Generate a chord progression for a given key and musical style."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "Musical key (e.g., C, D, F#)"
                    },
                    "style": {
                        "type": "string",
                        "enum": ["pop", "jazz", "blues", "folk", "rock", "classical", "modal", "edm"],
                        "description": "Progression style"
                    }
                },
                "required": ["key", "style"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.ctx
            .generate_chord_progression(args.key, args.style)
            .await
            .map_err(ToolInvocationError::from)
    }
}

#[derive(Clone)]
pub struct RigEuclideanRhythmTool {
    ctx: ToolRuntimeContext,
}

impl RigEuclideanRhythmTool {
    pub fn new(ctx: ToolRuntimeContext) -> Self {
        Self { ctx }
    }
}

#[derive(Clone, Deserialize)]
pub struct RigEuclideanRhythmArgs {
    hits: usize,
    steps: usize,
    sound: Option<String>,
}

impl RigTool for RigEuclideanRhythmTool {
    const NAME: &'static str = "generate_euclidean_rhythm";

    type Error = ToolInvocationError;
    type Args = RigEuclideanRhythmArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: Self::NAME.to_string(),
            description: "Generate a Euclidean rhythm pattern with evenly distributed hits."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "hits": {
                        "type": "number",
                        "description": "Number of hits to distribute"
                    },
                    "steps": {
                        "type": "number",
                        "description": "Total number of steps"
                    },
                    "sound": {
                        "type": "string",
                        "description": "Sound to use (default: bd)"
                    }
                },
                "required": ["hits", "steps"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.ctx
            .generate_euclidean_rhythm(args.hits, args.steps, args.sound)
            .await
            .map_err(ToolInvocationError::from)
    }
}
