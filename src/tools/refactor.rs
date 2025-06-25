use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
struct RefactorParams {
    path: String,
    #[serde(default)]
    auto_apply: bool,
    #[serde(default)]
    max_complexity: Option<u32>,
    #[serde(default)]
    focus: Option<String>, // "complexity", "coverage", "debt", "all"
}

#[derive(Debug)]
pub struct RefactorTool {
    #[allow(dead_code)]
    workspace: PathBuf,
}

impl RefactorTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    async fn analyze_with_pmat(&self, path: &str) -> Result<Value, ToolError> {
        // Use the PMAT tool to analyze code
        let pmat_tool = crate::tools::pmat::PmatTool::new();
        let params = json!({
            "command": "complexity",
            "path": path,
            "args": []
        });

        pmat_tool.execute(params).await
    }

    async fn generate_refactoring_suggestions(
        &self,
        analysis: &Value,
        focus: &str,
    ) -> Result<Vec<RefactoringSuggestion>, ToolError> {
        let mut suggestions = Vec::new();

        // Extract high complexity functions
        if focus == "complexity" || focus == "all" {
            if let Some(violations) = analysis["violations"].as_array() {
                for violation in violations {
                    if let (Some(file), Some(function), Some(line), Some(value)) = (
                        violation["file"].as_str(),
                        violation["function"].as_str(),
                        violation["line"].as_u64(),
                        violation["value"].as_u64(),
                    ) {
                        suggestions.push(RefactoringSuggestion {
                            file: file.to_string(),
                            function: function.to_string(),
                            line,
                            issue_type: "high_complexity".to_string(),
                            severity: if value > 30 { "high" } else { "medium" }.to_string(),
                            description: format!(
                                "Function '{}' has complexity of {}, exceeding recommended threshold",
                                function, value
                            ),
                            suggested_fix: self.generate_complexity_fix(function, value as u32),
                        });
                    }
                }
            }
        }

        // Add more analysis types (coverage, debt) as needed
        Ok(suggestions)
    }

    fn generate_complexity_fix(&self, function: &str, complexity: u32) -> String {
        // Generate refactoring suggestions based on complexity patterns
        let mut suggestions = Vec::new();

        if complexity > 20 {
            suggestions.push("Extract helper methods for nested conditionals");
            suggestions.push("Consider using early returns to reduce nesting");
            suggestions.push("Replace complex switch/if chains with lookup tables or polymorphism");
        }

        if complexity > 30 {
            suggestions.push("Split this function into multiple smaller functions");
            suggestions.push("Consider extracting a class or module for this functionality");
        }

        format!(
            "Refactoring suggestions for '{}':\n- {}",
            function,
            suggestions.join("\n- ")
        )
    }

    async fn apply_ai_refactoring(
        &self,
        suggestion: &RefactoringSuggestion,
    ) -> Result<String, ToolError> {
        // Check if AI is available
        let config = crate::config::Config::from_env();
        if !config.has_api_key() {
            return Ok(format!(
                "AI refactoring not available (no API key). Manual suggestion:\n{}",
                suggestion.suggested_fix
            ));
        }

        // Read the file content
        let file_tool = crate::tools::file::FileReadTool;
        let read_params = json!({
            "path": suggestion.file,
            "offset": suggestion.line.saturating_sub(10) as i64,
            "limit": 50
        });

        let file_content = file_tool.execute(read_params).await?;
        let content = file_content["content"].as_str().unwrap_or("");

        // Generate AI prompt
        let prompt = format!(
            "Analyze this code and provide a refactored version that reduces complexity:\n\n\
            File: {}\n\
            Function: {}\n\
            Issue: {}\n\
            Current complexity: {}\n\n\
            Code context:\n{}\n\n\
            Please provide:\n\
            1. The refactored code\n\
            2. Brief explanation of changes\n\
            3. Estimated new complexity\n\n\
            Keep the same functionality but improve readability and reduce complexity.",
            suggestion.file,
            suggestion.function,
            suggestion.description,
            suggestion.issue_type,
            content
        );

        // Call LLM
        let llm_tool = crate::tools::llm::LlmTool::new();
        let llm_params = json!({
            "prompt": prompt,
            "max_tokens": 1000,
            "temperature": 0.3
        });

        match llm_tool.execute(llm_params).await {
            Ok(response) => {
                let ai_suggestion = response["response"].as_str().unwrap_or("");
                Ok(format!(
                    "AI-Powered Refactoring Suggestion:\n\n{}\n\n\
                    Original suggestion: {}",
                    ai_suggestion, suggestion.suggested_fix
                ))
            }
            Err(e) => {
                warn!("AI refactoring failed: {}", e);
                Ok(format!(
                    "AI refactoring failed. Manual suggestion:\n{}",
                    suggestion.suggested_fix
                ))
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct RefactoringSuggestion {
    file: String,
    function: String,
    line: u64,
    issue_type: String,
    severity: String,
    description: String,
    suggested_fix: String,
}

impl Default for RefactorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for RefactorTool {
    fn name(&self) -> &str {
        "refactor"
    }

    fn description(&self) -> &str {
        "AI-powered code refactoring based on PMAT analysis"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: RefactorParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!("Running AI-powered refactoring analysis on {}", params.path);

        // Step 1: Analyze with PMAT
        let analysis = self.analyze_with_pmat(&params.path).await?;
        debug!("PMAT analysis complete");

        // Step 2: Generate refactoring suggestions
        let focus = params.focus.as_deref().unwrap_or("all");
        let suggestions = self
            .generate_refactoring_suggestions(&analysis, focus)
            .await?;

        if suggestions.is_empty() {
            return Ok(json!({
                "status": "success",
                "message": "No refactoring needed! Code meets quality standards.",
                "analysis": analysis
            }));
        }

        // Step 3: Apply AI to enhance suggestions
        let mut enhanced_suggestions = Vec::new();
        for suggestion in &suggestions {
            let enhanced = self.apply_ai_refactoring(suggestion).await?;
            enhanced_suggestions.push(json!({
                "file": suggestion.file,
                "function": suggestion.function,
                "line": suggestion.line,
                "issue": suggestion.description,
                "severity": suggestion.severity,
                "suggestion": enhanced
            }));
        }

        // Step 4: Optionally auto-apply (not implemented yet)
        if params.auto_apply {
            warn!("Auto-apply not yet implemented. Showing suggestions only.");
        }

        Ok(json!({
            "status": "success",
            "suggestions_count": enhanced_suggestions.len(),
            "suggestions": enhanced_suggestions,
            "original_analysis": analysis,
            "message": format!(
                "Found {} refactoring opportunities. {}",
                enhanced_suggestions.len(),
                if params.auto_apply {
                    "Auto-apply not yet implemented."
                } else {
                    "Review suggestions above."
                }
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_tool_creation() {
        let tool = RefactorTool::new();
        assert_eq!(tool.name(), "refactor");
        assert_eq!(
            tool.description(),
            "AI-powered code refactoring based on PMAT analysis"
        );
    }

    #[test]
    fn test_complexity_fix_generation() {
        let tool = RefactorTool::new();

        let fix = tool.generate_complexity_fix("complex_function", 25);
        assert!(fix.contains("Extract helper methods"));
        assert!(fix.contains("early returns"));

        let fix = tool.generate_complexity_fix("very_complex_function", 35);
        assert!(fix.contains("Split this function"));
        assert!(fix.contains("multiple smaller functions"));
    }

    #[test]
    fn test_refactoring_suggestion_creation() {
        let suggestion = RefactoringSuggestion {
            file: "test.rs".to_string(),
            function: "complex_fn".to_string(),
            line: 42,
            issue_type: "high_complexity".to_string(),
            severity: "high".to_string(),
            description: "Complexity exceeds threshold".to_string(),
            suggested_fix: "Extract methods".to_string(),
        };

        assert_eq!(suggestion.file, "test.rs");
        assert_eq!(suggestion.line, 42);
        assert_eq!(suggestion.severity, "high");
    }
}
