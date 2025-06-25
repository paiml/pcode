use super::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};

#[derive(Debug, Serialize, Deserialize)]
struct ProcessParams {
    command: String,
    args: Option<Vec<String>>,
    cwd: Option<String>,
    timeout_ms: Option<u64>,
}

pub struct ProcessTool;

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "Execute a system process"
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let params: ProcessParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        debug!("Executing process: {} {:?}", params.command, params.args);

        let mut cmd = Command::new(&params.command);

        if let Some(args) = &params.args {
            cmd.args(args);
        }

        if let Some(cwd) = &params.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let timeout_duration = Duration::from_millis(params.timeout_ms.unwrap_or(30000));

        let result = match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                serde_json::json!({
                    "exit_code": output.status.code().unwrap_or(-1),
                    "stdout": stdout,
                    "stderr": stderr,
                    "success": output.status.success()
                })
            }
            Ok(Err(e)) => {
                return Err(ToolError::Execution(format!(
                    "Process execution failed: {}",
                    e
                )));
            }
            Err(_) => {
                warn!("Process execution timed out");
                return Err(ToolError::Execution(
                    "Process execution timed out".to_string(),
                ));
            }
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_execution() {
        let tool = ProcessTool;

        // Test echo command
        let params = serde_json::json!({
            "command": "echo",
            "args": ["Hello, world!"]
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result["success"], true);
        assert!(result["stdout"].as_str().unwrap().contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_process_timeout() {
        let tool = ProcessTool;

        // Test timeout
        let params = serde_json::json!({
            "command": "sleep",
            "args": ["10"],
            "timeout_ms": 100
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
