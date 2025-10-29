// Tauri commands for RAG functionality

use super::{RagState, SearchResult};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticSearchRequest {
    #[serde(default)]
    pub query: Option<String>, // Text query (will be embedded)
    #[serde(default)]
    pub query_embedding: Option<Vec<f32>>, // Pre-computed embedding (alternative)
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SearchResult>,
    pub count: usize,
}

/// Initialize the RAG system from pre-computed embeddings JSON
#[tauri::command]
pub async fn init_rag(
    embeddings_json: String,
    state: State<'_, RagState>,
) -> Result<String, String> {
    // Validate size (e.g., 100MB limit)
    const MAX_JSON_SIZE: usize = 100 * 1024 * 1024;
    if embeddings_json.len() > MAX_JSON_SIZE {
        return Err(format!(
            "Embeddings JSON too large: {} bytes (max: {})",
            embeddings_json.len(),
            MAX_JSON_SIZE
        ));
    }

    state
        .load_from_json(&embeddings_json)
        .await
        .map_err(|e| format!("Failed to initialize RAG: {}", e))?;

    let count = state.count().await.unwrap_or(0);

    Ok(format!("RAG initialized with {} documents", count))
}

/// Perform semantic search using text query or pre-computed embedding
#[tauri::command]
pub async fn semantic_search(
    request: SemanticSearchRequest,
    state: State<'_, RagState>,
) -> Result<SemanticSearchResponse, String> {
    let limit = request.limit.unwrap_or(5).min(20); // Cap at 20 results

    // Determine query embedding
    let query_embedding = match (request.query, request.query_embedding) {
        (Some(text), None) => {
            // Embed text query using TF-IDF
            state
                .embed_query(&text)
                .await
                .map_err(|e| format!("Query embedding failed: {}", e))?
        }
        (None, Some(embedding)) => {
            // Use pre-computed embedding
            embedding
        }
        (Some(_), Some(_)) => {
            return Err("Provide either 'query' or 'query_embedding', not both".to_string());
        }
        (None, None) => {
            return Err("Must provide either 'query' or 'query_embedding'".to_string());
        }
    };

    let results = state
        .search_with_embedding(&query_embedding, limit)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    Ok(SemanticSearchResponse {
        count: results.len(),
        results,
    })
}

/// Get RAG system status
#[tauri::command]
pub async fn rag_status(state: State<'_, RagState>) -> Result<RagStatusResponse, String> {
    let retriever_ready = state.retriever.read().await.is_some();
    let document_count = state.count().await.unwrap_or(0);

    Ok(RagStatusResponse {
        initialized: retriever_ready,
        document_count,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagStatusResponse {
    pub initialized: bool,
    pub document_count: usize,
}
