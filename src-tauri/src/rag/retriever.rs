// In-memory retriever for semantic search over Strudel documentation

use super::embeddings::VectorStore;
use super::types::{ChunkType, EmbeddingChunk, SearchResult};
use anyhow::Result;
use std::collections::HashMap;

/// Retriever for searching Strudel documentation using in-memory vector search
pub struct StrudelRetriever {
    vector_store: VectorStore,
    chunks: HashMap<String, EmbeddingChunk>,
}

impl StrudelRetriever {
    /// Create a new retriever from pre-computed embeddings and chunks
    pub fn new(vector_store: VectorStore, chunks: Vec<EmbeddingChunk>) -> Result<Self> {
        let expected_len = chunks.len();
        let chunks_map: HashMap<String, EmbeddingChunk> = chunks
            .into_iter()
            .map(|chunk| (chunk.id.clone(), chunk))
            .collect();

        if chunks_map.len() != expected_len {
            anyhow::bail!("Duplicate chunk IDs detected");
        }

        Ok(Self {
            vector_store,
            chunks: chunks_map,
        })
    }

    /// Search for similar chunks using vector similarity
    pub fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        // Get similar document IDs from vector store
        let similar_ids = self.vector_store.search(query_embedding, limit)?;

        // Map IDs to full SearchResults
        let results: Vec<SearchResult> = similar_ids
            .into_iter()
            .filter_map(|(id, similarity)| {
                self.chunks.get(&id).map(|chunk| {
                    SearchResult::new(
                        chunk.clone(),
                        similarity,
                        1.0 - similarity, // distance = 1 - similarity for cosine
                    )
                })
            })
            .collect();

        Ok(results)
    }

    /// Search with optional type filtering
    pub fn search_filtered(
        &self,
        query_embedding: &[f32],
        limit: usize,
        chunk_types: Option<Vec<ChunkType>>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = self.search(query_embedding, limit * 2)?; // Get more to account for filtering

        // Filter by chunk type if specified
        if let Some(types) = chunk_types {
            results.retain(|r| types.contains(&r.chunk.chunk_type));
        }

        // Trim to requested limit
        results.truncate(limit);

        Ok(results)
    }

    /// Get count of indexed chunks
    pub fn count(&self) -> usize {
        self.chunks.len()
    }

    /// Get a chunk by ID
    pub fn get_chunk(&self, id: &str) -> Option<&EmbeddingChunk> {
        self.chunks.get(id)
    }

    /// Embed a query text using the same TF-IDF as documents
    pub fn embed_query(&self, text: &str) -> Vec<f32> {
        self.vector_store.embed_query(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::embeddings::EmbeddingEntry;
    use crate::rag::types::ChunkMetadata;

    #[test]
    fn test_retriever_search() {
        // Create test embeddings
        let embeddings = vec![
            EmbeddingEntry {
                id: "chunk1".to_string(),
                vector: vec![1.0, 0.0, 0.0],
            },
            EmbeddingEntry {
                id: "chunk2".to_string(),
                vector: vec![0.9, 0.1, 0.0],
            },
            EmbeddingEntry {
                id: "chunk3".to_string(),
                vector: vec![0.0, 1.0, 0.0],
            },
        ];

        // Create test chunks
        let chunks = vec![
            EmbeddingChunk {
                id: "chunk1".to_string(),
                chunk_type: ChunkType::Function,
                content: "scale function".to_string(),
                metadata: ChunkMetadata {
                    name: Some("scale".to_string()),
                    signature: None,
                    category: None,
                    code: None,
                    style: None,
                    tags: None,
                    description: None,
                },
            },
            EmbeddingChunk {
                id: "chunk2".to_string(),
                chunk_type: ChunkType::Function,
                content: "note function".to_string(),
                metadata: ChunkMetadata {
                    name: Some("note".to_string()),
                    signature: None,
                    category: None,
                    code: None,
                    style: None,
                    tags: None,
                    description: None,
                },
            },
            EmbeddingChunk {
                id: "chunk3".to_string(),
                chunk_type: ChunkType::Example,
                content: "jazz example".to_string(),
                metadata: ChunkMetadata {
                    name: None,
                    signature: None,
                    category: None,
                    code: Some("note('c e g')".to_string()),
                    style: Some("jazz".to_string()),
                    tags: None,
                    description: None,
                },
            },
        ];

        let vocabulary = vec![
            "word1".to_string(),
            "word2".to_string(),
            "word3".to_string(),
        ];
        let mut idf_scores = std::collections::HashMap::new();
        idf_scores.insert("word1".to_string(), 1.0);

        let vector_store = VectorStore::new(embeddings, vocabulary, idf_scores).unwrap();
        let retriever = StrudelRetriever::new(vector_store, chunks).unwrap();

        assert_eq!(retriever.count(), 3);

        // Search with query similar to chunk1
        let query = vec![1.0, 0.0, 0.0];
        let results = retriever.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].chunk.id, "chunk1");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_filtered_search() {
        let embeddings = vec![
            EmbeddingEntry {
                id: "func1".to_string(),
                vector: vec![1.0, 0.0],
            },
            EmbeddingEntry {
                id: "ex1".to_string(),
                vector: vec![0.9, 0.1],
            },
        ];

        let chunks = vec![
            EmbeddingChunk {
                id: "func1".to_string(),
                chunk_type: ChunkType::Function,
                content: "test".to_string(),
                metadata: ChunkMetadata {
                    name: None,
                    signature: None,
                    category: None,
                    code: None,
                    style: None,
                    tags: None,
                    description: None,
                },
            },
            EmbeddingChunk {
                id: "ex1".to_string(),
                chunk_type: ChunkType::Example,
                content: "test".to_string(),
                metadata: ChunkMetadata {
                    name: None,
                    signature: None,
                    category: None,
                    code: None,
                    style: None,
                    tags: None,
                    description: None,
                },
            },
        ];

        let vocabulary = vec!["word1".to_string(), "word2".to_string()];
        let mut idf_scores = std::collections::HashMap::new();
        idf_scores.insert("word1".to_string(), 1.0);

        let vector_store = VectorStore::new(embeddings, vocabulary, idf_scores).unwrap();
        let retriever = StrudelRetriever::new(vector_store, chunks).unwrap();

        // Search only for functions
        let query = vec![1.0, 0.0];
        let results = retriever
            .search_filtered(&query, 10, Some(vec![ChunkType::Function]))
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk.chunk_type, ChunkType::Function);
    }
}
