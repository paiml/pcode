use super::{Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct BashParams {
    /// Command to execute (can be a full bash command with pipes, etc)
    command: String,
    /// Working directory (optional, defaults to current)
    #[serde(default)]
    cwd: Option<String>,
    /// Environment variables to set
    #[serde(default)]
    env: Option<HashMap<String, String>>,
    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

pub struct BashTool {
    workspace: PathBuf,
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            workspace: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    async fn execute_bash(&self, params: &BashParams) -> Result<Value, ToolError> {
        // Validate working directory is within workspace
        let cwd = if let Some(ref dir) = params.cwd {
            let full_path = self.workspace.join(dir);
            if !full_path.starts_with(&self.workspace) {
                return Err(ToolError::InvalidParams(
                    "Working directory must be within workspace".to_string(),
                ));
            }
            full_path
        } else {
            self.workspace.clone()
        };

        // Build the command
        let mut cmd = Command::new("bash");
        cmd.arg("-c");
        cmd.arg(&params.command);
        cmd.current_dir(&cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Set up environment
        cmd.env_clear();

        // Add safe default environment
        cmd.env("PATH", "/usr/local/bin:/usr/bin:/bin");
        cmd.env(
            "HOME",
            env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
        );
        cmd.env(
            "USER",
            env::var("USER").unwrap_or_else(|_| "pcode".to_string()),
        );
        cmd.env("LANG", "en_US.UTF-8");

        // Add any custom environment variables
        if let Some(ref env_vars) = params.env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // Apply security restrictions
        #[cfg(target_os = "linux")]
        {
            // Prevent loading of LD_PRELOAD libraries
            cmd.env_remove("LD_PRELOAD");
            cmd.env_remove("LD_LIBRARY_PATH");
        }

        let timeout_duration = Duration::from_millis(params.timeout_ms);

        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                Ok(serde_json::json!({
                    "exit_code": output.status.code().unwrap_or(-1),
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string(),
                    "success": output.status.success(),
                    "command": params.command,
                }))
            }
            Ok(Err(e)) => Err(ToolError::Execution(format!("Process error: {}", e))),
            Err(_) => Err(ToolError::Execution(format!(
                "Command timeout after {}ms",
                params.timeout_ms
            ))),
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute bash commands and scripts"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: BashParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!("Executing bash command: {}", params.command);

        // Check for dangerous commands
        let dangerous_patterns = [
            "rm -rf /",
            "dd if=/dev/zero",
            ":(){ :|:& };:", // Fork bomb
            "> /dev/sda",
            "mkfs",
            "fdisk",
        ];

        let cmd_lower = params.command.to_lowercase();
        for pattern in &dangerous_patterns {
            if cmd_lower.contains(pattern) {
                return Err(ToolError::PermissionDenied(format!(
                    "Dangerous command pattern detected: {}",
                    pattern
                )));
            }
        }

        self.execute_bash(&params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_tool() {
        let tool = BashTool::new();

        // Test simple command
        let params = serde_json::json!({
            "command": "echo 'Hello, World!'"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_bash_tool_pipe() {
        let tool = BashTool::new();

        // Test command with pipe
        let params = serde_json::json!({
            "command": "echo 'line1\nline2\nline3' | grep line2"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"].as_str().unwrap().trim(), "line2");
    }

    #[tokio::test]
    async fn test_dangerous_command() {
        let tool = BashTool::new();

        let params = serde_json::json!({
            "command": "rm -rf /"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
