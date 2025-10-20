// Strudel AI Chat Tools
// Tools that the AI agent can use to interact with documentation and code

use agentai::tool::{toolbox, Tool, ToolBox, ToolError, ToolResult};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use crate::chatbridge::RateLimiter;
use anyhow::anyhow;

/// StrudelToolBox provides tools for the AI agent to:
/// - Search documentation on-demand
/// - Validate code before responding
/// - Access user's current code
/// - Find relevant examples
/// - List available sounds
#[derive(Clone)]
pub struct StrudelToolBox {
    pub full_docs: Arc<RwLock<Option<serde_json::Value>>>,
    pub examples: Arc<RwLock<Option<String>>>,
    pub code_context: Arc<RwLock<Option<String>>>,
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
}

#[toolbox]
impl StrudelToolBox {
    #[tool]
    /// Search the Strudel documentation for functions matching the query
    async fn search_strudel_docs(
        &self,
        /// The function name or keyword to search for (e.g., "scale", "delay", "euclid")
        query: String,
        /// Optional: Maximum number of results to return (default: 5)
        limit: Option<usize>,
    ) -> ToolResult {
        // Rate limiting check
        {
            let mut rate_limiter = self.rate_limiter.lock().await;
            rate_limiter
                .check_and_increment("search_strudel_docs")
                .map_err(|e| ToolError::from(anyhow!(e)))?;
        }

        // Input validation
        if query.len() > 500 {
            return Err(ToolError::from(anyhow!("Query too long (max 500 characters)")));
        }

        let limit = limit.unwrap_or(5).min(10); // Cap at 10 results

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

    // // TEMP DISABLED: Testing which tool causes Anthropic schema error
    // // #[tool]
    // // /// Get the current Strudel code from the user's editor
    // // async fn get_user_code(&self) -> ToolResult {
    // //     let code_context = self.code_context.read().await;

    // //     if let Some(code) = code_context.as_ref() {
    // //         Ok(format!("Current user code:\n```javascript\n{}\n```", code))
    // //     } else {
    // //         Ok("No code context available (user's editor may be empty)".to_string())
    // //     }
    // // }

    // // TEMP DISABLED: Testing which tool causes Anthropic schema error
    // // #[tool]
    // // /// Get Strudel example patterns filtered by style or genre
    // // async fn get_strudel_examples(
    // //     &self,
    //     /// Optional: Filter by style/genre (e.g., "techno", "jazz", "ambient")
    // //     style: Option<String>,
    // //     /// Optional: Maximum number of examples to return (default: 3)
    // //     limit: Option<usize>,
    // // ) -> ToolResult {
    //     let limit = limit.unwrap_or(3);
    //     let examples = self.examples.read().await;

    //     if let Some(ex_text) = examples.as_ref() {
    //         // If style filter provided, try to find matching examples
    //         if let Some(style_filter) = style {
    //             let style_lower = style_filter.to_lowercase();
    //             let lines: Vec<&str> = ex_text.lines().collect();
    //             let mut filtered_examples = Vec::new();
    //             let mut current_example = Vec::new();
    //             let mut matches_style = false;

    //             for line in lines {
    //                 if line.ends_with(':') && !line.starts_with("```") {
    //                     // New example title
    //                     if matches_style && !current_example.is_empty() {
    //                         filtered_examples.push(current_example.join("\n"));
    //                         if filtered_examples.len() >= limit {
    //                             break;
    //                         }
    //                     }
    //                     current_example.clear();
    //                     matches_style = line.to_lowercase().contains(&style_lower);
    //                 }

    //                 if matches_style || current_example.is_empty() {
    //                     current_example.push(line);
    //                 }
    //             }

    //             // Add last example if it matches
    //             if matches_style && !current_example.is_empty() && filtered_examples.len() < limit {
    //                 filtered_examples.push(current_example.join("\n"));
    //             }

    //             if !filtered_examples.is_empty() {
    //                 return Ok(filtered_examples.join("\n\n"));
    //             }
    //         }

    //         // No filter or no matches - return first N examples
    //         let lines: Vec<&str> = ex_text.lines().collect();
    //         let mut examples_found = 0;
    //         let mut result = Vec::new();

    //         for line in lines {
    //             result.push(line);
    //             if line.ends_with(':') && !line.starts_with("```") {
    //                 examples_found += 1;
    //                 if examples_found >= limit {
    //                     break;
    //                 }
    //             }
    //         }

    //         Ok(result.join("\n"))
    //     } else {
    //         Ok("No examples loaded".to_string())
    //     }
    // }

    #[tool]
    /// List available sound sources (samples, synths, or GM instruments)
    async fn list_available_sounds(
        &self,
        /// Type of sounds: "samples", "synths", or "gm"
        sound_type: String,
        /// Optional: Filter by name prefix (e.g., "gm_piano" would match "gm_piano*")
        filter: Option<String>,
    ) -> ToolResult {
        // Rate limiting check
        {
            let mut rate_limiter = self.rate_limiter.lock().await;
            rate_limiter
                .check_and_increment("list_available_sounds")
                .map_err(|e| ToolError::from(anyhow!(e)))?;
        }

        // Input validation
        if sound_type.len() > 50 {
            return Err(ToolError::from(anyhow!("Sound type parameter too long")));
        }
        if let Some(ref f) = filter {
            if f.len() > 100 {
                return Err(ToolError::from(anyhow!("Filter parameter too long")));
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
}
