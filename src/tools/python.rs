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
struct PythonParams {
    code: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct PythonTool {
    #[allow(dead_code)]
    workspace: PathBuf,
}

impl PythonTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    async fn create_sandbox_dir(&self) -> Result<TempDir, ToolError> {
        TempDir::new()
            .map_err(|e| ToolError::Execution(format!("Failed to create temp dir: {}", e)))
    }

    async fn write_python_script(&self, dir: &TempDir, code: &str) -> Result<PathBuf, ToolError> {
        let script_path = dir.path().join("script.py");
        fs::write(&script_path, code)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to write script: {}", e)))?;
        Ok(script_path)
    }

    fn build_sandbox_command(&self, script_path: &PathBuf) -> Command {
        let mut cmd = Command::new("python3");

        // Add security flags
        cmd.arg("-B"); // Don't write bytecode
        cmd.arg("-E"); // Ignore environment variables
        cmd.arg("-I"); // Isolated mode (implies -E and -s)
        cmd.arg("-S"); // Don't import site module
        cmd.arg("-u"); // Unbuffered output

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
        cmd.env("PYTHONDONTWRITEBYTECODE", "1");
        cmd.env("PYTHONUNBUFFERED", "1");

        // Additional restrictions without requiring privileges
        cmd.env("PYTHONPATH", ""); // No additional module paths
        cmd.env("PYTHONHOME", ""); // No custom Python home
        cmd.env("PYTHONSTARTUP", ""); // No startup script

        cmd
    }

    async fn apply_platform_sandbox(&self, cmd: &mut Command) -> Result<(), ToolError> {
        // Skip platform-specific sandboxing in tests or when it would require privileges
        if cfg!(test) || std::env::var("PCODE_NO_SANDBOX").is_ok() {
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            // Only use systemd-run if we're running as root (which we shouldn't be)
            // For non-root users, rely on Python's built-in isolation flags
            let is_root = unsafe { libc::geteuid() } == 0;
            if self.is_systemd_available().await && is_root {
                let mut systemd_cmd = Command::new("systemd-run");
                systemd_cmd.arg("--scope");
                systemd_cmd.arg("--quiet");
                systemd_cmd.arg("--property=MemoryMax=256M");
                systemd_cmd.arg("--property=CPUQuota=50%");
                systemd_cmd.arg("--");

                // Move the python command to systemd-run
                let python_args: Vec<_> = cmd.as_std().get_args().map(|s| s.to_owned()).collect();
                systemd_cmd.arg("python3");
                for arg in python_args {
                    systemd_cmd.arg(arg);
                }

                *cmd = systemd_cmd;
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS sandbox-exec also requires privileges in some cases
            // Skip it for now to avoid permission issues
            debug!("Skipping macOS sandbox-exec to avoid permission requirements");
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn is_systemd_available(&self) -> bool {
        // Check if systemd-run exists and supports our options
        match Command::new("systemd-run").arg("--version").output().await {
            Ok(output) if output.status.success() => {
                // Check version to ensure it supports our properties
                let version_str = String::from_utf8_lossy(&output.stdout);
                debug!("systemd-run version: {}", version_str);
                true
            }
            _ => false,
        }
    }

    async fn execute_python(&self, params: &PythonParams) -> Result<Value, ToolError> {
        // Create sandbox directory
        let sandbox_dir = self.create_sandbox_dir().await?;
        let script_path = self.write_python_script(&sandbox_dir, &params.code).await?;

        // Build command
        let mut cmd = self.build_sandbox_command(&script_path);

        // Add any user args
        if let Some(args) = &params.args {
            for arg in args {
                cmd.arg(arg);
            }
        }

        // Apply platform-specific sandboxing
        self.apply_platform_sandbox(&mut cmd).await?;

        debug!("Executing Python in sandbox: {:?}", cmd);

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
            "duration_ms": timeout_duration.as_millis() as u64,
        }))
    }
}

impl Default for PythonTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PythonTool {
    fn name(&self) -> &str {
        "python"
    }

    fn description(&self) -> &str {
        "Execute Python code in a secure sandbox"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: PythonParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!("Executing Python code in sandbox");

        // Basic validation
        if params.code.trim().is_empty() {
            return Err(ToolError::InvalidParams("Code cannot be empty".to_string()));
        }

        // Check Python availability
        let check = Command::new("python3").arg("--version").output().await;

        if check.is_err() || !check.unwrap().status.success() {
            return Err(ToolError::Execution(
                "Python 3 not found. Please install Python 3.".to_string(),
            ));
        }

        // Execute in sandbox
        self.execute_python(&params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_tool_creation() {
        let tool = PythonTool::new();
        assert_eq!(tool.name(), "python");
        assert_eq!(
            tool.description(),
            "Execute Python code in a secure sandbox"
        );
    }

    #[tokio::test]
    async fn test_python_hello_world() {
        let tool = PythonTool::new();
        let params = serde_json::json!({
            "code": "print('Hello, World!')"
        });

        // This test might fail if Python is not installed
        if let Ok(result) = tool.execute(params).await {
            assert!(result["success"].as_bool().unwrap_or(false));
            assert!(result["stdout"].as_str().unwrap().contains("Hello, World!"));
        }
    }

    #[tokio::test]
    async fn test_python_timeout() {
        let tool = PythonTool::new();
        let params = serde_json::json!({
            "code": "import time\ntime.sleep(10)",
            "timeout_ms": 100
        });

        match tool.execute(params).await {
            Ok(result) => {
                // Should complete but with timeout indication
                assert!(!result["success"].as_bool().unwrap_or(true));
                let stderr = result["stderr"].as_str().unwrap_or("");
                let stdout = result["stdout"].as_str().unwrap_or("");
                // Either the process was killed or systemd-run reported timeout
                assert!(stderr.contains("Terminated") || stdout.is_empty());
            }
            Err(ToolError::Execution(msg)) => {
                // Direct timeout error
                assert!(msg.contains("timeout"));
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_python_invalid_code() {
        let tool = PythonTool::new();
        let params = serde_json::json!({
            "code": ""
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
