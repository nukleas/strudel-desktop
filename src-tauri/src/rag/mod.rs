// RAG (Retrieval-Augmented Generation) module for Strudel
// Provides semantic search over documentation and examples using pre-computed embeddings

pub mod commands;
pub mod embeddings;
pub mod generator;
pub mod retriever;
pub mod types;

// Re-export Tauri commands
pub use commands::*;
pub use embeddings::{EmbeddingEntry, VectorStore};
pub use retriever::StrudelRetriever;
pub use types::{ChunkType, EmbeddingChunk, SearchResult};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main RAG state for the application
/// Uses pre-computed embeddings loaded from JSON at runtime
#[derive(Clone)]
pub struct RagState {
    pub retriever: Arc<RwLock<Option<StrudelRetriever>>>,
}

impl RagState {
    pub fn new() -> Self {
        Self {
            retriever: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize RAG system from pre-computed embeddings JSON
    /// Expected format: { "embeddings": [...], "chunks": [...], "vocabulary": [...], "idf_scores": {...} }
    pub async fn load_from_json(&self, json_data: &str) -> Result<()> {
        use std::collections::HashMap;

        #[derive(serde::Deserialize)]
        struct EmbeddingsData {
            embeddings: Vec<EmbeddingEntry>,
            chunks: Vec<EmbeddingChunk>,
            vocabulary: Vec<String>,
            idf_scores: HashMap<String, f32>,
        }

        let data: EmbeddingsData = serde_json::from_str(json_data)?;

        let vector_store = VectorStore::new(data.embeddings, data.vocabulary, data.idf_scores)?;
        let retriever = StrudelRetriever::new(vector_store, data.chunks)?;

        *self.retriever.write().await = Some(retriever);

        Ok(())
    }

    /// Embed a query text using TF-IDF
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let retriever_lock = self.retriever.read().await;
        match retriever_lock.as_ref() {
            Some(retriever) => Ok(retriever.embed_query(query)),
            None => Err(anyhow::anyhow!("RAG not initialized")),
        }
    }

    /// Search using a pre-computed query embedding
    /// For offline use, query embeddings must be computed externally
    pub async fn search_with_embedding(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let retriever_lock = self.retriever.read().await;

        match retriever_lock.as_ref() {
            Some(retriever) => retriever.search(query_embedding, limit),
            None => Err(anyhow::anyhow!("RAG not initialized")),
        }
    }

    /// Get the number of indexed documents
    pub async fn count(&self) -> Result<usize> {
        let retriever_lock = self.retriever.read().await;
        match retriever_lock.as_ref() {
            Some(retriever) => Ok(retriever.count()),
            None => Ok(0),
        }
    }
}

impl Default for RagState {
    fn default() -> Self {
        Self::new()
    }
}
