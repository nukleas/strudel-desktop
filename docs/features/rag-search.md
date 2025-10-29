# RAG (Retrieval-Augmented Generation) for Strudel Desktop

## 🎯 Overview

This module provides **offline semantic search** over Strudel documentation using:
- **Pure Rust** TF-IDF embeddings (no ML dependencies!)
- **In-memory vector search** with cosine similarity
- **Pre-computed embeddings** that ship with the app
- **100% offline** search capability

## 📦 Architecture

```
src-tauri/src/rag/
├── embeddings.rs   - VectorStore & cosine similarity
├── retriever.rs    - StrudelRetriever for search
├── generator.rs    - TF-IDF embedding generation
├── types.rs        - Data structures
├── commands.rs     - Tauri commands
└── mod.rs          - Module exports
```

## 🔧 Generating Embeddings

### Step 1: Get Strudel Documentation JSON

The Strudel docs are in `packages/core/` as `doc.json`. You need this format:

```json
{
  "docs": [
    {
      "name": "scale",
      "description": "<p>Apply musical scale</p>",
      "signature": ".scale(name)",
      "category": "pitch",
      "tags": ["melody", "harmony"]
    }
  ],
  "examples": [
    {
      "name": "Techno Bass",
      "code": "note(\"c2 [~ c3] c2 c3\").s(\"sawtooth\")",
      "description": "Four-on-the-floor bass pattern",
      "style": "techno"
    }
  ]
}
```

### Step 2: Generate Embeddings

```bash
cd src-tauri

# Generate embeddings (384 dimensions by default)
cargo run --bin generate_embeddings -- \
  ../../packages/core/doc.json \
  embeddings.json \
  384

# Output:
# 📚 Reading Strudel documentation from: ../../packages/core/doc.json
# 🔧 Generating embeddings with dimension: 384
# Building TF-IDF vocabulary from 150 documents...
# Generating embeddings...
# ✅ Generated 150 embeddings
# 💾 Saving to: embeddings.json
# 🎉 Done! Embeddings saved to embeddings.json
```

### Step 3: Bundle with Tauri App

Add to `tauri.conf.json`:

```json
{
  "tauri": {
    "bundle": {
      "resources": [
        "embeddings.json"
      ]
    }
  }
}
```

## 🚀 Usage in App

### Initialize RAG

```javascript
import { invoke } from '@tauri-apps/api/tauri';
import embeddingsData from './embeddings.json';

// Load embeddings on app startup
await invoke('init_rag', {
  embeddingsJson: JSON.stringify(embeddingsData)
});
```

### Search

```javascript
// Note: For offline search, you need pre-computed query embeddings
// OR use the same TF-IDF approach to embed queries at runtime

const results = await invoke('semantic_search', {
  request: {
    query_embedding: queryVector,  // 384-dim vector
    limit: 5
  }
});

// Results format:
// {
//   count: 5,
//   results: [
//     {
//       chunk: {
//         id: "func_0",
//         chunk_type: "function",
//         content: "scale Apply musical scale",
//         metadata: { name: "scale", ... }
//       },
//       score: 0.95,
//       distance: 0.05
//     }
//   ]
// }
```

### Check Status

```javascript
const status = await invoke('rag_status');
// { initialized: true, document_count: 150 }
```

## 🔄 Query Embedding Options

Since we're using TF-IDF (not neural embeddings), you have two options for query embeddings:

### Option A: Pre-compute Common Queries (Recommended)

Generate embeddings for common queries at build time:

```rust
// In generator.rs, add common queries
let common_queries = vec![
    "create drum pattern",
    "add reverb",
    "make melody",
    // ... etc
];
```

### Option B: Runtime TF-IDF Embedding (Future)

Add a Tauri command to embed queries at runtime using the same TF-IDF vocabulary:

```javascript
const queryVector = await invoke('embed_query', { text: userQuery });
const results = await invoke('semantic_search', {
  request: { query_embedding: queryVector, limit: 5 }
});
```

### Option C: API-based Embeddings (Hybrid)

For online users, use OpenAI/Anthropic API to embed queries:

```javascript
const embedding = await fetch('https://api.openai.com/v1/embeddings', {
  method: 'POST',
  headers: { 'Authorization': `Bearer ${apiKey}` },
  body: JSON.stringify({
    model: 'text-embedding-3-small',
    input: userQuery
  })
});
```

**Note:** This requires the document embeddings to also use the same API model.

## 📊 TF-IDF vs Neural Embeddings

| Feature | TF-IDF (Current) | Neural (Optional Upgrade) |
|---------|------------------|---------------------------|
| **Quality** | Good for keyword search | Excellent for semantic understanding |
| **Speed** | Very fast | Fast (after pre-compute) |
| **Size** | ~1MB for 150 docs | ~5MB for 150 docs |
| **Dependencies** | Zero! | Requires API or heavy libs |
| **Offline** | 100% | 100% (if pre-computed) |

## 🔮 Future Enhancements

1. **Runtime Query Embedding** - Add TF-IDF embedding for user queries
2. **Hybrid Search** - Combine keyword + semantic search
3. **Re-ranking** - Score by relevance + recency + popularity
4. **User Pattern Indexing** - Learn from user's successful patterns
5. **API Upgrade Path** - Optional switch to OpenAI embeddings for better quality

## 🧪 Testing

```bash
# Run tests
cargo test --lib rag

# Test embedding generation with sample docs
cargo run --bin generate_embeddings -- \
  test_docs.json \
  test_embeddings.json \
  100
```

## 📝 Technical Details

- **Algorithm**: TF-IDF (Term Frequency-Inverse Document Frequency)
- **Dimension**: 384 (configurable, matches BGE-small for future compat)
- **Search**: Cosine similarity (dot product of normalized vectors)
- **Storage**: JSON format (~10KB per 10 documents)
- **Memory**: All embeddings loaded at runtime (~5-10MB)

## 🤝 Contributing

To improve search quality without adding dependencies:

1. **Better tokenization** - Add stemming, stop words
2. **N-grams** - Include bigrams/trigrams
3. **BM25** - Upgrade from TF-IDF to BM25 algorithm
4. **Metadata boost** - Weight matches in function names higher

Or, for best quality, switch to API-based embeddings (see Option C above).
