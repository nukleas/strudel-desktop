// Proof of Concept tests for agentai toolbox struct pattern
// These tests validate that the toolbox approach will work before full implementation

#[cfg(test)]
mod tests {
    use agentai::{Agent, tool::{toolbox, Tool, ToolBox, ToolError, ToolResult}};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Test 1 & 2: Basic toolbox with simple data
    struct TestToolBox {
        data: String,
    }

    #[toolbox]
    impl TestToolBox {
        #[tool]
        /// Get the stored data
        async fn get_data(&self) -> ToolResult {
            println!("🔧 Tool called: get_data");
            Ok(self.data.clone())
        }

        #[tool]
        /// Add two numbers together
        async fn add_numbers(
            &self,
            /// First number
            a: i32,
            /// Second number
            b: i32,
        ) -> ToolResult {
            println!("🔧 Tool called: add_numbers({}, {})", a, b);
            Ok((a + b).to_string())
        }
    }

    // Test 3: Simulate our actual ChatState structure
    #[derive(Clone)]
    struct MockChatState {
        pub docs: Arc<Mutex<String>>,
    }

    struct StateAwareToolBox {
        pub state: Arc<MockChatState>,
    }

    #[toolbox]
    impl StateAwareToolBox {
        #[tool]
        /// Search the documentation for a query
        async fn search_docs(
            &self,
            /// The search query
            query: String,
        ) -> ToolResult {
            println!("🔧 Tool called: search_docs({})", query);
            let docs = self.state.docs.lock().await;
            Ok(format!("Found in docs: {} (query: {})", *docs, query))
        }
    }

    #[tokio::test]
    async fn test_basic_toolbox_struct_pattern() {
        println!("\n=== Test 1: Basic toolbox with simple data ===");

        let tools = TestToolBox {
            data: "test_data_123".to_string(),
        };

        let mut agent = Agent::new(
            "You are a test assistant. Use the get_data tool to retrieve the stored data.",
        );

        let result: anyhow::Result<String> = agent
            .run(
                "gpt-4o-mini",
                "Please get the data using the tool",
                Some(&tools),
            )
            .await;

        match result {
            Ok(response) => {
                println!("✅ Agent response: {}", response);
                // Tool should have been called and response should reference the data
                assert!(
                    response.to_lowercase().contains("test")
                        || response.to_lowercase().contains("data"),
                    "Response should mention the data retrieved"
                );
            }
            Err(e) => {
                panic!("❌ Test failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_toolbox_with_parameters() {
        println!("\n=== Test 2: Toolbox with function parameters ===");

        let tools = TestToolBox {
            data: "ignored".to_string(),
        };

        let mut agent =
            Agent::new("You are a math assistant. Use the add_numbers tool to add numbers.");

        let result: anyhow::Result<String> = agent
            .run("gpt-4o-mini", "Add 5 and 3 using the tool", Some(&tools))
            .await;

        match result {
            Ok(response) => {
                println!("✅ Agent response: {}", response);
                // Should contain "8" or "eight" somewhere
                assert!(
                    response.contains("8") || response.to_lowercase().contains("eight"),
                    "Response should contain the result 8"
                );
            }
            Err(e) => {
                panic!("❌ Test failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_toolbox_with_shared_state() {
        println!("\n=== Test 3: Toolbox with Arc<ChatState> (real use case) ===");

        let state = Arc::new(MockChatState {
            docs: Arc::new(Mutex::new("Strudel documentation content".to_string())),
        });

        let tools = StateAwareToolBox {
            state: Arc::clone(&state),
        };

        let mut agent = Agent::new(
            "You are a documentation assistant. Use the search_docs tool to search the documentation."
        );

        let result: anyhow::Result<String> = agent
            .run(
                "gpt-4o-mini",
                "Search the docs for 'pattern' information",
                Some(&tools),
            )
            .await;

        match result {
            Ok(response) => {
                println!("✅ Agent response: {}", response);
                // Response should mention the search or documentation
                assert!(
                    response.to_lowercase().contains("doc")
                        || response.to_lowercase().contains("found"),
                    "Response should mention documentation or search results"
                );
            }
            Err(e) => {
                panic!("❌ Test failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_agent_conversation_persistence() {
        println!("\n=== Test 4: Agent conversation memory ===");

        let tools = TestToolBox {
            data: "persistent_data".to_string(),
        };

        let mut agent =
            Agent::new("You are a memory test assistant. Remember what the user tells you.");

        // First message
        let result1: anyhow::Result<String> = agent
            .run(
                "gpt-4o-mini",
                "My favorite color is blue. Remember this.",
                Some(&tools),
            )
            .await;

        assert!(result1.is_ok(), "First message should succeed");
        println!("✅ First response: {}", result1.unwrap());

        // Second message - should remember context
        let result2: anyhow::Result<String> = agent
            .run("gpt-4o-mini", "What is my favorite color?", Some(&tools))
            .await;

        match result2 {
            Ok(response) => {
                println!("✅ Second response: {}", response);
                assert!(
                    response.to_lowercase().contains("blue"),
                    "Agent should remember the favorite color from first message"
                );
            }
            Err(e) => {
                panic!("❌ Conversation memory test failed: {:?}", e);
            }
        }
    }
}
