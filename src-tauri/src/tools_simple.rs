// Simplified tools for testing - only search_docs to isolate Anthropic schema issue

use agentai::tool::{toolbox, Tool, ToolBox, ToolError, ToolResult};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct StrudelToolBox {
    pub full_docs: Arc<RwLock<Option<serde_json::Value>>>,
    pub examples: Arc<RwLock<Option<String>>>,
    pub code_context: Arc<RwLock<Option<String>>>,
}

#[toolbox]
impl StrudelToolBox {
    #[tool]
    /// Search the Strudel documentation for functions matching the query
    async fn search_strudel_docs(
        &self,
        /// The function name or keyword to search for (e.g., "scale", "delay", "euclid")
        query: String,
    ) -> ToolResult {
        let full_docs = self.full_docs.read().await;

        if let Some(docs) = full_docs.as_ref() {
            if let Some(docs_array) = docs["docs"].as_array() {
                let query_lower = query.to_lowercase();
                let mut results = Vec::new();

                for doc in docs_array.iter().take(5) {
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

                            results.push(result);

                            if results.len() >= 5 {
                                break;
                            }
                        }
                    }
                }

                if !results.is_empty() {
                    return Ok(format!("Found {} function(s):\n\n{}", results.len(), results.join("\n\n")));
                }
            }
        }

        Ok(format!("No functions found matching '{}'", query))
    }
}
