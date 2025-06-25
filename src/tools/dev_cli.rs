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
struct DevCliParams {
    /// Tool to run (rg, pmat, cargo, etc)
    tool: String,
    /// Arguments to pass to the tool
    args: Vec<String>,
    /// Working directory (optional)
    #[serde(default)]
    cwd: Option<String>,
}

pub struct DevCliTool {
    workspace: PathBuf,
}

impl DevCliTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    fn get_tool_config(&self, tool: &str) -> Result<(&'static str, Vec<&'static str>), ToolError> {
        match tool {
            "rg" | "ripgrep" => Ok(("rg", vec!["--color", "never", "--no-heading"])),
            "fd" => Ok(("fd", vec!["--color", "never"])),
            "cargo" => Ok(("cargo", vec![])),
            "pmat" => Ok(("pmat", vec![])),
            "tokei" => Ok(("tokei", vec![])),
            "git" => Ok(("git", vec![])),
            "make" => Ok(("make", vec![])),
            "pytest" => Ok(("pytest", vec!["-v"])),
            "npm" => Ok(("npm", vec![])),
            "deno" => Ok(("deno", vec![])),
            _ => Err(ToolError::InvalidParams(format!(
                "Unknown tool: {}. Supported: rg, fd, cargo, pmat, tokei, git, make, pytest, npm, deno",
                tool
            ))),
        }
    }
}

impl Default for DevCliTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for DevCliTool {
    fn name(&self) -> &str {
        "dev_cli"
    }

    fn description(&self) -> &str {
        "Run development CLI tools (ripgrep, pmat, cargo, etc)"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: DevCliParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let (tool_binary, default_args) = self.get_tool_config(&params.tool)?;

        info!("Running {} with args: {:?}", tool_binary, params.args);

        // Set working directory
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

        // Build command
        let mut cmd = Command::new(tool_binary);

        // Add default args first
        for arg in default_args {
            cmd.arg(arg);
        }

        // Add user args
        for arg in &params.args {
            cmd.arg(arg);
        }

        cmd.current_dir(&cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        let timeout_duration = Duration::from_secs(30);

        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Parse tool-specific output
                let parsed_output = match params.tool.as_str() {
                    "rg" | "ripgrep" => self.parse_ripgrep_output(&stdout),
                    "cargo" if params.args.first().map(|s| s.as_str()) == Some("clippy") => {
                        self.parse_clippy_output(&stderr)
                    }
                    _ => None,
                };

                Ok(serde_json::json!({
                    "tool": params.tool,
                    "args": params.args,
                    "exit_code": output.status.code().unwrap_or(-1),
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string(),
                    "success": output.status.success(),
                    "parsed": parsed_output,
                }))
            }
            Ok(Err(e)) => Err(ToolError::Execution(format!("Process error: {}", e))),
            Err(_) => Err(ToolError::Execution("Command timeout (30s)".to_string())),
        }
    }
}

impl DevCliTool {
    fn parse_ripgrep_output(&self, output: &str) -> Option<Value> {
        let mut matches = Vec::new();

        for line in output.lines() {
            if let Some((file_line, content)) = line.split_once(':') {
                if let Some((file, line_num)) = file_line.rsplit_once(':') {
                    matches.push(serde_json::json!({
                        "file": file,
                        "line": line_num.parse::<u32>().ok(),
                        "content": content.trim(),
                    }));
                }
            }
        }

        if matches.is_empty() {
            None
        } else {
            Some(serde_json::json!({
                "matches": matches,
                "count": matches.len(),
            }))
        }
    }

    fn parse_warning_line(&self, line: &str) -> Option<(String, String)> {
        if line.starts_with("warning:") || line.starts_with("error:") {
            if let Some((level, rest)) = line.split_once(':') {
                if let Some(msg) = rest.strip_prefix(' ') {
                    return Some((level.to_string(), msg.to_string()));
                }
            }
        }
        None
    }

    fn extract_location(&self, line: &str) -> Option<String> {
        if line.contains("-->") {
            line.split("-->").nth(1).map(|loc| loc.trim().to_string())
        } else {
            None
        }
    }

    fn parse_clippy_output(&self, output: &str) -> Option<Value> {
        let mut warnings = Vec::new();
        let mut current_warning: Option<(String, String)> = None;

        for line in output.lines() {
            if let Some(warning) = self.parse_warning_line(line) {
                current_warning = Some(warning);
            } else if let Some(location) = self.extract_location(line) {
                if let Some((level, msg)) = current_warning.take() {
                    warnings.push(serde_json::json!({
                        "level": level,
                        "message": msg,
                        "location": location,
                    }));
                }
            }
        }

        if warnings.is_empty() {
            None
        } else {
            Some(serde_json::json!({
                "warnings": warnings,
                "count": warnings.len(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_config() {
        let tool = DevCliTool::new();

        assert!(tool.get_tool_config("rg").is_ok());
        assert!(tool.get_tool_config("cargo").is_ok());
        assert!(tool.get_tool_config("unknown").is_err());
    }

    #[tokio::test]
    async fn test_dev_cli_echo() {
        let tool = DevCliTool::new();

        // Most systems should have echo
        let params = serde_json::json!({
            "tool": "echo",
            "args": ["Hello"]
        });

        // This will fail because echo is not in the allowed list
        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
