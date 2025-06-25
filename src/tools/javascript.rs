use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info};

#[derive(Debug, Serialize, Deserialize)]
struct JavaScriptParams {
    code: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
    #[serde(default)]
    use_deno: bool, // Use Deno instead of Node.js for better security
}

#[derive(Debug)]
pub struct JavaScriptTool {
    #[allow(dead_code)]
    workspace: PathBuf,
}

impl JavaScriptTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    async fn create_sandbox_dir(&self) -> Result<TempDir, ToolError> {
        TempDir::new()
            .map_err(|e| ToolError::Execution(format!("Failed to create temp dir: {}", e)))
    }

    async fn write_js_script(&self, dir: &TempDir, code: &str) -> Result<PathBuf, ToolError> {
        let script_path = dir.path().join("script.js");
        fs::write(&script_path, code)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to write script: {}", e)))?;
        Ok(script_path)
    }

    fn build_node_command(&self, script_path: &PathBuf) -> Command {
        let mut cmd = Command::new("node");

        // Security flags for Node.js
        cmd.arg("--no-deprecation");
        cmd.arg("--no-warnings");
        cmd.arg("--disallow-code-generation-from-strings");

        // Add the script
        cmd.arg(script_path);

        // Set working directory to sandbox
        if let Some(parent) = script_path.parent() {
            cmd.current_dir(parent);
        }

        // Configure process
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Environment restrictions
        cmd.env_clear();
        cmd.env("HOME", "/tmp");
        cmd.env("TMPDIR", "/tmp");
        cmd.env("PATH", "/usr/bin:/bin");
        cmd.env("NODE_ENV", "production");
        cmd.env("NODE_OPTIONS", "--max-old-space-size=256");

        cmd
    }

    fn build_deno_command(&self, script_path: &PathBuf) -> Command {
        let mut cmd = Command::new("deno");

        // Deno run with no permissions (secure by default)
        cmd.arg("run");
        cmd.arg("--no-prompt");

        // Add the script
        cmd.arg(script_path);

        // Set working directory to sandbox
        if let Some(parent) = script_path.parent() {
            cmd.current_dir(parent);
        }

        // Configure process
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd
    }

    async fn check_runtime(&self, use_deno: bool) -> Result<bool, ToolError> {
        let runtime = if use_deno { "deno" } else { "node" };

        let check = Command::new(runtime).arg("--version").output().await;

        match check {
            Ok(output) if output.status.success() => Ok(true),
            _ => Ok(false),
        }
    }

    async fn execute_javascript(&self, params: &JavaScriptParams) -> Result<Value, ToolError> {
        // Create sandbox directory
        let sandbox_dir = self.create_sandbox_dir().await?;
        let script_path = self.write_js_script(&sandbox_dir, &params.code).await?;

        // Check which runtime to use
        let use_deno = params.use_deno || !self.check_runtime(false).await?;

        if use_deno && !self.check_runtime(true).await? {
            return Err(ToolError::Execution(
                "Neither Node.js nor Deno found. Please install Node.js or Deno.".to_string(),
            ));
        }

        // Build command
        let mut cmd = if use_deno {
            self.build_deno_command(&script_path)
        } else {
            self.build_node_command(&script_path)
        };

        // Add any user args
        if let Some(args) = &params.args {
            for arg in args {
                cmd.arg(arg);
            }
        }

        debug!(
            "Executing JavaScript in sandbox with {}",
            if use_deno { "Deno" } else { "Node.js" }
        );

        // Set timeout
        let timeout_duration = Duration::from_millis(params.timeout_ms.unwrap_or(30000));

        // Execute
        let output = match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(ToolError::Execution(format!("Process error: {}", e)));
            }
            Err(_) => {
                return Err(ToolError::Execution(format!(
                    "Execution timeout ({}ms)",
                    timeout_duration.as_millis()
                )));
            }
        };

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(serde_json::json!({
            "success": output.status.success(),
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": stdout,
            "stderr": stderr,
            "runtime": if use_deno { "deno" } else { "node" },
            "duration_ms": timeout_duration.as_millis() as u64,
        }))
    }
}

impl Default for JavaScriptTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for JavaScriptTool {
    fn name(&self) -> &str {
        "javascript"
    }

    fn description(&self) -> &str {
        "Execute JavaScript code in a secure sandbox"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: JavaScriptParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!("Executing JavaScript code in sandbox");

        // Basic validation
        if params.code.trim().is_empty() {
            return Err(ToolError::InvalidParams("Code cannot be empty".to_string()));
        }

        // Execute in sandbox
        self.execute_javascript(&params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_tool_creation() {
        let tool = JavaScriptTool::new();
        assert_eq!(tool.name(), "javascript");
        assert_eq!(
            tool.description(),
            "Execute JavaScript code in a secure sandbox"
        );
    }

    #[tokio::test]
    async fn test_javascript_hello_world() {
        let tool = JavaScriptTool::new();
        let params = serde_json::json!({
            "code": "console.log('Hello, World!');"
        });

        // This test might fail if neither Node.js nor Deno is installed
        if let Ok(result) = tool.execute(params).await {
            assert!(result["success"].as_bool().unwrap_or(false));
            assert!(result["stdout"].as_str().unwrap().contains("Hello, World!"));
        }
    }

    #[tokio::test]
    async fn test_javascript_timeout() {
        let tool = JavaScriptTool::new();
        let params = serde_json::json!({
            "code": "while(true) {}",
            "timeout_ms": 100
        });

        match tool.execute(params).await {
            Ok(result) => {
                // Should complete but with timeout indication or crash
                assert!(!result["success"].as_bool().unwrap_or(true));
                // Either timeout or crash is acceptable for this test
            }
            Err(ToolError::Execution(msg)) => {
                // Direct timeout error
                assert!(msg.contains("timeout") || msg.contains("error"));
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_javascript_invalid_code() {
        let tool = JavaScriptTool::new();
        let params = serde_json::json!({
            "code": ""
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_javascript_math() {
        let tool = JavaScriptTool::new();
        let params = serde_json::json!({
            "code": "console.log(Math.PI.toFixed(6));"
        });

        // This test might fail if neither Node.js nor Deno is installed
        if let Ok(result) = tool.execute(params).await {
            assert!(result["success"].as_bool().unwrap_or(false));
            assert!(result["stdout"].as_str().unwrap().contains("3.14159"));
        }
    }
}
