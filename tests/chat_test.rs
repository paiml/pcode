use async_trait::async_trait;
use pcode::{
    chat::InteractiveChat,
    tools::{Tool, ToolRegistry},
};
use serde_json::{json, Value};

// Mock tool for testing
struct MockTool;

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        "mock"
    }

    fn description(&self) -> &str {
        "Mock tool for testing"
    }

    async fn execute(&self, params: Value) -> Result<Value, pcode::tools::ToolError> {
        Ok(json!({ "echo": params }))
    }
}

#[test]
fn test_interactive_chat_creation() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MockTool));

    let _chat = InteractiveChat::new(registry);
    // Just verify it creates without panicking
}

#[test]
fn test_chat_with_empty_registry() {
    let registry = ToolRegistry::new();
    let _chat = InteractiveChat::new(registry);
    // Should work with empty registry
}
