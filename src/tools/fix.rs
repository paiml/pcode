use super::{Tool, ToolError};
use crate::tools::pmat::PmatTool;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct FixParams {
    /// Type of fix: complexity, format, lint
    fix_type: String,
    /// Path to file or directory to fix
    path: String,
    /// Dry run - show what would be fixed without changing files
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub file: String,
    pub issue: String,
    pub fixed: bool,
    pub description: String,
}

pub struct FixTool {
    workspace: PathBuf,
    pmat: PmatTool,
}

impl FixTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            pmat: PmatTool::new(),
        }
    }

    async fn fix_complexity(&self, path: &str, dry_run: bool) -> Result<Vec<FixResult>, ToolError> {
        let mut results = Vec::new();

        // First, analyze complexity
        let complexity_params = serde_json::json!({
            "command": "complexity",
            "path": path
        });

        let analysis = self.pmat.execute(complexity_params).await?;

        // Extract violations from PMAT analysis
        if let Some(violations) = analysis.get("violations").and_then(|v| v.as_array()) {
            for violation in violations {
                let file = violation.get("file").and_then(|v| v.as_str()).unwrap_or("");
                let function = violation
                    .get("function")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let message = violation
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let line = violation.get("line").and_then(|v| v.as_u64()).unwrap_or(0);

                results.push(FixResult {
                    file: file.to_string(),
                    issue: message.to_string(),
                    fixed: false,
                    description: if dry_run {
                        format!(
                            "Would refactor function '{}' at line {} to reduce complexity",
                            function, line
                        )
                    } else {
                        format!(
                            "Manual refactoring required for function '{}' at line {}",
                            function, line
                        )
                    },
                });
            }
        }

        Ok(results)
    }

    async fn fix_format(&self, path: &str, dry_run: bool) -> Result<Vec<FixResult>, ToolError> {
        let mut results = Vec::new();

        // For Rust files, use rustfmt
        let target_path = PathBuf::from(path);
        if target_path.is_file() && path.ends_with(".rs") {
            results.push(self.fix_rust_format(path, dry_run).await?);
        } else if target_path.is_dir() {
            // Find all Rust files
            for entry in walkdir::WalkDir::new(&target_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
                .filter(|e| !e.path().to_string_lossy().contains("target"))
            {
                if entry.file_type().is_file() {
                    results.push(
                        self.fix_rust_format(entry.path().to_str().unwrap(), dry_run)
                            .await?,
                    );
                }
            }
        }

        Ok(results)
    }

    async fn fix_rust_format(&self, path: &str, dry_run: bool) -> Result<FixResult, ToolError> {
        use tokio::process::Command;

        let args = if dry_run {
            vec!["--check", path]
        } else {
            vec![path]
        };

        let output = Command::new("rustfmt")
            .args(&args)
            .output()
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to run rustfmt: {}", e)))?;

        let fixed = output.status.success() && !dry_run;
        let description = if dry_run {
            if output.status.success() {
                "File is already formatted correctly".to_string()
            } else {
                "Would format file with rustfmt".to_string()
            }
        } else if output.status.success() {
            "Formatted with rustfmt".to_string()
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("rustfmt failed: {}", stderr)
        };

        Ok(FixResult {
            file: path.to_string(),
            issue: "Code formatting".to_string(),
            fixed,
            description,
        })
    }

    async fn fix_lint(&self, path: &str, dry_run: bool) -> Result<Vec<FixResult>, ToolError> {
        let mut results = Vec::new();

        // For Rust, use clippy with --fix
        if path.ends_with(".rs") || PathBuf::from(path).is_dir() {
            use tokio::process::Command;

            let mut cmd = Command::new("cargo");
            cmd.arg("clippy");

            if !dry_run {
                cmd.arg("--fix");
                cmd.arg("--allow-dirty");
                cmd.arg("--allow-staged");
            }

            if PathBuf::from(path).is_file() {
                // Clippy works on the whole project, not individual files
                cmd.arg("--");
                cmd.arg("-W");
                cmd.arg("clippy::all");
            }

            let output = cmd
                .output()
                .await
                .map_err(|e| ToolError::Execution(format!("Failed to run clippy: {}", e)))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Parse clippy output for warnings/errors
            let mut _fixed_count = 0;
            for line in stderr.lines().chain(stdout.lines()) {
                if line.contains("warning:") || line.contains("error:") {
                    let issue = line
                        .split("warning:")
                        .nth(1)
                        .or_else(|| line.split("error:").nth(1))
                        .unwrap_or("Unknown issue")
                        .trim();

                    if !dry_run && line.contains("fixed") {
                        _fixed_count += 1;
                    }

                    results.push(FixResult {
                        file: path.to_string(),
                        issue: issue.to_string(),
                        fixed: !dry_run && line.contains("fixed"),
                        description: if dry_run {
                            format!("Would fix: {}", issue)
                        } else if line.contains("fixed") {
                            format!("Fixed: {}", issue)
                        } else {
                            format!("Cannot auto-fix: {}", issue)
                        },
                    });
                }
            }

            if results.is_empty() {
                results.push(FixResult {
                    file: path.to_string(),
                    issue: "Linting".to_string(),
                    fixed: false,
                    description: if output.status.success() {
                        "No lint issues found".to_string()
                    } else {
                        "Clippy analysis completed".to_string()
                    },
                });
            }
        }

        Ok(results)
    }
}

impl Default for FixTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FixTool {
    fn name(&self) -> &str {
        "fix"
    }

    fn description(&self) -> &str {
        "Automatically fix code issues (complexity, formatting, linting)"
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let params: FixParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!(
            "Running fix {} on {} (dry_run: {})",
            params.fix_type, params.path, params.dry_run
        );

        // Validate path is within workspace
        let target_path = self.workspace.join(&params.path);
        if !target_path.starts_with(&self.workspace) {
            return Err(ToolError::InvalidParams(
                "Path must be within workspace".to_string(),
            ));
        }

        let results = match params.fix_type.as_str() {
            "complexity" => self.fix_complexity(&params.path, params.dry_run).await?,
            "format" => self.fix_format(&params.path, params.dry_run).await?,
            "lint" => self.fix_lint(&params.path, params.dry_run).await?,
            _ => {
                return Err(ToolError::InvalidParams(format!(
                    "Unknown fix type: {}. Use: complexity, format, lint",
                    params.fix_type
                )));
            }
        };

        let total_fixed = results.iter().filter(|r| r.fixed).count();
        let total_issues = results.len();

        Ok(serde_json::json!({
            "fix_type": params.fix_type,
            "path": params.path,
            "dry_run": params.dry_run,
            "summary": {
                "total_issues": total_issues,
                "fixed": total_fixed,
                "requires_manual": total_issues - total_fixed
            },
            "results": results
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_tool_creation() {
        let tool = FixTool::new();
        assert_eq!(tool.name(), "fix");
    }

    #[tokio::test]
    async fn test_fix_invalid_type() {
        let tool = FixTool::new();

        let params = serde_json::json!({
            "fix_type": "invalid",
            "path": "src/",
            "dry_run": true
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
