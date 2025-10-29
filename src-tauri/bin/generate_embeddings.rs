// Standalone tool to generate embeddings from Strudel documentation
// Usage: cargo run --bin generate_embeddings -- <docs.json> <output.json> [dimension]

use anyhow::Result;
use std::env;
use std::fs;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input_docs.json> <output_embeddings.json> [dimension]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  cargo run --bin generate_embeddings -- docs.json embeddings.json 384");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];
    let dimension: usize = args.get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(384); // Default to 384 (BGE-small dimension)

    println!("📚 Reading Strudel documentation from: {}", input_path);
    let docs_json = fs::read_to_string(input_path)?;

    println!("🔧 Generating embeddings with dimension: {}", dimension);
    let (embeddings, chunks, embedder) = app_lib::rag::generator::generate_from_strudel_docs(&docs_json, dimension)?;

    println!("✅ Generated {} embeddings", embeddings.len());
    println!("📝 Vocabulary size: {}", embedder.vocabulary.len());

    println!("💾 Saving to: {}", output_path);
    app_lib::rag::generator::save_to_json(&embeddings, &chunks, &embedder, output_path)?;

    println!("🎉 Done! Embeddings saved to {}", output_path);
    println!("   - Documents: {}", chunks.len());
    println!("   - Dimension: {}", dimension);
    println!("   - File size: ~{} KB", fs::metadata(output_path)?.len() / 1024);

    Ok(())
}
