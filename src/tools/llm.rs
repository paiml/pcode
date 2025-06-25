use super::{Tool, ToolError};
use crate::{config::Config, token_estimation::Tokenizer};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
struct LlmParams {
    prompt: String,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenEstimateParams {
    text: String,
    fast: Option<bool>,
}

pub struct LlmTool {
    config: Config,
}

impl LlmTool {
    pub fn new() -> Self {
        Self {
            config: Config::from_env(),
        }
    }
}

impl Default for LlmTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LlmTool {
    fn name(&self) -> &str {
        "llm"
    }

    fn description(&self) -> &str {
        "Interact with language model"
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let params: LlmParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        debug!("LLM request with prompt length: {}", params.prompt.len());

        let response = if let Some(_api_key) = &self.config.ai_studio_api_key {
            debug!("Using AI Studio API with key");
            // Here you would make an actual API call to AI Studio
            // For now, we'll still return a mock but indicate API key is present
            format!(
                "[AI Studio API Available] Mock response to: {}",
                params.prompt.chars().take(50).collect::<String>()
            )
        } else {
            warn!("No AI_STUDIO_API_KEY environment variable found, using mock response");
            format!(
                "Mock response to: {}",
                params.prompt.chars().take(50).collect::<String>()
            )
        };

        let tokenizer = Tokenizer::instance();
        let prompt_tokens = tokenizer.estimate_tokens(&params.prompt);
        let response_tokens = tokenizer.estimate_tokens(&response);

        Ok(serde_json::json!({
            "response": response,
            "prompt_tokens": prompt_tokens,
            "response_tokens": response_tokens,
            "total_tokens": prompt_tokens + response_tokens,
            "api_available": self.config.has_api_key()
        }))
    }
}

pub struct TokenEstimateTool;

#[async_trait]
impl Tool for TokenEstimateTool {
    fn name(&self) -> &str {
        "token_estimate"
    }

    fn description(&self) -> &str {
        "Estimate token count for text"
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let params: TokenEstimateParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let tokenizer = Tokenizer::instance();
        let tokens = if params.fast.unwrap_or(false) {
            tokenizer.estimate_tokens_fast(&params.text)
        } else {
            tokenizer.estimate_tokens(&params.text)
        };

        info!(
            "Estimated {} tokens for text of length {}",
            tokens,
            params.text.len()
        );

        Ok(serde_json::json!({
            "tokens": tokens,
            "text_length": params.text.len(),
            "avg_chars_per_token": params.text.len() as f32 / tokens as f32
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_tool() {
        let tool = LlmTool::new();
        let params = serde_json::json!({
            "prompt": "Hello, how are you?",
            "max_tokens": 100
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result["response"].is_string());
        assert!(result["prompt_tokens"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_token_estimate_tool() {
        let tool = TokenEstimateTool;
        let params = serde_json::json!({
            "text": "This is a test text for token estimation",
            "fast": false
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result["tokens"].as_u64().unwrap() > 0);
        assert_eq!(result["text_length"], 40);
    }
}
