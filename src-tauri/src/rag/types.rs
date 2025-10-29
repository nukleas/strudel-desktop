// Type definitions for RAG system

use serde::{Deserialize, Serialize};

/// Type of content chunk for embedding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChunkType {
    Function,    // Strudel function documentation
    Example,     // Code example pattern
    UserPattern, // User's saved pattern
    Error,       // Error solution
}

/// A chunk of content to be embedded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingChunk {
    pub id: String,
    pub chunk_type: ChunkType,
    pub content: String,
    pub metadata: ChunkMetadata,
}

/// Metadata associated with a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    // Function metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    // Example metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    // Common metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Result from semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: EmbeddingChunk,
    pub score: f32,
    pub distance: f32,
}

impl SearchResult {
    pub fn new(chunk: EmbeddingChunk, score: f32, distance: f32) -> Self {
        Self {
            chunk,
            score,
            distance,
        }
    }
}
