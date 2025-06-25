use super::{Tool, ToolError};
use crate::security::SecurityContext;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
struct PmatParams {
    /// Command to run: complexity, coverage, tdg, satd, or all
    command: String,
    /// Path to analyze (file or directory)
    path: String,
    /// Language hint (python, javascript, rust)
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityResult {
    pub file: String,
    pub function: String,
    pub complexity: u32,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageResult {
    pub file: String,
    pub line_coverage: f64,
    pub branch_coverage: Option<f64>,
    pub uncovered_lines: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatdResult {
    pub file: String,
    pub line: u32,
    pub debt_type: String,
    pub message: String,
}

pub struct PmatTool {
    workspace: PathBuf,
    python_path: Option<PathBuf>,
}

impl PmatTool {
    pub fn new() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            python_path: Self::find_python(),
        }
    }

    fn find_python() -> Option<PathBuf> {
        // Try to find Python in common locations
        let candidates = ["python3", "python", "/usr/bin/python3", "/usr/local/bin/python3"];
        
        for candidate in &candidates {
            if let Ok(output) = std::process::Command::new(candidate)
                .arg("--version")
                .output()
            {
                if output.status.success() {
                    debug!("Found Python at: {}", candidate);
                    return Some(PathBuf::from(candidate));
                }
            }
        }
        
        warn!("Python not found in PATH");
        None
    }

    async fn execute_python(&self, script: &str, args: Vec<String>) -> Result<String, ToolError> {
        let python = self.python_path.as_ref()
            .ok_or_else(|| ToolError::Execution("Python not found".to_string()))?;

        // Create a temporary Python script
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join(format!("pmat_{}.py", std::process::id()));
        
        tokio::fs::write(&script_path, script).await
            .map_err(|e| ToolError::Execution(format!("Failed to write script: {}", e)))?;

        // Build the command with security restrictions
        let mut cmd = Command::new(python);
        cmd.arg(&script_path);
        cmd.args(args);
        cmd.current_dir(&self.workspace);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Set environment variables for security
        cmd.env("PYTHONDONTWRITEBYTECODE", "1");
        cmd.env("PYTHONUNBUFFERED", "1");
        
        // Apply platform-specific sandboxing if available
        #[cfg(target_os = "linux")]
        Self::apply_linux_sandbox(&mut cmd);

        let timeout_duration = Duration::from_secs(30);
        
        let result = match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                // Clean up temp file
                let _ = tokio::fs::remove_file(&script_path).await;
                
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout).to_string()
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(ToolError::Execution(format!("Python error: {}", stderr)));
                }
            }
            Ok(Err(e)) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                return Err(ToolError::Execution(format!("Process error: {}", e)));
            }
            Err(_) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                return Err(ToolError::Execution("Script timeout (30s)".to_string()));
            }
        };

        Ok(result)
    }

    #[cfg(target_os = "linux")]
    fn apply_linux_sandbox(cmd: &mut Command) {
        // Note: Full sandboxing would require running as root or using containers
        // For now, we use basic restrictions
        cmd.env("PATH", "/usr/bin:/bin");
        cmd.env_remove("LD_PRELOAD");
        cmd.env_remove("LD_LIBRARY_PATH");
    }

    async fn analyze_complexity(&self, path: &str) -> Result<Vec<ComplexityResult>, ToolError> {
        let script = r#"
import ast
import os
import sys
import json

class ComplexityAnalyzer(ast.NodeVisitor):
    def __init__(self, filename):
        self.filename = filename
        self.results = []
        self.current_function = None
        self.complexity = 0
        self.line = 0

    def visit_FunctionDef(self, node):
        parent_func = self.current_function
        parent_complexity = self.complexity
        
        self.current_function = node.name
        self.complexity = 1  # Base complexity
        self.line = node.lineno
        
        self.generic_visit(node)
        
        self.results.append({
            "file": self.filename,
            "function": self.current_function,
            "complexity": self.complexity,
            "line": self.line
        })
        
        self.current_function = parent_func
        self.complexity = parent_complexity

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_If(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_While(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_For(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_AsyncFor(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_ExceptHandler(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_With(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_AsyncWith(self, node):
        self.complexity += 1
        self.generic_visit(node)

    def visit_BoolOp(self, node):
        self.complexity += len(node.values) - 1
        self.generic_visit(node)

def analyze_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        tree = ast.parse(content, filepath)
        analyzer = ComplexityAnalyzer(filepath)
        analyzer.visit(tree)
        return analyzer.results
    except Exception as e:
        return []

def analyze_path(path):
    all_results = []
    
    if os.path.isfile(path) and path.endswith('.py'):
        all_results.extend(analyze_file(path))
    elif os.path.isdir(path):
        for root, _, files in os.walk(path):
            for file in files:
                if file.endswith('.py'):
                    filepath = os.path.join(root, file)
                    all_results.extend(analyze_file(filepath))
    
    return all_results

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "."
    results = analyze_path(path)
    print(json.dumps(results))
"#;

        let output = self.execute_python(script, vec![path.to_string()]).await?;
        
        serde_json::from_str(&output)
            .map_err(|e| ToolError::Execution(format!("Failed to parse results: {}", e)))
    }

    async fn detect_satd(&self, path: &str) -> Result<Vec<SatdResult>, ToolError> {
        let script = r#"
import os
import re
import sys
import json

SATD_PATTERNS = [
    (r'\b(TODO|FIXME|HACK|XXX|REFACTOR|OPTIMIZE)\b', 'keyword'),
    (r'(?i)\b(temporary|workaround|quick\s*fix|hard\s*code|magic\s*number)\b', 'pattern'),
    (r'(?i)\b(not\s*sure|don\'t\s*know|need\s*to|should\s*be)\b', 'uncertainty'),
]

def analyze_file(filepath):
    results = []
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for line_num, line in enumerate(lines, 1):
            for pattern, debt_type in SATD_PATTERNS:
                if re.search(pattern, line):
                    results.append({
                        "file": filepath,
                        "line": line_num,
                        "debt_type": debt_type,
                        "message": line.strip()
                    })
                    break
    except Exception:
        pass
    
    return results

def analyze_path(path):
    all_results = []
    
    if os.path.isfile(path):
        all_results.extend(analyze_file(path))
    elif os.path.isdir(path):
        for root, _, files in os.walk(path):
            for file in files:
                if file.endswith(('.py', '.js', '.rs', '.java', '.cpp', '.c')):
                    filepath = os.path.join(root, file)
                    all_results.extend(analyze_file(filepath))
    
    return all_results

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "."
    results = analyze_path(path)
    print(json.dumps(results))
"#;

        let output = self.execute_python(script, vec![path.to_string()]).await?;
        
        serde_json::from_str(&output)
            .map_err(|e| ToolError::Execution(format!("Failed to parse results: {}", e)))
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
        let params: PmatParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        info!("Running PMAT {} analysis on {}", params.command, params.path);

        // Validate path is within workspace
        let target_path = self.workspace.join(&params.path);
        if !target_path.starts_with(&self.workspace) {
            return Err(ToolError::InvalidParams("Path must be within workspace".to_string()));
        }

        match params.command.as_str() {
            "complexity" => {
                let results = self.analyze_complexity(&params.path).await?;
                
                // Calculate summary statistics
                let max_complexity = results.iter().map(|r| r.complexity).max().unwrap_or(0);
                let avg_complexity = if results.is_empty() {
                    0.0
                } else {
                    results.iter().map(|r| r.complexity as f64).sum::<f64>() / results.len() as f64
                };
                
                let violations: Vec<_> = results.iter()
                    .filter(|r| r.complexity > 20)
                    .cloned()
                    .collect();

                Ok(serde_json::json!({
                    "command": "complexity",
                    "path": params.path,
                    "summary": {
                        "max_complexity": max_complexity,
                        "average_complexity": avg_complexity,
                        "total_functions": results.len(),
                        "violations": violations.len()
                    },
                    "violations": violations,
                    "details": results
                }))
            }
            
            "satd" => {
                let results = self.detect_satd(&params.path).await?;
                
                let by_type = results.iter().fold(HashMap::new(), |mut acc, r| {
                    *acc.entry(r.debt_type.clone()).or_insert(0) += 1;
                    acc
                });

                Ok(serde_json::json!({
                    "command": "satd",
                    "path": params.path,
                    "summary": {
                        "total_debt_items": results.len(),
                        "by_type": by_type
                    },
                    "items": results
                }))
            }
            
            _ => Err(ToolError::InvalidParams(
                format!("Unknown command: {}. Use: complexity, satd", params.command)
            ))
        }
    }
}

impl Default for PmatTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_python() {
        let python = PmatTool::find_python();
        // Python should be available in most environments
        assert!(python.is_some());
    }

    #[tokio::test]
    async fn test_pmat_tool() {
        let tool = PmatTool::new();
        
        // Test invalid command
        let params = serde_json::json!({
            "command": "invalid",
            "path": "."
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}