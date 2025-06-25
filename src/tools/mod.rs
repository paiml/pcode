pub mod bash;
pub mod dev_cli;
pub mod file;
pub mod llm;
pub mod pmat;
pub mod process;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool execution error: {0}")]
    Execution(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Tool not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        debug!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    pub async fn execute(&self, request: ToolRequest) -> ToolResponse {
        if let Some(tool) = self.tools.get(&request.tool) {
            match tool.execute(request.params).await {
                Ok(result) => ToolResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                },
                Err(e) => ToolResponse {
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        } else {
            ToolResponse {
                success: false,
                result: None,
                error: Some(format!("Tool '{}' not found", request.tool)),
            }
        }
    }

    pub fn list_tools(&self) -> Vec<(String, String)> {
        self.tools
            .values()
            .map(|tool| (tool.name().to_string(), tool.description().to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }

        fn description(&self) -> &str {
            "Test tool for unit tests"
        }

        async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
            Ok(serde_json::json!({"echo": params}))
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(TestTool));

        let request = ToolRequest {
            tool: "test".to_string(),
            params: serde_json::json!({"message": "hello"}),
        };

        let response = registry.execute(request).await;
        assert!(response.success);
        assert!(response.result.is_some());
    }
}
