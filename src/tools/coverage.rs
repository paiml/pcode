use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

#[derive(Debug, Serialize, Deserialize)]
struct CoverageParams {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    exclude_files: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct CoverageTool {
    workspace: PathBuf,
}

impl CoverageTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    async fn run_tarpaulin(&self, params: &CoverageParams) -> Result<String, ToolError> {
        let mut cmd = Command::new("cargo");
        cmd.arg("tarpaulin");

        // Output format
        match params.format.as_deref() {
            Some("json") => {
                cmd.arg("--out");
                cmd.arg("Json");
            }
            Some("html") => {
                cmd.arg("--out");
                cmd.arg("Html");
            }
            Some("lcov") => {
                cmd.arg("--out");
                cmd.arg("Lcov");
            }
            _ => {
                // Default to stdout for text summary
                cmd.arg("--out");
                cmd.arg("Stdout");
            }
        }

        // Exclude files
        if let Some(excludes) = &params.exclude_files {
            cmd.arg("--exclude-files");
            cmd.arg(excludes.join(" "));
        } else {
            // Default exclusions
            cmd.arg("--exclude-files");
            cmd.arg("target/*");
        }

        // Additional useful flags
        cmd.arg("--all-features");
        cmd.arg("--workspace");
        cmd.arg("--timeout");
        cmd.arg("120");

        // Set working directory
        if let Some(path) = &params.path {
            let target_path = self.workspace.join(path);
            cmd.current_dir(target_path);
        } else {
            cmd.current_dir(&self.workspace);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        info!("Running cargo tarpaulin for coverage analysis");
        debug!("Command: {:?}", cmd);

        let timeout_duration = Duration::from_secs(180); // 3 minutes

        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(ToolError::Execution(format!(
                        "Tarpaulin failed: {}",
                        stderr
                    )))
                }
            }
            Ok(Err(e)) => Err(ToolError::Execution(format!("Process error: {}", e))),
            Err(_) => Err(ToolError::Execution(
                "Coverage analysis timeout (180s)".to_string(),
            )),
        }
    }

    fn parse_coverage_output(&self, output: String) -> Result<Value, ToolError> {
        // If it's JSON, parse it directly
        if let Ok(json) = serde_json::from_str::<Value>(&output) {
            return Ok(json);
        }

        // Otherwise parse the text output
        let lines: Vec<&str> = output.lines().collect();
        let mut coverage_line = None;
        let mut files_covered = Vec::new();
        let uncovered_files: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("Coverage Results:") {
                // Next lines contain file coverage
                for j in (i + 1)..lines.len() {
                    let file_line = lines[j];
                    if file_line.trim().is_empty() {
                        break;
                    }
                    if file_line.contains("src/") || file_line.contains("lib.rs") {
                        files_covered.push(file_line.trim().to_string());
                    }
                }
            } else if line.contains("% coverage,") {
                coverage_line = Some(line.to_string());
            }
        }

        // Extract percentage from coverage line
        let coverage_percent = if let Some(line) = &coverage_line {
            line.split('%')
                .next()
                .and_then(|s| s.split_whitespace().last())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0)
        } else {
            0.0
        };

        // Find uncovered lines info
        let uncovered_lines: u64 = coverage_line
            .as_ref()
            .and_then(|line| {
                line.split("uncovered")
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .unwrap_or(0);

        Ok(serde_json::json!({
            "coverage_percent": coverage_percent,
            "uncovered_lines": uncovered_lines,
            "files_analyzed": files_covered.len(),
            "summary": coverage_line.unwrap_or_else(|| "No coverage data found".to_string()),
            "files": files_covered,
            "uncovered_files": uncovered_files
        }))
    }
}

impl Default for CoverageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CoverageTool {
    fn name(&self) -> &str {
        "coverage"
    }

    fn description(&self) -> &str {
        "Run cargo-tarpaulin for real code coverage analysis"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: CoverageParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        // Check if tarpaulin is installed
        let check_cmd = Command::new("cargo")
            .arg("tarpaulin")
            .arg("--version")
            .output()
            .await;

        if check_cmd.is_err() || !check_cmd.unwrap().status.success() {
            return Err(ToolError::Execution(
                "cargo-tarpaulin not installed. Install with: cargo install cargo-tarpaulin"
                    .to_string(),
            ));
        }

        // Run tarpaulin
        let output = self.run_tarpaulin(&params).await?;

        // Parse and return results
        self.parse_coverage_output(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_tool_creation() {
        let tool = CoverageTool::new();
        assert_eq!(tool.name(), "coverage");
        assert_eq!(
            tool.description(),
            "Run cargo-tarpaulin for real code coverage analysis"
        );
    }

    #[test]
    fn test_parse_coverage_output() {
        let tool = CoverageTool::new();
        let sample_output = r#"
Jun 25 12:00:00.000  INFO cargo_tarpaulin: Running Tarpaulin
Jun 25 12:00:00.000  INFO cargo_tarpaulin: Building project
Jun 25 12:00:00.000  INFO cargo_tarpaulin: Launching test
Jun 25 12:00:00.000  INFO cargo_tarpaulin: running 62 tests

Coverage Results:
|| src/chat.rs: 85.2%
|| src/config.rs: 100.0%
|| src/context.rs: 100.0%
|| src/lib.rs: 78.5%
|| src/main.rs: 72.3%

82.45% coverage, 1234/1498 lines covered, 264 uncovered lines
        "#;

        let result = tool
            .parse_coverage_output(sample_output.to_string())
            .unwrap();
        assert_eq!(result["coverage_percent"], 82.45);
        assert_eq!(result["uncovered_lines"], 264);
        assert_eq!(result["files_analyzed"], 5);
    }
}

