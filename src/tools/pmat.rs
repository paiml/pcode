use super::{Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct PmatParams {
    /// Command to run: complexity, satd, tdg, big-o, etc.
    command: String,
    /// Path to analyze (file or directory)
    path: String,
    /// Additional arguments
    #[serde(default)]
    args: Vec<String>,
}

pub struct PmatTool {
    workspace: PathBuf,
}

impl PmatTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    async fn execute_pmat(
        &self,
        command: &str,
        path: &str,
        args: &[String],
    ) -> Result<String, ToolError> {
        let mut cmd = Command::new("pmat");
        cmd.arg("analyze");
        cmd.arg(command);

        // Different commands use different path flags
        match command {
            "complexity" => {
                cmd.arg("--project-path");
                cmd.arg(path);
            }
            _ => {
                cmd.arg("--path");
                cmd.arg(path);
            }
        }

        cmd.arg("--format");
        cmd.arg("json");

        // Add any additional arguments
        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.current_dir(&self.workspace);

        let timeout_duration = Duration::from_secs(60);

        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(ToolError::Execution(format!("PMAT error: {}", stderr)))
                }
            }
            Ok(Err(e)) => Err(ToolError::Execution(format!("Process error: {}", e))),
            Err(_) => Err(ToolError::Execution("PMAT timeout (60s)".to_string())),
        }
    }

    async fn parse_json_output(&self, output: String) -> Result<Value, ToolError> {
        // PMAT outputs JSON by default for most commands
        serde_json::from_str(&output)
            .map_err(|e| ToolError::Execution(format!("Failed to parse PMAT output: {}", e)))
    }
}

impl Default for PmatTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for PmatTool {
    fn name(&self) -> &str {
        "pmat"
    }

    fn description(&self) -> &str {
        "Run PMAT (Pragmatic Metrics for Agile Teams) analysis"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: PmatParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!(
            "Running PMAT {} analysis on {}",
            params.command, params.path
        );

        // Validate path is within workspace
        let target_path = self.workspace.join(&params.path);
        if !target_path.starts_with(&self.workspace) {
            return Err(ToolError::InvalidParams(
                "Path must be within workspace".to_string(),
            ));
        }

        // Execute PMAT command
        let output = self
            .execute_pmat(&params.command, &params.path, &params.args)
            .await?;

        // PMAT outputs JSON when we use --format json
        self.parse_json_output(output).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmat_tool_creation() {
        let tool = PmatTool::new();
        assert_eq!(tool.name(), "pmat");
    }

    #[tokio::test]
    async fn test_pmat_invalid_command() {
        let tool = PmatTool::new();

        // Test with a command that doesn't exist
        let params = serde_json::json!({
            "command": "invalid_command",
            "path": "."
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
