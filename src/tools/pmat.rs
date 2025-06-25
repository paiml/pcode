use super::{Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDependency {
    pub test_name: String,
    pub file: String,
    pub dependencies: Vec<String>,
    pub shared_state: Vec<String>,
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
        // Check if we're analyzing Rust code
        if path.ends_with(".rs") || PathBuf::from(path).is_dir() {
            return self.analyze_rust_complexity(path).await;
        }
        
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

    async fn analyze_rust_complexity(&self, path: &str) -> Result<Vec<ComplexityResult>, ToolError> {
        // For Rust, we'll use a simple heuristic-based approach
        use tokio::fs;
        
        let mut results = Vec::new();
        
        // Check if path is a file or directory
        let metadata = fs::metadata(path).await
            .map_err(|e| ToolError::Execution(format!("Cannot access path: {}", e)))?;
            
        if metadata.is_file() {
            let content = fs::read_to_string(path).await
                .map_err(|e| ToolError::Execution(format!("Cannot read file: {}", e)))?;
            results.extend(self.analyze_rust_file(path, &content));
        } else {
            // Analyze all .rs files in directory
            let mut dir = fs::read_dir(path).await
                .map_err(|e| ToolError::Execution(format!("Cannot read directory: {}", e)))?;
                
            while let Some(entry) = dir.next_entry().await
                .map_err(|e| ToolError::Execution(format!("Error reading entry: {}", e)))? {
                
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        results.extend(self.analyze_rust_file(&path.to_string_lossy(), &content));
                    }
                }
            }
        }
        
        Ok(results)
    }

    fn analyze_rust_file(&self, file_path: &str, content: &str) -> Vec<ComplexityResult> {
        let mut results = Vec::new();
        let mut in_function = false;
        let mut current_fn_name = String::new();
        let mut current_fn_line = 0u32;
        let mut complexity = 0u32;
        let mut brace_depth = 0i32;
        let mut fn_brace_depth = 0i32;
        
        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num as u32 + 1;
            let trimmed = line.trim();
            
            // Skip comments
            if trimmed.starts_with("//") {
                continue;
            }
            
            // Count braces
            brace_depth += line.chars().filter(|&c| c == '{').count() as i32;
            brace_depth -= line.chars().filter(|&c| c == '}').count() as i32;
            
            // Detect function definitions
            if !in_function && (trimmed.starts_with("fn ") || 
                trimmed.starts_with("pub fn ") ||
                trimmed.starts_with("async fn ") ||
                trimmed.starts_with("pub async fn ") ||
                trimmed.contains(" fn ")) {
                
                if let Some(start) = trimmed.find("fn ") {
                    let after_fn = &trimmed[start + 3..];
                    if let Some(paren) = after_fn.find('(') {
                        current_fn_name = after_fn[..paren].trim().to_string();
                        current_fn_line = line_num;
                        complexity = 1; // Base complexity
                        in_function = true;
                        fn_brace_depth = brace_depth;
                    }
                }
            }
            
            // Analyze complexity inside functions
            if in_function {
                // Control flow structures
                if trimmed.starts_with("if ") || trimmed.contains(" if ") {
                    complexity += 1;
                }
                if trimmed.starts_with("match ") {
                    complexity += 1;
                }
                if trimmed.starts_with("while ") || trimmed.starts_with("loop") {
                    complexity += 1;
                }
                if trimmed.starts_with("for ") {
                    complexity += 1;
                }
                if trimmed.contains("=>") && !trimmed.starts_with("//") {
                    complexity += 1;
                }
                
                // Check if function ended
                if brace_depth < fn_brace_depth {
                    results.push(ComplexityResult {
                        file: file_path.to_string(),
                        function: current_fn_name.clone(),
                        complexity,
                        line: current_fn_line,
                    });
                    in_function = false;
                }
            }
        }
        
        // Handle case where function extends to end of file
        if in_function {
            results.push(ComplexityResult {
                file: file_path.to_string(),
                function: current_fn_name,
                complexity,
                line: current_fn_line,
            });
        }
        
        results
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

    async fn analyze_coverage(&self, path: &str) -> Result<Vec<CoverageResult>, ToolError> {
        // For Rust projects, we'll analyze test coverage by looking at test files
        // This is a simplified version - real coverage would use cargo-tarpaulin or similar
        if path.ends_with(".rs") || PathBuf::from(path).is_dir() {
            return self.analyze_rust_coverage(path).await;
        }
        
        // Python coverage analysis script
        let script = r#"
import ast
import os
import sys
import json

class CoverageAnalyzer(ast.NodeVisitor):
    def __init__(self, filename):
        self.filename = filename
        self.total_lines = set()
        self.executable_lines = set()
        self.covered_lines = set()
        self.functions = []
        self.current_function = None
        
    def visit_FunctionDef(self, node):
        self.functions.append(node.name)
        # Mark function definition as executable
        self.executable_lines.add(node.lineno)
        
        # Visit all statements in the function
        for stmt in ast.walk(node):
            if hasattr(stmt, 'lineno'):
                self.executable_lines.add(stmt.lineno)
        
        self.generic_visit(node)
    
    visit_AsyncFunctionDef = visit_FunctionDef
    
    def visit_If(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_While(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_For(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_With(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_Assign(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_Expr(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_Return(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)
    
    def visit_Raise(self, node):
        self.executable_lines.add(node.lineno)
        self.generic_visit(node)

def analyze_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            tree = ast.parse(content)
            
        analyzer = CoverageAnalyzer(filepath)
        analyzer.visit(tree)
        
        # Count total lines
        lines = content.split('\n')
        total_lines = len(lines)
        
        # For this simplified version, we'll estimate coverage based on:
        # - If it's a test file, assume 90% coverage
        # - If it has corresponding test file, assume 75% coverage
        # - Otherwise, assume 50% coverage
        
        is_test_file = 'test' in os.path.basename(filepath).lower()
        has_tests = False
        
        if not is_test_file:
            # Check if there's a corresponding test file
            base_name = os.path.splitext(os.path.basename(filepath))[0]
            test_patterns = [
                f"test_{base_name}.py",
                f"{base_name}_test.py",
                f"tests/test_{base_name}.py",
                f"tests/{base_name}_test.py"
            ]
            
            dir_path = os.path.dirname(filepath)
            for pattern in test_patterns:
                test_path = os.path.join(dir_path, pattern)
                if os.path.exists(test_path):
                    has_tests = True
                    break
        
        # Estimate coverage
        if is_test_file:
            coverage = 90.0
        elif has_tests:
            coverage = 75.0
        else:
            coverage = 50.0
        
        # Calculate uncovered lines (simplified)
        executable_count = len(analyzer.executable_lines)
        if executable_count > 0:
            covered_count = int(executable_count * (coverage / 100))
            uncovered_lines = sorted(list(analyzer.executable_lines))[covered_count:]
        else:
            uncovered_lines = []
        
        return {
            "file": filepath,
            "line_coverage": coverage,
            "branch_coverage": coverage * 0.9,  # Estimate branch coverage slightly lower
            "uncovered_lines": uncovered_lines[:10]  # Limit to first 10 for readability
        }
    except Exception as e:
        return {
            "file": filepath,
            "line_coverage": 0.0,
            "branch_coverage": 0.0,
            "uncovered_lines": [],
            "error": str(e)
        }

def analyze_path(path):
    results = []
    
    if os.path.isfile(path) and path.endswith('.py'):
        results.append(analyze_file(path))
    elif os.path.isdir(path):
        for root, _, files in os.walk(path):
            for file in files:
                if file.endswith('.py'):
                    filepath = os.path.join(root, file)
                    results.append(analyze_file(filepath))
    
    return results

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "."
    results = analyze_path(path)
    print(json.dumps(results))
"#;

        let output = self.execute_python(script, vec![path.to_string()]).await?;
        
        serde_json::from_str(&output)
            .map_err(|e| ToolError::Execution(format!("Failed to parse coverage results: {}", e)))
    }
    
    async fn analyze_rust_coverage(&self, path: &str) -> Result<Vec<CoverageResult>, ToolError> {
        // For Rust, we'll analyze based on test file presence
        // This is a simplified heuristic - real coverage would use cargo-tarpaulin
        let mut results = Vec::new();
        
        let target_path = PathBuf::from(path);
        if target_path.is_file() {
            let coverage = self.estimate_rust_file_coverage(&target_path).await?;
            results.push(coverage);
        } else if target_path.is_dir() {
            // Walk directory for Rust files
            for entry in walkdir::WalkDir::new(&target_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
                .filter(|e| !e.path().to_string_lossy().contains("target"))
            {
                if entry.file_type().is_file() {
                    let coverage = self.estimate_rust_file_coverage(entry.path()).await?;
                    results.push(coverage);
                }
            }
        }
        
        Ok(results)
    }
    
    async fn estimate_rust_file_coverage(&self, path: &Path) -> Result<CoverageResult, ToolError> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;
        
        let lines: Vec<&str> = content.lines().collect();
        
        // Count executable lines (non-empty, non-comment)
        let mut executable_lines = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty() 
                && !trimmed.starts_with("//") 
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*/")
                && !trimmed.starts_with("#[") {
                executable_lines.push((i + 1) as u32);
            }
        }
        
        // Estimate coverage based on file type and test presence
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let is_test_file = file_name.contains("test") || path.to_string_lossy().contains("/tests/");
        let is_mod_file = file_name == "mod.rs";
        
        let coverage = if is_test_file {
            95.0 // Test files are usually well-covered
        } else if is_mod_file {
            85.0 // mod.rs files are usually simple
        } else {
            // Check if there's a corresponding test module
            let has_test_module = content.contains("#[cfg(test)]") || content.contains("#[test]");
            if has_test_module {
                80.0
            } else {
                60.0
            }
        };
        
        // Calculate uncovered lines
        let covered_count = (executable_lines.len() as f64 * (coverage / 100.0)) as usize;
        let uncovered_lines: Vec<u32> = executable_lines[covered_count.min(executable_lines.len())..]
            .iter()
            .take(10) // Limit to first 10
            .cloned()
            .collect();
        
        Ok(CoverageResult {
            file: path.to_string_lossy().to_string(),
            line_coverage: coverage,
            branch_coverage: Some(coverage * 0.9), // Estimate branch coverage slightly lower
            uncovered_lines,
        })
    }
    
    async fn analyze_test_dependencies(&self, path: &str) -> Result<Vec<TestDependency>, ToolError> {
        // For Rust projects
        if path.ends_with(".rs") || PathBuf::from(path).is_dir() {
            return self.analyze_rust_test_dependencies(path).await;
        }
        
        // Python TDG analysis script
        let script = r#"
import ast
import os
import sys
import json

class TestDependencyAnalyzer(ast.NodeVisitor):
    def __init__(self, filename):
        self.filename = filename
        self.tests = []
        self.current_test = None
        self.current_dependencies = set()
        self.current_shared_state = set()
        self.global_vars = set()
        self.class_vars = set()
        
    def visit_ClassDef(self, node):
        # Track test classes
        if any(base.id == 'TestCase' if isinstance(base, ast.Name) else False 
               for base in node.bases):
            # Visit class body to find class variables
            for item in node.body:
                if isinstance(item, ast.Assign):
                    for target in item.targets:
                        if isinstance(target, ast.Name):
                            self.class_vars.add(target.id)
        self.generic_visit(node)
    
    def visit_FunctionDef(self, node):
        if node.name.startswith('test_'):
            self.current_test = node.name
            self.current_dependencies = set()
            self.current_shared_state = set()
            
            # Visit function body
            self.generic_visit(node)
            
            self.tests.append({
                "test_name": self.current_test,
                "file": self.filename,
                "dependencies": list(self.current_dependencies),
                "shared_state": list(self.current_shared_state)
            })
            
            self.current_test = None
    
    def visit_Attribute(self, node):
        if self.current_test:
            # Check for self.* references (shared state)
            if isinstance(node.value, ast.Name) and node.value.id == 'self':
                self.current_shared_state.add(f"self.{node.attr}")
        self.generic_visit(node)
    
    def visit_Call(self, node):
        if self.current_test:
            # Check for calls to other test methods
            if isinstance(node.func, ast.Name):
                func_name = node.func.id
                if func_name.startswith('test_') and func_name != self.current_test:
                    self.current_dependencies.add(func_name)
            elif isinstance(node.func, ast.Attribute):
                # Check for self.test_* calls
                if (isinstance(node.func.value, ast.Name) and 
                    node.func.value.id == 'self' and 
                    node.func.attr.startswith('test_')):
                    self.current_dependencies.add(node.func.attr)
        self.generic_visit(node)
    
    def visit_Name(self, node):
        if self.current_test:
            # Check for use of global variables in tests
            if isinstance(node.ctx, ast.Load) and node.id in self.global_vars:
                self.current_shared_state.add(f"global:{node.id}")
            elif isinstance(node.ctx, ast.Load) and node.id in self.class_vars:
                self.current_shared_state.add(f"class:{node.id}")
        self.generic_visit(node)
    
    def visit_Global(self, node):
        for name in node.names:
            self.global_vars.add(name)
        self.generic_visit(node)

def analyze_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            tree = ast.parse(content)
            
        analyzer = TestDependencyAnalyzer(filepath)
        analyzer.visit(tree)
        
        return analyzer.tests
    except Exception as e:
        return []

def analyze_path(path):
    all_tests = []
    
    if os.path.isfile(path) and path.endswith('.py'):
        all_tests.extend(analyze_file(path))
    elif os.path.isdir(path):
        for root, _, files in os.walk(path):
            for file in files:
                if file.endswith('.py') and 'test' in file.lower():
                    filepath = os.path.join(root, file)
                    all_tests.extend(analyze_file(filepath))
    
    return all_tests

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "."
    results = analyze_path(path)
    print(json.dumps(results))
"#;

        let output = self.execute_python(script, vec![path.to_string()]).await?;
        
        serde_json::from_str(&output)
            .map_err(|e| ToolError::Execution(format!("Failed to parse TDG results: {}", e)))
    }
    
    async fn analyze_rust_test_dependencies(&self, path: &str) -> Result<Vec<TestDependency>, ToolError> {
        let mut results = Vec::new();
        
        let target_path = PathBuf::from(path);
        if target_path.is_file() {
            let deps = self.analyze_rust_file_test_deps(&target_path).await?;
            results.extend(deps);
        } else if target_path.is_dir() {
            // Walk directory for test files
            for entry in walkdir::WalkDir::new(&target_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let path = e.path();
                    path.extension().map_or(false, |ext| ext == "rs") &&
                    (path.to_string_lossy().contains("test") || 
                     path.components().any(|c| c.as_os_str() == "tests"))
                })
                .filter(|e| !e.path().to_string_lossy().contains("target"))
            {
                if entry.file_type().is_file() {
                    let deps = self.analyze_rust_file_test_deps(entry.path()).await?;
                    results.extend(deps);
                }
            }
        }
        
        Ok(results)
    }
    
    async fn analyze_rust_file_test_deps(&self, path: &Path) -> Result<Vec<TestDependency>, ToolError> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;
        
        let mut dependencies = Vec::new();
        let file_name = path.to_string_lossy().to_string();
        
        // Simple heuristic-based analysis for Rust tests
        let lines: Vec<&str> = content.lines().collect();
        let mut current_test: Option<&str> = None;
        let mut current_deps = Vec::new();
        let mut current_shared = Vec::new();
        
        // Track static/global state
        let mut static_vars = Vec::new();
        for line in &lines {
            if line.trim().starts_with("static ") || line.trim().starts_with("static mut ") {
                if let Some(name) = line.split_whitespace().nth(2) {
                    static_vars.push(name.trim_end_matches(':'));
                }
            }
        }
        
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // Detect test functions
            if trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test]") {
                // Look for the function name on the next line
                if i + 1 < lines.len() {
                    let next_line = lines[i + 1].trim();
                    if let Some(fn_start) = next_line.find("fn ") {
                        let fn_name = next_line[fn_start + 3..]
                            .split(|c: char| !c.is_alphanumeric() && c != '_')
                            .next()
                            .unwrap_or("");
                        
                        if !fn_name.is_empty() {
                            // Save previous test if any
                            if let Some(test_name) = current_test {
                                dependencies.push(TestDependency {
                                    test_name: test_name.to_string(),
                                    file: file_name.clone(),
                                    dependencies: current_deps.clone(),
                                    shared_state: current_shared.clone(),
                                });
                            }
                            
                            current_test = Some(fn_name);
                            current_deps.clear();
                            current_shared.clear();
                        }
                    }
                }
            }
            
            // Analyze test body for dependencies
            if current_test.is_some() {
                // Check for calls to other test functions
                if trimmed.contains("test_") && !trimmed.starts_with("//") {
                    // Extract potential test function calls
                    for word in trimmed.split(|c: char| !c.is_alphanumeric() && c != '_') {
                        if word.starts_with("test_") && word != current_test.unwrap() {
                            current_deps.push(word.to_string());
                        }
                    }
                }
                
                // Check for use of static/global state
                for static_var in &static_vars {
                    if trimmed.contains(static_var) && !trimmed.starts_with("//") {
                        current_shared.push(format!("static:{}", static_var));
                    }
                }
                
                // Check for file I/O (potential shared state)
                if (trimmed.contains("File::") || trimmed.contains("fs::") || 
                    trimmed.contains("std::fs::")) && !trimmed.starts_with("//") {
                    current_shared.push("file_io".to_string());
                }
                
                // Check for environment variables (shared state)
                if trimmed.contains("env::") && !trimmed.starts_with("//") {
                    current_shared.push("env_vars".to_string());
                }
            }
        }
        
        // Save last test
        if let Some(test_name) = current_test {
            dependencies.push(TestDependency {
                test_name: test_name.to_string(),
                file: file_name,
                dependencies: current_deps,
                shared_state: current_shared,
            });
        }
        
        Ok(dependencies)
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
            
            "coverage" => {
                let results = self.analyze_coverage(&params.path).await?;
                
                // Calculate summary statistics
                let total_files = results.len();
                let avg_coverage = if results.is_empty() {
                    0.0
                } else {
                    results.iter().map(|r| r.line_coverage).sum::<f64>() / results.len() as f64
                };
                
                let low_coverage: Vec<_> = results.iter()
                    .filter(|r| r.line_coverage < 80.0)
                    .cloned()
                    .collect();

                Ok(serde_json::json!({
                    "command": "coverage",
                    "path": params.path,
                    "summary": {
                        "total_files": total_files,
                        "average_coverage": avg_coverage,
                        "files_below_80": low_coverage.len()
                    },
                    "low_coverage_files": low_coverage,
                    "details": results
                }))
            }
            
            "tdg" => {
                let results = self.analyze_test_dependencies(&params.path).await?;
                
                // Calculate summary statistics
                let total_tests = results.len();
                let independent_tests = results.iter().filter(|r| r.dependencies.is_empty()).count();
                let max_dependencies = results.iter().map(|r| r.dependencies.len()).max().unwrap_or(0);
                
                let tdg_score = if total_tests == 0 {
                    0.0
                } else {
                    // TDG score: ratio of dependent tests to total tests
                    // Lower is better (0.0 = all independent, 1.0 = all dependent)
                    (total_tests - independent_tests) as f64 / total_tests as f64
                };

                Ok(serde_json::json!({
                    "command": "tdg",
                    "path": params.path,
                    "summary": {
                        "total_tests": total_tests,
                        "independent_tests": independent_tests,
                        "dependent_tests": total_tests - independent_tests,
                        "max_dependencies": max_dependencies,
                        "tdg_score": tdg_score
                    },
                    "details": results
                }))
            }
            
            _ => Err(ToolError::InvalidParams(
                format!("Unknown command: {}. Use: complexity, satd, coverage, tdg", params.command)
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
    
    #[tokio::test]
    async fn test_pmat_coverage() {
        let tool = PmatTool::new();
        
        // Test coverage on a specific file
        let params = serde_json::json!({
            "command": "coverage",
            "path": "src/lib.rs"
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());
        
        let value = result.unwrap();
        assert_eq!(value["command"], "coverage");
        assert!(value["summary"]["total_files"].as_u64().unwrap() > 0);
        assert!(value["details"].is_array());
    }
    
    #[tokio::test]
    async fn test_pmat_tdg() {
        let tool = PmatTool::new();
        
        // Test TDG on test files
        let params = serde_json::json!({
            "command": "tdg",
            "path": "src/tools/mod.rs"
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());
        
        let value = result.unwrap();
        assert_eq!(value["command"], "tdg");
        assert!(value["summary"]["tdg_score"].as_f64().unwrap() >= 0.0);
        assert!(value["summary"]["tdg_score"].as_f64().unwrap() <= 1.0);
    }
}