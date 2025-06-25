pub mod protocol;
pub mod transport;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error};

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[async_trait]
pub trait McpHandler: Send + Sync {
    async fn handle_request(&self, request: McpRequest) -> McpResponse;
}

pub struct McpProtocol {
    handlers: HashMap<String, Box<dyn McpHandler>>,
}

impl Default for McpProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl McpProtocol {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler(&mut self, method: String, handler: Box<dyn McpHandler>) {
        debug!("Registering handler for method: {}", method);
        self.handlers.insert(method, handler);
    }

    pub async fn process_request(&self, request: McpRequest) -> McpResponse {
        debug!("Processing MCP request: {:?}", request);

        if let Some(handler) = self.handlers.get(&request.method) {
            handler.handle_request(request).await
        } else {
            error!("No handler found for method: {}", request.method);
            McpResponse {
                id: request.id,
                result: None,
                error: Some(format!("Tool not found: {}", request.method)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait]
    impl McpHandler for TestHandler {
        async fn handle_request(&self, request: McpRequest) -> McpResponse {
            McpResponse {
                id: request.id,
                result: Some(serde_json::json!({"echo": request.params})),
                error: None,
            }
        }
    }

    #[tokio::test]
    async fn test_mcp_protocol() {
        let mut protocol = McpProtocol::new();
        protocol.register_handler("test".to_string(), Box::new(TestHandler));

        let request = McpRequest {
            id: "1".to_string(),
            method: "test".to_string(),
            params: serde_json::json!({"message": "hello"}),
        };

        let response = protocol.process_request(request).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
