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

    async fn call_ai_studio(
        &self,
        api_key: &str,
        params: &LlmParams,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        // Google AI Studio API endpoint for Gemini
        // Using the latest Gemini 2.5 Flash model for best performance
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            api_key
        );

        let request_body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": params.prompt
                }]
            }],
            "generationConfig": {
                "temperature": params.temperature.unwrap_or(0.7),
                "maxOutputTokens": params.max_tokens.unwrap_or(500),
            }
        });

        let response = client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("API error: {}", error_text).into());
        }

        let json: serde_json::Value = response.json().await?;

        // Extract the text from the response
        if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
            Ok(text.to_string())
        } else {
            Err("Failed to parse AI Studio response".into())
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

        let response = if let Some(api_key) = &self.config.ai_studio_api_key {
            debug!("Using AI Studio API");

            // Call Google AI Studio API (Gemini)
            match self.call_ai_studio(api_key, &params).await {
                Ok(response) => response,
                Err(e) => {
                    warn!("AI Studio API call failed: {}", e);
                    return Err(ToolError::Execution(format!("AI Studio API error: {}", e)));
                }
            }
        } else {
            warn!("No AI_STUDIO_API_KEY environment variable found");
            return Err(ToolError::Execution(
                "AI_STUDIO_API_KEY not set. Please set it to use the LLM tool.".to_string(),
            ));
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

        // Test should handle both cases: with and without API key
        match tool.execute(params).await {
            Ok(result) => {
                // If API key is set, verify response
                assert!(result["response"].is_string());
                assert!(result["prompt_tokens"].as_u64().unwrap() > 0);
            }
            Err(ToolError::Execution(msg)) => {
                // If no API key, verify error message
                assert!(msg.contains("AI_STUDIO_API_KEY not set"), "Expected error about API key, got: {}", msg);
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
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
