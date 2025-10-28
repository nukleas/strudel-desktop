// Embedding generator using TF-IDF for pure Rust, offline embedding generation
// This runs at build time to create embeddings from Strudel documentation

use super::types::{ChunkMetadata, ChunkType, EmbeddingChunk};
use super::embeddings::EmbeddingEntry;
use anyhow::Result;
use std::collections::HashMap;

/// Simple TF-IDF based embedding generator
/// Not as powerful as neural embeddings, but works offline and is fast
pub struct TfIdfEmbedder {
    pub vocabulary: Vec<String>,
    pub idf_scores: HashMap<String, f32>,
    pub dimension: usize,
}

impl TfIdfEmbedder {
    /// Create a new TF-IDF embedder from a corpus of documents
    pub fn new(documents: &[String], dimension: usize) -> Result<Self> {
        let mut word_doc_count: HashMap<String, usize> = HashMap::new();
        let total_docs = documents.len() as f32;

        // Count document frequency for each word
        for doc in documents {
            let words: std::collections::HashSet<String> = doc
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
                .filter(|s| !s.is_empty() && s.len() > 2) // Filter short words
                .map(String::from)
                .collect();

            for word in words {
                *word_doc_count.entry(word).or_insert(0) += 1;
            }
        }

        // Calculate IDF scores and build vocabulary
        let mut vocab_idf: Vec<(String, f32)> = word_doc_count
            .into_iter()
            .map(|(word, doc_freq)| {
                let idf = (total_docs / doc_freq as f32).ln();
                (word, idf)
            })
            .collect();

        // Sort by IDF and take top N words for vocabulary
        vocab_idf.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        vocab_idf.truncate(dimension);

        let vocabulary: Vec<String> = vocab_idf.iter().map(|(w, _)| w.clone()).collect();
        let idf_scores: HashMap<String, f32> = vocab_idf.into_iter().collect();

        Ok(Self {
            vocabulary,
            idf_scores,
            dimension,
        })
    }

    /// Generate TF-IDF embedding for a document
    pub fn embed(&self, text: &str) -> Vec<f32> {
        let words: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        // Count term frequency
        let mut term_freq: HashMap<String, usize> = HashMap::new();
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
}

/// Generate embeddings from Strudel documentation JSON
/// Returns (embeddings, chunks, embedder) where embedder contains vocabulary for runtime queries
pub fn generate_from_strudel_docs(
    docs_json: &str,
    dimension: usize,
) -> Result<(Vec<EmbeddingEntry>, Vec<EmbeddingChunk>, TfIdfEmbedder)> {
    use serde_json::Value;

    let docs: Value = serde_json::from_str(docs_json)?;

    let mut chunks = Vec::new();
    let mut texts = Vec::new();

    // Process function documentation
    if let Some(docs_array) = docs["docs"].as_array() {
        for (idx, doc) in docs_array.iter().enumerate() {
            if let (Some(name), Some(desc)) = (doc["name"].as_str(), doc["description"].as_str()) {
                let clean_desc = desc
                    .replace("<p>", "")
                    .replace("</p>", "")
                    .replace("<code>", "")
                    .replace("</code>", "");

                let content = format!("{} {}", name, clean_desc);
                texts.push(content.clone());

                let tags = doc["tags"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let chunk = EmbeddingChunk {
                    id: format!("func_{}", idx),
                    chunk_type: ChunkType::Function,
                    content: content.clone(),
                    metadata: ChunkMetadata {
                        name: Some(name.to_string()),
                        signature: doc["signature"].as_str().map(|s| s.to_string()),
                        category: doc["category"].as_str().map(|s| s.to_string()),
                        description: Some(clean_desc),
                        tags: Some(tags),
                        code: None,
                        style: None,
                    },
                };

                chunks.push(chunk);
            }
        }
    }

    // Process examples if available
    if let Some(examples_array) = docs["examples"].as_array() {
        for (idx, example) in examples_array.iter().enumerate() {
            if let (Some(name), Some(code)) = (example["name"].as_str(), example["code"].as_str()) {
                let description = example["description"].as_str().unwrap_or("");
                let content = format!("{} {} {}", name, description, code);
                texts.push(content.clone());

                let chunk = EmbeddingChunk {
                    id: format!("ex_{}", idx),
                    chunk_type: ChunkType::Example,
                    content,
                    metadata: ChunkMetadata {
                        name: Some(name.to_string()),
                        signature: None,
                        category: None,
                        description: Some(description.to_string()),
                        tags: None,
                        code: Some(code.to_string()),
                        style: example["style"].as_str().map(|s| s.to_string()),
                    },
                };

                chunks.push(chunk);
            }
        }
    }

    // Build TF-IDF embedder from corpus
    println!("Building TF-IDF vocabulary from {} documents...", texts.len());
    let embedder = TfIdfEmbedder::new(&texts, dimension)?;

    // Generate embeddings
    println!("Generating embeddings...");
    let embeddings: Vec<EmbeddingEntry> = chunks
        .iter()
        .map(|chunk| {
            let vector = embedder.embed(&chunk.content);
            EmbeddingEntry {
                id: chunk.id.clone(),
                vector,
            }
        })
        .collect();

    Ok((embeddings, chunks, embedder))
}

/// Save embeddings, chunks, and vocabulary to JSON file
pub fn save_to_json(
    embeddings: &[EmbeddingEntry],
    chunks: &[EmbeddingChunk],
    embedder: &TfIdfEmbedder,
    output_path: &str,
) -> Result<()> {
    use serde::Serialize;
    use std::fs::File;
    use std::io::Write;

    #[derive(Serialize)]
    struct EmbeddingsData {
        embeddings: Vec<EmbeddingEntry>,
        chunks: Vec<EmbeddingChunk>,
        vocabulary: Vec<String>,
        idf_scores: HashMap<String, f32>,
        dimension: usize,
        count: usize,
    }

    let data = EmbeddingsData {
        dimension: embeddings.first().map(|e| e.vector.len()).unwrap_or(0),
        count: embeddings.len(),
        embeddings: embeddings.to_vec(),
        chunks: chunks.to_vec(),
        vocabulary: embedder.vocabulary.clone(),
        idf_scores: embedder.idf_scores.clone(),
    };

    let json = serde_json::to_string_pretty(&data)?;
    let mut file = File::create(output_path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfidf_embedder() {
        let docs = vec![
            "create a drum pattern with bass".to_string(),
            "make a melody using scale".to_string(),
            "add reverb and delay effects".to_string(),
        ];

        let embedder = TfIdfEmbedder::new(&docs, 10).unwrap();

        let emb1 = embedder.embed("drum pattern");
        let emb2 = embedder.embed("melody scale");

        assert_eq!(emb1.len(), 10);
        assert_eq!(emb2.len(), 10);

        // Embeddings should be normalized
        let mag: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 0.01 || mag == 0.0);
    }

    #[test]
    fn test_generate_from_docs() {
        let docs_json = r#"{
            "docs": [
                {
                    "name": "scale",
                    "description": "<p>Apply musical scale</p>",
                    "signature": ".scale(name)",
                    "category": "pitch",
                    "tags": ["melody", "harmony"]
                }
            ]
        }"#;

        let (embeddings, chunks, _embedder) = generate_from_strudel_docs(docs_json, 50).unwrap();

        assert_eq!(embeddings.len(), 1);
        assert_eq!(chunks.len(), 1);
        assert_eq!(embeddings[0].id, "func_0");
        assert_eq!(chunks[0].chunk_type, ChunkType::Function);
    }
}
