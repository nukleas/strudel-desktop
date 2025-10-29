// In-memory vector store with pre-computed embeddings
// No ML dependencies - embeddings are generated at build time and shipped with the app

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Pre-computed embedding entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    pub id: String,
    pub vector: Vec<f32>,
}

/// Simple in-memory vector store with TF-IDF query embedding capability
#[derive(Debug, Clone)]
pub struct VectorStore {
    embeddings: Vec<EmbeddingEntry>,
    dimension: usize,
    vocabulary: Vec<String>,
    idf_scores: std::collections::HashMap<String, f32>,
}

impl VectorStore {
    /// Create a new vector store from pre-computed embeddings with vocabulary
    pub fn new(
        embeddings: Vec<EmbeddingEntry>,
        vocabulary: Vec<String>,
        idf_scores: std::collections::HashMap<String, f32>,
    ) -> Result<Self> {
        let dimension = embeddings
            .first()
            .map(|e| e.vector.len())
            .ok_or_else(|| anyhow::anyhow!("No embeddings provided"))?;

        // Validate all embeddings have the same dimension
        for entry in &embeddings {
            if entry.vector.len() != dimension {
                return Err(anyhow::anyhow!(
                    "Inconsistent embedding dimensions: expected {}, got {}",
                    dimension,
                    entry.vector.len()
                ));
            }
        }

        // Validate vocabulary matches dimension
        if vocabulary.len() != dimension {
            return Err(anyhow::anyhow!(
                "Vocabulary size ({}) doesn't match dimension ({})",
                vocabulary.len(),
                dimension
            ));
        }

        Ok(Self {
            embeddings,
            dimension,
            vocabulary,
            idf_scores,
        })
    }

    /// Embed a query text using TF-IDF (same as document embeddings)
    pub fn embed_query(&self, text: &str) -> Vec<f32> {
        let words: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        // Count term frequency
        let mut term_freq: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for word in &words {
            *term_freq.entry(word.clone()).or_insert(0) += 1;
        }

        let total_terms = words.len() as f32;

        // Build TF-IDF vector
        let mut vector = vec![0.0; self.dimension];
        for (idx, vocab_word) in self.vocabulary.iter().enumerate() {
            if let Some(&count) = term_freq.get(vocab_word) {
                let tf = count as f32 / total_terms;
                let idf = self.idf_scores.get(vocab_word).unwrap_or(&1.0);
                vector[idx] = tf * idf;
            }
        }

        // Normalize vector
        let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut vector {
                *val /= magnitude;
            }
        }

        vector
    }

    /// Search for the k most similar embeddings using cosine similarity
    pub fn search(&self, query_vector: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        if query_vector.len() != self.dimension {
            return Err(anyhow::anyhow!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.dimension,
                query_vector.len()
            ));
        }

        let mut similarities: Vec<(String, f32)> = self
            .embeddings
            .iter()
            .map(|entry| {
                let similarity = cosine_similarity(&entry.vector, query_vector);
                (entry.id.clone(), similarity)
            })
            .collect();

        // Sort by similarity (highest first)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k results
        Ok(similarities.into_iter().take(k).collect())
    }

    /// Get the number of embeddings in the store
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Calculate cosine similarity between two vectors
/// Returns a value between -1 and 1, where 1 means identical direction
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 1.0);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert_eq!(cosine_similarity(&c, &d), 0.0);

        let e = vec![1.0, 0.0, 0.0];
        let f = vec![-1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&e, &f), -1.0);
    }

    #[test]
    fn test_vector_store() {
        let embeddings = vec![
            EmbeddingEntry {
                id: "doc1".to_string(),
                vector: vec![1.0, 0.0, 0.0],
            },
            EmbeddingEntry {
                id: "doc2".to_string(),
                vector: vec![0.9, 0.1, 0.0],
            },
            EmbeddingEntry {
                id: "doc3".to_string(),
                vector: vec![0.0, 1.0, 0.0],
            },
        ];

        let vocabulary = vec![
            "word1".to_string(),
            "word2".to_string(),
            "word3".to_string(),
        ];
        let mut idf_scores = std::collections::HashMap::new();
        idf_scores.insert("word1".to_string(), 1.0);
        idf_scores.insert("word2".to_string(), 1.5);
        idf_scores.insert("word3".to_string(), 2.0);

        let store = VectorStore::new(embeddings, vocabulary, idf_scores).unwrap();
        assert_eq!(store.len(), 3);
        assert_eq!(store.dimension(), 3);

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1"); // Most similar
        assert!(results[0].1 > results[1].1); // Similarity scores decrease
    }

    #[test]
    fn test_embed_query() {
        let embeddings = vec![EmbeddingEntry {
            id: "doc1".to_string(),
            vector: vec![1.0, 0.0],
        }];

        let vocabulary = vec!["drum".to_string(), "pattern".to_string()];
        let mut idf_scores = std::collections::HashMap::new();
        idf_scores.insert("drum".to_string(), 2.0);
        idf_scores.insert("pattern".to_string(), 1.5);

        let store = VectorStore::new(embeddings, vocabulary, idf_scores).unwrap();

        let embedding = store.embed_query("drum pattern");
        assert_eq!(embedding.len(), 2);

        // Should be normalized
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01 || magnitude == 0.0);
    }
}
